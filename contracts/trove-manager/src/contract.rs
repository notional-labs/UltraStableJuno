
use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage, Addr, Uint128, StdError, Decimal256, Uint256, CosmosMsg, WasmMsg, to_binary, StdResult, Deps, Decimal};

use cw2::set_contract_version;
use cw20::BalanceResponse;
use cw_utils::maybe_addr;

use ultra_base::querier::MCR;
use ultra_base::role_provider::Role;
use ultra_base::sorted_troves;
use ultra_base::ultra_math::{dec_pow, compute_cr};

use crate::error::ContractError;
use crate::state::{SudoParams, SUDO_PARAMS, ADMIN, ROLE_CONSUMER, MANAGER, TROVES, SNAPSHOTS, TROVE_OWNER_IDX};
use ultra_base::trove_manager::{InstantiateMsg, ExecuteMsg, QueryMsg, Status, Trove, Manager, RewardSnapshot, RedemptionTotals};


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
            last_fee_operation_time: env.block.time,
            total_stake_snapshot: Uint128::zero(),
            total_collateral_snapshot: Uint128::zero(),
            total_stake: Uint128::zero(),
            total_liquidation_juno: Uint128::zero(),
            total_liquidation_ultra_debt: Uint128::zero(),
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
        ExecuteMsg::RedeemCollateral { 
            ultra_amount, 
            first_redemption_hint, 
            upper_partial_redemption_hint, 
            lower_partial_redemption_hint, 
            max_iterations, 
            max_fee_percentage 
        } => {
            execute_redeem_collateral(
                deps, 
                env, 
                info, 
                ultra_amount, 
                first_redemption_hint, 
                upper_partial_redemption_hint, 
                lower_partial_redemption_hint, 
                max_iterations, 
                max_fee_percentage
            )
        },
        ExecuteMsg::ApplyPendingRewards { borrower } => {
            execute_apply_pending_rewards(deps, env, info, borrower)
        },
        ExecuteMsg::UpdateTroveRewardSnapshots { borrower } => {
            execute_update_trove_reward_snapshots(deps, env, info, borrower)
        },
        ExecuteMsg::RemoveStake { borrower } => {
            execute_remove_stake(deps, env, info, borrower)
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
        .add_attribute("action", "liquidate")
        .add_attribute("borrower", borrower);
    Ok(res)
}

pub fn execute_redeem_collateral(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    ultra_amount: Uint128,
    first_redemption_hint: Option<String>,
    upper_partial_redemption_hint: String,
    lower_partial_redemption_hint: String,
    max_iterations: Uint128,
    max_fee_percentage: Decimal256,
) -> Result<Response, ContractError> {
    let api = deps.api;
    let first_redemption_hint = maybe_addr(api, first_redemption_hint)?;
    let upper_partial_redemption_hint = deps.api.addr_validate(&upper_partial_redemption_hint)?;
    let lower_partial_redemption_hint = deps.api.addr_validate(&lower_partial_redemption_hint)?;

    let oracle_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::PriceFeed)?;
    let default_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::DefaultPool)?;
    let active_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::ActivePool)?;
    let ultra_token_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::UltraToken)?;
    if max_fee_percentage >= Decimal256::permille(5) &&  max_fee_percentage <= Decimal256::one() {
        return Err(ContractError::MaxFeePercentageInvalid {  })
    }

    // TODO: Fix some related to LQTY token and bootstrap

    let price: Decimal = deps.querier
        .query_wasm_smart(
            oracle_addr, 
            &ultra_base::oracle::QueryMsg::ExchangeRate { 
                denom: ultra_base::oracle::NATIVE_JUNO_DENOM.to_string()
            }
        )?;
    let price = Decimal256::from(price);
    if get_tcr(deps.as_ref(), price, default_pool_addr.clone(), active_pool_addr.clone())? < MCR {
        return  Err(ContractError::TCRLessThanMCR {  });
    }

    if ultra_amount.is_zero() {
        return Err(ContractError::AmountIsZero {  });
    }

    let ultra_balance: BalanceResponse = deps.querier
        .query_wasm_smart(
            ultra_token_addr.to_string(),
            &ultra_token::msg::QueryMsg::Balance { address: info.sender.to_string() }
        )?;
    if ultra_balance.balance < ultra_amount {
        return Err(ContractError::InsufficientBalance {  })
    } 

    let mut totals = RedemptionTotals::default();
    totals.total_ultra_debt_supply_at_start = {
        let active_debt: Uint128 = deps.querier
            .query_wasm_smart(
                default_pool_addr.to_string(), 
                &ultra_base::default_pool::QueryMsg::GetULTRADebt {  }
            )?;
        let closed_debt: Uint128 = deps.querier
            .query_wasm_smart(
                active_pool_addr.to_string(),
                &ultra_base::active_pool::QueryMsg::GetULTRADebt {  }
            )?;
        active_debt.checked_add(closed_debt).map_err(StdError::overflow)?
    };
    // Confirm redeemer's balance is less than total LUSD supply
    if ultra_balance.balance > totals.total_ultra_debt_supply_at_start {
        return Err(ContractError::BalanceOverSupply {  });
    }

    totals.remaining_ultra_debt = ultra_amount;
    let currentBorrower: Addr;

    let res = Response::new()
        .add_attribute("action", "redeem_collateral")
        .add_attribute("first_redemption_hint", format!("{:?}",first_redemption_hint.map(|addr| addr.to_string())))
        .add_attribute("upper_partial_redemption_hint", upper_partial_redemption_hint.to_string())
        .add_attribute("lower_partial_redemption_hint", lower_partial_redemption_hint.to_string());
    Ok(res)
}

