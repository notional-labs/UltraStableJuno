use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("BorrowerOperation: Caller is not borrower")]
    CallerIsNotBorrower {},

    #[error("BorrowerOperation: Max fee percentage must be between 0.5% and 100%")]
    InvalidMaxFeePercentage {},

    #[error("BorrowerOperation:  Trove's net debt must be greater than minimum")]
    InvalidMinNetDebt {},

    #[error("BorrowerOperation:  In Recovery Mode new troves must have ICR >= CCR")]
    ICRNotAboveCCR {},

    #[error("BorrowerOperation:  An operation that would result in ICR < MCR is not permitted")]
    ICRNotAboveMCR {},

    #[error("BorrowerOperation:  An operation that would result in TCR < CCR is not permitted")]
    NewTCRNotAboveCCR {},

    #[error("BorrowerOperation: Debt increase requires non-zero debtChange")]
    ZeroDebtChange {},

    #[error("BorrowerOperation: Cannot withdraw and add coll")]
    SingularCollChange {},

    #[error("BorrowerOperation: There must be either a collateral change or a debt change")]
    ZeroAdjustment {},

    #[error("BorrowerOperation: Cannot decrease your Trove's ICR in Recovery Mode")]
    NewICRBelowOldICR {},

    #[error("BorrowerOperation: Amount repaid must not be larger than the Trove's debt")]
    InvalidUltraRepayment {}, 

    #[error("BorrowerOperation:  Caller doesnt have enough Ultra to make repayment")]
    InsufficientUltra {}, 
}
