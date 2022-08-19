use cosmwasm_std::StdError;
use thiserror::Error;
use ultra_controllers::roles::RolesError;

#[derive(Error, Debug)]
pub enum ContractError {

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },


    #[error("{0}")]
    UnauthorizedForRole(#[from] RolesError)

}