pub fn execute_apply_pending_rewards(
    deps: DepsMut, 
    env: Env, 
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
    let active_pool_addr = ROLE_CONSUMER
        .load_role_address(deps.as_ref(), Role::ActivePool)?;
    let default_pool_addr = ROLE_CONSUMER
        .load_role_address(deps.as_ref(), Role::DefaultPool)?;

    let cosmos_msgs = apply_pending_rewards(deps, env, info, borrower_addr, active_pool_addr, default_pool_addr)?;
    let res = Response::new()
        .add_attribute("action", "apply_pending_rewards")
        .add_attribute("borrower", borrower)
        .add_messages(cosmos_msgs);
    Ok(res)
}

pub fn execute_update_trove_reward_snapshots(
    deps: DepsMut, 
    env: Env, 
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
    let manager = MANAGER.load(deps.storage)?;

    update_trove_reward_snapshots(
        deps, 
        env, 
        info,
        borrower_addr, 
        manager.total_liquidation_juno,
        manager.total_liquidation_ultra_debt
    )?;
    let res = Response::new()
        .add_attribute("action", "update_trove_reward_snapshots")
        .add_attribute("borrower", borrower)
        .add_attribute("juno", manager.total_liquidation_juno.to_string())
        .add_attribute("ultra_debt", manager.total_liquidation_ultra_debt.to_string());
    Ok(res)
}

pub fn execute_remove_stake(
    deps: DepsMut, 
    env: Env, 
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
    remove_stake(deps, env, info, borrower_addr)?;
    let res = Response::new()
        .add_attribute("action", "remove_stake")
        .add_attribute("borrower", borrower);
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

    let mut manager = MANAGER.load(deps.storage)?;

    let borrower_addr = deps.api.addr_validate(&borrower)?;
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr)?;
    let (trove_owner, mut trove) = TROVES.load(deps.storage, trove_idx.to_string())?; 
    let new_stake: Uint128;

    if manager.total_collateral_snapshot == Uint128::zero(){
        new_stake = trove.coll;
    } else {
        /*
            * The following assert holds true because:
            * - The system always contains >= 1 trove
            * - When we close or liquidate a trove, we redistribute the pending rewards, so if all troves were closed/liquidated,
            * rewards would’ve been emptied and totalCollateralSnapshot would be zero too.
            */
        if manager.total_stake_snapshot == Uint128::zero() {
            return Err(ContractError::TotalStakeSnapshotIsZero {  })
        }

        new_stake = trove.coll
            .checked_mul(manager.total_stake_snapshot)
            .map_err(StdError::overflow)?
            .checked_div(manager.total_collateral_snapshot)
            .map_err(StdError::divide_by_zero)?;
    }

    
    manager.total_stake = manager.total_stake
        .checked_sub(trove.stake)
        .map_err(StdError::overflow)?
        .checked_add(new_stake)
        .map_err(StdError::overflow)?;
    MANAGER.save(deps.storage, &manager)?;

    trove.stake = new_stake;
    TROVES.save(deps.storage, trove_idx.to_string(), &(trove_owner, trove))?;

    let res = Response::new()
        .add_attribute("action", "update_stake_and_total_stakes")
        .add_attribute("borrower", borrower)
        .add_attribute("new_stake", new_stake.to_string())
        .add_attribute("new_total_stake", manager.total_stake.to_string());
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
                return Err(ContractError::TroveNotActive {})
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
            return Err(ContractError::TroveNotActive {  })
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
            return Err(ContractError::TroveNotActive {  })
        }
        let (owner, mut trove) = t.unwrap();
        trove.coll = trove.coll
            .checked_add(coll_decrease)
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
            return Err(ContractError::TroveNotActive {  })
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
            return Err(ContractError::TroveNotActive {  })
        }
        let (owner, mut trove) = t.unwrap();
        trove.debt = trove.debt
            .checked_add(debt_decrease)
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

