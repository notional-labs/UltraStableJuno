use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use ultra_controllers::roles::RoleProvider;

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State<'a> {
    pub role_provider: RoleProvider<'a>,
}

impl<'a> Default for State<'a> {
    fn default() -> Self {
        State {
            role_provider: RoleProvider::new("roles", "roles__roles_by_addr_idx")
        }
    }
}
