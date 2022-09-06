use std::str::FromStr;
use std::vec;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, CosmosMsg, Decimal, Decimal256, Deps, DepsMut, Empty,
    Env, Event, MessageInfo, Response, StdError, StdResult, Storage, Uint128, Uint256, WasmMsg,
};

use cw2::set_contract_version;
use ultra_base::role_provider::Role;
use ultra_base::{active_pool, reward_distributor, trove_manager};

use crate::error::ContractError;
use crate::state::{State, SudoParams, SUDO_PARAMS};
use ultra_base::borrower_operations::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg};
use ultra_base::querier::{
    check_recovery_mode, fetch_price, query_borrowing_fee, query_entire_system_coll,
    query_entire_system_debt, require_user_accepts_fee,
};

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store sudo params
    let sudo_params = SudoParams {
        name: msg.name,
        owner: deps.api.addr_validate(&msg.owner)?,
    };

    SUDO_PARAMS.save(deps.storage, &sudo_params)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::OpenTrove {
            max_fee_percentage,
            ultra_amount,
            upper_hint,
            lower_hint,
        } => execute_open_trove(
            deps,
            env,
            info,
            max_fee_percentage,
            ultra_amount,
            // ignoring hints for now, on the assumption we can use storage indexing instead.
            // upper_hint,
            // lower_hint,
        ),
        // called during `OpenTrove` execution
        ExecuteMsg::CollectBorrowingFee {
            max_fee_percentage,
            stable_amount,
        } => _execute_open_trove_collect_borrowing_fee(
            deps,
            env,
            info,
            max_fee_percentage,
            stable_amount,
            // ignoring hints for now, on the assumption we can use storage indexing instead.
            // upper_hint,
            // lower_hint,
        ),
        ExecuteMsg::AdjustTrove {
            borrower,
            coll_withdrawal,
            ultra_change,
            is_debt_increase,
            max_fee_percentage,
            upper_hint,
            lower_hint,
        } => todo!(),
        // execute_adjust_trove(
        //     deps,
        //     env,
        //     info,
        //     borrower,
        //     coll_withdrawal,
        //     ultra_change,
        //     is_debt_increase,
        //     max_fee_percentage,
        //     upper_hint,
        //     lower_hint,
        // ),
        ExecuteMsg::CloseTrove {} => todo!(), // execute_close_trove(deps, env, info),
        ExecuteMsg::AddColl {
            upper_hint,
            lower_hint,
        } => todo!(), // execute_add_coll(deps, env, info, upper_hint, lower_hint),
        ExecuteMsg::WithdrawColl {
            coll_amount,
            upper_hint,
            lower_hint,
        } => todo!(), // execute_withdraw_coll(deps, env, info, coll_amount, upper_hint, lower_hint),
        ExecuteMsg::ClaimCollateral {} => todo!(), // execute_claim_collateral(deps, env, info),
        ExecuteMsg::RepayULTRA {
            active_pool_addr,
            ultra_token_addr,
            account,
            ultra_amount,
            upper_hint,
            lower_hint,
        } => todo!(),
        // execute_repay_ultra(
        //     deps,
        //     env,
        //     info,
        //     active_pool_addr,
        //     ultra_token_addr,
        //     account,
        //     ultra_amount,
        //     upper_hint,
        //     lower_hint,
        // ),
        ExecuteMsg::WithdrawULTRA {
            max_fee_percentage,
            ultra_amount,
            upper_hint,
            lower_hint,
        } => todo!(),
        // execute_withdraw_ultra(
        //     deps,
        //     env,
        //     info,
        //     max_fee_percentage,
        //     ultra_amount,
        //     upper_hint,
        //     lower_hint,
        // ),
        ExecuteMsg::MoveJUNOGainToTrove {
            borrower,
            upper_hint,
            lower_hint,
        } => todo!(), //  execute_move_juno_gain_to_trove(deps, env, info, borrower, upper_hint, lower_hint),
    }
}

