use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ultra_controllers::roles::RoleConsumer;
use cw_controllers::Admin;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AssetsInPool {
    pub juno: Uint128,
    pub ultra_debt: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SudoParams {
    pub name: String,
    pub owner: Addr,
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
pub const ASSETS_IN_POOL: Item<AssetsInPool> = Item::new("assets_in_pool");
pub const ADMIN: Admin = Admin::new("admin");
pub const ROLE_CONSUMER : RoleConsumer = RoleConsumer::new("role_provider");