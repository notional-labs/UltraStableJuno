use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("InvalidChannel")]
    InvalidChannel {},

    #[error("Only support unordered channel")]
    InvalidChannelOrder {},

    #[error("Channel version must be '{0}'")]
    InvalidChannelVersion(&'static str),

    #[error("Counterparty version must be '{0}'")]
    InvalidCounterpartyVersion(&'static str),

    #[error("TokenInNotFound")]
    TokenInNotFound {},
}
