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

    #[error("TroveManager: Trove is not exist")]
    TroveNotExist {},

    #[error("TroveManager: decay_base_rate must be between 0 and 1")]
    DecayBaseRateLargerThanOne {},
}
