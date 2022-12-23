use std::str::FromStr;

use cosmwasm_std::{to_binary, Addr, Decimal256, Deps, StdError, Uint128};

use crate::{state::ROLE_CONSUMER, ContractError};
use ultra_base::{role_provider::Role, querier::check_recovery_mode};

// TODO: Verify logic behind ICR and CCR, MCR
pub fn require_ICR_above_CCR(ICR: Decimal256) -> Result<(), ContractError> {
    if ICR < Decimal256::from_str("1500000000000000000")? {
        // CCR = 1500000000000000000 ~ 150%
        return Err(ContractError::ICRNotAboveCCR {});
    }
    Ok(())
}

pub fn require_ICR_above_MCR(ICR: Decimal256) -> Result<(), ContractError> {
    if ICR < Decimal256::from_str("1100000000000000000")? {
        // MCR = 1100000000000000000 ~ 110%
        return Err(ContractError::ICRNotAboveCCR {});
    }
    Ok(())
}

pub fn require_newTCR_above_CCR(ICR: Decimal256) -> Result<(), ContractError> {
    if ICR < Decimal256::from_str("1500000000000000000")? {
        // CCR = 1500000000000000000 ~ 150%
        return Err(ContractError::ICRNotAboveCCR {});
    }
    Ok(())
}

pub fn require_newICR_above_oldICR(
    newICR: Decimal256,
    oldICR: Decimal256,
) -> Result<(), ContractError> {
    if newICR < oldICR {
        return Err(ContractError::NewICRBelowOldICR {});
    }
    Ok(())
}

// TODO: check decimal of ultra_gas_compensation
pub fn require_valid_ultra_repayment(
    current_debt: Uint128,
    debt_repayment: Uint128,
) -> Result<(), ContractError> {
    if current_debt
        > debt_repayment
            .checked_sub(Uint128::from(50_000_000u128))
            .map_err(StdError::overflow)?
    {
        return Err(ContractError::InvalidUltraRepayment {});
    }
    Ok(())
}

pub fn require_sufficient_ultra_balance(
    deps: Deps,
    borrower: String,
    debt_repayment: Uint128,
) -> Result<(), ContractError> {
    let ultra = ROLE_CONSUMER.load_role_address(deps, Role::UltraToken)?;
    let balance: Uint128 = deps.querier.query_wasm_smart(
        ultra,
        &to_binary(&ultra_base::ultra_token::QueryMsg::Balance {
            address: borrower,
        })?,
    )?;

    if balance < debt_repayment {
        return Err(ContractError::InsufficientUltra {});
    }
    Ok(())
}

pub fn require_at_least_min_net_debt(net_debt: Uint128) -> Result<(), ContractError> {
    if net_debt < Uint128::from(1950000000000000000000u64) {
        return Err(ContractError::InvalidMaxFeePercentage {});
    }
    Ok(())
}

pub fn require_non_zero_debt_change(ultra_change: Uint128) -> Result<(), ContractError> {
    if ultra_change == Uint128::zero() {
        return Err(ContractError::ZeroDebtChange {});
    }
    Ok(())
}

pub fn require_singular_coll_change(
    coll_withdrawal: Uint128,
    payment: Uint128,
) -> Result<(), ContractError> {
    if payment > Uint128::zero() && coll_withdrawal > Uint128::zero() {
        return Err(ContractError::SingularCollChange {});
    }
    Ok(())
}

pub fn require_non_zero_adjustment(
    coll_withdrawal: Uint128,
    ultra_change: Uint128,
    payment: Uint128,
) -> Result<(), ContractError> {
    if payment == Uint128::zero()
        && coll_withdrawal == Uint128::zero()
        && ultra_change == Uint128::zero()
    {
        return Err(ContractError::ZeroAdjustment {});
    }
    Ok(())
}

pub fn require_trove_is_active(
    deps: Deps,
    trove_manager: Addr,
    borrower: Addr,
) -> Result<(), ContractError> {
    // TODO: GetTroveStatus missing borrower param
    let status = deps.querier.query_wasm_smart(
        ROLE_CONSUMER.load_role_address(deps, Role::TroveManager)?,
        &to_binary(&ultra_base::trove_manager::QueryMsg::GetTroveStatus {})?,
    )?;
    Ok(())
}

pub fn require_valid_new_ICR_and_valid_new_TCR(
    recovery_mode: bool,
    old_ICR: Decimal256,
    new_ICR: Decimal256,
    new_TCR: Decimal256,
) -> Result<(), ContractError> {
    require_ICR_above_MCR(new_ICR)?;

    if recovery_mode {
        require_newTCR_above_CCR(new_TCR)?;
    } else {
        require_newICR_above_oldICR(new_ICR, old_ICR)?;
    }
    Ok(())
}

pub fn require_valid_maxFeePercentage(max_fee_percentage: Decimal256) -> Result<(), ContractError> {
    if max_fee_percentage < Decimal256::from_atomics(5000000000000000u128, 18).unwrap()
        || max_fee_percentage >= Decimal256::from_atomics(1000000000000000000u128, 18).unwrap()
    {
        return Err(ContractError::InvalidMaxFeePercentage {});
    }
    Ok(())
}