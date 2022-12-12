use cosmwasm_std::{Decimal256, StdError, StdResult, Uint128, Uint256};

pub fn compute_cr(coll: Uint128, debt: Uint128, price: Decimal256) -> StdResult<Decimal256> {
    if debt != Uint128::zero() {
        let new_coll_ratio: Decimal256 = Decimal256::from_ratio(
            Uint256::from_u128(coll.u128())
                .checked_mul(Decimal256::atomics(&price))
                .map_err(StdError::overflow)?, 
            Uint256::from_u128(debt.u128()));
            
        Ok(new_coll_ratio)
    } else {
        Ok(Decimal256::MAX)
    }
}

pub fn compute_nominal_cr(coll: Uint128, debt: Uint128) -> StdResult<Decimal256> {
    if debt != Uint128::zero() {
        let nomial_coll_ratio: Decimal256 = Decimal256::from_ratio(coll, debt);
        Ok(nomial_coll_ratio)
    } else {
        Ok(Decimal256::MAX)
    }
}