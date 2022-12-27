use cosmwasm_std::{Addr, Uint256, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub name: String,
    pub owner: String,
    pub max_size: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateAdmin { 
        admin : Addr 
    }, 
    UpdateRole { 
        role_provider: Addr 
    },
    Insert {
        id: String,
        nicr: Uint128,
        prev_id: Option<String>,
        next_id: Option<String>,
    },
    ReInsert {
        id: String,
        new_nicr: Uint128,
        prev_id: Option<String>,
        next_id: Option<String>,
    },
    Remove {
        id: String,
    },
    SetParams {
        size: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    GetParams {},
    GetData {},
    GetSize {},
    GetMaxSize {},
    GetFirst {},
    GetLast {},
    GetNext {
        id: String,
    },
    GetPrev {
        id: String,
    },
    Contains {
        id: String,
    },
    FindInsertPosition {
        nicr: Uint128,
        prev_id: String,
        next_id: String,
    },
    ValidInsertPosition {
        nicr: Uint128,
        prev_id: String,
        next_id: String,
    },
    IsEmpty {},
    IsFull {},
    GetBorrowerOperationAddress {},
    GetTroveManagerAddress {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SudoMsg {
    /// Update the contract parameters
    /// Can only be called by governance
    UpdateParams {
        name: Option<String>,
        owner: Option<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ParamsResponse {
    pub name: String,
    pub owner: Addr,
}
