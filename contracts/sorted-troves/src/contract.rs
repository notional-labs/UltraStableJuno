
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage, Addr, Uint128, StdError, Decimal256, Uint256, QuerierWrapper, Deps};

use cw2::set_contract_version;
use cw_utils::maybe_addr;
use ultra_base::role_provider::Role;

use crate::{state::{SudoParams, SUDO_PARAMS, ADMIN, ROLE_CONSUMER, NODES, DATA, Data, Node}, ContractError};
use ultra_base::sorted_troves::{InstantiateMsg, ExecuteMsg, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:trove-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // set admin so that only admin can access to update role function
    let api = deps.api;
    ADMIN.set(deps.branch(), maybe_addr(api, Some(msg.owner.clone()))?)?;
    
     // store sudo params
     let sudo_params = SudoParams {
        name: msg.name,
        owner: deps.api.addr_validate(&msg.owner)?,
    };
    SUDO_PARAMS.save(deps.storage, &sudo_params)?;

    DATA.save(deps.storage, &Data { 
        head: None, 
        tail: None, 
        max_size: Uint256::zero(), 
        size: Uint256::zero()})?;
    Ok(Response::default())
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
) -> Result<Response, ContractError> {
    match msg{
        ExecuteMsg::UpdateAdmin { admin } => {
            Ok(ADMIN.execute_update_admin(deps, info, Some(admin))?)
        }
        ExecuteMsg::UpdateRole { role_provider } => {
            execute_update_role(deps, env, info, role_provider)
        }
        ExecuteMsg::Insert { id, nicr, prev_id, next_id } => {
            execute_insert(deps, env, info, id, nicr, prev_id, next_id)
        }
        ExecuteMsg::Remove { id } => {
            execute_remove(deps, env, info, id)
        }
        ExecuteMsg::ReInsert { id, new_nicr, prev_id, next_id } => {
            execute_reinsert(deps, env, info, id, new_nicr, prev_id, next_id)
        }
        ExecuteMsg::SetParams { size } => {
            execute_set_params(deps, env, info, size)
        }
    }
}

pub fn execute_update_role(
    deps: DepsMut, 
    _env: Env,
    info: MessageInfo,
    role_provider: Addr
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    ROLE_CONSUMER.add_role_provider(deps.storage, role_provider.clone())?;

    let res = Response::new()
        .add_attribute("action", "update_role")
        .add_attribute("role_provider_addr", role_provider);
    Ok(res)
}

pub fn execute_insert(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    id: String,
    nicr: Uint256,
    prev_id: Option<String>,
    next_id: Option<String>,
) -> Result<Response, ContractError> {
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations, Role::TroveManager],
        )?;

    let id_addr = deps.api.addr_validate(&id)?;
    let mut data = DATA.load(deps.storage)?;

    if data.is_full(){
        return Err(ContractError::ListIsFull {  })
    }
    if NODES.may_load(deps.storage, id_addr.clone())?.is_some() {
        return Err(ContractError::ListAlreadyContainsId {})
    }
    if nicr.is_zero() {
        return Err(ContractError::NICRMustBePositive {  })
    }

    let mut prev_id = maybe_addr(deps.api, prev_id)?;
    let mut next_id = maybe_addr(deps.api, next_id)?;

    if !validate_insert_position(deps.as_ref(), nicr, prev_id.clone(), next_id.clone())? {
        (prev_id, next_id) = find_insert_position(deps.as_ref(), nicr, prev_id, next_id)?;
    }

    if prev_id.is_none() && next_id.is_none() {
        // Insert as head and tail
        data.head = Some(id_addr.clone());
        data.tail = Some(id_addr.clone());    
    } else if prev_id.is_none() {
        // Insert before `prev_id` as the head
        NODES.save(deps.storage, id_addr.clone(), &Node{
            prev_id: None,
            next_id: data.head.clone()
        })?;

        NODES.update(deps.storage, data.head.clone().unwrap(), | node| -> Result<Node, ContractError> {
            if node.is_none() {
                Ok(Node{
                    prev_id: Some(id_addr.clone()),
                    next_id: None
                })
            } else {
                let mut node = node.unwrap();
                node.prev_id = Some(id_addr.clone());
                Ok(node)
            }
        })?;

        data.head = Some(id_addr);
    } else if next_id.is_none() {
        // Insert after `next_id` as the tail
        NODES.save(deps.storage, id_addr.clone(), &Node{
            prev_id: data.tail.clone(),
            next_id: None
        })?;

        NODES.update(deps.storage, data.tail.clone().unwrap(), | node| -> Result<Node, ContractError> {
            if node.is_none() {
                Ok(Node{
                    prev_id: None,
                    next_id: Some(id_addr.clone())
                })
            } else {
                let mut node = node.unwrap();
                node.next_id = Some(id_addr.clone());
                Ok(node)
            }
        })?;

        data.tail = Some(id_addr);
    } else {
        NODES.save(deps.storage, id_addr.clone(), &Node{
                    prev_id: prev_id.clone(),
                    next_id: next_id.clone(),
        })?;
        NODES.update(deps.storage, prev_id.clone().unwrap(), | node| -> Result<Node, ContractError> {
            if node.is_none() {
                Ok(Node{
                    prev_id: None,
                    next_id: Some(id_addr.clone())
                })
            } else {
                let mut node = node.unwrap();
                node.next_id =  Some(id_addr.clone());
                Ok(node)
            }
        })?;
        NODES.update(deps.storage, next_id.clone().unwrap(), | node| -> Result<Node, ContractError> {
            if node.is_none() {
                Ok(Node{
                    prev_id: Some(id_addr.clone()),
                    next_id:  None
                })
            } else {
                let mut node = node.unwrap();
                node.prev_id =  Some(id_addr.clone());
                Ok(node)
            }
        })?;
    }

    data.size += Uint256::from_u128(1u128);
    DATA.save(deps.storage, &data)?;
    // if data.size > Uint256::from_u128(1u128) {

    // } else {
    //     data.head = 
    // }

    let res = Response::new()
        .add_attribute("action", "insert")
        .add_attribute("id", id.to_string())
        .add_attribute("nicr", nicr);
    Ok(res)
}

