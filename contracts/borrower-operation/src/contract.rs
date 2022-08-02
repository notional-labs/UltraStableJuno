
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::state::{AddressesSet, SudoParams, ADDRESSES_SET, SUDO_PARAMS};
use ultra_base::borrower_operations::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg};
use ultra_base::querier::{query_entire_system_coll, query_entire_system_debt};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:active-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const NATIVE_JUNO_DENOM: &str = "ujuno";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store sudo params
    let sudo_params = SudoParams {
        name: msg.name,
        owner: deps.api.addr_validate(&msg.owner)?,
    };

    SUDO_PARAMS.save(deps.storage, &sudo_params)?;

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
        ExecuteMsg::OpenTrove {
            max_fee_percentage,
            ultra_amount,
            upper_hint,
            lower_hint,
        } => execute_open_trove(
            deps,
            env,
            info,
            max_fee_percentage,
            ultra_amount,
            upper_hint,
            lower_hint,
        ),
        ExecuteMsg::AdjustTrove {
            borrower,
            coll_withdrawal,
            ultra_change,
            is_debt_increase,
            max_fee_percentage,
            upper_hint,
            lower_hint,
        } => execute_adjust_trove(
            deps,
            env,
            info,
            borrower,
            coll_withdrawal,
            ultra_change,
            is_debt_increase,
            max_fee_percentage,
            upper_hint,
            lower_hint,
        ),
        ExecuteMsg::CloseTrove {} => execute_close_trove(deps, env, info),
        ExecuteMsg::AddColl {
            upper_hint,
            lower_hint,
        } => execute_add_coll(deps, env, info, upper_hint, lower_hint),
        ExecuteMsg::WithdrawColl {
            coll_amount,
            upper_hint,
            lower_hint,
        } => execute_withdraw_coll(deps, env, info, coll_amount, upper_hint, lower_hint),
        ExecuteMsg::ClaimCollateral {} => execute_claim_collateral(deps, env, info),
        ExecuteMsg::RepayULTRA {
            active_pool_addr,
            ultra_token_addr,
            account,
            ultra_amount,
            upper_hint,
            lower_hint,
        } => execute_repay_ultra(
            deps,
            env,
            info,
            active_pool_addr,
            ultra_token_addr,
            account,
            ultra_amount,
            upper_hint,
            lower_hint,
        ),
        ExecuteMsg::WithdrawULTRA {
            max_fee_percentage,
            ultra_amount,
            upper_hint,
            lower_hint,
        } => execute_withdraw_ultra(
            deps,
            env,
            info,
            max_fee_percentage,
            ultra_amount,
            upper_hint,
            lower_hint,
        ),
        ExecuteMsg::MoveJUNOGainToTrove {
            borrower,
            upper_hint,
            lower_hint,
        } => execute_move_juno_gain_to_trove(deps, env, info, borrower, upper_hint, lower_hint),
        ExecuteMsg::SetAddresses {
            trove_manager_address,
            active_pool_address,
            default_pool_address,
            stability_pool_address,
            coll_surplus_pool_address,
            price_feed_contract_address,
            sorted_troves_address,
            ultra_token_address,
            reward_pool_address,
        } => execute_set_addresses(
            deps,
            env,
            info,
            trove_manager_address,
            active_pool_address,
            default_pool_address,
            stability_pool_address,
            coll_surplus_pool_address,
            price_feed_contract_address,
            sorted_troves_address,
            ultra_token_address,
            reward_pool_address,
        ),
    }
}

pub fn execute_open_trove(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let res = Response::new()
        .add_attribute("action", "open_trove");
    Ok(res)
}

pub fn execute_set_addresses(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    trove_manager_address: String,
    active_pool_address: String,
    default_pool_address: String,
    stability_pool_address: String,
    coll_surplus_pool_address: String,
    price_feed_contract_address: String,
    sorted_troves_address: String,
    ultra_token_contract_address: String,
    reward_pool_address: String,
) -> Result<Response, ContractError> {
    only_owner(deps.storage, &info)?;

    let new_addresses_set = AddressesSet {
        trove_manager_address: deps.api.addr_validate(&trove_manager_address)?,
        stability_pool_address: deps.api.addr_validate(&stability_pool_address)?,
        default_pool_address: deps.api.addr_validate(&default_pool_address)?,
        active_pool_address: deps.api.addr_validate(&active_pool_address)?,
        coll_surplus_pool_address: deps.api.addr_validate(&coll_surplus_pool_address)?,
        ultra_token_contract_address: deps.api.addr_validate(&ultra_token_contract_address)?,
        price_feed_contract_address: deps.api.addr_validate(&price_feed_contract_address)?,
        sorted_troves_address: deps.api.addr_validate(&sorted_troves_address)?,
        reward_pool_address: deps.api.addr_validate(&reward_pool_address)?,
    };

    ADDRESSES_SET.save(deps.storage, &new_addresses_set)?;
    let res = Response::new()
        .add_attribute("action", "set_addresses")
        .add_attribute("trove_manager_address", trove_manager_address)
        .add_attribute("stability_pool_address", stability_pool_address)
        .add_attribute("default_pool_address", default_pool_address)
        .add_attribute("active_pool_address", active_pool_address)
        .add_attribute("coll_surplus_pool_address", coll_surplus_pool_address)
        .add_attribute("ultra_token_contract_address", ultra_token_contract_address)
        .add_attribute("price_feed_contract_address", price_feed_contract_address)
        .add_attribute("sorted_troves_address", sorted_troves_address)
        .add_attribute("reward_pool_address", reward_pool_address);
    Ok(res)
}

