
use std::str::FromStr;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage, Addr, Uint128, StdError, Decimal256, Uint256, CosmosMsg, WasmMsg, to_binary, StdResult, Deps, Decimal};

use cw2::set_contract_version;
use cw20::BalanceResponse;
use cw_utils::maybe_addr;

use ultra_base::querier::{MCR, REDEMPTION_FEE_FLOOR, ULTRA_GAS_COMPENSATE, MIN_NET_DEBT, check_recovery_mode, get_tcr, query_entire_system_debt, query_entire_system_coll, PERCENT_DIVISOR, CCR};
use ultra_base::role_provider::Role;
use ultra_base::ultra_math::{dec_pow, compute_cr, compute_nominal_cr};

use crate::error::ContractError;
use crate::state::{SudoParams, SUDO_PARAMS, ADMIN, ROLE_CONSUMER, MANAGER, TROVES, SNAPSHOTS, TROVE_OWNER_IDX};
use ultra_base::trove_manager::{InstantiateMsg, ExecuteMsg, QueryMsg, Status, Trove, Manager, RewardSnapshot, RedemptionTotals, SingleRedemptionValues, LiquidationTotals, LiquidationValues, EntireDebtAndCollResponse};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:trove-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ONE_MINUTE: u64 = 60_000_000_000;

/*
    * BETA: 18 digit decimal. Parameter by which to divide the redeemed fraction, in order to calc the new base rate from a redemption.
    * Corresponds to (1 / ALPHA) in the white paper.
    */
pub const BETA: u8 = 2;

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
            base_rate: Decimal::zero(),
            last_fee_operation_time: env.block.time,
            total_stake_snapshot: Uint128::zero(),
            total_collateral_snapshot: Uint128::zero(),
            total_stake: Uint128::zero(),
            total_liquidation_juno: Uint128::zero(),
            total_liquidation_ultra_debt: Uint128::zero(),
            last_juno_error_redistribution: Uint128::zero(),
            last_ultra_debt_error_redistribution: Uint128::zero()
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
        ExecuteMsg::LiquidateTroves { n } => {
            execute_liquidate_troves(deps, env, info, n)
        },
        ExecuteMsg::BatchLiquidateTroves { borrowers } => {
            execute_batch_liquidate_troves(deps, env, info, borrowers)
        },
        ExecuteMsg::RedeemCollateral { 
            ultra_amount, 
            first_redemption_hint, 
            upper_partial_redemption_hint, 
            lower_partial_redemption_hint, 
            partial_redemption_hint_nicr,
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
                partial_redemption_hint_nicr,
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

pub fn execute_liquidate_troves(
    deps: DepsMut, 
    _env: Env, 
    _info: MessageInfo, 
    n: Uint128
) -> Result<Response, ContractError> {

    let res = Response::new()
        .add_attribute("action", "liquidate");
    Ok(res)
}

pub fn execute_batch_liquidate_troves(
    mut deps: DepsMut, 
    _env: Env, 
    _info: MessageInfo, 
    borrowers: Vec<String>
) -> Result<Response, ContractError> {
    // convert borrowers to addr
    let api = deps.api;
    let mut borrowers_addr: Vec<Addr> = vec![];
    let mut cosmos_msg: Vec<CosmosMsg> = vec![];
    for borrower in borrowers {
        let borrower_addr = maybe_addr(api, Some(borrower))?;
        if borrower_addr.is_some() {
            borrowers_addr.push(borrower_addr.unwrap());
        }
    }

    let oracle_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::PriceFeed)?;
    let stability_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::StabilityPool)?;
    let active_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::ActivePool)?;
    let default_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::DefaultPool)?;
    let sorted_troves_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::SortedTroves)?;

    let mut totals = LiquidationTotals::default();
    let price: Decimal = deps.querier
        .query_wasm_smart(
            oracle_addr, 
            &ultra_base::oracle::QueryMsg::ExchangeRate { 
                denom: ultra_base::oracle::NATIVE_JUNO_DENOM.to_string()
            }
        )?;
    let ultra_in_stability_pool: Uint128 = deps.querier
        .query_wasm_smart(
            stability_pool_addr.clone(), 
            &ultra_base::stability_pool::QueryMsg::GetTotalUltraDeposits {  }
        )?;
        
    let recovery_mode_at_start = check_recovery_mode(
        &deps.querier, 
        price,
        active_pool_addr.clone(),
        default_pool_addr.clone()
    );

    // Perform the appropriate liquidation sequence - tally the values, and obtain their totals
    // if recovery_mode_at_start {
    //     totals = {
    //         let single_liquidation = LiquidationValues::default();
    //         let entirl_system_debt = query_entire_system_debt(
    //             &deps.querier, 
    //             active_pool_addr,
    //             default_pool_addr)?;
    //         let entire_system_coll = query_entire_system_coll(
    //             &deps.querier, 
    //             active_pool_addr,
    //             default_pool_addr)?;
            
    //         let user: Option<Addr> = deps.querier
    //             .query_wasm_smart(
    //                 sorted_troves_addr, 
    //                 &ultra_base::sorted_troves::QueryMsg::GetLast {  })?;
    //         let first_user = deps.querier
    //             .query_wasm_smart(
    //                 sorted_troves_addr, 
    //                 &ultra_base::sorted_troves::QueryMsg::GetFirst {  })?;
    //         for _ in ..n{

    //         }
    //         false
    //     }
    // }
    if recovery_mode_at_start {
        let mut back_to_normal_mode = false;
        let mut remain_ultra_in_stability_pool = ultra_in_stability_pool;
        let mut single_liquidation;
        let mut entire_system_debt = query_entire_system_debt(
            &deps.querier, 
            active_pool_addr.clone(),
            default_pool_addr.clone())?;
        let mut entire_system_coll = query_entire_system_coll(
            &deps.querier, 
            active_pool_addr.clone(),
            default_pool_addr.clone())?;
        let mut msg;
        for borrower_addr in borrowers_addr {
            let icr = get_current_icr(deps.as_ref(), borrower_addr.to_string(), price)?;
            if !back_to_normal_mode{
                // Skip this trove if ICR is greater than MCR and Stability Pool is empty
                if icr >= MCR && remain_ultra_in_stability_pool.is_zero() { continue }
        
                let tcr = compute_cr(entire_system_coll, entire_system_debt, price)?;

                
                (single_liquidation, msg) = liquidate_recovery_mode(
                    deps.branch(), 
                    borrower_addr, 
                    icr, 
                    remain_ultra_in_stability_pool, 
                    tcr, 
                    price
                )?;
                if msg.is_some() {
                    cosmos_msg.push(msg.unwrap());
                }

                // Update aggregate trackers
                remain_ultra_in_stability_pool = remain_ultra_in_stability_pool
                    .checked_sub(single_liquidation.debt_to_offset)
                    .map_err(StdError::overflow)?;
                entire_system_debt = entire_system_debt 
                    .checked_sub(single_liquidation.debt_to_offset)
                    .map_err(StdError::overflow)?;
                entire_system_coll = entire_system_coll
                    .checked_sub(single_liquidation.coll_to_send_to_sp)
                    .map_err(StdError::overflow)?;

                // Add liquidation values to their respective running totals
                totals = add_liquidation_values_to_totals(totals, single_liquidation)?;

                back_to_normal_mode = !(compute_cr(entire_system_coll, entire_system_debt, price)? < CCR)
            } else if icr < MCR {
                (single_liquidation, msg) = liquidate_normal_mode(
                    deps.branch(), 
                    borrower_addr, 
                    remain_ultra_in_stability_pool, 
                )?;
                if msg.is_some() {
                    cosmos_msg.push(msg.unwrap());
                }

                remain_ultra_in_stability_pool = remain_ultra_in_stability_pool
                    .checked_sub(single_liquidation.debt_to_offset)
                    .map_err(StdError::overflow)?;

                // Add liquidation values to their respective running totals
                totals = add_liquidation_values_to_totals(totals, single_liquidation)?;
            } // In Normal Mode skip troves with ICR >= MCR
        }
    } else {
        let mut msg;

        let mut single_liquidation;
        let mut remain_ultra_in_stability_pool = ultra_in_stability_pool;

        for borrower_addr in borrowers_addr {
            let icr = get_current_icr(deps.as_ref(), borrower_addr.to_string(), price)?;
            if icr < MCR {
                (single_liquidation, msg) = liquidate_normal_mode(
                    deps.branch(), borrower_addr, remain_ultra_in_stability_pool)?;
                if msg.is_some() {
                    cosmos_msg.push(msg.unwrap());
                }
                remain_ultra_in_stability_pool = remain_ultra_in_stability_pool
                    .checked_sub(single_liquidation.debt_to_offset)
                    .map_err(StdError::overflow)?;
                  
                // Add liquidation values to their respective running totals
                totals = add_liquidation_values_to_totals(totals, single_liquidation)?;
            }
        }
    }

    if totals.total_debt_in_sequence.is_zero() {
        return Err(ContractError::NothingToLiquidate {  })
    }

    // Move liquidated ETH and LUSD to the appropriate pools
    let offset_msg: CosmosMsg = WasmMsg::Execute { 
        contract_addr: stability_pool_addr.to_string(), 
        msg: to_binary(&ultra_base::stability_pool::ExecuteMsg::Offset {  })?, 
        funds: vec![] 
    }.into();
    cosmos_msg.push(offset_msg);

    let res = Response::new()
        .add_attribute("action", "batch_liquidate_troves")
        .add_messages(cosmos_msg);
    Ok(res)
}

