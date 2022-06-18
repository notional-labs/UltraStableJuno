use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AddressesSet {
    pub trove_manager_address: Addr,
    pub active_pool_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AssetsInPool {
    pub juno: Uint128,
    pub usj_debt: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SudoParams {
    pub name: String,
    pub owner: Addr,
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
pub const ADDRESSES_SET: Item<AddressesSet> = Item::new("addresses_set");
pub const ASSETS_IN_POOL: Item<AssetsInPool> = Item::new("assets_in_pool");
