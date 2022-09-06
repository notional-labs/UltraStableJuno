use cosmwasm_std::{Addr, Decimal256, Uint128, Uint256};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ultra_controllers::roles::RoleConsumer;

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

// temp state only lives for the duration of a single execution
pub struct TempState<'a> {
    pub borrowing_fee: Item<'a, Uint256>,
    pub net_debt: Item<'a, Uint256>,
}

pub struct State<'a> {
    pub roles: RoleConsumer<'a>,
    pub temp: TempState<'a>,
}

impl<'a> Default for State<'a> {
    fn default() -> Self {
        State {
            roles: RoleConsumer::new("role_provider_address"),
            temp: TempState {
                borrowing_fee: Item::new("temp_borrowing_fee"),
                net_debt: Item::new("temp_net_debt"),
            },
        }
    }
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");
