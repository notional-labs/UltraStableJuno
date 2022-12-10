
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage, Addr, Uint128, StdError};

use cw2::set_contract_version;
use ultra_base::role_provider::Role;

use crate::error::ContractError;
use crate::state::{SudoParams, SUDO_PARAMS, State, TROVES};
use ultra_base::trove_manager::{InstantiateMsg, ExecuteMsg, QueryMsg, Status};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:trove-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    msg: ExecuteMsg
) -> Result<Response, ContractError> {
    match msg{
        ExecuteMsg::Liquidate { borrower } => {
            execute_liquidate(deps, env, info, borrower)
        },

        ExecuteMsg::SetTroveStatus { borrower, status } => {
            execute_set_trove_status(deps, env, info, borrower, status)
        }
        ExecuteMsg::IncreaseTroveColl { borrower, coll_increase } => {
            execute_increase_trove_coll(deps, env, info, borrower, coll_increase)
        }
        ExecuteMsg::DecreaseTroveColl { borrower, coll_decrease } => {
            execute_decrease_trove_coll(deps, env, info, borrower, coll_decrease)
        }
        ExecuteMsg::IncreaseTroveDebt { borrower, debt_increase } => {
            execute_increase_trove_debt(deps, env, info, borrower, debt_increase)
        }
        ExecuteMsg::DecreaseTroveDebt { borrower, debt_decrease } => {
            execute_decrease_trove_debt(deps, env, info, borrower, debt_decrease)
        }
    }
}

pub fn execute_liquidate(
    deps: DepsMut, 
    _env: Env, 
    _info: MessageInfo, 
    borrower: String
) -> Result<Response, ContractError> {

    let res = Response::new()
        .add_attribute("action", "liquidate");
    Ok(res)
}

pub fn execute_set_trove_status(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String,
    status: Status
) -> Result<Response, ContractError> {
    let state : State = State::default();
    state
        .roles
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    TROVES.update(deps.storage, borrower_addr, |trove| {
        if trove.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let mut trove = trove.unwrap();
        trove.status = status.clone();
        Ok(trove)
    })?;
    let res = Response::new()
        .add_attribute("action", "set_trove_status")
        .add_attribute("borrower", borrower)
        .add_attribute("new_status", format!("{:?}", status));
    Ok(res)
}

pub fn execute_increase_trove_coll(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String, 
    coll_increase: Uint128,
) -> Result<Response, ContractError> {
    let state : State = State::default();
    state
        .roles
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    TROVES.update(deps.storage, borrower_addr, |trove| {
        if trove.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let mut trove = trove.unwrap();
        trove.juno = trove.juno
            .checked_add(coll_increase)
            .map_err(StdError::overflow)?;
        Ok(trove)
    })?;
    let res = Response::new()
        .add_attribute("action", "increase_trove_coll")
        .add_attribute("borrower", borrower)
        .add_attribute("amount", coll_increase);
    Ok(res)
}


pub fn execute_decrease_trove_coll(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String, 
    coll_decrease: Uint128,
) -> Result<Response, ContractError> {
    let state : State = State::default();
    state
        .roles
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    TROVES.update(deps.storage, borrower_addr, |trove| {
        if trove.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let mut trove = trove.unwrap();
        trove.juno = trove.juno
            .checked_sub(coll_decrease)
            .map_err(StdError::overflow)?;
        Ok(trove)
    })?;
    let res = Response::new()
        .add_attribute("action", "decrease_trove_coll")
        .add_attribute("borrower", borrower)
        .add_attribute("amount", coll_decrease);
    Ok(res)
}

pub fn execute_increase_trove_debt(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String, 
    debt_increase: Uint128,
) -> Result<Response, ContractError> {
    let state : State = State::default();
    state
        .roles
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    TROVES.update(deps.storage, borrower_addr, |trove| {
        if trove.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let mut trove = trove.unwrap();
        trove.ultra_debt = trove.ultra_debt
            .checked_add(debt_increase)
            .map_err(StdError::overflow)?;
        Ok(trove)
    })?;
    let res = Response::new()
        .add_attribute("action", "increase_trove_debt")
        .add_attribute("borrower", borrower)
        .add_attribute("amount", debt_increase);
    Ok(res)
}

pub fn execute_decrease_trove_debt(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String, 
    debt_decrease: Uint128,
) -> Result<Response, ContractError> {
    let state : State = State::default();
    state
        .roles
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    TROVES.update(deps.storage, borrower_addr, |trove| {
        if trove.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let mut trove = trove.unwrap();
        trove.ultra_debt = trove.ultra_debt
            .checked_sub(debt_decrease)
            .map_err(StdError::overflow)?;
        Ok(trove)
    })?;
    let res = Response::new()
        .add_attribute("action", "increase_trove_debt")
        .add_attribute("borrower", borrower)
        .add_attribute("amount", debt_decrease);
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