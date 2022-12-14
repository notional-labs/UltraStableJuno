use cosmwasm_std::{Deps, Addr, Uint256};
use ultra_base::role_provider::Role;

use crate::{ContractError, state::{DATA, ROLE_CONSUMER, NODES}};

pub fn validate_insert_position(
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

pub fn find_insert_position(
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
            || nicr > prev_id_nicr{
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
            || nicr < next_id_nicr{
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

pub fn descend_list(deps: Deps, nicr: Uint256, start_id: Option<Addr>) -> Result<(Option<Addr>, Option<Addr>), ContractError>{
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

pub fn ascend_list(deps: Deps, nicr: Uint256, start_id: Option<Addr>) -> Result<(Option<Addr>, Option<Addr>), ContractError>{
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