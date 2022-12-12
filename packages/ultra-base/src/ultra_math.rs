use cosmwasm_std::{Decimal256, StdError, StdResult, Uint128, Uint256};

pub const NICR_PRECISION: u32 = 20; 
pub fn compute_cr(coll: Uint128, debt: Uint128, price: Decimal256) -> StdResult<Decimal256> {
    if debt != Uint128::zero() {
        let new_coll_ratio: Decimal256 = Decimal256::from_ratio(
            Uint256::from_u128(coll.u128())
                .checked_mul(Decimal256::atomics(&price))
                .map_err(StdError::overflow)?, 
            debt
                .checked_mul(Uint128::from(10u128).wrapping_pow(18))
                .map_err(StdError::overflow)?
            );   
        Ok(new_coll_ratio)
    } else {
        Ok(Decimal256::MAX)
    }
}

pub fn compute_nominal_cr(coll: Uint128, debt: Uint128) -> StdResult<Decimal256> {
    if debt != Uint128::zero() {
        let nomial_coll_ratio: Decimal256 = Decimal256::from_ratio(
            Uint256::from_u128(coll.u128())
            .checked_mul(Uint256::from(10u128).wrapping_pow(NICR_PRECISION))
            .map_err(StdError::overflow)?, 
            debt);
        Ok(nomial_coll_ratio)
    } else {
        Ok(Decimal256::MAX)
    }
}