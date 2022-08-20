use std::vec;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128,
};

use cw2::set_contract_version;
use ultra_base::role_provider::Role;

use crate::error::ContractError;
use crate::state::{
    AddressesSet, AssetsInPool, State, SudoParams, ADDRESSES_SET, ASSETS_IN_POOL, SUDO_PARAMS,
};
use ultra_base::active_pool::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg};

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

    // initial assets in pool
    let assets_in_pool = AssetsInPool {
        juno: Uint128::zero(),
        ultra_debt: Uint128::zero(),
    };

    SUDO_PARAMS.save(deps.storage, &sudo_params)?;
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;

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
        ExecuteMsg::IncreaseULTRADebt { amount } => {
            execute_increase_ultra_debt(deps, env, info, amount)
        }
        ExecuteMsg::DecreaseULTRADebt { amount } => {
            execute_decrease_ultra_debt(deps, env, info, amount)
        }
        ExecuteMsg::SendJUNO { recipient, amount } => {
            execute_send_juno(deps, env, info, recipient, amount)
        }
    }
}

pub fn execute_increase_ultra_debt(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let state = State::default();

    state.roles.assert_role(
        deps.as_ref(),
        &info.sender,
        vec![Role::BorrowerOperations, Role::TroveManager],
    )?;

    let mut assets_in_pool = ASSETS_IN_POOL.load(deps.storage)?;
    assets_in_pool.ultra_debt += amount;
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;
    let res = Response::new()
        .add_attribute("action", "increase_ultra_debt")
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_decrease_ultra_debt(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let state = State::default();

    state.roles.assert_role(
        deps.as_ref(),
        &info.sender,
        vec![
            Role::BorrowerOperations,
            Role::TroveManager,
            Role::StabilityPool,
        ],
    )?;

    let mut assets_in_pool = ASSETS_IN_POOL.load(deps.storage)?;
    assets_in_pool.ultra_debt = assets_in_pool
        .ultra_debt
        .checked_sub(amount)
        .map_err(StdError::overflow)?;
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;
    let res = Response::new()
        .add_attribute("action", "decrease_ultra_debt")
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
    let state = State::default();

    state.roles.assert_role(
        deps.as_ref(),
        &info.sender,
        vec![Role::BorrowerOperations, Role::TroveManager],
    )?;

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
        QueryMsg::GetULTRADebt {} => to_binary(&query_ultra_debt_state(deps)?),
    }
}

pub fn query_juno_state(deps: Deps) -> StdResult<Uint128> {
    let info = ASSETS_IN_POOL.load(deps.storage)?;
    let res = info.juno;
    Ok(res)
}

pub fn query_ultra_debt_state(deps: Deps) -> StdResult<Uint128> {
    let info = ASSETS_IN_POOL.load(deps.storage)?;
    let res = info.ultra_debt;
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
