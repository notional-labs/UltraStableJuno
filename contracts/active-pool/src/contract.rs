use std::vec;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::state::{
    AddressesSet, AssetsInPool, SudoParams, ADDRESSES_SET, ASSETS_IN_POOL, SUDO_PARAMS,
};
use ultra_base::active_pool::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:active-pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const NATIVE_JUNO_DENOM: &str = "ujuno";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set the contract version in the contract storage
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store sudo params (name and owner address)
    let sudo_params = SudoParams {
        name: msg.name,
        owner: deps.api.addr_validate(&msg.owner)?,
    };

    // initial assets in pool
    let assets_in_pool = AssetsInPool {
        juno: Uint128::zero(),
        ultra_debt: Uint128::zero(),
    };

    // save sudo params and initial assets in pool in contract storage
    SUDO_PARAMS.save(deps.storage, &sudo_params)?;
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // Match on the `ExecuteMsg` to determine which function to call
    match msg {
        ExecuteMsg::IncreaseULTRADebt { amount } => {
            // Call the `execute_increase_ultra_debt` function
            execute_increase_ultra_debt(deps, env, info, amount)
        }
        ExecuteMsg::DecreaseULTRADebt { amount } => {
            // Call the `execute_decrease_ultra_debt` function
            execute_decrease_ultra_debt(deps, env, info, amount)
        }
        ExecuteMsg::SendJUNO { recipient, amount } => {
            // Call the `execute_send_juno` function
            execute_send_juno(deps, env, info, recipient, amount)
        }
        ExecuteMsg::SetAddresses {
            borrower_operations_address,
            trove_manager_address,
            stability_pool_address,
            default_pool_address,
        } =>
            // Call the `execute_set_addresses` function
            execute_set_addresses(
                deps,
                env,
                info,
                borrower_operations_address,
                trove_manager_address,
                stability_pool_address,
                default_pool_address,
            ),
    }
}

pub fn execute_increase_ultra_debt(
    deps: DepsMut, // A struct that holds references to mutable dependencies (e.g., storage)
    _env: Env, // An environment variable that holds information about the blockchain and the current transaction
    info: MessageInfo, // Information about the message that triggered this function call
    amount: Uint128, // The amount to increase the ultra debt by
) -> Result<Response, ContractError> {

    // Check that the message sender is either the BO or the TM
    only_bo_or_tm(deps.storage, &info)?;

    // Load the current assets in the pool
    let mut assets_in_pool = ASSETS_IN_POOL.load(deps.storage)?;

    // Increase the ultra debt by the specified amount
    assets_in_pool.ultra_debt += amount;

    // Save the updated assets in the pool
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;

    // Create a response object with information about the action taken
    let res = Response::new()
        .add_attribute("action", "increase_ultra_debt")
        .add_attribute("amount", amount);

    // Return the response
    Ok(res)
}