pub fn execute_redeem_collateral(
    mut deps: DepsMut, 
    env: Env, 
    info: MessageInfo, 
    ultra_amount: Uint128,
    first_redemption_hint: Option<String>,
    upper_partial_redemption_hint: String,
    lower_partial_redemption_hint: String,
    partial_redemption_hint_nicr: Decimal,
    max_iterations: Uint128,
    max_fee_percentage: Decimal,
) -> Result<Response, ContractError> {
    let api = deps.api;
    let first_redemption_hint = maybe_addr(api, first_redemption_hint)?;

    let oracle_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::PriceFeed)?;
    let default_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::DefaultPool)?;
    let active_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::ActivePool)?;
    let ultra_token_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::UltraToken)?;
    let sorted_troves_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::SortedTroves)?;
    let surplus_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::CollateralSurplusPool)?;
    if max_fee_percentage >= Decimal::permille(5) &&  max_fee_percentage <= Decimal::one() {
        return Err(ContractError::MaxFeePercentageInvalid {  })
    }

    let manager = MANAGER.load(deps.storage)?;
    // TODO: Fix some related to LQTY token and bootstrap

    let price: Decimal = deps.querier
        .query_wasm_smart(
            oracle_addr, 
            &ultra_base::oracle::QueryMsg::ExchangeRate { 
                denom: ultra_base::oracle::NATIVE_JUNO_DENOM.to_string()
            }
        )?;
    if get_tcr(&deps.querier, price, default_pool_addr.clone(), active_pool_addr.clone())? < MCR {
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
    let mut current_borrower: Option<Addr>;

    if is_valid_first_redemption_hint(
        deps.as_ref(), 
        sorted_troves_addr.clone(), 
        first_redemption_hint.clone(), 
        price)? {
        current_borrower = first_redemption_hint.clone();
    } else {
        current_borrower = deps.querier
            .query_wasm_smart(
                sorted_troves_addr.to_string(), 
                &ultra_base::sorted_troves::QueryMsg::GetLast {  } 
            )?;
        // Find the first trove with ICR >= MCR
        while current_borrower.clone().is_some() 
            && get_current_icr(deps.as_ref(), current_borrower.clone().unwrap().to_string(), price)? < MCR {
            
            current_borrower = deps.querier
                .query_wasm_smart(
                    sorted_troves_addr.to_string(), 
                    &ultra_base::sorted_troves::QueryMsg::GetPrev { 
                        id: current_borrower.unwrap().to_string()
                    } 
                )?;
        }
    }

    let mut max_iterations = max_iterations;
    let mut cosmos_msgs = vec![];
    let querier = deps.querier; 
    // Loop through the Troves starting from the one with lowest collateral ratio until _amount of LUSD is exchanged for collateral
    while current_borrower.is_some() 
        && totals.remaining_ultra_debt > Uint128::zero() 
        && max_iterations > Uint128::zero() {
        max_iterations = max_iterations.checked_sub(Uint128::one()).map_err(StdError::overflow)?;
        
        // Save the address of the Trove preceding the current one, before potentially modifying the list
        let sorted_troves_addr = sorted_troves_addr.clone();
        let next_user_to_check = querier
            .query_wasm_smart(
                sorted_troves_addr.to_string(), 
                &ultra_base::sorted_troves::QueryMsg::GetPrev { 
                    id: current_borrower.clone().unwrap().to_string()
                } 
            )?;
        
        let msgs = apply_pending_rewards(
            deps.branch(), 
            env.clone(), 
            info.clone(), 
            current_borrower.clone().unwrap(), 
            active_pool_addr.clone(),
            default_pool_addr.clone())?;

        for msg in msgs {
            cosmos_msgs.push(msg);
        }

        let mut single_redeemtion = SingleRedemptionValues::default();
        let current_idx = TROVE_OWNER_IDX.load(deps.storage, current_borrower.clone().unwrap())?;
        let mut current_trove = TROVES.load(deps.storage, current_idx.to_string())?.1;

        // Determine the remaining amount (lot) to be redeemed, capped by the entire debt of the Trove minus the gas compensation
        single_redeemtion.ultra_debt_lot = Uint128::min(
            totals.remaining_ultra_debt, 
            current_trove.debt
                .checked_sub(ULTRA_GAS_COMPENSATE)
                .map_err(StdError::overflow)?
        );
        // Get the ETHLot of equivalent value in USD
        single_redeemtion.juno_lot = single_redeemtion.ultra_debt_lot
            .checked_mul(Uint128::new(10u128).pow(18))
            .map_err(StdError::overflow)?
            .checked_div(totals.price.atomics())
            .map_err(StdError::divide_by_zero)?;
        
        // Decrease the debt and collateral of the current Trove according to the Ultra lot and corresponding Juno to send
        let new_coll = current_trove.debt
            .checked_sub(single_redeemtion.ultra_debt_lot)
            .map_err(StdError::overflow)?;
        let new_debt = current_trove.coll
            .checked_sub(single_redeemtion.juno_lot)
            .map_err(StdError::overflow)?;
        
        if new_debt == ULTRA_GAS_COMPENSATE {
            // No debt left in the Trove (except for the gas compensation), therefore the trove gets closed
            // TODO:
            remove_stake(deps.branch(),  current_borrower.clone().unwrap())?;
            close_trove(deps.branch(), current_borrower.clone().unwrap(), sorted_troves_addr)?;
            
             // TODO: update burn function
            // let burn_msg: CosmosMsg = CosmosMsg::Wasm(
            //     WasmMsg::Execute { 
            //         contract_addr: ultra_token_addr.to_string(), 
            //         msg: to_binary(&ultra_token::msg::ExecuteMsg::Burn { 
            //             amount: totals.total_ultra_debt_to_redeem
            //         })?,
            //         funds: vec![] 
            //     }
            // );
            // cosmos_msgs.push(burn_msg);
            
            // Update Active Pool Ultra, and send Juno to account
            let decrease_ultra_debt_msg: CosmosMsg = CosmosMsg::Wasm(
                    WasmMsg::Execute { 
                        contract_addr: active_pool_addr.to_string(), 
                        msg: to_binary(&ultra_base::active_pool::ExecuteMsg::DecreaseULTRADebt { 
                            amount: ULTRA_GAS_COMPENSATE
                        })?,
                        funds: vec![] 
                    }
                );
            cosmos_msgs.push(decrease_ultra_debt_msg);

            // send Juno from Active Pool to CollSurplus Pool
            let surplus_pool_addr = surplus_pool_addr.clone();
            let account_surplus_msg: CosmosMsg = CosmosMsg::Wasm(
                WasmMsg::Execute { 
                    contract_addr: surplus_pool_addr.to_string(), 
                    msg: to_binary(&ultra_base::coll_surplus_pool::ExecuteMsg::AccountSurplus { 
                        account: current_borrower.clone().unwrap(), 
                        amount: new_coll
                    })?,
                    funds: vec![] 
                }
            );
            cosmos_msgs.push(account_surplus_msg);
            let send_juno_msg: CosmosMsg = CosmosMsg::Wasm(
                WasmMsg::Execute { 
                    contract_addr: active_pool_addr.to_string(), 
                    msg: to_binary(&ultra_base::active_pool::ExecuteMsg::SendJUNO { 
                        recipient: surplus_pool_addr, 
                        amount: new_coll
                    })?,
                    funds: vec![] 
                }
            );
            cosmos_msgs.push(send_juno_msg); 
        } else {
            let new_nicr = compute_nominal_cr(new_coll, new_debt)?;
            /*
            * If the provided hint is out of date, we bail since trying to reinsert without a good hint will almost
            * certainly result in running out of gas. 
            *
            * If the resultant net debt of the partial is less than the minimum, net debt we bail.
            */
            if new_nicr != partial_redemption_hint_nicr || 
                new_debt.checked_sub(ULTRA_GAS_COMPENSATE)
                    .map_err(StdError::overflow)? <= MIN_NET_DEBT {
                        single_redeemtion.cancelled_partial = true;
                        break;
                    } else {
                        let reinsert_msg: CosmosMsg = CosmosMsg::Wasm(
                            WasmMsg::Execute { 
                                contract_addr: sorted_troves_addr.to_string(), 
                                msg: to_binary(&ultra_base::sorted_troves::ExecuteMsg::ReInsert { 
                                    id: current_borrower.clone().unwrap().to_string(), 
                                    new_nicr: new_nicr.atomics(), 
                                    prev_id: Some(upper_partial_redemption_hint.clone()),
                                    next_id: Some(lower_partial_redemption_hint.clone()) })?, 
                                funds: vec![] 
                            }
                        );
                        cosmos_msgs.push(reinsert_msg);

                        current_trove.debt = new_debt;
                        current_trove.coll = new_coll;
                        TROVES.save(deps.branch().storage, current_idx.to_string(), &(
                            current_borrower.clone().unwrap(),
                            current_trove
                        ))?;

                        update_stake_and_total_stakes(deps.branch(), current_borrower.clone().unwrap())?;
                    }
        }

        totals.total_ultra_debt_to_redeem = totals.total_ultra_debt_to_redeem
            .checked_add(single_redeemtion.ultra_debt_lot)
            .map_err(StdError::overflow)?;
        totals.total_juno_drawn = totals.total_juno_drawn
            .checked_add(single_redeemtion.juno_lot)
            .map_err(StdError::overflow)?;
        totals.remaining_ultra_debt = totals.remaining_ultra_debt
            .checked_sub(single_redeemtion.ultra_debt_lot)
            .map_err(StdError::overflow)?;

        current_borrower = next_user_to_check;
    }

    if totals.total_juno_drawn.is_zero() {
        return Err(ContractError::UnableToRedeem {  });
    }

    // Decay the base_rate due to time passed, and then increase it according to the size of this redemption.
    // Use the saved total UltraDebt supply value, from before it was reduced by the redemption.
    update_base_rate_from_redeemtion(deps, env, totals.clone())?;

    
    // Calculate the juno fee
    let total_juno_drawn = Decimal::new(totals.total_juno_drawn);
    let redemption_fee = Decimal::min(
        REDEMPTION_FEE_FLOOR
            .checked_add(manager.base_rate)
            .map_err(StdError::overflow)?, 
        Decimal::one())
        .checked_mul(total_juno_drawn)
        .map_err(StdError::overflow)?;

    if redemption_fee < total_juno_drawn {
        return Err(ContractError::FeeEatUpAllReturns {  });
    }
    totals.juno_fee = redemption_fee.atomics()
        .checked_div(Uint128::from(10u128).pow(18))
        .map_err(StdError::divide_by_zero)?;
    
    // require user accept fee
    let fee_percentage = Decimal::from_ratio(
        totals.juno_fee, 
        totals.total_juno_drawn
    );
    if fee_percentage > max_fee_percentage {
        return  Err(ContractError::FeeIsNotAccepted {  });
    }
    
    // TODO: Fix some related to LQTY token and staking contracts

    totals.juno_to_send_to_redeemer = totals.total_juno_drawn - totals.juno_fee;
    
    // Burn the total UltraDebt that is cancelled with debt, and send the redeemed Juno to info.sender
    // TODO: update burn function
    // let burn_msg: CosmosMsg = CosmosMsg::Wasm(
    //     WasmMsg::Execute { 
    //         contract_addr: ultra_token_addr.to_string(), 
    //         msg: to_binary(&ultra_token::msg::ExecuteMsg::Burn { 
    //             amount: totals.total_ultra_debt_to_redeem
    //         })?,
    //         funds: vec![] 
    //     }
    // );
    // cosmos_msgs.push(burn_msg);
    
    // Update Active Pool, and send juno to info.sender
    let decrease_debt_msg: CosmosMsg = CosmosMsg::Wasm(
        WasmMsg::Execute {
            contract_addr: active_pool_addr.to_string(),
            msg: to_binary(&ultra_base::active_pool::ExecuteMsg::DecreaseULTRADebt { 
                amount: totals.total_ultra_debt_to_redeem
            })?,
            funds: vec![]
        }
    );
    cosmos_msgs.push(decrease_debt_msg);

    let send_juno_msg: CosmosMsg = CosmosMsg::Wasm(
        WasmMsg::Execute {
            contract_addr: active_pool_addr.to_string(),
            msg: to_binary(&ultra_base::active_pool::ExecuteMsg::SendJUNO { 
                recipient: info.sender, 
                amount: totals.juno_to_send_to_redeemer
            })?,
            funds: vec![]
        }
    );
    cosmos_msgs.push(send_juno_msg);
    let res = Response::new()
        .add_attribute("action", "redeem_collateral")
        .add_attribute("first_redemption_hint", format!("{:?}",first_redemption_hint.map(|addr| addr.to_string())))
        .add_attribute("upper_partial_redemption_hint", upper_partial_redemption_hint)
        .add_attribute("lower_partial_redemption_hint", lower_partial_redemption_hint)
        .add_messages(cosmos_msgs);
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
    remove_stake(deps, borrower_addr)?;
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

    let borrower_addr = deps.api.addr_validate(&borrower)?;
    let (new_stake, new_total_stake) = update_stake_and_total_stakes(deps, borrower_addr)?;

    let res = Response::new()
        .add_attribute("action", "update_stake_and_total_stakes")
        .add_attribute("borrower", borrower)
        .add_attribute("new_stake", new_stake.to_string())
        .add_attribute("new_total_stake", new_total_stake.to_string());
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
    let sorted_troves = ROLE_CONSUMER
        .load_role_address(
            deps.as_ref(), 
            Role::SortedTroves
    )?;
    close_trove(deps, borrower_addr.clone(), sorted_troves.clone())?;

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

    let decay_base_rate =  calc_decayed_base_rate(deps.as_ref(), env.clone())?;
    if decay_base_rate > Decimal::one() {
        return Err(ContractError::DecayBaseRateLargerThanOne {})
    }
    manager.base_rate = decay_base_rate;

    let last_fee_operation_time = manager.last_fee_operation_time.nanos();
    let time_pass : u64 = env.block.time.nanos() - last_fee_operation_time;
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

fn update_stake_and_total_stakes(
    deps: DepsMut,
    borrower_addr: Addr
) -> Result<(Uint128, Uint128), ContractError> {
    let mut manager = MANAGER.load(deps.storage)?;

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
    Ok((new_stake, manager.total_stake))
}

fn close_trove(
    deps: DepsMut, 
    borrower_addr: Addr,
    sorted_troves: Addr
) -> Result<(), ContractError> {
    let trove_count = MANAGER.load(deps.storage)?.trove_owner_count;

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
    Ok(()) 
}
fn remove_stake(
    deps: DepsMut, 
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

fn calc_decayed_base_rate(deps: Deps, env: Env) -> StdResult<Decimal>{
    // Half-life of 12h. 12h = 720 min
    // (1/2) = d^720 => d = (1/2)^(1/720)
    // 18 digit of decimal places
    let minute_decay_factor: Decimal = Decimal::from_str("0.999037758833783388")?;
    
    let manager = MANAGER.load(deps.storage)?;
    let last_fee_operation_time = manager.last_fee_operation_time.nanos();
    let base_rate = manager.base_rate;
    
    let time_pass : u64 = env.block.time.nanos() - last_fee_operation_time;
    let minus_pass = time_pass / ONE_MINUTE;

    // calculate new base rate
    let decay_factor: Decimal = dec_pow(minute_decay_factor, minus_pass)?;
    let decay_base_rate =  base_rate.saturating_mul(decay_factor);
    Ok(decay_base_rate)
}
pub fn current_trove_amounts(deps: Deps, borrower_addr: Addr) -> Result<(Uint128, Uint128), StdError> {
    let pending_juno_reward = get_pending_juno_reward(deps, borrower_addr.clone())?;
    let pending_ultra_debt_reward = get_pending_ultra_debt_reward(deps, borrower_addr.clone())?;

    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr.clone())?;
    let (_, trove) = TROVES.load(deps.storage, trove_idx.to_string())?;
    
    let current_juno = trove.coll
        .checked_add(pending_juno_reward)
        .map_err(StdError::overflow)?;
    
    let current_ultra_debt = trove.debt
        .checked_add(pending_ultra_debt_reward)
        .map_err(StdError::overflow)?;    
    
    return Ok((current_juno, current_ultra_debt));
}

fn update_base_rate_from_redeemtion(
    deps: DepsMut, 
    env: Env, 
    totals: RedemptionTotals
) -> Result<(), ContractError> {
    let mut manager = MANAGER.load(deps.storage)?;
    let decay_base_rate = calc_decayed_base_rate(deps.as_ref(), env.clone())?;
    /* Convert the drawn ETH back to LUSD at face value rate (1 LUSD:1 USD), in order to get
     * the fraction of total supply that was redeemed at face value. */
    let redeemed_ultra_debt_fraction = Decimal::from_ratio(
        totals.total_juno_drawn
            .checked_mul(Decimal::atomics(&totals.price))
            .map_err(StdError::overflow)?, 
        totals.total_ultra_debt_supply_at_start
            .checked_mul(Uint128::from(10u128).pow(18))
            .map_err(StdError::overflow)?
    );

    let mut new_base_rate = Decimal::from_ratio(
        Decimal::atomics(
            &decay_base_rate
                .checked_add(redeemed_ultra_debt_fraction)
                .map_err(StdError::overflow)?
        ),
        Uint128::new(BETA as u128)
    );
    // cap baseRate at a maximum of 100%
    new_base_rate = Decimal::min(new_base_rate, Decimal::one());
    //assert(new_base_rate <= 1); // This is already enforced in the line above
    if new_base_rate.is_zero() {
        return  Err(ContractError::BaseRateIsZero {  });
    }

    // Update the base rate state variable
    manager.base_rate = new_base_rate;

    let last_fee_operation_time = manager.last_fee_operation_time.nanos();
    let time_pass : u64 = env.block.time.nanos() - last_fee_operation_time;
    // Update last fee operation time 
    if time_pass >= ONE_MINUTE {
        manager.last_fee_operation_time = env.block.time;
    }

    MANAGER.save(deps.storage, &manager)?;
    Ok(())
}

fn liquidate_recovery_mode(
    mut deps: DepsMut,
    user: Addr,
    icr: Decimal,
    remain_ultra_in_stability_pool: Uint128,
    tcr: Decimal,
    price: Decimal
) -> Result<(LiquidationValues, Option<CosmosMsg>), ContractError>{
    let manager = MANAGER.load(deps.storage)?;
    let mut msg: Option<CosmosMsg> = None;

    let mut single_liquidation = LiquidationValues::default();
    if manager.trove_owner_count < Uint128::one() {
        return Ok((single_liquidation, msg))
    }

    let active_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::ActivePool)?;
    let default_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::DefaultPool)?;
    let sorted_troves_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::SortedTroves)?;
    let coll_surplus_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::CollateralSurplusPool)?;

    let entire_debt_and_coll = get_entire_debt_and_coll(deps.as_ref(), user.clone())?;
    single_liquidation.entire_trove_debt = entire_debt_and_coll.debt;
    single_liquidation.entire_trove_coll = entire_debt_and_coll.coll;
    single_liquidation.coll_gas_compensation = single_liquidation.entire_trove_coll
        .checked_div(Uint128::from(PERCENT_DIVISOR as u128))
        .map_err(StdError::divide_by_zero)?;
    single_liquidation.ultra_gas_compensation = ULTRA_GAS_COMPENSATE;
    
    let coll_to_liquidate = single_liquidation.entire_trove_coll
        .checked_sub(single_liquidation.coll_gas_compensation)
        .map_err(StdError::overflow)?;
    
    // If ICR <= 100%, purely redistribute the Trove across all active Troves
    if icr <= Decimal::one(){
        move_pending_trove_rewards_to_active_pool(
            active_pool_addr, 
            default_pool_addr, 
            entire_debt_and_coll.pending_ultra_debt_reward, 
            entire_debt_and_coll.pending_juno_reward)?;

        remove_stake(deps.branch(), user.clone())?;

        single_liquidation.debt_to_redistribute = single_liquidation.entire_trove_debt;
        single_liquidation.coll_to_redistribute = coll_to_liquidate;

        close_trove(deps.branch(), user.clone(), sorted_troves_addr)?;
    }
    // If 100% < ICR < MCR, offset as much as possible, and redistribute the remainder
    else if icr < MCR {
        move_pending_trove_rewards_to_active_pool(
            active_pool_addr, 
            default_pool_addr, 
            entire_debt_and_coll.pending_ultra_debt_reward, 
            entire_debt_and_coll.pending_juno_reward)?;

        remove_stake(deps.branch(), user.clone())?;
        
        if remain_ultra_in_stability_pool.is_zero() {
            single_liquidation.debt_to_redistribute = single_liquidation.entire_trove_debt;  
            single_liquidation.coll_to_redistribute = coll_to_liquidate;
        } else {
            /*
            * Offset as much debt & collateral as possible against the Stability Pool, and redistribute the remainder
            * between all active troves.
            *
            *  If the trove's debt is larger than the deposited LUSD in the Stability Pool:
            *
            *  - Offset an amount of the trove's debt equal to the LUSD in the Stability Pool
            *  - Send a fraction of the trove's collateral to the Stability Pool, equal to the fraction of its offset debt
            *
            */
            single_liquidation.debt_to_offset = Uint128::min(
                single_liquidation.entire_trove_debt, 
                remain_ultra_in_stability_pool);
            single_liquidation.coll_to_send_to_sp = coll_to_liquidate
                .checked_mul(single_liquidation.debt_to_offset)
                .map_err(StdError::overflow)?
                .checked_div(single_liquidation.entire_trove_debt)
                .map_err(StdError::divide_by_zero)?;
            single_liquidation.debt_to_redistribute = single_liquidation.entire_trove_debt
                .checked_sub(single_liquidation.debt_to_offset)
                .map_err(StdError::overflow)?;  
            single_liquidation.coll_to_redistribute = coll_to_liquidate
                .checked_sub(single_liquidation.coll_to_send_to_sp)
                .map_err(StdError::overflow)?;
        }
        close_trove(deps.branch(), user.clone(), sorted_troves_addr)?;
    }
    /*
        * If 110% <= ICR < current TCR (accounting for the preceding liquidations in the current sequence)
        * and there is LUSD in the Stability Pool, only offset, with no redistribution,
        * but at a capped rate of 1.1 and only if the whole debt can be liquidated.
        * The remainder due to the capped rate will be claimable as collateral surplus.
        */
    else if single_liquidation.entire_trove_debt <= remain_ultra_in_stability_pool
        && icr < tcr {
        move_pending_trove_rewards_to_active_pool(
            active_pool_addr, 
            default_pool_addr, 
            entire_debt_and_coll.pending_ultra_debt_reward, 
            entire_debt_and_coll.pending_juno_reward)?;
        if remain_ultra_in_stability_pool.is_zero(){
            return Err(ContractError::RemainUltraInStabilityPoolIsZero {  })
        }
        remove_stake(deps.branch(), user.clone())?;
        single_liquidation = capped_offset_vals(
            single_liquidation.entire_trove_debt, 
            single_liquidation.entire_trove_coll, 
            price)?;
        close_trove(deps.branch(), user.clone(), sorted_troves_addr)?;
        if single_liquidation.coll_surplus.is_zero() {
            msg = Some(
                WasmMsg::Execute { 
                    contract_addr: coll_surplus_pool_addr.to_string(), 
                    msg: to_binary(&ultra_base::coll_surplus_pool::ExecuteMsg::AccountSurplus { 
                        account: user, 
                        amount: single_liquidation.coll_surplus })?, 
                    funds: vec![] }.into()
            )
        }
    }
    Ok((single_liquidation, msg))
}