pub fn execute_remove(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    id: String,
)-> Result<Response, ContractError> {
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::TroveManager],
        )?;

    let mut data = DATA.load(deps.storage)?;
    let id_addr = deps.api.addr_validate(&id)?;
    if NODES.may_load(deps.storage, id_addr.clone())?.is_none() {
        return Err(ContractError::ListNotContainId {})
    }

    if data.size > Uint256::from_u128(1u128) {
        // List contains more than a single node
        if data.head == Some(id_addr.clone()) {
            // The removed node is the head
            // Set head to next node
            data.head = NODES.load(deps.storage, id_addr.clone())?.next_id;
            // Set prev pointer of new head to null
            NODES.update(deps.storage, data.head.clone().unwrap(), |node| -> Result<Node, ContractError>{
                if node.is_none() {
                    Ok(Node{
                        prev_id: None,
                        next_id: None
                    })
                } else {
                    let mut node = node.unwrap();
                    node.prev_id =  None;
                    Ok(node)
                }
            })?;
        } else if data.tail == Some(id_addr.clone()){
            // The removed node is the tail
            // Set tail to previous node
            data.tail = NODES.load(deps.storage, id_addr.clone())?.prev_id;
            // Set next pointer of new tail to null
            NODES.update(deps.storage, data.tail.clone().unwrap(), |node| -> Result<Node, ContractError>{
                if node.is_none() {
                    Ok(Node{
                        prev_id: None,
                        next_id: None
                    })
                } else {
                    let mut node = node.unwrap();
                    node.next_id =  None;
                    Ok(node)
                }
            })?;
        } else {
            // The removed node is neither the head nor the tail
            let node_id = NODES.load(deps.storage, id_addr.clone())?;

            // Set next pointer of previous node to the next node           
            NODES.update(deps.storage, node_id.prev_id.clone().unwrap(), |node| -> Result<Node, ContractError>{
                if node.is_none() {
                    Ok(Node{
                        prev_id: node_id.next_id.clone(),
                        next_id: None
                    })
                } else {
                    let mut node = node.unwrap();
                    node.prev_id = node_id.next_id.clone();
                    Ok(node)
                }
            })?;
            // Set prev pointer of next node to the previous node
            NODES.update(deps.storage, node_id.next_id.unwrap(), |node| -> Result<Node, ContractError>{
                if node.is_none() {
                    Ok(Node{
                        prev_id: None,
                        next_id: node_id.prev_id
                    })
                } else {
                    let mut node = node.unwrap();
                    node.next_id = node_id.prev_id;
                    Ok(node)
                }
            })?;
        }
    } else { 
        // List contains a single node
        // Set the head and tail to None
        data.head = None;
        data.tail = None;
    }

    NODES.remove(deps.storage, id_addr);
    data.size -= Uint256::from_u128(1u128);
    DATA.save(deps.storage, &data)?;
    let res = Response::new()
        .add_attribute("action", "remove")
        .add_attribute("id", id.to_string());
    Ok(res)
}

