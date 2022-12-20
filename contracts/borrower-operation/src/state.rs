use cosmwasm_std::{Addr, CanonicalAddr, Decimal256, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ultra_controllers::roles::RoleConsumer;
use cw_controllers::Admin;

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

pub struct State<'a> {
    pub roles: RoleConsumer<'a>,
}

impl<'a> Default for State<'a> {
    fn default() -> Self {
        State {
            roles: RoleConsumer::new("role_provider_address"),
        }
    }
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
pub const ROLE_CONSUMER : RoleConsumer = RoleConsumer::new("role_provider");
pub const ADMIN: Admin = Admin::new("admin");
