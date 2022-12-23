#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coin, to_binary, Addr, Attribute, BankMsg, Binary, CanonicalAddr, CosmosMsg, Decimal256,
    Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::maybe_addr;
use ultra_base::role_provider::Role;
use ultra_base::ultra_math::{compute_cr, compute_nominal_cr};
use ultra_base::{active_pool, trove_manager, ultra_token};
use ultra_controllers::roles::RolesError;

use crate::assert::{
    require_ICR_above_CCR, require_ICR_above_MCR, require_at_least_min_net_debt,
    require_newTCR_above_CCR, require_non_zero_adjustment, require_non_zero_debt_change,
    require_singular_coll_change, require_sufficient_ultra_balance, require_trove_is_active,
    require_valid_maxFeePercentage, require_valid_new_ICR_and_valid_new_TCR,
    require_valid_ultra_repayment,
};
use crate::error::ContractError;
use crate::helper::{
    get_coll_change, get_new_ICR_from_trove_change, get_new_TCR_from_trove_change,
    get_new_nominal_ICR_from_trove_change, get_new_trove_amount, trigger_borrowing_fee, withdraw_ultra_msgs, repay_ultra_msgs,
};
use crate::state::{SudoParams, ADMIN, ROLE_CONSUMER, SUDO_PARAMS};
use ultra_base::borrower_operations::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg};
use ultra_base::querier::{
    check_recovery_mode, query_entire_system_coll, query_entire_system_debt,
};

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

    let api = deps.api;
    ADMIN.set(deps.branch(), maybe_addr(api, Some(msg.owner.clone()))?)?;

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
        ExecuteMsg::UpdateAdmin { admin } => {
            Ok(ADMIN.execute_update_admin(deps, info, Some(admin))?)
        }
        ExecuteMsg::UpdateRole { role_provider } => {
            execute_update_role(deps, env, info, role_provider)
        }
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
            ultra_amount,
            upper_hint,
            lower_hint,
        } => execute_repay_ultra(deps, env, info, ultra_amount, upper_hint, lower_hint),
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
    }
}

pub fn execute_update_role(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    role_provider: Addr,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    ROLE_CONSUMER.add_role_provider(deps.storage, role_provider.clone())?;

    let res = Response::new()
        .add_attribute("action", "update_role")
        .add_attribute("role_provider_addr", role_provider);
    Ok(res)
}

