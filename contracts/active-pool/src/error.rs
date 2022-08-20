use cosmwasm_std::StdError;
use thiserror::Error;
use ultra_controllers::roles::RolesError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    UnauthorizedForRole(#[from] RolesError),

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("ActivePool: Caller is neither BO nor Default Pool")]
    CallerIsNeitherBONorDP {},

    #[error("ActivePool: Caller is neither BorrowerOperations nor TroveManager nor StabilityPool")]
    CallerIsNeitherBONorTMNorSP {},

    #[error("ActivePool: Caller is neither BorrowerOperations nor TroveManager")]
    CallerIsNeitherBONorTM {},
}
