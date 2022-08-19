#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Empty, to_binary};
use ultra_base::role_provider::HasAnyRoleResponse;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::State;

const CONTRACT_NAME: &str = "crates.io:ultra-role-provider";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State::default();
    for (role, grantee) in msg.roles {
        state.role_provider.set(deps.storage, &role, grantee)?;
    }
    Ok(Response::new().add_attribute("action", "ultra_role_provider/instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let state = State::default();
    match msg {
        ExecuteMsg::UpdateRole { role, address } => {
            state.role_provider.execute_update_role::<Empty, Empty>(deps, info, role, Some(address)).map_err(ContractError::UnauthorizedForRole)
        },
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let state = State::default();
    match msg {
        QueryMsg::HasAnyRole { address, roles } => {
            let has_role = state.role_provider.has_any_role(deps.storage, &roles, &address)?;
            to_binary(&HasAnyRoleResponse {
                has_role
            })
        },
        QueryMsg::RoleAddress { role } => to_binary(&state.role_provider.query_role(deps, role)?),
    }
}

#[cfg(test)]
mod tests {}