pub fn execute_open_trove(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    max_fee_percentage: Decimal256,
    ultra_amount: Uint128,
    upper_hint: String,
    lower_hint: String,
) -> Result<Response, ContractError> {
    let mut attributes: Vec<Attribute> = vec![attr("action", "open_trove")];
    let res: Response = Response::new().add_attribute("action", "open_trove");

    if max_fee_percentage > Decimal256::zero() {
        require_valid_maxFeePercentage(max_fee_percentage);
    }

    // TODO: make this a function
    let oracle = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::PriceFeed)?;
    let price = deps.querier.query_wasm_smart(
        oracle,
        &to_binary(&ultra_base::oracle::QueryMsg::ExchangeRate {
            denom: "juno".to_string(), // TODO: hardcode.
        })?,
    )?;

    let active_pool = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::ActivePool)?;
    let default_pool: Addr = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::DefaultPool)?;
    let recovery_mode = check_recovery_mode(&deps.querier, price, active_pool, default_pool)?;

    let mut net_debt: Uint128 = ultra_amount;
    let ultra_fee: Uint128;

    if !recovery_mode && ultra_amount > Uint128::zero() {
        ultra_fee = trigger_borrowing_fee(
            deps.as_ref(),
            ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::TroveManager)?,
            ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::UltraToken)?,
            ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::UltraToken)?,
            ultra_amount,
            max_fee_percentage,
        )?;
        net_debt += ultra_fee;
    }

    require_at_least_min_net_debt(net_debt)?;

    let composite_debt: Uint128 = net_debt + Uint128::from(1950000000000000000000u64);

    let coin_denom = "juno".to_string();
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to bond", coin_denom))
        })?;

    let ICR = compute_cr(payment.amount, composite_debt, price)?;
    let NICR = compute_nominal_cr(payment.amount, composite_debt)?;
    let newTCR: Decimal256;
    if recovery_mode {
        require_ICR_above_CCR(ICR)?;
    } else {
        require_ICR_above_MCR(ICR)?;
        newTCR = get_new_TCR_from_trove_change(
            deps.as_ref(),
            payment.amount,
            true,
            composite_debt,
            true,
            price,
        )?;
        require_newTCR_above_CCR(ICR)?;
    }

    let mut messages = vec![];

    let trove_manager = ROLE_CONSUMER
        .load_role_address(deps.as_ref(), Role::TroveManager)?
        .to_string();

    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager,
        msg: to_binary(&trove_manager::ExecuteMsg::SetTroveStatus {
            borrower: info.sender.to_string(),
            num: Uint128::one(),
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager,
        msg: to_binary(&trove_manager::ExecuteMsg::IncreaseTroveColl {
            borrower: info.sender.to_string(),
            coll_increase: payment.amount,
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager,
        msg: to_binary(&trove_manager::ExecuteMsg::IncreaseTroveDebt {
            borrower: info.sender.to_string(),
            debt_increase: composite_debt,
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager,
        msg: to_binary(&trove_manager::ExecuteMsg::UpdateTroveRewardSnapshots {
            borrower: info.sender.to_string(),
        })?,
        funds: vec![],
    });

    // TODO: get Stake value callback from trove manager (to emit event)
    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager,
        msg: to_binary(&trove_manager::ExecuteMsg::UpdateStakeAndTotalStakes {
            borrower: info.sender.to_string(),
        })?,
        funds: vec![],
    });

    let sort_troves = ROLE_CONSUMER
        .load_role_address(deps.as_ref(), Role::SortedTroves)?
        .to_string();
    // TODO: Logic conflict. We need NICR to be Uint256 or Decimal256 ?

    deps.api.addr_validate(&upper_hint)?;
    deps.api.addr_validate(&lower_hint)?;
    messages.push(WasmMsg::Execute {
        contract_addr: sort_troves,
        msg: to_binary(&ultra_base::sorted_troves::ExecuteMsg::Insert {
            id: info.sender.to_string(),
            nicr: NICR,
            prev_id: Some(upper_hint),
            next_id: Some(lower_hint),
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager,
        msg: to_binary(&trove_manager::ExecuteMsg::AddTroveOwnerToArray {
            borrower: info.sender.to_string(),
        })?,
        funds: vec![],
    });

    let active_pool = ROLE_CONSUMER
        .load_role_address(deps.as_ref(), Role::ActivePool)?
        .to_string();
    // TODO: Confirm recipient.
    // TODO: Check if amount param is redundant
    // TODO: Check if calling the right function
    messages.push(WasmMsg::Execute {
        contract_addr: active_pool,
        msg: to_binary(&active_pool::ExecuteMsg::SendJUNO {
            recipient: info.sender,
            amount: payment.amount,
        })?,
        funds: vec![],
    });

    let ultra = ROLE_CONSUMER
        .load_role_address(deps.as_ref(), Role::UltraToken)?
        .to_string();
    if ultra_amount > Uint128::zero() {
        messages.append(&mut withdraw_ultra_msgs(
            deps.as_ref(),
            info.sender.to_string(),
            ultra_amount,
            net_debt,
        )?);
    }

    // TODO: add GasPool to consumer
    let gas_pool = ROLE_CONSUMER
        .load_role_address(deps.as_ref(), Role::GasPool)?
        .to_string();
    messages.append(&mut withdraw_ultra_msgs(
        deps.as_ref(),
        gas_pool,
        ultra_amount,
        net_debt,
    )?);

    // TODO: Add events for TroveCreated, TroveUpdated, UltraBorrowingFeePaid

    Ok(Response::new()
        .add_attributes(attributes)
        .add_messages(messages))
}

pub fn execute_adjust_trove(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    borrower: String,
    coll_withdrawal: Uint128,
    ultra_change: Uint128,
    is_debt_increase: bool,
    max_fee_percentage: Decimal256,
    upper_hint: String,
    lower_hint: String,
) -> Result<Response, ContractError> {
    let (msgs, attrs) = adjust_trove_msg(
        deps.as_ref(),
        env,
        info,
        borrower,
        coll_withdrawal,
        ultra_change,
        is_debt_increase,
        max_fee_percentage,
        upper_hint,
        lower_hint,
    )?;

    Ok(Response::new().add_messages(msgs).add_attributes(attrs))
}

pub fn execute_close_trove(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut attributes: Vec<Attribute> = vec![attr("action", "close_trove")];

    require_trove_is_active(
        deps.as_ref(),
        ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::TroveManager)?,
        info.sender,
    )?;
    let price: Decimal256 = deps.querier.query_wasm_smart(
        ROLE_CONSUMER
            .load_role_address(deps.as_ref(), Role::PriceFeed)?
            .to_string(),
        &to_binary(&ultra_base::oracle::QueryMsg::ExchangeRate {
            denom: "juno".to_string(), // TODO: hardcode.
        })?,
    )?;
    let active_pool = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::ActivePool)?;
    let default_pool = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::DefaultPool)?;
    let trove_manager = ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::TroveManager)?;
    if !check_recovery_mode(&deps.querier, price, active_pool, default_pool)? {
        return Err(ContractError::RecoveryMode {});
    }

    let mut messages: Vec<WasmMsg> = vec![];
    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager.to_string(),
        msg: to_binary(
            &ultra_base::trove_manager::ExecuteMsg::ApplyPendingRewards {
                borrower: info.sender.to_string(),
            },
        )?,
        funds: vec![],
    });

    let coll: Uint128 = deps.querier.query_wasm_smart(
        trove_manager,
        &to_binary(&ultra_base::trove_manager::QueryMsg::GetTroveColl {})?,
    )?;

    let debt: Uint128 = deps.querier.query_wasm_smart(
        trove_manager,
        &to_binary(&ultra_base::trove_manager::QueryMsg::GetTroveDebt {})?,
    )?;

    // TODO: replace 50_000_000u128 -> ultra_gas_compensation
    require_sufficient_ultra_balance(
        deps.as_ref(),
        info.sender.to_string(),
        debt.checked_sub(Uint128::from(50_000_000u128))
            .map_err(StdError::overflow)?,
    )?;

    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager.to_string(),
        msg: to_binary(&ultra_base::trove_manager::ExecuteMsg::RemoveStake {
            borrower: info.sender.to_string(),
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager.to_string(),
        msg: to_binary(&ultra_base::trove_manager::ExecuteMsg::CloseTrove {
            borrower: info.sender.to_string(),
        })?,
        funds: vec![],
    });

    messages.append(&mut repay_ultra_msgs(
        deps.as_ref(),
        info.sender.to_string(),
        debt.checked_sub(Uint128::from(50_000_000u128))
            .map_err(StdError::overflow)?,
    )?);

    // TODO: Add GasPool to ROLE_CONSUMER
    messages.append(&mut repay_ultra_msgs(
        deps.as_ref(),
        ROLE_CONSUMER
            .load_role_address(deps.as_ref(), Role::GasPool)?
            .to_string(),
        debt.checked_sub(Uint128::from(50_000_000u128))
            .map_err(StdError::overflow)?,
    )?);

    messages.push(WasmMsg::Execute {
        contract_addr: active_pool.to_string(),
        msg: to_binary(&ultra_base::active_pool::ExecuteMsg::SendJUNO {
            recipient: info.sender,
            amount: coll,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_attributes(attributes)
        .add_messages(messages))
}

pub fn execute_add_coll(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    upper_hint: String,
    lower_hint: String,
) -> Result<Response, ContractError> {
    let (msgs, attrs) = adjust_trove_msg(
        deps.as_ref(),
        env,
        info,
        info.sender.to_string(),
        Uint128::zero(),
        Uint128::zero(),
        false,
        Decimal256::zero(),
        upper_hint,
        lower_hint,
    )?;

    Ok(Response::new().add_attributes(attrs).add_messages(msgs))
}

pub fn execute_withdraw_coll(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    coll_withdrawal: Uint128,
    upper_hint: String,
    lower_hint: String,
) -> Result<Response, ContractError> {
    let (msgs, attrs) = adjust_trove_msg(
        deps.as_ref(),
        env,
        info,
        info.sender.to_string(),
        coll_withdrawal,
        Uint128::zero(),
        false,
        Decimal256::zero(),
        upper_hint,
        lower_hint,
    )?;

    Ok(Response::new().add_attributes(attrs).add_messages(msgs))
}

pub fn execute_claim_collateral(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let coll_surplus_pool: Addr =
        ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::CollateralSurplusPool)?;
    let messages = vec![WasmMsg::Execute {
        contract_addr: coll_surplus_pool.to_string(),
        msg: to_binary(&ultra_base::coll_surplus_pool::ExecuteMsg::ClaimColl {
            account: info.sender,
        })?,
        funds: vec![],
    }];

    // TODO: add events
    Ok(Response::new().add_messages(messages))
}

pub fn execute_repay_ultra(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ultra_amount: Uint128,
    upper_hint: String,
    lower_hint: String,
) -> Result<Response, ContractError> {
    let (msgs, attrs) = adjust_trove_msg(
        deps.as_ref(),
        env,
        info,
        info.sender.to_string(),
        Uint128::zero(),
        ultra_amount,
        false,
        Decimal256::zero(),
        upper_hint,
        lower_hint,
    )?;

    Ok(Response::new().add_attributes(attrs).add_messages(msgs))
}

pub fn execute_withdraw_ultra(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_fee_percentage: Decimal256,
    ultra_amount: Uint128,
    upper_hint: String,
    lower_hint: String,
) -> Result<Response, ContractError> {
    let (msgs, attrs) = adjust_trove_msg(
        deps.as_ref(),
        env,
        info,
        info.sender.to_string(),
        Uint128::zero(),
        ultra_amount,
        true,
        max_fee_percentage,
        upper_hint,
        lower_hint,
    )?;

    Ok(Response::new().add_attributes(attrs).add_messages(msgs))
}

pub fn execute_move_juno_gain_to_trove(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    borrower: String,
    upper_hint: String,
    lower_hint: String,
) -> Result<Response, ContractError> {
    if info.sender != ROLE_CONSUMER.load_role_address(deps.as_ref(), Role::StabilityPool)? {
        return Err(ContractError::UnauthorizedForRole(
            RolesError::UnauthorizedForRole {
                label: Role::StabilityPool.to_string(),
            },
        ));
    }

    let (msgs, attrs) = adjust_trove_msg(
        deps.as_ref(),
        env,
        info,
        info.sender.to_string(),
        Uint128::zero(),
        Uint128::zero(),
        false,
        Decimal256::zero(),
        upper_hint,
        lower_hint,
    )?;

    Ok(Response::new().add_attributes(attrs).add_messages(msgs))
}

fn adjust_trove_msg(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    borrower: String,
    coll_withdrawal: Uint128,
    ultra_change: Uint128,
    is_debt_increase: bool,
    max_fee_percentage: Decimal256,
    upper_hint: String,
    lower_hint: String,
) -> Result<(Vec<WasmMsg>, Vec<Attribute>), ContractError> {
    let mut attributes: Vec<Attribute> = vec![attr("action", "adjust_trove")];

    let coin_denom = "juno".to_string();
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to bond", coin_denom))
        })?;

    if is_debt_increase {
        require_valid_maxFeePercentage(max_fee_percentage)?;
        require_non_zero_debt_change(ultra_change)?;
    }
    require_singular_coll_change(coll_withdrawal, payment.amount)?;
    require_non_zero_adjustment(coll_withdrawal, ultra_change, payment.amount)?;

    if info.sender != borrower
        && (info.sender != ROLE_CONSUMER.load_role_address(deps, Role::StabilityPool)?
            || ultra_change != Uint128::zero())
    {
        return Err(ContractError::UnauthorizedOwner {});
    }

    let oracle = ROLE_CONSUMER.load_role_address(deps, Role::PriceFeed)?;
    let price = deps.querier.query_wasm_smart(
        oracle,
        &to_binary(&ultra_base::oracle::QueryMsg::ExchangeRate {
            denom: "juno".to_string(), // TODO: hardcode.
        })?,
    )?;

    let active_pool = ROLE_CONSUMER.load_role_address(deps, Role::ActivePool)?;
    let default_pool: Addr = ROLE_CONSUMER.load_role_address(deps, Role::DefaultPool)?;
    let recovery_mode = check_recovery_mode(&deps.querier, price, active_pool, default_pool)?;

    let trove_manager = ROLE_CONSUMER.load_role_address(deps, Role::TroveManager)?;
    let mut messages: Vec<WasmMsg> = vec![];

    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager.to_string(),
        msg: to_binary(
            &ultra_base::trove_manager::ExecuteMsg::ApplyPendingRewards { borrower: borrower },
        )?,
        funds: vec![],
    });

    let (coll_change, is_coll_increase) = get_coll_change(payment.amount, coll_withdrawal)?;
    let mut net_debt_change = ultra_change;

    if is_debt_increase && !recovery_mode {
        let ultra_fee = trigger_borrowing_fee(
            deps,
            ROLE_CONSUMER.load_role_address(deps, Role::TroveManager)?,
            ROLE_CONSUMER.load_role_address(deps, Role::UltraToken)?,
            ROLE_CONSUMER.load_role_address(deps, Role::ActivePool)?,
            ultra_change,
            max_fee_percentage,
        )?;
        net_debt_change = net_debt_change
            .checked_add(ultra_fee)
            .map_err(StdError::overflow)?;
    }

    // TODO: Logic conflict. Whether we need borrower param
    let debt = deps.querier.query_wasm_smart(
        trove_manager,
        &to_binary(&ultra_base::trove_manager::QueryMsg::GetTroveDebt {})?,
    )?;
    let coll = deps.querier.query_wasm_smart(
        trove_manager,
        &to_binary(&ultra_base::trove_manager::QueryMsg::GetTroveColl {})?,
    )?;

    let old_ICR = compute_cr(coll, debt, price)?;
    let new_ICR = get_new_ICR_from_trove_change(
        coll,
        debt,
        coll_change,
        true,
        net_debt_change,
        is_debt_increase,
        price,
    )?;

    if coll_withdrawal != Uint128::zero() || is_debt_increase {
        if coll_withdrawal > coll {
            // TODO: is there any better way to do this ?
            return Err(ContractError::Std(StdError::GenericErr {
                msg: "coll_withdrawal > coll".to_string(),
            }));
        }
        let new_TCR = get_new_TCR_from_trove_change(
            deps,
            coll_change,
            is_coll_increase,
            net_debt_change,
            is_debt_increase,
            price,
        )?;

        require_valid_new_ICR_and_valid_new_TCR(recovery_mode, old_ICR, new_ICR, new_TCR)?;
    }

    if !is_debt_increase && ultra_change > Uint128::zero() {
        require_at_least_min_net_debt(
            debt.checked_sub(net_debt_change)
                .map_err(StdError::overflow)?,
        )?;
        require_valid_ultra_repayment(debt, net_debt_change)?;
        require_sufficient_ultra_balance(deps, borrower, net_debt_change)?;
    }

    // TODO: need SubMsg implementation
    // let (new_coll: Uint128 , new_debt: Uint128) = update_trove_from_adjustment()?;
    messages.push(WasmMsg::Execute {
        contract_addr: trove_manager.to_string(),
        msg: to_binary(
            &ultra_base::trove_manager::ExecuteMsg::UpdateStakeAndTotalStakes {
                borrower: borrower,
            },
        )?,
        funds: vec![],
    });

    let new_NICR: Decimal256 = get_new_nominal_ICR_from_trove_change(
        coll,
        debt,
        coll_change,
        is_coll_increase,
        net_debt_change,
        is_debt_increase,
    )?;
    messages.push(WasmMsg::Execute {
        contract_addr: ROLE_CONSUMER
            .load_role_address(deps, Role::SortedTroves)?
            .to_string(),
        msg: to_binary(&ultra_base::sorted_troves::ExecuteMsg::ReInsert {
            id: borrower,
            new_nicr: new_NICR,
            prev_id: Some(upper_hint),
            next_id: Some(lower_hint),
        })?,
        funds: vec![],
    });

    if is_debt_increase {
        messages.append(&mut withdraw_ultra_msgs(
            deps,
            borrower,
            ultra_change,
            net_debt_change,
        )?);
    } else {
        messages.append(&mut repay_ultra_msgs(deps, borrower, ultra_change)?);
    }

    if is_coll_increase {
        // TODO: missing function in active pool
    } else {
        messages.push(WasmMsg::Execute {
            contract_addr: active_pool.to_string(),
            msg: to_binary(&ultra_base::active_pool::ExecuteMsg::SendJUNO {
                recipient: deps.api.addr_validate(&borrower)?,
                amount: coll_change,
            })?,
            funds: vec![],
        });
    }
    // TODO: emit events
    Ok((messages, attributes))
}

// TODO: missing query arms
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
