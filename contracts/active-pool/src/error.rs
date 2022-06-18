use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("ActivePool: Caller is neither BO nor Default Pool")]
    CallerIsNeitherBONorDP {},

    #[error("ActivePool: Caller is neither BorrowerOperations nor TroveManager nor StabilityPool")]
    CallerIsNeitherBONorTMNorSP {},

    #[error("ActivePool: Caller is neither BorrowerOperations nor TroveManager")]
    CallerIsNeitherBONorTM {},
}
