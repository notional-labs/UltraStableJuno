use std::vec;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg};
use crate::state::{SudoParams, ADDRESSES_SET, ASSETS_IN_POOL, SUDO_PARAMS};

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

    // store contract info
    let data = SudoParams {
        name: msg.name,
        owner: deps.api.addr_validate(&msg.owner)?,
    };
    SUDO_PARAMS.save(deps.storage, &data)?;

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
        ExecuteMsg::IncreaseUSJDebt { amount } => {
            execute_increase_usj_debt(deps, env, info, amount)
        }
        ExecuteMsg::DecreaseUSJDebt { amount } => {
            execute_decrease_usj_debt(deps, env, info, amount)
        }
        ExecuteMsg::SendJUNO { recipient, amount } => {
            execute_send_juno(deps, env, info, recipient, amount)
        }
        ExecuteMsg::SetAddresses {
            borrower_operations_address,
            trove_manager_address,
            stability_pool_address,
            default_pool_address,
        } => execute_set_addresses(
            deps,
            env,
            info,
            borrower_operations_address,
            trove_manager_address,
            stability_pool_address,
            default_pool_address,
        ),
    }
}

pub fn execute_increase_usj_debt(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    only_bo_or_tm(deps.storage, &info)?;

    let mut assets_in_pool = ASSETS_IN_POOL.load(deps.storage)?;
    assets_in_pool.usj_debt += amount;
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;
    let res = Response::new()
        .add_attribute("action", "increase_usj_debt")
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_decrease_usj_debt(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    only_bo_or_tm_or_sp(deps.storage, &info)?;

    let mut assets_in_pool = ASSETS_IN_POOL.load(deps.storage)?;
    assets_in_pool.usj_debt = assets_in_pool
        .usj_debt
        .checked_sub(amount)
        .map_err(StdError::overflow)?;
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;
    let res = Response::new()
        .add_attribute("action", "decrease_usj_debt")
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_send_juno(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    only_bo_or_tm_or_sp(deps.storage, &info)?;

    let mut assets_in_pool = ASSETS_IN_POOL.load(deps.storage)?;
    assets_in_pool.juno = assets_in_pool
        .juno
        .checked_sub(amount)
        .map_err(StdError::overflow)?;
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;
    let send_msg = BankMsg::Send {
        to_address: recipient.to_string(),
        amount: vec![coin(amount.u128(), NATIVE_JUNO_DENOM.to_string())],
    };
    let res = Response::new()
        .add_message(send_msg)
        .add_attribute("action", "send_juno")
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_set_addresses(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    borrower_operations_address: Addr,
    trove_manager_address: Addr,
    stability_pool_address: Addr,
    default_pool_address: Addr,
) -> Result<Response, ContractError> {
    only_owner(deps.storage, &info)?;

    let mut addresses_set = ADDRESSES_SET.load(deps.storage)?;

    addresses_set.borrower_operations_address = borrower_operations_address.clone();
    addresses_set.trove_manager_address = trove_manager_address.clone();
    addresses_set.stability_pool_address = stability_pool_address.clone();
    addresses_set.default_pool_address = default_pool_address.clone();

    ADDRESSES_SET.save(deps.storage, &addresses_set)?;
    let res = Response::new()
        .add_attribute("action", "set_addresses")
        .add_attribute("borrower_operations_address", borrower_operations_address)
        .add_attribute("trove_manager_address", trove_manager_address)
        .add_attribute("stability_pool_address", stability_pool_address)
        .add_attribute("default_pool_address", default_pool_address);
    Ok(res)
}

/// Checks to enfore only borrower operations or default pool can call
fn only_bo_or_dp(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let addresses_set = ADDRESSES_SET.load(store)?;
    if addresses_set.borrower_operations_address != info.sender.as_ref()
        && addresses_set.default_pool_address != info.sender.as_ref()
    {
        return Err(ContractError::CallerIsNeitherBONorDP {});
    }
    Ok(info.sender.clone())
}
/// Checks to enfore only borrower operations or trove manager or stability pool can call
fn only_bo_or_tm_or_sp(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let addresses_set = ADDRESSES_SET.load(store)?;
    if addresses_set.borrower_operations_address != info.sender.as_ref()
        && addresses_set.trove_manager_address != info.sender.as_ref()
        && addresses_set.stability_pool_address != info.sender.as_ref()
    {
        return Err(ContractError::CallerIsNeitherBONorTMNorSP {});
    }
    Ok(info.sender.clone())
}
/// Checks to enfore only borrower operations or trove manager can call
fn only_bo_or_tm(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let addresses_set = ADDRESSES_SET.load(store)?;
    if addresses_set.borrower_operations_address != info.sender.as_ref()
        && addresses_set.trove_manager_address != info.sender.as_ref()
    {
        return Err(ContractError::CallerIsNeitherBONorTM {});
    }
    Ok(info.sender.clone())
}
/// Checks to enfore only owner can call
fn only_owner(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
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
        QueryMsg::GetJUNO {} => to_binary(&query_juno_state(deps)?),
        QueryMsg::GetUSJDebt {} => to_binary(&query_usj_debt_state(deps)?),
        QueryMsg::GetBorrowerOperationsAddress {} => {
            to_binary(&query_borrower_operations_address(deps)?)
        }
        QueryMsg::GetStabilityPoolAddress {} => to_binary(&query_stability_pool_address(deps)?),
        QueryMsg::GetDefaultPoolAddress {} => to_binary(&query_default_pool_address(deps)?),
        QueryMsg::GetTroveManagerAddress {} => to_binary(&query_trove_manager_address(deps)?),
    }
}

pub fn query_juno_state(deps: Deps) -> StdResult<Uint128> {
    let info = ASSETS_IN_POOL.load(deps.storage)?;
    let res = info.juno;
    Ok(res)
}

pub fn query_usj_debt_state(deps: Deps) -> StdResult<Uint128> {
    let info = ASSETS_IN_POOL.load(deps.storage)?;
    let res = info.usj_debt;
    Ok(res)
}

pub fn query_params(deps: Deps) -> StdResult<ParamsResponse> {
    let info = SUDO_PARAMS.load(deps.storage)?;
    let res = ParamsResponse {
        name: info.name,
        owner: info.owner,
    };
    Ok(res)
}

pub fn query_borrower_operations_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let borrower_operations_address = addresses_set.borrower_operations_address;
    Ok(borrower_operations_address)
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

pub fn query_trove_manager_address(deps: Deps) -> StdResult<Addr> {
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    let trove_manager_address = addresses_set.trove_manager_address;
    Ok(trove_manager_address)
}
