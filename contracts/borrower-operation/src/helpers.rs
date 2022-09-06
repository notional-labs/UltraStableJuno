use cosmwasm_std::{Addr, Decimal256, DepsMut, StdResult, Uint256};

pub fn trigger_borrowing_fee(
    deps: DepsMut,
    trove_manager_addr: Addr,
    stable_token_addr: Addr,
    stable_token_amount: Addr,
    max_fee_percentage: Decimal256,
) -> StdResult<Uint256> {
    // decay trove manager's base rate state variable
    todo!();
}

// function _triggerBorrowingFee(ITroveManager _troveManager, ILUSDToken _lusdToken, uint _LUSDAmount, uint _maxFeePercentage) internal returns (uint) {
//     _troveManager.decayBaseRateFromBorrowing(); // decay the baseRate state variable
//     uint LUSDFee = _troveManager.getBorrowingFee(_LUSDAmount);

//     _requireUserAcceptsFee(LUSDFee, _LUSDAmount, _maxFeePercentage);

//     // Send fee to LQTY staking contract
//     lqtyStaking.increaseF_LUSD(LUSDFee);
//     _lusdToken.mint(lqtyStakingAddress, LUSDFee);

//     return LUSDFee;
// }
