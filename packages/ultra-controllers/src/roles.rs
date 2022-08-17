// based on https://github.com/CosmWasm/cw-plus/blob/main/packages/controllers/src/admin.rs
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{fmt};
use thiserror::Error;

use cosmwasm_std::{
    attr, Addr, CustomQuery, Deps, DepsMut, MessageInfo, Response, StdError, StdResult, Storage,
};
use cw_storage_plus::{IndexedMap, index_list, MultiIndex};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    ActivePool,
    TroveManager,
    Owner,
    StabilityPool
}

impl ToString for Role {
    fn to_string(&self) -> String {
        match &self {
            Role::ActivePool => "active_pool",
            Role::TroveManager => "trove_manager",
            Role::Owner => "owner",
            Role::StabilityPool => "stability_pool",
        }.into()
    }
}

// TODO: should the return values end up in utils, so eg. cw4 can import them as well as this module?
/// Returned from Permissions.query_role()
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PermissionsResponse {
    pub address: Option<String>,
    pub role: Role
}

/// Errors returned from Admin
#[derive(Error, Debug, PartialEq)]
pub enum PermissionsError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Caller is not {label}")]
    UnauthorizedForRole { label: String },
}

pub type PermissionRecord = Addr;

/// stringified role
pub type PermissionPK<'a> = &'a str;

#[index_list(PermissionRecord)]
pub struct PermissionsIndexes<'a> {
    // find all roles for one address
    // allow for edge case where one address has multiple roles. 
    // e.g. `owner` is also `generator`
    roles_by_addr: MultiIndex<'a, Addr, PermissionRecord, PermissionPK<'a>>
}

// state/logic
pub struct Permissions<'a>(IndexedMap<'a, PermissionPK<'a>, PermissionRecord, PermissionsIndexes<'a>>);

