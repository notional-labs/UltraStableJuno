use std::str::FromStr;

use cosmwasm_std::{Decimal256, Uint128};

use crate::ContractError;

// TODO: Verify logic behind ICR and CCR, MCR
pub fn require_ICR_above_CCR (ICR : Decimal256) -> Result<(), ContractError> {
    if ICR < Decimal256::from_str("1500000000000000000")? { // CCR = 1500000000000000000 ~ 150%
        return Err(ContractError::ICRNotAboveCCR {}); 
    }
    Ok(())
    
}

pub fn require_ICR_above_MCR (ICR : Decimal256) -> Result<(), ContractError> {
    if ICR < Decimal256::from_str("1100000000000000000")? { // MCR = 1100000000000000000 ~ 110%
        return Err(ContractError::ICRNotAboveCCR {}); 
    }
    Ok(())
    
}

pub fn require_newTCR_above_CCR (ICR : Decimal256) -> Result<(), ContractError> {
    if ICR < Decimal256::from_str("1500000000000000000")? { // CCR = 1500000000000000000 ~ 150%
        return Err(ContractError::ICRNotAboveCCR {}); 
    }
    Ok(())
    
}

pub fn assert_at_least_min_net_debt(net_debt: Uint128) -> Result<(), ContractError> {
    if net_debt < Uint128::from(1950000000000000000000u64) {
        return Err(ContractError::InvalidMaxFeePercentage {});
    }
    Ok(())
}