pub fn execute_reinsert(
    mut deps: DepsMut, 
    env: Env, 
    info: MessageInfo, 
    id: String,
    new_nicr: Uint256,
    prev_id: Option<String>,
    next_id: Option<String>,
) -> Result<Response, ContractError> {
    ROLE_CONSUMER
        .assert_role(
            deps.as_ref(), 
            &info.sender,
            vec![Role::BorrowerOperations, Role::TroveManager],
        )?;

    execute_remove(deps.branch(), env.clone(), info.clone(), id.clone())?;
    execute_insert(deps, env, info, id.clone(), new_nicr, prev_id, next_id)?; 
    let res = Response::new()
        .add_attribute("action", "reinsert")
        .add_attribute("id", id.to_string())
        .add_attribute("new_nicr", new_nicr);
    Ok(res)
}

pub fn execute_set_params(
    deps: DepsMut, 
    _env: Env, 
    _info: MessageInfo, 
    size: Uint256
) -> Result<Response, ContractError> {
    if size.is_zero() {
        return Err(ContractError::SizeIsZero {  })
    }
    DATA.update(deps.storage, |mut data| -> Result<Data, ContractError> {
        data.max_size = size;
        Ok(data)
    })?;    

    let res = Response::new()
        .add_attribute("action", "set_params")
        .add_attribute("size", size);
    Ok(res)
}


fn validate_insert_position(
    deps: Deps, 
    nicr: Uint256, 
    prev_id: Option<Addr>, 
    next_id: Option<Addr>
) -> Result<bool, ContractError> {
    let data = DATA.load(deps.storage)?;
    let trove_manager = ROLE_CONSUMER.load_role_address(deps, Role::TroveManager)?;
    if prev_id.is_none() && next_id.is_none() {
        return Ok(data.is_empty() )
    } else if prev_id.is_none() {
        // `next_id` is the head of the list
        let next_id_nicr: Uint256 = deps.querier.query_wasm_smart(
            trove_manager,
            &ultra_base::trove_manager::QueryMsg::GetNominalICR { 
                borrower:  next_id.clone().unwrap().to_string()
            }
        )?;
        return Ok(data.head == next_id && nicr >= next_id_nicr)
    } else if next_id.is_none() {
        // `prev_id` is the tail of the list
        let prev_id_nicr: Uint256 = deps.querier.query_wasm_smart(
            trove_manager,
            &ultra_base::trove_manager::QueryMsg::GetNominalICR { 
                borrower:  prev_id.clone().unwrap().to_string()
            }
        )?;
        return Ok(data.tail == prev_id && nicr <= prev_id_nicr)
    } else {
        let prev_id = prev_id.unwrap();
        let next_id = next_id.unwrap();
        let prev_id_nicr: Uint256 = deps.querier.query_wasm_smart(
            trove_manager.clone(),
            &ultra_base::trove_manager::QueryMsg::GetNominalICR { 
                borrower:  prev_id.to_string()
            }
        )?;
        let next_id_nicr: Uint256 = deps.querier.query_wasm_smart(
            trove_manager,
            &ultra_base::trove_manager::QueryMsg::GetNominalICR { 
                borrower:  next_id.to_string()
            }
        )?;

        return Ok(
            NODES.load(deps.storage, prev_id)?.next_id == Some(next_id)
            && prev_id_nicr >= nicr
            && nicr >= next_id_nicr
        )
    }
}

