use cosmwasm_std::{Addr, Uint256, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ultra_controllers::roles::RoleConsumer;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SudoParams {
    pub name: String,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Data {
    pub head: Option<Addr>,
    pub tail: Option<Addr>,
    pub max_size: Uint128,
    pub size: Uint128
}

impl Data {
    pub fn is_full(&self) -> bool {
        return self.size == self.max_size
    }

    pub fn is_empty(&self) -> bool {
        return self.size == Uint128::zero()
    }
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Node {
    pub next_id: Option<Addr>,
    pub prev_id: Option<Addr>,
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
pub const ADMIN: Admin = Admin::new("admin");
pub const ROLE_CONSUMER : RoleConsumer = RoleConsumer::new("role_provider");
pub const DATA: Item<Data> = Item::new("data");
pub const NODES: Map<Addr, Node> = Map::new("nodes");