pub fn execute_decrease_ultra_debt(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {

    // Ensure that the caller is either the Board of Directors, Treasury Manager, or Shareholder Proxy
    only_bo_or_tm_or_sp(deps.storage, &info)?;

    // Load the current value of the assets in the pool
    let mut assets_in_pool = ASSETS_IN_POOL.load(deps.storage)?;

    // Check that the new value of ultra_debt will not overflow, then update the value
    assets_in_pool.ultra_debt = assets_in_pool
        .ultra_debt
        .checked_sub(amount)
        .map_err(StdError::overflow)?;

    // Save the updated value of the assets in the pool
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?;

    // Create and return the response object
    let res = Response::new()
        .add_attribute("action", "decrease_ultra_debt")
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_send_juno(
    deps: DepsMut, // a set of dependencies
    _env: Env, // an environment object
    info: MessageInfo, // a message info object
    recipient: Addr, // the recipient address
    amount: Uint128, // the amount of JUNO tokens to send
) -> Result<Response, ContractError> {
    only_bo_or_tm_or_sp(deps.storage, &info)?; // check that the caller is BO, TM, or SP

    let mut assets_in_pool = ASSETS_IN_POOL.load(deps.storage)?; // retrieve assets in pool from storage
    assets_in_pool.juno = assets_in_pool
        .juno
        .checked_sub(amount) // subtract the specified amount of JUNO tokens from the pool
        .map_err(StdError::overflow)?; // return error if there is an overflow
    ASSETS_IN_POOL.save(deps.storage, &assets_in_pool)?; // save updated assets in pool to storage

    let send_msg = BankMsg::Send { // construct a BankMsg::Send message
        to_address: recipient.to_string(),
        amount: vec![coin(amount.u128(), NATIVE_JUNO_DENOM.to_string())],
    };
    let res = Response::new() // create a new response
        .add_message(send_msg) // add the BankMsg::Send message to the response
        .add_attribute("action", "send_juno") // add an attribute to the response
        .add_attribute("recipient", recipient) // add an attribute to the response
        .add_attribute("amount", amount); // add an attribute to the response
    Ok(res) // return the response
}

// This function updates the set of contract addresses that the current contract depends on.
// It only allows the contract owner to update these addresses.
pub fn execute_set_addresses(
    deps: DepsMut, // The contract dependencies, including the storage and the API
    _env: Env, // The contract environment, which provides information about the blockchain
    info: MessageInfo, // Information about the message that triggered this contract execution
    borrower_operations_address: String, // The new address of the borrower operations contract
    trove_manager_address: String, // The new address of the trove manager contract
    stability_pool_address: String, // The new address of the stability pool contract
    default_pool_address: String, // The new address of the default pool contract
) -> Result<Response, ContractError> {
    // Ensure that only the contract owner can update the addresses set
    only_owner(deps.storage, &info)?;

    // Validate and convert the new addresses to their HEX representation
    let new_addresses_set = AddressesSet {
        borrower_operations_address: deps.api.addr_validate(&borrower_operations_address)?,
        trove_manager_address: deps.api.addr_validate(&trove_manager_address)?,
        stability_pool_address: deps.api.addr_validate(&stability_pool_address)?,
        default_pool_address: deps.api.addr_validate(&default_pool_address)?,
    };

    // Save the new addresses set in the contract storage
    ADDRESSES_SET.save(deps.storage, &new_addresses_set)?;

    // Build and return the response with the updated addresses set
    let res = Response::new()
        .add_attribute("action", "set_addresses")
        .add_attribute("borrower_operations_address", borrower_operations_address)
        .add_attribute("trove_manager_address", trove_manager_address)
        .add_attribute("stability_pool_address", stability_pool_address)
        .add_attribute("default_pool_address", default_pool_address);
    Ok(res)
}

/// Checks to enforce that only borrower operations or default pool can call
fn only_bo_or_dp(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    // Load the set of addresses
    let addresses_set = ADDRESSES_SET.load(store)?;
    // Check if the caller is not the borrower operations address or the default pool address
    if addresses_set.borrower_operations_address != info.sender.as_ref()
        && addresses_set.default_pool_address != info.sender.as_ref()
    {
        // Return an error if the caller is not authorized
        return Err(ContractError::CallerIsNeitherBONorDP {});
    }
    // Return the caller's address if the caller is authorized
    Ok(info.sender.clone())
}

/// Checks to enforce that only borrower operations or trove manager or stability pool can call
fn only_bo_or_tm_or_sp(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    // Load the set of addresses
    let addresses_set = ADDRESSES_SET.load(store)?;
    // Check if the caller is not the borrower operations address, the trove manager address, or the stability pool address
    if addresses_set.borrower_operations_address != info.sender.as_ref()
        && addresses_set.trove_manager_address != info.sender.as_ref()
        && addresses_set.stability_pool_address != info.sender.as_ref()
    {
        // Return an error if the caller is not authorized
        return Err(ContractError::CallerIsNeitherBONorTMNorSP {});
    }
    // Return the caller's address if the caller is authorized
    Ok(info.sender.clone())
}

/// This function checks if the caller of the contract is either the borrower operations address or the trove manager address.
/// If the caller is not one of these addresses, it returns an error.
///
/// # Arguments
///
/// * `store`: A reference to the contract's storage
/// * `info`: Information about the message that called the contract
///
/// # Returns
///
/// * An `Addr` representing the caller of the contract if the caller is authorized
/// * An error if the caller is not authorized
fn only_bo_or_tm(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    // Load the addresses set from storage
    let addresses_set = ADDRESSES_SET.load(store)?;

    // Check if the caller is the borrower operations address or the trove manager address.
    // If the caller is not one of these addresses, return an error.
    if addresses_set.borrower_operations_address != info.sender.as_ref()
        && addresses_set.trove_manager_address != info.sender.as_ref()
    {
        return Err(ContractError::CallerIsNeitherBONorTM {});
    }

    // If the caller is authorized, return the caller's address
    Ok(info.sender.clone())
}

/// This function checks if the caller of the contract is the owner of the contract.
/// If the caller is not the owner, it returns an error.
///
/// # Arguments
///
/// * `store`: A reference to the contract's storage
/// * `info`: Information about the message that called the contract
///
/// # Returns
///
/// * An `Addr` representing the caller of the contract if the caller is authorized
/// * An error if the caller is not authorized
fn only_owner(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    // Load the contract's parameters from storage
    let params = SUDO_PARAMS.load(store)?;

    // Check if the caller is the owner of the contract.
    // If the caller is not the owner, return an error.
    if params.owner != info.sender.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }

    // If the caller is authorized, return the caller's address
    Ok(info.sender.clone())
}

// This line sets the entry point for the code depending on whether the "library"
// feature is enabled or not.
#[cfg_attr(not(feature = "library"), entry_point)]

// This function processes different types of messages and returns the result as a binary value.
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // Match the type of the message.
    match msg {
        // If the message is a request to get the parameters, call `query_params` and return the result as a binary value.
        QueryMsg::GetParams {} => to_binary(&query_params(deps)?),

        // If the message is a request to get the JUNO state, call `query_juno_state` and return the result as a binary value.
        QueryMsg::GetJUNO {} => to_binary(&query_juno_state(deps)?),

        // If the message is a request to get the ULTRADebt state, call `query_ultra_debt_state` and return the result as a binary value.
        QueryMsg::GetULTRADebt {} => to_binary(&query_ultra_debt_state(deps)?),

        // If the message is a request to get the borrower operations address, call `query_borrower_operations_address` and return the result as a binary value.
        QueryMsg::GetBorrowerOperationsAddress {} => {
            to_binary(&query_borrower_operations_address(deps)?)
        }

        // If the message is a request to get the stability pool address, call `query_stability_pool_address` and return the result as a binary value.
        QueryMsg::GetStabilityPoolAddress {} => to_binary(&query_stability_pool_address(deps)?),

        // If the message is a request to get the default pool address, call `query_default_pool_address` and return the result as a binary value.
        QueryMsg::GetDefaultPoolAddress {} => to_binary(&query_default_pool_address(deps)?),

        // If the message is a request to get the trove manager address, call `query_trove_manager_address` and return the result as a binary value.
        QueryMsg::GetTroveManagerAddress {} => to_binary(&query_trove_manager_address(deps)?),
    }
}