// liquity source: https://github.com/liquity/dev/blob/main/packages/contracts/contracts/BorrowerOperations.sol#L156
pub fn execute_open_trove(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_fee_percentage: Decimal256,
    amount: Uint256,
) -> Result<Response, ContractError> {
    let state = State::default();
    // fetch price from pricefeed.
    let price_feed_addr = state
        .roles
        .load_role_address(deps.as_ref(), Role::PriceFeed)?;
    let active_pool_addr = state
        .roles
        .load_role_address(deps.as_ref(), Role::ActivePool)?;
    let default_pool_addr = state
        .roles
        .load_role_address(deps.as_ref(), Role::DefaultPool)?;
    let trove_manager_addr = state
        .roles
        .load_role_address(deps.as_ref(), Role::TroveManager)?;

    let price = fetch_price(&deps.querier, price_feed_addr)?;
    // 2. check if in recovery mode.
    let is_recovery_mode =
        check_recovery_mode(&deps.querier, price, active_pool_addr, default_pool_addr)?;
    // 3. require max fee percentage!
    require_valid_max_fee_percentage(max_fee_percentage)?;
    // 4. require that trove is not active!
    require_trove_is_not_active(deps.storage, &trove_manager_addr, &info.sender)?;

    state.temp.net_debt.save(deps.storage, &amount.clone())?;

    let mut msgs: Vec<CosmosMsg<Empty>> = vec![];
    if !is_recovery_mode {
        // Update Decay Rate for Trove Manager
        let decay_wasm_msg = to_binary(&trove_manager::ExecuteMsg::DecayBaseRateFromBorrowing {})?;
        let decay_base_rate_msg: CosmosMsg<Empty> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: trove_manager_addr.to_string(),
            msg: decay_wasm_msg,
            funds: vec![],
        });
        msgs.push(decay_base_rate_msg);

        // Attempt to Collect Fee from Sender
        let collect_borrowing_fee_wasm_msg = to_binary(&ExecuteMsg::CollectBorrowingFee {
            max_fee_percentage,
            stable_amount: amount,
        })?;
        let collect_borrowing_fee_msg: CosmosMsg<Empty> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: collect_borrowing_fee_wasm_msg,
            funds: vec![],
        });
        msgs.push(collect_borrowing_fee_msg);
    }

    // TODO: Add descriptive attributes
    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "open_trove");
    Ok(res)
}

