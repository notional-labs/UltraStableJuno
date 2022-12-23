use cosmwasm_std::{to_binary, Addr, Decimal256, Deps, StdError, Uint128, WasmMsg};
use ultra_base::{
    role_provider::Role,
    ultra_math::{compute_cr, compute_nominal_cr}, active_pool, ultra_token,
};

use crate::{state::ROLE_CONSUMER, ContractError};

pub fn trigger_borrowing_fee(
    deps: Deps,
    trove_manager: Addr,
    ultra_addr: Addr,
    lqty_staking: Addr,
    ultra_amount: Uint128,
    max_fee_percentage: Decimal256,
) -> Result<Uint128, ContractError> {
    let trove_manager = ROLE_CONSUMER.load_role_address(deps, Role::TroveManager)?;
    let mut msg = vec![];
    msg.push(WasmMsg::Execute {
        contract_addr: trove_manager.to_string(),
        msg: to_binary(&ultra_base::trove_manager::ExecuteMsg::DecayBaseRateFromBorrowing {})?,
        funds: vec![],
    });

    // TODO: implement "reply" to get UltraFee after calling "DecayBaseRateFromBorrowing"
    // TODO: do we really need lqtyStaking ?

    // let ultra_token = ROLE_CONSUMER.load_role_address(deps, Role::UltraToken)?;
    // msg.push(WasmMsg::Execute {
    //     contract_addr: ultra_token.to_string(),
    //     msg: to_binary(&ultra_base::ultra_token::ExecuteMsg::Mint { recipient: (), amount: () })?,
    //     funds: vec![],
    // });
    Ok(Uint128::zero())
}

pub fn get_coll_change(
    coll_received: Uint128,
    request_coll_withdrawal: Uint128,
) -> Result<(Uint128, bool), ContractError> {
    let coll_change: Uint128;
    let is_coll_increase: bool;
    if coll_received != Uint128::zero() {
        coll_change = coll_received;
        is_coll_increase = true;
    } else {
        coll_change = request_coll_withdrawal;
    }
    Ok((coll_change, is_coll_increase))
}

pub fn get_new_trove_amount(
    coll: Uint128,
    debt: Uint128,
    coll_change: Uint128,
    is_coll_increase: bool,
    debt_change: Uint128,
    is_debt_increase: bool,
) -> Result<(Uint128, Uint128), ContractError> {
    let mut new_coll = coll;
    let mut new_debt = coll;

    new_coll = if is_coll_increase {
        coll.checked_add(coll_change).map_err(StdError::overflow)?
    } else {
        coll.checked_sub(coll_change).map_err(StdError::overflow)?
    };

    new_debt = if is_debt_increase {
        debt.checked_add(debt_change).map_err(StdError::overflow)?
    } else {
        debt.checked_sub(debt_change).map_err(StdError::overflow)?
    };

    Ok((new_coll, new_debt))
}

pub fn get_new_nominal_ICR_from_trove_change(
    coll: Uint128,
    debt: Uint128,
    coll_change: Uint128,
    is_coll_increase: bool,
    debt_change: Uint128,
    is_debt_increase: bool,
) -> Result<Decimal256, ContractError> {
    let (new_coll, new_debt) = get_new_trove_amount(
        coll,
        debt,
        coll_change,
        is_coll_increase,
        debt_change,
        is_debt_increase,
    )?;

    let new_ICR = compute_nominal_cr(coll, debt)?;
    Ok(new_ICR)
}

pub fn get_new_TCR_from_trove_change(
    deps: Deps,
    coll_change: Uint128,
    is_coll_increase: bool,
    debt_change: Uint128,
    is_debt_increase: bool,
    price: Decimal256,
) -> Result<Decimal256, ContractError> {
    let active_pool = ROLE_CONSUMER.load_role_address(deps, Role::ActivePool)?;
    let default_pool = ROLE_CONSUMER.load_role_address(deps, Role::DefaultPool)?;

    let active_coll: Uint128 = deps.querier.query_wasm_smart(
        active_pool.to_string(),
        &to_binary(&ultra_base::active_pool::QueryMsg::GetJUNO {})?,
    )?;
    let liquidated_coll: Uint128 = deps.querier.query_wasm_smart(
        default_pool.to_string(),
        &to_binary(&ultra_base::default_pool::QueryMsg::GetJUNO {})?,
    )?;

    let mut total_coll: Uint128 = active_coll
        .checked_add(liquidated_coll)
        .map_err(StdError::overflow)?;

    let active_debt: Uint128 = deps.querier.query_wasm_smart(
        active_pool.to_string(),
        &to_binary(&ultra_base::active_pool::QueryMsg::GetULTRADebt {})?,
    )?;
    let closed_debt: Uint128 = deps.querier.query_wasm_smart(
        default_pool.to_string(),
        &to_binary(&ultra_base::default_pool::QueryMsg::GetULTRADebt {})?,
    )?;

    let mut total_debt: Uint128 = active_debt
        .checked_add(closed_debt)
        .map_err(StdError::overflow)?;

    total_coll = if is_coll_increase {
        total_coll
            .checked_add(coll_change)
            .map_err(StdError::overflow)?
    } else {
        total_coll
            .checked_sub(coll_change)
            .map_err(StdError::overflow)?
    };

    total_debt = if is_debt_increase {
        total_debt
            .checked_add(debt_change)
            .map_err(StdError::overflow)?
    } else {
        total_debt
            .checked_sub(debt_change)
            .map_err(StdError::overflow)?
    };

    let new_TCR = compute_cr(total_coll, total_debt, price)?;
    Ok(new_TCR)
}

