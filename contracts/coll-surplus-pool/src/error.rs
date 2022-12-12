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

    #[error("CollSurplusPool: Caller is not Borrower Operations")]
    CallerIsNotBO {},

    #[error("CollSurplusPool: Caller is not TroveManager")]
    CallerIsNotTM {},

    #[error("CollSurplusPool: Caller is not Active Pool")]
    CallerIsNotAP {},

    #[error("CollSurplusPool: No collateral available to claim")]
    NoCollAvailableToClaim {},
}
