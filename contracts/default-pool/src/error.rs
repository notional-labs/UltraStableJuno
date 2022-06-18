use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("DefaultPool: Caller is not the ActivePool")]
    CallerIsNotAP {},

    #[error("DefaultPool: Caller is not the TroveManager")]
    CallerIsNotTM {},
}