// This function retrieves the JUNO state from storage and returns it as a result.
pub fn query_juno_state(deps: Deps) -> StdResult<Uint128> {
    // Load the assets in the pool from storage.
    let info = ASSETS_IN_POOL.load(deps.storage)?;

    // Retrieve the JUNO state from the assets in the pool.
    let res = info.juno;

    // Return the JUNO state.
    Ok(res)
}

// This function queries the current ultra debt state
pub fn query_ultra_debt_state(deps: Deps) -> StdResult<Uint128> {
    // Load the assets in the pool from storage
    let info = ASSETS_IN_POOL.load(deps.storage)?;

    // Return the current ultra debt state
    let res = info.ultra_debt;
    Ok(res)
}

// This function queries the current parameters
pub fn query_params(deps: Deps) -> StdResult<ParamsResponse> {
    // Load the sudo parameters from storage
    let info = SUDO_PARAMS.load(deps.storage)?;

    // Return the current name and owner of the parameters
    let res = ParamsResponse {
        name: info.name,
        owner: info.owner,
    };
    Ok(res)
}

// This function queries the current borrower operations address
pub fn query_borrower_operations_address(deps: Deps) -> StdResult<Addr> {
    // Load the addresses set from storage
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;

    // Return the current borrower operations address
    let borrower_operations_address = addresses_set.borrower_operations_address;
    Ok(borrower_operations_address)
}

// This function queries the current stability pool address
pub fn query_stability_pool_address(deps: Deps) -> StdResult<Addr> {
    // Load the addresses set from storage
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;

    // Return the current stability pool address
    let stability_pool_address = addresses_set.stability_pool_address;
    Ok(stability_pool_address)
}

// This function retrieves the default pool address from the ADDRESSES_SET,
// loads it from storage, and returns it as a StdResult.
pub fn query_default_pool_address(deps: Deps) -> StdResult<Addr> {
    // Load the addresses set from storage.
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    // Retrieve the default pool address from the set.
    let default_pool_address = addresses_set.default_pool_address;
    // Return the default pool address as a StdResult.
    Ok(default_pool_address)
}

// This function retrieves the trove manager address from the ADDRESSES_SET,
// loads it from storage, and returns it as a StdResult.
pub fn query_trove_manager_address(deps: Deps) -> StdResult<Addr> {
    // Load the addresses set from storage.
    let addresses_set = ADDRESSES_SET.load(deps.storage)?;
    // Retrieve the trove manager address from the set.
    let trove_manager_address = addresses_set.trove_manager_address;
    // Return the trove manager address as a StdResult.
    Ok(trove_manager_address)
}
