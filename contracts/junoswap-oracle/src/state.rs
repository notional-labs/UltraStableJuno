use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal256, Uint128};
use cw_storage_plus::Item;
use usj_base::asset::{AssetInfo, PoolInfo};

/// This structure stores the latest cumulative and average token prices for the target pool
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceCumulativeLast {
    pub price1_cumulative_last: Uint128,
    pub price2_cumulative_last: Uint128,
    pub price_1_average: Decimal256,
    pub price_2_average: Decimal256,
    pub block_timestamp_last: u64,
}

/// Global configuration for the contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub pool_contract_addr: Addr,
    pub asset_infos: [AssetInfo; 2],
    pub pool: PoolInfo,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PRICE_LAST: Item<PriceCumulativeLast> = Item::new("price_last");
