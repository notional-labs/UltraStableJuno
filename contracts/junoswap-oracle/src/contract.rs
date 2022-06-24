use crate::error::ContractError;
use crate::state::{Config, PriceCumulativeLast, CONFIG, PRICE_LAST};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, Uint256,
};
use cw2::set_contract_version;
use usj_base::asset::{AssetInfo, PoolInfo};
use usj_base::oracle::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use usj_base::querier::query_pool_info;

const CONTRACT_NAME: &str = "junoswap-oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Time between two consecutive TWAP updates.
pub const PERIOD: Uint128 = Uint128::new(1200u128);

/// Decimal precision for TWAP results
pub const TWAP_PRECISION: u8 = 6;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let pool_contract_address = deps.api.addr_validate(&msg.pool_contract_address)?;
    let pool_info: PoolInfo = query_pool_info(&deps.querier, pool_contract_address.clone())?;

    let pool_info_clone = pool_info.clone();

    let config = Config {
        owner: info.sender,
        pool_contract_addr: pool_contract_address,
        asset_infos: [pool_info_clone.token1_denom, pool_info_clone.token2_denom],
        pool: pool_info,
    };
    CONFIG.save(deps.storage, &config)?;

    let init_price = PriceCumulativeLast {
        price1_cumulative_last: Uint128::zero(),
        price2_cumulative_last: Uint128::zero(),
        price_1_average: Decimal256::zero(),
        price_2_average: Decimal256::zero(),
        block_timestamp_last: env.block.time.seconds(),
    };
    PRICE_LAST.save(deps.storage, &init_price)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Update {} => update(deps, env),
    }
}

pub fn update(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let pool_info: PoolInfo = query_pool_info(&deps.querier, config.pool_contract_addr)?;

    let price_last = PRICE_LAST.load(deps.storage)?;

    let time_elapsed = Uint128::from(env.block.time.seconds() - price_last.block_timestamp_last);
    let price_precision = Uint128::from(10u128.pow(TWAP_PRECISION.into()));

    // Ensure that at least one full period has passed since the last update
    if time_elapsed < PERIOD {
        return Err(ContractError::WrongPeriod {});
    }

    let x = pool_info.token1_reserve;
    let y = pool_info.token2_reserve;

    let price1_cumulative_new = price_last.price1_cumulative_last.wrapping_add(
        time_elapsed
            .checked_mul(price_precision)
            .map_err(StdError::overflow)?
            .multiply_ratio(y, x),
    );

    let price2_cumulative_new = price_last.price2_cumulative_last.wrapping_add(
        time_elapsed
            .checked_mul(price_precision)
            .map_err(StdError::overflow)?
            .multiply_ratio(x, y),
    );

    let price_1_average = Decimal256::from_ratio(
        Uint256::from(price1_cumulative_new.wrapping_sub(price_last.price1_cumulative_last)),
        time_elapsed,
    );

    let price_2_average = Decimal256::from_ratio(
        Uint256::from(price2_cumulative_new.wrapping_sub(price_last.price2_cumulative_last)),
        time_elapsed,
    );

    let prices = PriceCumulativeLast {
        price1_cumulative_last: price1_cumulative_new,
        price2_cumulative_last: price2_cumulative_new,
        price_1_average,
        price_2_average,
        block_timestamp_last: env.block.time.seconds(),
    };
    PRICE_LAST.save(deps.storage, &prices)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Consult { token, amount } => to_binary(&consult(deps, token, amount)?),
    }
}

/// Multiplies a token amount by its latest TWAP value and returns the result as a [`Uint256`] if the operation was successful
fn consult(deps: Deps, token: AssetInfo, amount: Uint128) -> Result<Uint256, StdError> {
    let config = CONFIG.load(deps.storage)?;
    let price_last = PRICE_LAST.load(deps.storage)?;
    let price_precision = Uint256::from(10_u128.pow(TWAP_PRECISION.into()));

    let price_average = if config.asset_infos[0].equal(&token) {
        price_last.price_1_average
    } else if config.asset_infos[1].equal(&token) {
        price_last.price_2_average
    } else {
        return Err(StdError::generic_err("Invalid Token"));
    };

    Ok(Uint256::from(amount) * price_average / price_precision)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
