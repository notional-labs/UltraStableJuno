use cosmwasm_std::{Addr, Decimal256, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AddressesSet {
    pub trove_manager_address: Addr,
    pub stability_pool_address: Addr,
    pub default_pool_address: Addr,
    pub active_pool_address: Addr,
    pub coll_surplus_pool_address: Addr,
    pub ultra_token_contract_address: Addr,
    pub price_feed_contract_address: Addr,
    pub sorted_troves_address: Addr,
    pub reward_pool_address: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Trove {
    price: Uint128,
    ultra_fee: Uint128,
    net_debt: Uint128,
    composite_debt: Uint128,
    icr: Decimal256,
    nicr: Decimal256,
    array_index: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SudoParams {
    pub name: String,
    pub owner: Addr,
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
pub const ADDRESSES_SET: Item<AddressesSet> = Item::new("addresses_set");
