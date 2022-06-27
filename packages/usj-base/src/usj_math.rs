use cosmwasm_std::{Decimal256, StdError, StdResult, Uint128};

pub fn compute_cr(coll: Uint128, debt: Uint128, price: Decimal256) -> StdResult<Decimal256> {
    if debt != Uint128::zero() {
        let new_coll_ratio: Decimal256 = Decimal256::from_ratio(coll, debt)
            .checked_mul(price)
            .map_err(StdError::overflow)?;
        Ok(new_coll_ratio)
    } else {
        Ok(Decimal256::MAX)
    }
}