fn remove_stake(
    deps: DepsMut, 
    _env: Env, 
    _info: MessageInfo, 
    borrower_addr: Addr
) -> Result<(), ContractError> {
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr.clone())?;
    let (_, mut trove) = TROVES.load(deps.storage, trove_idx.to_string())?;
    
    let mut manager = MANAGER.load(deps.storage)?;

    manager.total_stake = manager.total_stake
        .checked_sub(trove.stake)
        .map_err(StdError::overflow)?;
    MANAGER.save(deps.storage, &manager)?;
    trove.stake = Uint128::zero();
    TROVES.save(deps.storage, trove_idx.to_string(), &(borrower_addr, trove))?;
    Ok(())
}

fn update_trove_reward_snapshots(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    borrower_addr: Addr,
    total_liquidation_juno: Uint128, 
    total_liquidation_ultra_debt: Uint128
) -> Result<(), ContractError> {
    SNAPSHOTS.save(deps.storage, borrower_addr, &RewardSnapshot { 
        juno: total_liquidation_juno, 
        ultra_debt: total_liquidation_ultra_debt 
    })?;
    Ok(())
}

fn apply_pending_rewards(
    deps: DepsMut, 
    env: Env, 
    info: MessageInfo, 
    borrower_addr: Addr,
    active_pool_addr: Addr, 
    default_pool_addr: Addr 
) -> Result<Vec<CosmosMsg>, ContractError> {
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr.clone())?;
    let (_, mut trove) = TROVES.load(deps.storage, trove_idx.to_string())?;
    let snapshot = SNAPSHOTS.load(deps.storage, borrower_addr.clone())?;
    let manager = MANAGER.load(deps.storage)?;
    let total_liquidation_juno = manager.total_liquidation_juno;
    let total_liquidation_ultra_debt = manager.total_liquidation_ultra_debt;
    
    let cosmos_msgs;
    // TODO: Check LUSD contract whether if (snapshot.juno < total_liquidation_juno) means snapshot not exits or not
    
    /*
        * A Trove has pending rewards if its snapshot is less than the current rewards per-unit-staked sum:
        * this indicates that rewards have occured since the snapshot was made, and the user therefore has
        * pending rewards
        */
    if trove.status == Status::Active && snapshot.juno < total_liquidation_juno{
        if trove.status != Status::Active {
            return Err(ContractError::TroveNotActive {  })
        }

        // Compute pending rewards
        let pending_juno_reward = get_pending_juno_reward(deps.as_ref(), borrower_addr.clone())?;
        let pending_ultra_debt_reward = get_pending_ultra_debt_reward(deps.as_ref(), borrower_addr.clone())?;

        // Apply pending rewards to trove's state
        trove.coll = trove.coll
            .checked_add(pending_juno_reward)
            .map_err(StdError::overflow)?;
        trove.debt = trove.debt
            .checked_add(pending_ultra_debt_reward)
            .map_err(StdError::overflow)?;

        update_trove_reward_snapshots(
            deps, 
            env, 
            info, 
            borrower_addr, 
            total_liquidation_juno, 
            total_liquidation_ultra_debt)?;
        
        // Transfer from DefaultPool to ActivePool
        cosmos_msgs = move_pending_trove_rewards_to_active_pool(
            active_pool_addr,
            default_pool_addr,
            pending_ultra_debt_reward,
            pending_juno_reward
        )?;
    } else {
        cosmos_msgs = vec![];
    }
    Ok(cosmos_msgs)
}

