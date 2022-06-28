use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("CollSurplusPool: Caller is not Borrower Operations")]
    CallerIsNotBO {},

    #[error("CollSurplusPool: Caller is not TroveManager")]
    CallerIsNotTM {},

    #[error("CollSurplusPool: Caller is not Active Pool")]
    CallerIsNotAP {},

    #[error("CollSurplusPool: No collateral available to claim")]
    NoCollAvailableToClaim {},
}