fn find_insert_position(
    deps: Deps, 
    nicr: Uint256, 
    prev_id: Option<Addr>, 
    next_id: Option<Addr>
) -> Result<(Option<Addr>, Option<Addr>), ContractError>{
    let data = DATA.load(deps.storage)?;
    let trove_manager = ROLE_CONSUMER.load_role_address(deps, Role::TroveManager)?;

    let mut prev_id = prev_id;
    let mut next_id = next_id;
    if prev_id.is_some() {
        let prev_id_nicr: Uint256 = deps.querier.query_wasm_smart(
            trove_manager.clone(),
            &ultra_base::trove_manager::QueryMsg::GetNominalICR { 
                borrower:  prev_id.clone().unwrap().to_string()
            }
        )?;
        if NODES.may_load(deps.storage, prev_id.clone().unwrap())?.is_none()
            && nicr > prev_id_nicr{
            // `prev_id` does not exist anymore or now has a smaller NICR than the given NICR
            prev_id = None;
        }
    } 

    if next_id.is_some() {
        let next_id_nicr: Uint256 = deps.querier.query_wasm_smart(
            trove_manager.clone(),
            &ultra_base::trove_manager::QueryMsg::GetNominalICR { 
                borrower:  next_id.clone().unwrap().to_string()
            }
        )?;
        if NODES.may_load(deps.storage, next_id.clone().unwrap())?.is_none()
            && nicr < next_id_nicr{
            // `prev_id` does not exist anymore or now has a smaller NICR than the given NICR
            next_id = None;
        }
    }

    if prev_id.is_none() && next_id.is_none() {
        // No hint - descend list starting from head
        return descend_list(deps, nicr, data.head);
    } else if prev_id.is_none() {
        // No `prev_id` for hint - ascend list starting from `next_id`
        return ascend_list(deps, nicr, next_id);
    } else if next_id.is_none() {
        // No `next_id` for hint - descend list starting from `prev_id`
        return descend_list(deps, nicr, prev_id);
    } else {
        // Descend list starting from `prevId`
        return descend_list(deps, nicr, prev_id);

    }
}

fn descend_list(deps: Deps, nicr: Uint256, start_id: Option<Addr>) -> Result<(Option<Addr>, Option<Addr>), ContractError>{
    let data = DATA.load(deps.storage)?;
    let trove_manager = ROLE_CONSUMER.load_role_address(deps, Role::TroveManager)?;
    if start_id.is_none() {
        return Err(ContractError::StartIdIsNone {  })
    }
    let start_id_nicr: Uint256 = deps.querier.query_wasm_smart(
        trove_manager.clone(),
        &ultra_base::trove_manager::QueryMsg::GetNominalICR { 
            borrower:  start_id.clone().unwrap().to_string()
        }
    )?;

    // If `start_id` is the head, check if the insert position is before the head
    if data.head == start_id && nicr >= start_id_nicr {
        return Ok((None, start_id))
    } 

    let mut prev_id = start_id.clone();
    let mut next_id = NODES.load(deps.storage, prev_id.clone().unwrap())?.next_id;

    while prev_id.is_some() && !validate_insert_position(deps, nicr, prev_id.clone(), next_id.clone())?{
        prev_id = NODES.load(deps.storage, prev_id.clone().unwrap())?.next_id;
        next_id = NODES.load(deps.storage, prev_id.clone().unwrap())?.next_id;
    } 
    Ok((prev_id, next_id))
}

fn ascend_list(deps: Deps, nicr: Uint256, start_id: Option<Addr>) -> Result<(Option<Addr>, Option<Addr>), ContractError>{
    let data = DATA.load(deps.storage)?;
    let trove_manager = ROLE_CONSUMER.load_role_address(deps, Role::TroveManager)?;
    if start_id.is_none() {
        return Err(ContractError::StartIdIsNone {  })
    }
    let start_id_nicr: Uint256 = deps.querier.query_wasm_smart(
        trove_manager.clone(),
        &ultra_base::trove_manager::QueryMsg::GetNominalICR { 
            borrower:  start_id.clone().unwrap().to_string()
        }
    )?;

    // If `start_id` is the head, check if the insert position is before the head
    if data.tail == start_id && nicr <= start_id_nicr {
        return Ok((start_id, None))
    } 

    let mut next_id = start_id.clone();
    let mut prev_id = NODES.load(deps.storage, next_id.clone().unwrap())?.prev_id;

    while prev_id.is_some() && !validate_insert_position(deps, nicr, prev_id.clone(), next_id.clone())?{
        next_id = NODES.load(deps.storage, next_id.clone().unwrap())?.prev_id;
        prev_id = NODES.load(deps.storage, next_id.clone().unwrap())?.prev_id;
    } 
    Ok((prev_id, next_id))
}