fn liquidate_normal_mode(
    mut deps: DepsMut,
    user: Addr,
    remain_ultra_in_stability_pool: Uint128
)-> Result<(LiquidationValues, Option<CosmosMsg>), ContractError>{
    let msg: Option<CosmosMsg> = None;

    let active_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::ActivePool)?;
    let default_pool_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::DefaultPool)?;
    let sorted_troves_addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::SortedTroves)?;

    let mut single_liquidation = LiquidationValues::default();
    let entire_debt_and_coll = get_entire_debt_and_coll(deps.as_ref(), user.clone())?;

    // TODO: add msg to cosmos_msg
    move_pending_trove_rewards_to_active_pool(
        active_pool_addr, 
        default_pool_addr, 
        entire_debt_and_coll.pending_ultra_debt_reward, 
        entire_debt_and_coll.pending_juno_reward)?;
    remove_stake(deps.branch(), user.clone())?;

    single_liquidation.coll_gas_compensation =  single_liquidation.entire_trove_coll
        .checked_div(Uint128::from(PERCENT_DIVISOR as u128))
        .map_err(StdError::divide_by_zero)?;
    single_liquidation.ultra_gas_compensation = ULTRA_GAS_COMPENSATE;
    let coll_to_liquidate = single_liquidation.entire_trove_coll
        .checked_sub(single_liquidation.coll_gas_compensation)
        .map_err(StdError::overflow)?;
    
    if remain_ultra_in_stability_pool.is_zero() {
        single_liquidation.debt_to_redistribute = single_liquidation.entire_trove_debt;  
        single_liquidation.coll_to_redistribute = coll_to_liquidate;
    } else {
        /*
        * Offset as much debt & collateral as possible against the Stability Pool, and redistribute the remainder
        * between all active troves.
        *
        *  If the trove's debt is larger than the deposited LUSD in the Stability Pool:
        *
        *  - Offset an amount of the trove's debt equal to the LUSD in the Stability Pool
        *  - Send a fraction of the trove's collateral to the Stability Pool, equal to the fraction of its offset debt
        *
        */
        single_liquidation.debt_to_offset = Uint128::min(
            single_liquidation.entire_trove_debt, 
            remain_ultra_in_stability_pool);
        single_liquidation.coll_to_send_to_sp = coll_to_liquidate
            .checked_mul(single_liquidation.debt_to_offset)
            .map_err(StdError::overflow)?
            .checked_div(single_liquidation.entire_trove_debt)
            .map_err(StdError::divide_by_zero)?;
        single_liquidation.debt_to_redistribute = single_liquidation.entire_trove_debt
            .checked_sub(single_liquidation.debt_to_offset)
            .map_err(StdError::overflow)?;  
        single_liquidation.coll_to_redistribute = coll_to_liquidate
            .checked_sub(single_liquidation.coll_to_send_to_sp)
            .map_err(StdError::overflow)?;
    }
    close_trove(deps.branch(), user.clone(), sorted_troves_addr)?;
    Ok((single_liquidation, msg))
}
fn capped_offset_vals(
    entire_trove_debt: Uint128,
    entire_trove_coll: Uint128,
    price: Decimal
) -> Result<LiquidationValues, ContractError> {
    let mut single_liquidation = LiquidationValues::default();

    single_liquidation.entire_trove_coll = entire_trove_coll;
    single_liquidation.entire_trove_debt = entire_trove_debt;

    let coll_to_offset = Decimal::from_ratio(
        Decimal::from_ratio(
            entire_trove_debt, 1u128)
            .checked_mul(MCR)
            .map_err(StdError::overflow)?
            .atomics(), 
        price.atomics());
    
    single_liquidation.coll_gas_compensation =  coll_to_offset
        .atomics()
        .checked_div(Uint128::from((PERCENT_DIVISOR * 10u8.pow(18)) as u128 ) )
        .map_err(StdError::divide_by_zero)?;    
    single_liquidation.ultra_gas_compensation = ULTRA_GAS_COMPENSATE;
    single_liquidation.debt_to_offset = entire_trove_debt;
    single_liquidation.coll_to_send_to_sp = coll_to_offset
        .checked_sub(Decimal::new(single_liquidation.ultra_gas_compensation))
        .map_err(StdError::overflow)?
        .atomics()
        .checked_div(Uint128::from(10u8.pow(18) as u128))
        .map_err(StdError::divide_by_zero)?;
    single_liquidation.coll_surplus = entire_trove_coll
        .checked_sub(coll_to_offset
            .atomics()
            .checked_div(Uint128::from(10u8.pow(18) as u128))
            .map_err(StdError::divide_by_zero)?
        )
        .map_err(StdError::overflow)?;
    
    Ok(single_liquidation)
}

