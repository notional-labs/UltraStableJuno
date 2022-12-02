use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfo{
    pub token_1: String,
    pub token_2: String,
    pub pool_id: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Price{
    pub token1_by_token2: String,
    pub last_update: u64
}

pub const CONTRACT_INFO : Item<ContractInfo> = Item::new("contract_info");
pub const CHANNEL: Item<Option<String>> = Item::new("channel");
pub const PRICE : Item<Price> = Item::new("price");