// liquity source https://github.com/liquity/dev/blob/e76b000e9558640e9479b8080786a9fbc47ed570/packages/contracts/contracts/BorrowerOperations.sol#L363-L374
pub fn _execute_open_trove_collect_borrowing_fee(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    max_fee_percentage: Decimal256,
    stable_amount: Uint256,
) -> Result<Response, ContractError> {
    // can only be called by itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }
    // already decayed base rate.
    // get borrowing fee.
    let state = State::default();
    let trove_manager_addr = state
        .roles
        .load_role_address(deps.as_ref(), Role::TroveManager)?;
    let borrowing_fee = query_borrowing_fee(&deps.querier, trove_manager_addr, stable_amount)?;

    // ensure user accepts fee.
    require_user_accepts_fee(
        &deps.querier,
        borrowing_fee,
        stable_amount,
        max_fee_percentage,
    )?;

    // setting temp states https://github.com/liquity/dev/blob/e76b000e9558640e9479b8080786a9fbc47ed570/packages/contracts/contracts/BorrowerOperations.sol#L170-L171
    state
        .temp
        .borrowing_fee
        .save(deps.storage, &borrowing_fee)?;

    state
        .temp
        .net_debt
        .update(deps.storage, |net_debt| Ok(net_debt + &borrowing_fee))
        .map_err(ContractError::Std)?;

    // Mint borrowing fee amount as stablecoin.
    let stable_token_address = state
        .roles
        .load_role_address(deps.as_ref(), Role::UltraToken)?;
    let mint_cw20_msg = cw20::Cw20ExecuteMsg::Mint {
        amount: borrowing_fee.try_into().map_err(StdError::from)?,
        recipient: env.contract.address.to_string(),
    };
    let mint_cosmos_msg: CosmosMsg<Empty> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stable_token_address.to_string(),
        msg: to_binary(&mint_cw20_msg)?,
        funds: vec![],
    });

    // idea being this is, or emulates a DAODAO reward distributor contract.
    let distributor_contract_address = state
        .roles
        .load_role_address(deps.as_ref(), Role::BorrowerFeeDistributor)?;
    // TODO: Migrate to TokenFactory cosmos messages
    let send_and_fund_cw20_msg = cw20::Cw20ExecuteMsg::Send {
        amount: borrowing_fee.try_into().map_err(StdError::from)?,
        contract: distributor_contract_address.to_string(),
        msg: to_binary(&reward_distributor::ExecuteMsg::Fund {})?,
    };
    let send_and_fund_cosmos_msg: CosmosMsg<Empty> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stable_token_address.to_string(),
        msg: to_binary(&send_and_fund_cw20_msg)?,
        funds: vec![],
    });

    // prefer events over attibutes here, as attributes can get messy in the root wasm event.
    let fee_collection_event = Event::new("ultra/borrower_operations/borrowing_fee_collected")
        .add_attribute("amount", borrowing_fee.to_string())
        .add_attribute("stable_amount", stable_amount.to_string())
        .add_attribute("max_fee_percentage", max_fee_percentage.to_string())
        .add_attribute("borrowing_fee", borrowing_fee.to_string());
    // TODO: Add detailed response attributes
    Ok(Response::new()
        .add_attribute("action", "collect_borrowing_fee")
        .add_event(fee_collection_event)
        .add_message(mint_cosmos_msg)
        .add_message(send_and_fund_cosmos_msg))
}

/// Checks to enfore only borrower can call
fn only_borrower(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    return Err(ContractError::CallerIsNotBorrower {});
}
/// Checks to enfore only owner can call
fn only_owner(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let params = SUDO_PARAMS.load(store)?;
    if params.owner != info.sender.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }
    Ok(info.sender.clone())
}

///
fn require_valid_max_fee_percentage(max_fee_percentage: Decimal256) -> Result<(), ContractError> {
    if max_fee_percentage <= Decimal256::percent(100) {
        return Err(ContractError::InvalidMaxFeePercentage {});
    }
    Ok(())
}

///
fn require_trove_is_active(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    todo!("Implement with https://github.com/liquity/dev/blob/e76b000e9558640e9479b8080786a9fbc47ed570/packages/contracts/contracts/BorrowerOperations.sol#L477");
    let params = SUDO_PARAMS.load(store)?;
    if params.owner != info.sender.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }
    Ok(info.sender.clone())
}

///
fn require_trove_is_not_active(
    store: &dyn Storage,
    trove_manager: &Addr,
    borrower: &Addr,
) -> Result<(), ContractError> {
    todo!("Implement with https://github.com/liquity/dev/blob/e76b000e9558640e9479b8080786a9fbc47ed570/packages/contracts/contracts/BorrowerOperations.sol#L482");
    let params = SUDO_PARAMS.load(store)?;
    if params.owner != borrower.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetParams {} => to_binary(&query_params(deps)?),
        QueryMsg::GetEntireSystemColl {
            active_pool_addr,
            default_pool_addr,
        } => to_binary(&query_entire_system_coll(
            &deps.querier,
            active_pool_addr,
            default_pool_addr,
        )?),
        QueryMsg::GetEntireSystemDebt {
            active_pool_addr,
            default_pool_addr,
        } => to_binary(&query_entire_system_debt(
            &deps.querier,
            active_pool_addr,
            default_pool_addr,
        )?),
    }
}

pub fn query_params(deps: Deps) -> StdResult<ParamsResponse> {
    let info = SUDO_PARAMS.load(deps.storage)?;
    let res = ParamsResponse {
        name: info.name,
        owner: info.owner,
    };
    Ok(res)
}
