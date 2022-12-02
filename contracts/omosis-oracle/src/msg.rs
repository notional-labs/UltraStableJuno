use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub token_1: String,
    pub token_2: String,
    pub pool_id: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SpotPrice(SpotPriceMsg),
    EstimateSwap(EstimateSwapMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SpotPriceMsg {
    pub channel: String,
    pub token_in: String,
    /// How long the packet lives in seconds. If not specified, use default_timeout
    pub timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimateSwapMsg {
    pub channel: String,
    pub sender: String,
    pub token_in: String,
    pub amount: Uint128,
    /// How long the packet lives in seconds. If not specified, use default_timeout
    pub timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetPrice {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct GetPriceResponse {
    pub token_1: String,
    pub token_2: String,
    pub token1_by_token2: String,
    pub last_update: u64
}

