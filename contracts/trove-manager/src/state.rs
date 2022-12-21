use cosmwasm_std::{Addr, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ultra_base::trove_manager::{Trove, Manager, RewardSnapshot};
use ultra_controllers::roles::RoleConsumer;


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SudoParams {
    pub name: String,
    pub owner: Addr,
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
pub const ADMIN: Admin = Admin::new("admin");
pub const ROLE_CONSUMER : RoleConsumer = RoleConsumer::new("role_provider");
pub const MANAGER: Item<Manager> = Item::new("manager");
pub const SNAPSHOTS: Map<Addr, RewardSnapshot> = Map::new("reward_snapshots");
pub const TROVE_OWNER_IDX: Map<Addr, Uint128> = Map::new("trove_owner_idx");
pub const TROVES: Map<String, (Addr, Trove)> = Map::new("troves");