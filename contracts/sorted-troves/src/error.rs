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

    #[error("SortedTroves: List does not contain the id")]
    ListNotContainId {},

    #[error("SortedTroves: List already contains the id")]
    ListAlreadyContainsId {},

    #[error("SortedTroves: List is full")]
    ListIsFull {},
    
    #[error("SortedTroves: NICR must be positive")]
    NICRMustBePositive {},

    #[error("SortedTroves: Size can’t be zero")]
    SizeIsZero {},

    #[error("SortedTroves: Start Id is None")]
    StartIdIsNone {},
}