/// Checks to enfore only borrower o can call
fn only_borrower(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    return Err(ContractError::CallerIsNotBorrower {});
}
/// Checks to enfore only owner can call
fn only_owner(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let params = SUDO_PARAMS.load(store)?;
    if params.owner != info.sender.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }
    Ok(info.sender.clone())
}

/// 
fn require_trove_is_active(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let params = SUDO_PARAMS.load(store)?;
    if params.owner != info.sender.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }
    Ok(info.sender.clone())
}

/// 
fn require_trove_is_not_active(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let params = SUDO_PARAMS.load(store)?;
    if params.owner != info.sender.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }
    Ok(info.sender.clone())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetParams {} => to_binary(&query_params(deps)?),
        QueryMsg::GetEntireSystemColl {
            active_pool_addr,
            default_pool_addr,
        } => to_binary(&query_entire_system_coll(
            &deps.querier,
            active_pool_addr,
            default_pool_addr,
        )?),
        QueryMsg::GetEntireSystemDebt {
            active_pool_addr,
            default_pool_addr,
        } => to_binary(&query_entire_system_debt(
            &deps.querier,
            active_pool_addr,
            default_pool_addr,
        )?),
        QueryMsg::GetStabilityPoolAddress {} => to_binary(&query_stability_pool_address(deps)?),
        QueryMsg::GetDefaultPoolAddress {} => to_binary(&query_default_pool_address(deps)?),
        QueryMsg::GetActivePoolAddress {} => to_binary(&query_active_pool_address(deps)?),
        QueryMsg::GetCollSurplusPoolAddress {} => {
            to_binary(&query_coll_surplus_pool_address(deps)?)
        }
        QueryMsg::GetULTRATokenContractAddress {} => {
            to_binary(&query_ultra_token_contract_address(deps)?)
        }
        QueryMsg::GetTroveManagerAddress {} => to_binary(&query_trove_manager_address(deps)?),
        QueryMsg::GetPriceFeedContractAddress {} => {
            to_binary(&query_price_feed_contract_address(deps)?)
        }
        QueryMsg::GetSortedTrovesAddress {} => to_binary(&query_sorted_troves_address(deps)?),
        QueryMsg::GetRewardPoolAddress {} => to_binary(&query_reward_pool_address(deps)?),
    }
}

pub fn query_params(deps: Deps) -> StdResult<ParamsResponse> {
    let info = SUDO_PARAMS.load(deps.storage)?;
    let res = ParamsResponse {
        name: info.name,
        owner: info.owner,
    };
    Ok(res)
}

pub fn query_stability_pool_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let stability_pool_address = addresses_set.stability_pool_address;
    Ok(stability_pool_address)
}

pub fn query_default_pool_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let default_pool_address = addresses_set.default_pool_address;
    Ok(default_pool_address)
}

pub fn query_active_pool_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let active_pool_address = addresses_set.active_pool_address;
    Ok(active_pool_address)
}

pub fn query_coll_surplus_pool_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let coll_surplus_pool_address = addresses_set.coll_surplus_pool_address;
    Ok(coll_surplus_pool_address)
}

pub fn query_trove_manager_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let trove_manager_address = addresses_set.trove_manager_address;
    Ok(trove_manager_address)
}

pub fn query_ultra_token_contract_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let ultra_token_contract_address = addresses_set.ultra_token_contract_address;
    Ok(ultra_token_contract_address)
}

pub fn query_price_feed_contract_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let price_feed_contract_address = addresses_set.price_feed_contract_address;
    Ok(price_feed_contract_address)
}

pub fn query_sorted_troves_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let sorted_troves_address = addresses_set.sorted_troves_address;
    Ok(sorted_troves_address)
}

pub fn query_reward_pool_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let reward_pool_address = addresses_set.reward_pool_address;
    Ok(reward_pool_address)
}
