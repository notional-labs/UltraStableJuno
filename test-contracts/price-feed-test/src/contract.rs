#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Decimal256, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Price, PRICE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:price-feed-testnet";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // initial price
    let price = Price {
        price: Decimal256::zero(),
    };

    PRICE.save(deps.storage, &price)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetJunoPrice { price } => execute_set_juno_price(deps, env, info, price),
    }
}

pub fn execute_set_juno_price(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    price: Decimal256,
) -> Result<Response, ContractError> {
    let new_price = Price { price };
    PRICE.save(deps.storage, &new_price)?;
    let res = Response::new().add_attribute("action", "set_price");
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetJunoPrice {} => to_binary(&query_juno_price(deps)?),
    }
}

pub fn query_juno_price(deps: Deps) -> StdResult<Decimal256> {
    let info = PRICE.load(deps.storage)?;
    let res = info.price;
    Ok(res)
}
