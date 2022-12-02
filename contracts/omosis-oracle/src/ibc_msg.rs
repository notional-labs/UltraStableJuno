use cosmwasm_std::{Decimal, Uint128, Uint64, Binary};
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PacketMsg{
    pub client_id: Option<String>,
    pub query: GammPacket,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GammPacket {
    SpotPrice(SpotPricePacket),
    EstimateSwap(EstimateSwapPacket),
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimateSwapPacket {
    pub pool: Uint64,
    pub sender: String,
    pub amount: Uint128,
    pub token_in: String,
    pub token_out: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SpotPricePacket {
    pub pool: Uint64,
    pub token_in: String,
    pub token_out: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SpotPriceAck {
    pub price: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EstimateSwapAck {
    pub amount: Uint128,
}
#[derive(Serialize, Deserialize, Clone, Debug,PartialEq, JsonSchema)]
pub enum PacketAck{
    Result(Binary),
    Error(String)
}