fn move_pending_trove_rewards_to_active_pool(
    active_pool_addr: Addr,
    default_pool_addr: Addr,
    ultra_debt: Uint128,
    juno: Uint128
) -> Result<Vec<CosmosMsg>, StdError>{
    let mut cosmos_msgs = vec![];
    
    let decrease_ultra_debt_msg: CosmosMsg = CosmosMsg::Wasm(
        WasmMsg::Execute { 
            contract_addr: default_pool_addr.to_string(), 
            msg: to_binary(&ultra_base::default_pool::ExecuteMsg::DecreaseULTRADebt { 
                amount: ultra_debt
            })?, 
            funds: vec![] 
        }
    );

    let increase_ultra_debt_msg: CosmosMsg = CosmosMsg::Wasm(
        WasmMsg::Execute { 
            contract_addr: active_pool_addr.to_string(), 
            msg: to_binary(&ultra_base::active_pool::ExecuteMsg::IncreaseULTRADebt { 
                amount: ultra_debt
            })?, 
            funds: vec![] 
        }
    );

    let send_juno_to_active_pool: CosmosMsg = CosmosMsg::Wasm(
        WasmMsg::Execute { 
            contract_addr: default_pool_addr.to_string(), 
            msg: to_binary(&ultra_base::default_pool::ExecuteMsg::SendJUNOToActivePool { 
                amount: juno 
            })?,
            funds: vec![] 
        }
    ); 
    cosmos_msgs.push(decrease_ultra_debt_msg);
    cosmos_msgs.push(increase_ultra_debt_msg);
    cosmos_msgs.push(send_juno_to_active_pool);
    Ok(cosmos_msgs)
}

pub fn is_valid_first_redemption_hint(
    deps: Deps, 
    sorted_troves_addr: Addr, 
    first_redemption_hint: Option<Addr>, 
    price: Decimal256
) -> StdResult<bool> { 

    if first_redemption_hint.is_none() {}
    Ok(false)
}
pub fn get_pending_juno_reward(deps: Deps, borrower_addr: Addr) -> StdResult<Uint128>{
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr.clone())?;
    let (_, trove) = TROVES.load(deps.storage, trove_idx.to_string())?;
    let snapshot_juno = SNAPSHOTS.load(deps.storage, borrower_addr.clone())?.juno;
    let total_liquidation_juno = MANAGER.load(deps.storage)?.total_liquidation_juno;
    let reward_per_unit_staked = total_liquidation_juno
        .checked_sub(snapshot_juno)
        .map_err(StdError::overflow)?;
    
    if reward_per_unit_staked.is_zero() || trove.status != Status::Active {
        return  Ok(Uint128::zero())
    }

    let pending_juno_reward = trove.stake
            .checked_mul(reward_per_unit_staked)
            .map_err(StdError::overflow)?
            .checked_div(Uint128::from(10u128).pow(18))
            .map_err(StdError::divide_by_zero)?;
        
    Ok(pending_juno_reward)
}

pub fn get_pending_ultra_debt_reward(deps: Deps, borrower_addr: Addr) -> StdResult<Uint128>{
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr.clone())?;
    let (_, trove) = TROVES.load(deps.storage, trove_idx.to_string())?;
    let snapshot_ultra_debt = SNAPSHOTS.load(deps.storage, borrower_addr.clone())?.ultra_debt;
    let total_liquidation_ultra_debt = MANAGER.load(deps.storage)?.total_liquidation_ultra_debt;
    let reward_per_unit_staked = total_liquidation_ultra_debt
        .checked_sub(snapshot_ultra_debt)
        .map_err(StdError::overflow)?;
    
    if reward_per_unit_staked.is_zero() || trove.status != Status::Active {
        return  Ok(Uint128::zero())
    }

    let pending_ultra_debt_reward = trove.stake
            .checked_mul(reward_per_unit_staked)
            .map_err(StdError::overflow)?
            .checked_div(Uint128::from(10u128).pow(18))
            .map_err(StdError::divide_by_zero)?;
        
    Ok(pending_ultra_debt_reward)
}

pub fn get_tcr(deps: Deps, price: Decimal256, default_pool_addr: Addr, active_pool_addr: Addr) -> StdResult<Decimal256>{
    let entire_system_coll = {
        let active_coll: Uint128 = deps.querier
            .query_wasm_smart(
                active_pool_addr.to_string(),
                &ultra_base::active_pool::QueryMsg::GetJUNO {  }
            )?;
        let liquidated_coll: Uint128 = deps.querier
            .query_wasm_smart(
                default_pool_addr.to_string(), 
                &ultra_base::default_pool::QueryMsg::GetJUNO {  }
            )?;
        active_coll.checked_add(liquidated_coll).map_err(StdError::overflow)?
    };

    let entire_system_debt = {
        let active_debt: Uint128 = deps.querier
            .query_wasm_smart(
                default_pool_addr.to_string(), 
                &ultra_base::default_pool::QueryMsg::GetULTRADebt {  }
            )?;
        let closed_debt: Uint128 = deps.querier
            .query_wasm_smart(
                active_pool_addr.to_string(),
                &ultra_base::active_pool::QueryMsg::GetULTRADebt {  }
            )?;
        active_debt.checked_add(closed_debt).map_err(StdError::overflow)?
    };

    compute_cr(entire_system_coll, entire_system_debt, price)
}
