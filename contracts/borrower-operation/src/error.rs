use cosmwasm_std::StdError;
use thiserror::Error;
use ultra_controllers::roles::RolesError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    RolesError(#[from] RolesError),

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("Unauthorized.")]
    Unauthorized {},

    #[error("Invalid max fee percentage. Must be <= 100%")]
    InvalidMaxFeePercentage {},

    #[error("BorrowerOperation: Caller is not borrower")]
    CallerIsNotBorrower {},
}
