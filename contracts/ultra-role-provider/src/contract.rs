#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;
use ultra_base::role_provider::{HasAnyRoleResponse, Role, RoleAddressResponse, AllRolesResponse};

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
        QueryMsg::AllRoles {  } => {
            let mut roles : Vec<(Role, Option<String>)> = vec![];
            for role in Role::iterator(){
                let role_address = state.role_provider
                    .get(deps.storage, &role)?
                    .map(String::from);
                roles.push((role.clone(), role_address));
            }
            to_binary(&AllRolesResponse { roles })
        }
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
    addr.map_or_else(
        || Err(StdError::generic_err("role not found")),
        |addr| Ok(RoleAddressResponse { address: addr }),
    )
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_info, mock_env},
        Addr, DepsMut,
    };
    use ultra_base::{role_provider::{Role, InstantiateMsg}};
    use ultra_controllers::roles::RolesError;

    use crate::{
        contract::{execute_update_role, query_role_address, instantiate},
        ContractError,
    };

    fn setup_contract(deps: DepsMut) {
        let owner = Addr::unchecked("big boss");
        let active_pool = Addr::unchecked("active pool");
        let trove_manager = Addr::unchecked("trove manager");
        let stability_pool = Addr::unchecked("stability pool");
        let borrower_operations = Addr::unchecked("borrower operations");

        let msg = InstantiateMsg {
            active_pool: active_pool.to_string(),
            trove_manager: trove_manager.to_string(),
            owner: owner.to_string(),
            stability_pool: stability_pool.to_string(),
            borrower_operations: borrower_operations.to_string()
        };

        let info = mock_info(owner.as_str(), &[]);
        let res = instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_execute_query() {
        let mut deps = mock_dependencies();

        // initial setup
        let owner = Addr::unchecked("big boss");
        let imposter = Addr::unchecked("imposter");
        let friend = Addr::unchecked("buddy");
        
        setup_contract(deps.as_mut());

        // query shows results
        let res = query_role_address(deps.as_ref(), Role::Owner).unwrap();
        assert_eq!(owner.to_string(), res.address);

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
        assert_eq!(friend.to_string(), res.address);
    }
}
