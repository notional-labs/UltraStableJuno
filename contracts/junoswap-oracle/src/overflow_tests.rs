use cosmwasm_std::{Decimal256, Uint128, Uint256};
use std::ops::Mul;

#[test]
fn decimal_overflow() {
    let price_cumulative_current = Uint128::from(100u128);
    let price_cumulative_last = Uint128::from(192738282u128);
    let time_elapsed: u64 = 86400;
    let amount = Uint128::from(1000u128);
    let price_average = Decimal256::from_ratio(
        Uint256::from(price_cumulative_current.wrapping_sub(price_cumulative_last)),
        time_elapsed,
    );

    println!("{}", price_average);

    let res: Uint128 = price_average.mul(Uint256::from(amount)).try_into().unwrap();
    println!("{}", res);
}