// this is the core business logic we expose
impl<'a> Permissions<'a> {
    pub fn new(namespace: &'a str, roles_by_addr_idx_namespace: &'a str) -> Self {
        Permissions(IndexedMap::new(namespace, PermissionsIndexes::<'a> {
            roles_by_addr: MultiIndex::new(|addr| addr.clone(), namespace, roles_by_addr_idx_namespace)
        }))
    }

    pub fn delete(&self, store: &mut dyn Storage, role: &Role) -> StdResult<()> {
        self.0.remove(store, &role.to_string())
    }

    pub fn set(&self, store: &mut dyn Storage, role: &Role, grantee: Addr) -> StdResult<()> {
        self.0.save(store, &role.to_string(),  &grantee)
    }

    pub fn get(&self, store: &dyn Storage, role: &Role) -> StdResult<Option<PermissionRecord>> {
        self.0.may_load(store, &role.to_string())
    }

    /// Returns Ok(true) if this user has the role, Ok(false) if not and an Error if
    /// we hit an error with Api or Storage usage
    pub fn has_role(&self, store: &dyn Storage, role: &Role, caller: &Addr) -> StdResult<bool> {
        self.0.may_load(store, &role.to_string())?.map_or_else(|| Ok(false), |addr| Ok(&addr == caller))
    }

    /// Returns Ok(true) if this user has any of the roles, Ok(false) if not and an Error if
    /// we hit an error with Api or Storage usage
    pub fn has_any_role(&self, store: &dyn Storage, roles: &[Role], caller: &Addr) -> StdResult<bool> {
        for role in roles {
           if self.has_role(store, role, caller)? {
            // if any exists, stop iteration and return true result
             return Ok(true)
           } else {
            continue;
           }
        }
        // if nothing was returned, none exists. return false.
        Ok(false)
    }

    /// Like has_any_role but returns PermissionsError::UnauthorizedForRole if not authorized.
    /// Helper for a nice one-line auth check.
    pub fn assert_any_role(&self, store: &dyn Storage, roles: &[Role], caller: &Addr) -> Result<(), PermissionsError> {
        for role in roles {
            if !self.has_role(store, &role, caller)? {
                continue
            } else {
                return Ok(())
            }
        }
        let label = roles.into_iter().map(|r| r.to_string()).collect::<Vec<String>>().join(" | ");
        Err(PermissionsError::UnauthorizedForRole { label })
    }

    /// Like has_role but returns PermissionsError::UnauthorizedForRole if not authorized.
    /// Helper for a nice one-line auth check.
    pub fn assert_role(
        &self,
        store: &dyn Storage,
        role: &Role,
        caller: &Addr,
    ) -> Result<(), PermissionsError> {
        if !self.has_role(store, &role, caller)? {
            Err(PermissionsError::UnauthorizedForRole { label: role.to_string() })
        } else {
            Ok(())
        }
    }

    pub fn execute_update_owner<C, Q: CustomQuery>(
        &self,
        deps: DepsMut<Q>,
        info: MessageInfo,
        new_owner: Option<Addr>,
    ) -> Result<Response<C>, PermissionsError>
    where
        C: Clone + fmt::Debug + PartialEq + JsonSchema,
    {
        self.assert_role(deps.storage, &Role::Owner, &info.sender)?;

        let owner_str = match new_owner.as_ref() {
            Some(owner ) => owner.to_string(),
            None => "None".to_string(),
        };
        let attributes = vec![
            attr("action", "update_owner"),
            attr("owner", owner_str),
            attr("sender", info.sender),
        ];

        match new_owner {
            Some(owner) => self.set(deps.storage, &Role::Owner, owner),
            None => self.delete(deps.storage, &Role::Owner)
        }?;
 
        Ok(Response::new().add_attributes(attributes))
    }

    pub fn query_role<Q: CustomQuery>(&self, deps: Deps<Q>, role: Role) -> StdResult<PermissionsResponse> {
        let addr = self.get(deps.storage, &role)?.map(String::from);
        Ok(PermissionsResponse { address: addr, role })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_info};
    use cosmwasm_std::Empty;

    #[test]
    fn set_and_get_owner() {
        let mut deps = mock_dependencies();
        let control = Permissions::new("foo", "foo__roles_by_addr");

        // initialize and check
        let owner = Addr::unchecked("owner");
        control.set(deps.as_mut().storage, &Role::Owner, owner.clone()).unwrap();
        let got = control.get(deps.as_ref().storage, &Role::Owner).unwrap();
        assert_eq!(owner, got.unwrap());

        // clear it and check
        control.delete(deps.as_mut().storage, &Role::Owner).unwrap();
        let got = control.get(deps.as_ref().storage, &Role::Owner).unwrap();
        assert_eq!(None, got);
    }

    #[test]
    fn role_checks() {
        let mut deps = mock_dependencies();

        let control = Permissions::new("foo", "foo__idx");
        let owner = Addr::unchecked("big boss");
        let imposter = Addr::unchecked("imposter");

        // ensure checks proper with owner set
        control.set(deps.as_mut().storage, &Role::Owner, owner.clone()).unwrap();
        assert!(control.has_role(deps.as_ref().storage, &Role::Owner, &owner).unwrap());
        assert!(!(control.has_role(deps.as_ref().storage, &Role::Owner, &imposter).unwrap()));
        control.assert_role(deps.as_ref().storage, &Role::Owner, &owner).unwrap();
        let err = control.assert_role(deps.as_ref().storage, &Role::Owner, &imposter).unwrap_err();
        assert_eq!(PermissionsError::UnauthorizedForRole { label: Role::Owner.to_string() }, err);

        // same checks for `any` variants
        assert!(control.has_any_role(deps.as_ref().storage, &[Role::ActivePool, Role::Owner, Role::StabilityPool], &owner).unwrap());
        assert!(!(control.has_any_role(deps.as_ref().storage, &[Role::ActivePool, Role::Owner, Role::StabilityPool], &imposter).unwrap()));
        control.assert_any_role(deps.as_ref().storage, &[Role::ActivePool, Role::Owner, Role::StabilityPool], &owner).unwrap();
        let err = control.assert_any_role(deps.as_ref().storage, &[Role::Owner, Role::ActivePool, Role::StabilityPool], &imposter).unwrap_err();
        assert_eq!(PermissionsError::UnauthorizedForRole { label: format!("{} | {} | {}", Role::Owner.to_string(), Role::ActivePool.to_string(), Role::StabilityPool.to_string()) }, err);

        // ensure checks proper with owner None
        control.delete(deps.as_mut().storage, &Role::Owner).unwrap();
        assert!(!(control.has_role(deps.as_ref().storage, &Role::Owner, &owner).unwrap()));
        assert!(!(control.has_role(deps.as_ref().storage, &Role::Owner, &imposter).unwrap()));
        let err = control.assert_role(deps.as_ref().storage, &Role::Owner, &owner).unwrap_err();
        assert_eq!(PermissionsError::UnauthorizedForRole { label: Role::Owner.to_string() }, err);
        let err = control.assert_role(deps.as_ref().storage, &Role::Owner, &imposter).unwrap_err();
        assert_eq!(PermissionsError::UnauthorizedForRole { label: Role::Owner.to_string() }, err);
    }

    #[test]
    fn test_execute_query() {
        let mut deps = mock_dependencies();

        // initial setup
        let control = Permissions::new("foo", "foo__idx");
        let owner = Addr::unchecked("big boss");
        let imposter = Addr::unchecked("imposter");
        let friend = Addr::unchecked("buddy");
        control.set(deps.as_mut().storage, &Role::Owner, owner.clone()).unwrap();

        // query shows results
        let res = control.query_role(deps.as_ref(), Role::Owner).unwrap();
        assert_eq!(Some(owner.to_string()), res.address);

        // imposter cannot update
        let info = mock_info(imposter.as_ref(), &[]);
        let new_admin = Some(friend.clone());
        let err = control
            .execute_update_owner::<Empty, Empty>(deps.as_mut(), info, new_admin.clone())
            .unwrap_err();
        assert_eq!(PermissionsError::UnauthorizedForRole { label: Role::Owner.to_string() }, err);

        // owner can update
        let info = mock_info(owner.as_ref(), &[]);
        let res = control
            .execute_update_owner::<Empty, Empty>(deps.as_mut(), info, new_admin)
            .unwrap();
        assert_eq!(0, res.messages.len());

        // query shows results
        let res = control.query_role(deps.as_ref(), Role::Owner).unwrap();
        assert_eq!(Some(friend.to_string()), res.address);
    }
}