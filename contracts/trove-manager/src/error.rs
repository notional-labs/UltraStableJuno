use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;
use ultra_controllers::roles::RolesError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Admin(#[from] AdminError),
    
    #[error("{0}")]
    UnauthorizedForRole(#[from] RolesError),

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("TroveManager: Trove is not exist or is closed")]
    TroveNotActive {},

    #[error("TroveManager: Trove existed")]
    TroveExist {},

    #[error("TroveManager: Only one trove in the system")]
    OnlyOneTroveExist,
    
    #[error("TroveManager: decay_base_rate must be between 0 and 1")]
    DecayBaseRateLargerThanOne {},

    #[error("TroveManager: total_stake_snapshot must be positive")]
    TotalStakeSnapshotIsZero {},

    #[error("TroveManager: max_fee_percentage must be between 0.5% and 100%")]
    MaxFeePercentageInvalid {},

    #[error("TroveManager: Cannot redeem when TCR < MCR")]
    TCRLessThanMCR {},

    #[error("TroveManager: Amount must be greater than zero")]
    AmountIsZero {},

    #[error("TroveManager: Requested redemption amount must be <= user's ultra token balance")]
    InsufficientBalance {}, 

    #[error("TroveManager: Redeemer's balance over total UltraDebt supply")]
    BalanceOverSupply {},

    #[error("TroveManager: Unable to redeem any amount")]
    UnableToRedeem {},

    #[error("TroveManager: Base rate is always non-zero after redemption")]
    BaseRateIsZero {},

    #[error("TroveManager: Fee would eat up all returned collateral")]
    FeeEatUpAllReturns {},

    #[error("TroveManager: Fee exceeded provided maximum")]
    FeeIsNotAccepted {},

    #[error("TroveManager: Remain Ultra In Stability Pool is zero")]
    RemainUltraInStabilityPoolIsZero {},

    #[error("TroveManager: nothing to liquidate")]
    NothingToLiquidate {},
}
