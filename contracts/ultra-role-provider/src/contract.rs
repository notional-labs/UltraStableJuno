#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;
use ultra_base::role_provider::{HasAnyRoleResponse, Role, RoleAddressResponse};

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State::default();
    for (role, address) in vec![
        (Role::Owner, msg.owner),
        (Role::StabilityPool, msg.stability_pool),
        (Role::TroveManager, msg.trove_manager),
        (Role::ActivePool, msg.active_pool),
        (Role::BorrowerOperations, msg.borrower_operations),
    ] {
        let address = deps.api.addr_validate(&address)?;
        state.role_provider.set(deps.storage, &role, address)?;
    }
    Ok(Response::new().add_attribute("action", "ultra/role_provider/instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateRole { role, address } => {
            execute_update_role(deps, info, role, Some(address))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg<Role>) -> StdResult<Binary> {
    let state = State::default();
    match msg {
        QueryMsg::HasAnyRole { address, roles } => {
            let address = deps.api.addr_validate(&address)?;
            let has_role = state
                .role_provider
                .has_any_role(deps.storage, &roles, &address)?;
            to_binary(&HasAnyRoleResponse { has_role })
        }
        QueryMsg::RoleAddress { role } => to_binary(&query_role_address(deps, role)?),
    }
}

pub fn execute_update_role(
    deps: DepsMut,
    info: MessageInfo,
    role: Role,
    address: Option<String>,
) -> Result<Response, ContractError> {
    let state = State::default();

    // Only owner can update roles.
    state
        .role_provider
        .assert_role(deps.storage, &Role::Owner, &info.sender)?;

    match &address {
        Some(address) => {
            let address = deps.api.addr_validate(address)?;
            state.role_provider.set(deps.storage, &role, address)
        }
        None => {
            // owner role cannot be deleted
            if role != Role::Owner {
                state.role_provider.delete(deps.storage, &role)
            } else {
                Err(StdError::generic_err("owner cannot be deleted!"))
            }
        }
    }?;

    let grantee_attr = address.unwrap_or("None".to_string());

    Ok(Response::new().add_attributes(vec![
        ("action", "update_role"),
        ("role", &role.to_string()),
        ("grantee", &grantee_attr),
        ("sender", info.sender.as_str()),
    ]))
}

pub fn query_role_address(deps: Deps, role: Role) -> StdResult<RoleAddressResponse> {
    let state = State::default();
    let addr = state
        .role_provider
        .get(deps.storage, &role)?
        .map(String::from);
    Ok(RoleAddressResponse { address: addr })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_info},
        Addr, Empty,
    };
    use ultra_base::role_provider::Role;
    use ultra_controllers::roles::{RoleProvider, RolesError};

    use crate::{
        contract::{execute, execute_update_role, query, query_role_address},
        ContractError,
    };

    #[test]
    fn test_execute_query() {
        let mut deps = mock_dependencies();

        // initial setup
        let control = RoleProvider::new("foo", "foo__idx");
        let owner = Addr::unchecked("big boss");
        let imposter = Addr::unchecked("imposter");
        let friend = Addr::unchecked("buddy");
        control
            .set(deps.as_mut().storage, &Role::Owner, owner.clone())
            .unwrap();

        // query shows results
        let res = query_role_address(deps.as_ref(), Role::Owner).unwrap();
        assert_eq!(Some(owner.to_string()), res.address);

        // imposter cannot update
        let info = mock_info(imposter.as_ref(), &[]);
        let new_admin = Some(friend.clone());
        let err = execute_update_role(
            deps.as_mut(),
            info,
            Role::Owner,
            new_admin.clone().map(|a| a.to_string()),
        )
        .unwrap_err();
        assert_eq!(
            ContractError::UnauthorizedForRole(RolesError::UnauthorizedForRole {
                label: Role::Owner.to_string()
            }),
            err
        );

        // owner can update
        let info = mock_info(owner.as_ref(), &[]);
        let res = execute_update_role(
            deps.as_mut(),
            info,
            Role::Owner,
            new_admin.map(|a| a.to_string()),
        )
        .unwrap();
        assert_eq!(0, res.messages.len());

        // query shows results
        let res = query_role_address(deps.as_ref(), Role::Owner).unwrap();
        assert_eq!(Some(friend.to_string()), res.address);
    }
}