fn add_liquidation_values_to_totals(
    old_totals: LiquidationTotals,
    single_liquidation: LiquidationValues
) -> Result<LiquidationTotals, StdError> {
    Ok(LiquidationTotals{
        total_coll_gas_compensation: old_totals.total_coll_gas_compensation
            .checked_add(single_liquidation.coll_gas_compensation)
            .map_err(StdError::overflow)?,
        total_ultra_gas_compensation: old_totals.total_ultra_gas_compensation
            .checked_add(single_liquidation.ultra_gas_compensation)
            .map_err(StdError::overflow)?,
        total_debt_in_sequence: old_totals.total_debt_in_sequence
            .checked_add(single_liquidation.entire_trove_debt)
            .map_err(StdError::overflow)?,
        total_coll_in_sequence: old_totals.total_coll_in_sequence
            .checked_add(single_liquidation.entire_trove_coll)
            .map_err(StdError::overflow)?,
        total_debt_to_offset: old_totals.total_debt_to_offset
            .checked_add(single_liquidation.debt_to_offset)
            .map_err(StdError::overflow)?,
        total_coll_to_send_to_sp: old_totals.total_coll_to_send_to_sp
            .checked_add(single_liquidation.coll_to_send_to_sp)
            .map_err(StdError::overflow)?,
        total_debt_to_redistribute: old_totals.total_debt_to_redistribute
            .checked_add(single_liquidation.debt_to_redistribute)
            .map_err(StdError::overflow)?,
        total_coll_to_redistribute: old_totals.total_coll_to_redistribute
            .checked_add(single_liquidation.coll_to_redistribute)
            .map_err(StdError::overflow)?,
        total_coll_surplus: old_totals.total_coll_surplus
            .checked_add(single_liquidation.coll_surplus)
            .map_err(StdError::overflow)?
    })
}

