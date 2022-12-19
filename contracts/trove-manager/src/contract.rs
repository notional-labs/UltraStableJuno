
use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage, Addr, Uint128, StdError, Decimal256, Uint256, CosmosMsg, WasmMsg, to_binary};

use cw2::set_contract_version;
use cw_utils::maybe_addr;
use ultra_base::role_provider::Role;
use ultra_base::ultra_math::dec_pow;

use crate::error::ContractError;
use crate::state::{SudoParams, SUDO_PARAMS, ADMIN, ROLE_CONSUMER, Manager, RewardSnapshot, MANAGER, TROVES, SNAPSHOTS, TROVE_OWNER_IDX};
use ultra_base::trove_manager::{InstantiateMsg, ExecuteMsg, QueryMsg, Status, Trove};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:trove-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ONE_MINUTE: u64 = 60_000_000_000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // set admin so that only admin can access to update role function
    let api = deps.api;
    ADMIN.set(deps.branch(), maybe_addr(api, Some(msg.owner.clone()))?)?;
    
     // store sudo params
     let sudo_params = SudoParams {
        name: msg.name,
        owner: deps.api.addr_validate(&msg.owner)?,
    };
    SUDO_PARAMS.save(deps.storage, &sudo_params)?;

    MANAGER
        .save(deps.storage, &Manager{
            trove_owner_count: Uint128::zero(),
            base_rate: Decimal256::zero(),
            last_fee_operation_time: env.block.time
        })?;
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
        ExecuteMsg::UpdateAdmin { admin } => {
            Ok(ADMIN.execute_update_admin(deps, info, Some(admin))?)
        }
        ExecuteMsg::UpdateRole { role_provider } => {
            execute_update_role(deps, env, info, role_provider)
        }
        ExecuteMsg::Liquidate { borrower } => {
            execute_liquidate(deps, env, info, borrower)
        },
        ExecuteMsg::UpdateStakeAndTotalStakes { borrower } => {
            execute_update_stake_and_total_stakes(deps, env, info, borrower)
        },
        ExecuteMsg::CloseTrove { borrower } => {
            execute_close_trove(deps, env, info, borrower)
        },
        ExecuteMsg::AddTroveOwnerToArray { borrower } => {
            execute_add_trove_owner_to_array(deps, env, info, borrower)
        }
        ExecuteMsg::DecayBaseRateFromBorrowing {  } => {
            execute_decay_base_rate_from_borrowing(deps, env, info)
        }
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

pub fn execute_update_role(
    deps: DepsMut, 
    _env: Env,
    info: MessageInfo,
    role_provider: Addr
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    ROLE_CONSUMER.add_role_provider(deps.storage, role_provider.clone())?;

    let res = Response::new()
        .add_attribute("action", "update_role")
        .add_attribute("role_provider_addr", role_provider);
    Ok(res)
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

pub fn execute_update_stake_and_total_stakes(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String
) -> Result<Response, ContractError> {
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;

    
    let res = Response::new()
        .add_attribute("action", "update_stake_and_total_stakes")
        .add_attribute("borrower", borrower);
    Ok(res)
}

pub fn execute_close_trove(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String
) -> Result<Response, ContractError> {
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;

    let borrower_addr = deps.api.addr_validate(&borrower)?;
    let trove_count = MANAGER.load(deps.storage)?.trove_owner_count;

    let sorted_troves = ROLE_CONSUMER
        .load_role_address(
            deps.as_ref(), 
            Role::SortedTroves
    )?;
    let size: Uint256 = deps
        .querier
        .query_wasm_smart(
            sorted_troves.clone(), 
            &ultra_base::sorted_troves::QueryMsg::GetSize {  })?;
        
    if trove_count <= Uint128::from(1u128) && size <= Uint256::from_u128(1u128) {
        return Err(ContractError::OnlyOneTroveExist {});
    }


    // Remove trove by index
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr.clone())?; 
    let last_trove_idx = trove_count - Uint128::one();   

    if trove_idx == last_trove_idx {
        TROVE_OWNER_IDX.remove(deps.storage, borrower_addr.clone());
        TROVES.remove(deps.storage, trove_idx.to_string());
    } else {
        let (last_trove_owner, last_trove) = TROVES.load(deps.storage, last_trove_idx.to_string())?; 
    
        TROVE_OWNER_IDX.remove(deps.storage, borrower_addr.clone());
        TROVE_OWNER_IDX.save(deps.storage, last_trove_owner.clone(), &trove_idx)?;
        TROVES.remove(deps.storage, last_trove_idx.to_string());
        TROVES.save(deps.storage, last_trove_idx.to_string(), &(last_trove_owner, last_trove))?;
    }

    SNAPSHOTS.remove(deps.storage, borrower_addr.clone());
    MANAGER.update(deps.storage, |mut manager | -> Result<Manager, ContractError>{
        manager.trove_owner_count -= Uint128::one();
        Ok(manager)
    })?;

    let remove_borrower_msg: CosmosMsg = CosmosMsg::Wasm(
        WasmMsg::Execute{
            contract_addr: sorted_troves.to_string(),
            msg: to_binary(&ultra_base::sorted_troves::ExecuteMsg::Remove { 
                id: borrower_addr.to_string() 
            })?,
            funds: vec![]
        }
    );
    let res = Response::new()
        .add_attribute("action", "close_trove")
        .add_attribute("borrower", borrower)
        .add_message(remove_borrower_msg);
    Ok(res)
}

