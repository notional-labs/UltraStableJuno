#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, Storage, Uint128,
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::state::{SudoParams, SUDO_PARAMS};
use ultra_base::borrower_operations::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg};
use ultra_base::querier::{query_entire_system_coll, query_entire_system_debt};

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
            upper_hint,
            lower_hint,
        ),
        ExecuteMsg::AdjustTrove {
            borrower,
            coll_withdrawal,
            ultra_change,
            is_debt_increase,
            max_fee_percentage,
            upper_hint,
            lower_hint,
        } => execute_adjust_trove(
            deps,
            env,
            info,
            borrower,
            coll_withdrawal,
            ultra_change,
            is_debt_increase,
            max_fee_percentage,
            upper_hint,
            lower_hint,
        ),
        ExecuteMsg::CloseTrove {} => execute_close_trove(deps, env, info),
        ExecuteMsg::AddColl {
            upper_hint,
            lower_hint,
        } => execute_add_coll(deps, env, info, upper_hint, lower_hint),
        ExecuteMsg::WithdrawColl {
            coll_amount,
            upper_hint,
            lower_hint,
        } => execute_withdraw_coll(deps, env, info, coll_amount, upper_hint, lower_hint),
        ExecuteMsg::ClaimCollateral {} => execute_claim_collateral(deps, env, info),
        ExecuteMsg::RepayULTRA {
            active_pool_addr,
            ultra_token_addr,
            account,
            ultra_amount,
            upper_hint,
            lower_hint,
        } => execute_repay_ultra(
            deps,
            env,
            info,
            active_pool_addr,
            ultra_token_addr,
            account,
            ultra_amount,
            upper_hint,
            lower_hint,
        ),
        ExecuteMsg::WithdrawULTRA {
            max_fee_percentage,
            ultra_amount,
            upper_hint,
            lower_hint,
        } => execute_withdraw_ultra(
            deps,
            env,
            info,
            max_fee_percentage,
            ultra_amount,
            upper_hint,
            lower_hint,
        ),
        ExecuteMsg::MoveJUNOGainToTrove {
            borrower,
            upper_hint,
            lower_hint,
        } => execute_move_juno_gain_to_trove(deps, env, info, borrower, upper_hint, lower_hint),
    }
}

pub fn execute_open_trove(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let res = Response::new().add_attribute("action", "open_trove");
    Ok(res)
}

/// Checks to enfore only borrower o can call
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
fn require_trove_is_active(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let params = SUDO_PARAMS.load(store)?;
    if params.owner != info.sender.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }
    Ok(info.sender.clone())
}

///
fn require_trove_is_not_active(
    store: &dyn Storage,
    info: &MessageInfo,
) -> Result<Addr, ContractError> {
    let params = SUDO_PARAMS.load(store)?;
    if params.owner != info.sender.as_ref() {
        return Err(ContractError::UnauthorizedOwner {});
    }
    Ok(info.sender.clone())
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