fn redistribute_debt_and_coll(
    deps: DepsMut,
    debt: Uint128,
    coll: Uint128
) -> Result<Vec<CosmosMsg>, ContractError> {
    if debt.is_zero() {
        return Ok(vec![])
    }
    let mut manager = MANAGER.load(deps.storage)?;
    /*
        * Add distributed coll and debt rewards-per-unit-staked to the running totals. Division uses a "feedback"
        * error correction, to keep the cumulative error low in the running totals L_ETH and L_LUSDDebt:
        *
        * 1) Form numerators which compensate for the floor division errors that occurred the last time this
        * function was called.
        * 2) Calculate "per-unit-staked" ratios.
        * 3) Multiply each ratio back by its denominator, to reveal the current floor division error.
        * 4) Store these errors for use in the next correction when this function is called.
        * 5) Note: static analysis tools complain about this "division before multiplication", however, it is intended.
        */
    
        let juno_numerator = coll
            .checked_add(manager.last_juno_error_redistribution)
            .map_err(StdError::overflow)?;
        let ultra_debt_numerator = debt
            .checked_add(manager.last_ultra_debt_error_redistribution)
            .map_err(StdError::overflow)?;
        
        // Get the per-unit-staked terms
        let juno_reward_per_unit_stake = Decimal::from_ratio(
            juno_numerator, manager.total_stake);
        let ultra_debt_reward_per_unit_stake = Decimal::from_ratio(
            ultra_debt_numerator, manager.total_stake); 
        
        manager.last_juno_error_redistribution = juno_numerator
            .checked_sub(juno_reward_per_unit_stake
                .checked_mul(Decimal::new(manager.total_stake))
                .map_err(StdError::overflow)?
                .atomics()
                .checked_div(Uint128::from(10u128.pow(18)))
                .map_err(StdError::divide_by_zero)?
            ).map_err(StdError::overflow)?;
        
        manager.last_ultra_debt_error_redistribution = juno_numerator
            .checked_sub(ultra_debt_reward_per_unit_stake
                .checked_mul(Decimal::new(manager.total_stake))
                .map_err(StdError::overflow)?
                .atomics()
                .checked_div(Uint128::from(10u128.pow(18)))
                .map_err(StdError::divide_by_zero)?
            ).map_err(StdError::overflow)?;
        
        // Add per-unit-staked terms to the running totals
        manager.total_liquidation_juno = manager.total_liquidation_juno
            .checked_add(
                juno_reward_per_unit_stake
                    .atomics()
                    .checked_div(Uint128::from(10u128.pow(18)))
                    .map_err(StdError::divide_by_zero)?
            ).map_err(StdError::overflow)?;
    Ok(vec![])
}
pub fn is_valid_first_redemption_hint(
    deps: Deps, 
    sorted_troves_addr: Addr, 
    first_redemption_hint: Option<Addr>, 
    price: Decimal
) -> StdResult<bool> { 
    if first_redemption_hint.is_none() {
        return Ok(false);
    }
    let first_redemption_hint = first_redemption_hint.unwrap();
    let contains: bool = deps.querier
        .query_wasm_smart(
            sorted_troves_addr.to_string(), 
            &ultra_base::sorted_troves::QueryMsg::Contains { 
                id:  first_redemption_hint.to_string()
            }
        )?;
    
    let current_icr = get_current_icr(deps, first_redemption_hint.to_string(), price)?;

    if !contains || current_icr < MCR {
        return Ok(false);
    }

    let next_trove: Option<Addr> = deps.querier
        .query_wasm_smart(
            sorted_troves_addr.to_string(), 
            &ultra_base::sorted_troves::QueryMsg::GetNext { 
                id:  first_redemption_hint.to_string()
            }
        )?;
    if next_trove.is_none() {
        return Ok(false);
    }
    Ok( get_current_icr(deps, next_trove.unwrap().to_string(), price)? < MCR)
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

pub fn get_current_icr(deps: Deps, borrower: String, price: Decimal) -> StdResult<Decimal>{
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    let (current_juno, current_ultra_debt) = current_trove_amounts(deps, borrower_addr)?;

    Ok(compute_cr(current_juno, current_ultra_debt, price)?)
}

pub fn get_current_nominal_icr(deps: Deps, borrower: String) -> StdResult<Decimal>{
    let borrower_addr = deps.api.addr_validate(&borrower)?;
    let (current_juno, current_ultra_debt) = current_trove_amounts(deps, borrower_addr)?;

    Ok(compute_nominal_cr(current_juno, current_ultra_debt)?)
}

pub fn get_entire_debt_and_coll(deps: Deps, borrower_addr: Addr) -> StdResult<EntireDebtAndCollResponse> {
    let trove_idx = TROVE_OWNER_IDX.load(deps.storage, borrower_addr.clone())?;
    let (_, trove) = TROVES.load(deps.storage, trove_idx.to_string())?;

    let mut debt = trove.debt;
    let mut coll = trove.coll;

    let pending_ultra_debt_reward = get_pending_ultra_debt_reward(deps, borrower_addr.clone())?;
    let pending_juno_reward = get_pending_juno_reward(deps, borrower_addr)?;

    debt = debt.checked_add(pending_ultra_debt_reward)
        .map_err(StdError::overflow)?;
    coll = coll.checked_add(pending_juno_reward)
        .map_err(StdError::overflow)?;

    Ok(EntireDebtAndCollResponse{
        debt,
        coll,
        pending_ultra_debt_reward,
        pending_juno_reward
    })
}