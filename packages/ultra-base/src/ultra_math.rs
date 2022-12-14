use cosmwasm_std::{Decimal256, StdError, StdResult, Uint128, Uint256};

pub const NICR_PRECISION: u32 = 20; 
pub const DECIMAL_PRECISION: u32 = 18;

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

// pub fn compute_powf() -> StdResult<Decimal256> {
    
//     Ok(Decimal256::MAX)
// }

// pub fn compute_nth_root(num: Uint128, nth: u32) -> StdResult<Decimal256> {
//     let mut v: u128 =1;
//     if nth == 0 {
//         return Err(StdError::overflow(OverflowError::new(OverflowOperation::Pow, num, format!("1 / {}", nth))))
//     }

//     if nth == 1 {
//         return Ok(Decimal256::raw(num.u128()))
//     }
//     let mut tp = Uint128::from(v.pow(nth));
//     while tp < num {
//         v <<= 1;
//         tp = Uint128::from(v.pow(nth));
//     }
//     if tp == num {
//         return Ok(Decimal256::raw());
//     }
//     Ok(Decimal256::MAX)
// }
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

/* 
    * dec_pow: Exponentiation function for 18-digit decimal base, and integer exponent n.
    * 
    * Uses the efficient "exponentiation by squaring" algorithm. O(log(n)) complexity. 
    * 
    * Called by two functions that represent time in units of minutes:
    * 1) TroveManager: calculate decayed BaseRate
    * 2) CommunityIssuance: get cumulative issuance fraction 
    * 
    * The exponent is capped to avoid reverting due to overflow. The cap 525600000 equals
    * "minutes in 1000 years": 60 * 24 * 365 * 1000
    * 
    * If a period of > 1000 years is ever used as an exponent in either of the above functions, the result will be
    * negligibly different from just passing the cap, since: 
    *
    * In function 1), the decayed base rate will be 0 for 1000 years or > 1000 years
    * In function 2), the difference in tokens issued at 1000 years and any time > 1000 years, will be negligible
    */

pub fn dec_pow(base: Decimal256, minute: u64) -> StdResult<Decimal256> {
    let mut minute = if minute > 525600000 { 525600000 } else { minute };
    if minute == 0 { 
        return Ok(Decimal256::one());
    }

    let mut y = Decimal256::raw(10u128.pow(DECIMAL_PRECISION as u32));
    let mut x = base;

    while minute > 1 {
        if minute % 2 == 0 {
            x = round_mul(x, x);
            minute /= 2;
        } else {
            y = round_mul(x, y);
            x = round_mul(x, x);
            minute >>= 1;
        }
    }
    Ok(round_mul(x, y))
}

/* 
    * Multiply two decimal numbers and use normal rounding rules:
    * -round product up if 19'th mantissa digit >= 5
    * -round product down if 19'th mantissa digit < 5
    *
    * Used only inside the exponentiation, dec_pow().
    */
pub fn round_mul(x: Decimal256, y: Decimal256) -> Decimal256 {
    x.saturating_mul(y)
        .saturating_add(Decimal256::percent(50))
        .floor()
}