pub fn get_new_ICR_from_trove_change(
    coll: Uint128,
    debt: Uint128,
    coll_change: Uint128,
    is_coll_increase: bool,
    debt_change: Uint128,
    is_debt_increase: bool,
    price: Decimal256,
) -> Result<Decimal256, ContractError> {
    let (new_coll, new_debt) = get_new_trove_amount(
        coll,
        debt,
        coll_change,
        is_coll_increase,
        debt_change,
        is_debt_increase,
    )?;

    let new_ICR = compute_cr(coll, debt, price)?;
    Ok(new_ICR)
}

fn get_newTCT_from_trove_change(
    deps: Deps,
    coll_change: Uint128,
    is_coll_increase: bool,
    debt_change: Uint128,
    is_debt_increase: bool,
    price: Decimal256,
) -> Result<Decimal256, ContractError> {
    let active_pool = ROLE_CONSUMER.load_role_address(deps, Role::ActivePool)?;
    let default_pool = ROLE_CONSUMER.load_role_address(deps, Role::DefaultPool)?;

    let active_coll: Uint128 = deps.querier.query_wasm_smart(
        active_pool,
        &to_binary(&ultra_base::active_pool::QueryMsg::GetJUNO {})?,
    )?;
    let liquidated_coll: Uint128 = deps.querier.query_wasm_smart(
        active_pool,
        &to_binary(&ultra_base::default_pool::QueryMsg::GetJUNO {})?,
    )?;

    let mut total_coll: Uint128 = active_coll
        .checked_add(liquidated_coll)
        .map_err(StdError::overflow)?;

    let active_debt: Uint128 = deps.querier.query_wasm_smart(
        active_pool,
        &to_binary(&ultra_base::active_pool::QueryMsg::GetULTRADebt {})?,
    )?;

    let closed_debt: Uint128 = deps.querier.query_wasm_smart(
        active_pool,
        &to_binary(&ultra_base::default_pool::QueryMsg::GetULTRADebt {})?,
    )?;
    let mut total_debt: Uint128 = active_debt
        .checked_add(closed_debt)
        .map_err(StdError::overflow)?;

    total_coll = if is_coll_increase {
        total_coll
            .checked_add(coll_change)
            .map_err(StdError::overflow)?
    } else {
        total_coll
            .checked_sub(coll_change)
            .map_err(StdError::overflow)?
    };

    total_debt = if is_debt_increase {
        total_debt
            .checked_add(total_debt)
            .map_err(StdError::overflow)?
    } else {
        total_debt
            .checked_sub(total_debt)
            .map_err(StdError::overflow)?
    };

    let newTCR: Decimal256 = compute_cr(total_coll, total_debt, price)?;
    Ok(newTCR)
}

pub fn withdraw_ultra_msgs(
    deps: Deps,
    account: String,
    ultra_amount: Uint128,
    net_debt_increase: Uint128,
) -> Result<Vec<WasmMsg>, ContractError> {
    let mut msgs = vec![];
    msgs.push(WasmMsg::Execute {
        contract_addr: ROLE_CONSUMER
            .load_role_address(deps, Role::ActivePool)?
            .to_string(),
        msg: to_binary(&active_pool::ExecuteMsg::IncreaseULTRADebt {
            amount: net_debt_increase,
        })?,
        funds: vec![],
    });

    // TODO: Check burn logic.
    msgs.push(WasmMsg::Execute {
        contract_addr: ROLE_CONSUMER
            .load_role_address(deps, Role::UltraToken)?
            .to_string(),
        msg: to_binary(&ultra_token::ExecuteMsg::Mint {
            recipient: account,
            amount: ultra_amount,
        })?,
        funds: vec![],
    });
    Ok(msgs)
}

pub fn repay_ultra_msgs(
    deps: Deps,
    account: String,
    ultra_amount: Uint128,
) -> Result<Vec<WasmMsg>, ContractError> {
    let mut msgs = vec![];
    msgs.push(WasmMsg::Execute {
        contract_addr: ROLE_CONSUMER
            .load_role_address(deps, Role::ActivePool)?
            .to_string(),
        msg: to_binary(&active_pool::ExecuteMsg::DecreaseULTRADebt {
            amount: ultra_amount,
        })?,
        funds: vec![],
    });

    // TODO: Check burn logic.
    msgs.push(WasmMsg::Execute {
        contract_addr: ROLE_CONSUMER
            .load_role_address(deps, Role::UltraToken)?
            .to_string(),
        msg: to_binary(&ultra_token::ExecuteMsg::Burn {
            amount: ultra_amount,
        })?,
        funds: vec![],
    });
    Ok(msgs)
}
