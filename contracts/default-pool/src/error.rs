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

    #[error("DefaultPool: Caller is not the ActivePool")]
    CallerIsNotAP {},

    #[error("DefaultPool: Caller is not the TroveManager")]
    CallerIsNotTM {},
}
