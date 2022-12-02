use cosmwasm_std::{ StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("OverflowError")]
    OverflowError {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("IllegalPrice")]
    IllegalPrice {},

    #[error("VerifyFail")]
    VerifyFail {},

    #[error("PriceNotExist")]
    PriceNotExist {},

    #[error("NotLatestData")]
    NotLatestData {},
}