use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AddressesSet {
    pub borrower_operations_address: Addr,
    pub trove_manager_address: Addr,
    pub active_pool_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TotalCollsInPool {
    pub juno: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SudoParams {
    pub name: String,
    pub owner: Addr,
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
pub const ADDRESSES_SET: Item<AddressesSet> = Item::new("addresses_set");
pub const TOTAL_COLLS_IN_POOL: Item<TotalCollsInPool> = Item::new("total_colls_in_pool");
pub const COLL_OF_ACCOUNT: Map<Addr, Uint128> = Map::new("coll-of-account");