pub fn execute_add_trove_owner_to_array(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String
) -> Result<Response, ContractError> {
    ROLE_CONSUMER
    .assert_role(
        deps.as_ref(), 
        &info.sender,
        vec![Role::BorrowerOperations],
    )?;

    let borrower_addr = deps.api.addr_validate(&borrower)?;

    let index = MANAGER.load(deps.storage)?.trove_owner_count;
    MANAGER
        .update(deps.storage, |mut manager| -> Result<Manager, ContractError> {
            manager.trove_owner_count = index
                .checked_add(Uint128::from(1u128))
                .map_err(StdError::overflow)?;
            Ok(manager)
        })?;
    
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr.clone())?;    
    TROVES  
        .update(deps.storage, trove_idx.to_string(), |t| {
            if t.is_some() {
                return Err(ContractError::TroveExist {})
            }
            let trove = Trove{
                coll: Uint128::zero(),
                debt: Uint128::zero(),
                stake: Uint128::zero(),
                status: Status::Active,
                owner: borrower_addr.clone(),
                index
            };
            Ok((borrower_addr, trove))
        })?;

    let res = Response::new()
        .add_attribute("action", "add_trove_owner_to_array")
        .add_attribute("trove_owner", borrower)
        .add_attribute("trove_index", index.to_string());
    Ok(res)
}

pub fn execute_decay_base_rate_from_borrowing(
    deps: DepsMut, 
    env: Env, 
    info: MessageInfo
) -> Result<Response, ContractError> {
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;

    let mut manager = MANAGER.load(deps.storage)?;

    // Half-life of 12h. 12h = 720 min
    // (1/2) = d^720 => d = (1/2)^(1/720)
    // 18 digit of decimal places
    let minute_decay_factor: Decimal256 = Decimal256::from_str("0.999037758833783388")?;
    
    let last_fee_operation_time = manager.last_fee_operation_time.nanos();
    let base_rate = manager.base_rate;
    
    let time_pass : u64 = env.block.time.nanos() - last_fee_operation_time;
    let minus_pass = time_pass / ONE_MINUTE;

    // calculate new base rate
    let decay_factor: Decimal256 = dec_pow(minute_decay_factor, minus_pass)?;
    let decay_base_rate =  base_rate.saturating_mul(decay_factor);
    if decay_base_rate > Decimal256::one() {
        return Err(ContractError::DecayBaseRateLargerThanOne {})
    }
    manager.base_rate = decay_base_rate;

    // Update last fee operation time 
    if time_pass >= ONE_MINUTE {
        manager.last_fee_operation_time = env.block.time;
    }

    MANAGER.save(deps.storage, &manager)?;
    let res = Response::new()
        .add_attribute("action", "decay_base_rate_from_borrowing")
        .add_attribute("new_base_rate", decay_base_rate.to_string())
        .add_attribute("last_fee_operation_time", last_fee_operation_time.to_string());
    Ok(res)
}

pub fn execute_set_trove_status(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    borrower: String,
    status: Status
) -> Result<Response, ContractError> {
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr)?;    
    TROVES    
        .update(deps.storage, trove_idx.to_string(), |t| {
            if t.is_none() {
                return Err(ContractError::TroveNotExist {})
            }
            let (owner, mut trove) = t.unwrap();
            trove.status = status.clone();
            Ok((owner, trove))
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
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;

    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr)?;    
    TROVES
        .update(deps.storage, trove_idx.to_string(), |t| {
        if t.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let (owner, mut trove) = t.unwrap();
        trove.coll = trove.coll
            .checked_add(coll_increase)
            .map_err(StdError::overflow)?;
        Ok((owner, trove))
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
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;

    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr)?;    
    TROVES.update(deps.storage, trove_idx.to_string(), |t| {
        if t.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let (owner, mut trove) = t.unwrap();
        trove.coll = trove.coll
            .checked_sub(coll_decrease)
            .map_err(StdError::overflow)?;
        Ok((owner, trove))
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
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;

    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr)?;
    TROVES
        .update(deps.storage, trove_idx.to_string(), |t| {
        if t.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let (owner, mut trove) = t.unwrap();
        trove.debt = trove.debt
            .checked_add(debt_increase)
            .map_err(StdError::overflow)?;
        Ok((owner, trove))
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
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations],
        )?;
    
    let borrower_addr = deps.api.addr_validate(&borrower)?;

    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr)?;
    TROVES
        .update(deps.storage, trove_idx.to_string(), |t| {
        if t.is_none() {
            return Err(ContractError::TroveNotExist {})
        }
        let (owner, mut trove) = t.unwrap();
        trove.debt = trove.debt
            .checked_sub(debt_decrease)
            .map_err(StdError::overflow)?;
        Ok((owner, trove))
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
