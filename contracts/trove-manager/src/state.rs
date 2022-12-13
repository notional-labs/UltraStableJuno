use cosmwasm_std::{Addr, Uint128, Timestamp, Decimal256};
use cw_controllers::Admin;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ultra_base::trove_manager::Troves;
use ultra_controllers::roles::RoleConsumer;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Manager {
    pub trove_owner_count: Uint128,
    pub base_rate : Decimal256,
    pub last_fee_operation_time : Timestamp,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SudoParams {
    pub name: String,
    pub owner: Addr,
}

pub struct State<'a> {
    pub troves: Troves<'a>,
    pub manager: Item<'a, Manager>
}

impl<'a> Default for State<'a> {
    fn default() -> Self {
        State {
            troves: Troves::new("troves", "troves__troves_by_addr"),
            manager: Item::new("manager")
        }
    }
}
pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
pub const ADMIN: Admin = Admin::new("admin");
pub const ROLE_CONSUMER : RoleConsumer = RoleConsumer::new("role_provider");