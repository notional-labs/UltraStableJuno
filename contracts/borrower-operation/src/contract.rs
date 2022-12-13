#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Storage, Uint128, WasmMsg, attr, Attribute,
};

use cw2::set_contract_version;
use ultra_base::ultra_math::{compute_cr, compute_nominal_cr};
use ultra_base::{active_pool, trove_manager, ultra_token};

use crate::assert::{require_ICR_above_CCR, require_ICR_above_MCR, require_newTCR_above_CCR};
use crate::error::ContractError;
use crate::state::{Config, SudoParams, CONFIG, SUDO_PARAMS};
use ultra_base::borrower_operations::{ExecuteMsg, InstantiateMsg, ParamsResponse, QueryMsg};
use ultra_base::querier::{
    check_recovery_mode, query_entire_system_coll, query_entire_system_debt,
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

    let config = Config {
        trove_manager: deps.api.addr_canonicalize(&msg.trove_manager)?,
        active_pool: deps.api.addr_canonicalize(&msg.active_pool)?,
        default_pool: deps.api.addr_canonicalize(&msg.default_pool)?,
        stability_pool: deps.api.addr_canonicalize(&msg.stability_pool)?,
        gas_pool: deps.api.addr_canonicalize(&msg.gas_pool)?,
        coll_surplus_pool: deps.api.addr_canonicalize(&msg.coll_surplus_pool)?,
        price_feed: deps.api.addr_canonicalize(&msg.price_feed)?,
        sorted_troves: deps.api.addr_canonicalize(&msg.sorted_troves)?,
        ultra: deps.api.addr_canonicalize(&msg.ultra)?,
        lqty_staking: deps.api.addr_canonicalize(&msg.lqty_staking)?,
    };

    CONFIG.save(deps.storage, &config)?;

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
    max_fee_percentage: Uint128,
    ultra_amount: Uint128,
    upper_hint: String,
    lower_hint: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let mut attributes: Vec<Attribute> = vec![
        attr("action", "open_trove"),
    ];
    let res: Response = Response::new().add_attribute("action", "open_trove");

    if max_fee_percentage > Uint128::zero() {
        require_valid_maxFeePercentage(max_fee_percentage);
    }
    let price = getPrice()?;
    let active_pool: Addr = deps.api.addr_humanize(&config.active_pool)?;
    let default_pool: Addr = deps.api.addr_humanize(&config.default_pool)?;
    let recovery_mode = check_recovery_mode(&deps.querier, price, active_pool, default_pool)?;

    let mut net_debt: Uint128 = ultra_amount;
    let ultra_fee: Uint128;

    if !recovery_mode && ultra_amount > Uint128::zero() {
        ultra_fee = trigger_borrowing_fee(
            deps.as_ref(),
            deps.api.addr_humanize(&config.trove_manager)?,
            deps.api.addr_humanize(&config.ultra)?,
            deps.api.addr_humanize(&config.lqty_staking)?,
            ultra_amount,
            max_fee_percentage,
        )?;
        net_debt += ultra_fee;
    }

    assert_at_least_min_net_debt(net_debt)?;

    let composite_debt: Uint128 = net_debt + Uint128::from(1950000000000000000000u64);

    let coin_denom = "juno".to_string();
    let payment = info
        .funds
        .iter()
        .find(|x| x.denom == coin_denom && x.amount > Uint128::zero())
        .ok_or_else(|| {
            StdError::generic_err(format!("No {} assets are provided to bond", coin_denom))
        })?;

    let ICR = compute_cr(payment.amount, composite_debt, price)?;
    let NICR = compute_nominal_cr(payment.amount, composite_debt)?;
    let newTCR: Uint128;
    if recovery_mode {
        require_ICR_above_CCR(ICR)?;
    } else {
        require_ICR_above_MCR(ICR)?;
        newTCR: Uint128 = get_newTCT_from_trove_change()?;
        require_newTCR_above_CCR(ICR)?;
    }

    let mut messages = vec![];

    messages.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.trove_manager)?.to_string(),
        msg: to_binary(&trove_manager::ExecuteMsg::SetTroveStatus {
            borrower: info.sender.to_string(),
            num: Uint128::one(),
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.trove_manager)?.to_string(),
        msg: to_binary(&trove_manager::ExecuteMsg::IncreaseTroveColl {
            borrower: info.sender.to_string(),
            coll_increase: payment.amount,
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.trove_manager)?.to_string(),
        msg: to_binary(&trove_manager::ExecuteMsg::IncreaseTroveDebt {
            borrower: info.sender.to_string(),
            debt_increase: composite_debt,
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.trove_manager)?.to_string(),
        msg: to_binary(&trove_manager::ExecuteMsg::UpdateTroveRewardSnapshots {
            borrower: info.sender.to_string(),
        })?,
        funds: vec![],
    });

    // TODO: get Stake value callback from trove manager (to emit event)
    messages.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.trove_manager)?.to_string(),
        msg: to_binary(&trove_manager::ExecuteMsg::UpdateStakeAndTotalStakes {
            borrower: info.sender.to_string(),
        })?,
        funds: vec![],
    });

    messages.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.trove_manager)?.to_string(),
        msg: to_binary(&trove_manager::ExecuteMsg::AddTroveOwnerToArray {
            borrower: info.sender.to_string(),
        })?,
        funds: vec![],
    });

    // TODO: Confirm recipient.
    // TODO: Check if amount param is redundant
    messages.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.active_pool)?.to_string(),
        msg: to_binary(&active_pool::ExecuteMsg::SendJUNO {
            recipient: info.sender,
            amount: payment.amount,
        })?,
        funds: vec![],
    });

    if ultra_amount > Uint128::zero() {
        messages.append(&mut withdraw_ultra_msgs(
            deps.as_ref(),
            config.active_pool,
            config.ultra,
            info.sender,
            ultra_amount,
            net_debt,
        )?);
    }
    messages.append(&mut withdraw_ultra_msgs(
        deps.as_ref(),
        config.active_pool,
        config.ultra,
        deps.api.addr_humanize(&config.gas_pool)?,
        ultra_amount,
        net_debt,
    )?);

    // TODO: Add events for TroveCreated, TroveUpdated, UltraBorrowingFeePaid

    Ok(Response::new().add_attributes(attributes).add_messages(messages))
}

fn require_valid_maxFeePercentage(max_fee_percentage: Uint128) -> Result<(), ContractError> {
    if max_fee_percentage < Uint128::from(5_000_000_000_000_000u128)
        || max_fee_percentage >= Uint128::from(1000000000000000000u128)
    {
        return Err(ContractError::InvalidMaxFeePercentage {});
    }
    Ok(())
}

fn trigger_borrowing_fee(
    deps: Deps,
    trove_manager: Addr,
    ultra_addr: Addr,
    lqty_staking: Addr,
    ultra_amount: Uint128,
    max_fee_percentage: Uint128,
) -> Result<Uint128, ContractError> {
    Ok(Uint128::zero())
}

fn withdraw_ultra_msgs(
    deps: Deps,
    active_pool: CanonicalAddr,
    ultra_addr: CanonicalAddr,
    account: Addr,
    ultra_amount: Uint128,
    net_debt_increase: Uint128,
) -> Result<Vec<WasmMsg>, ContractError> {
    let mut msgs = vec![];
    msgs.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&active_pool)?.to_string(),
        msg: to_binary(&active_pool::ExecuteMsg::IncreaseULTRADebt {
            amount: net_debt_increase,
        })?,
        funds: vec![],
    });

    // TODO: Check burn logic.
    msgs.push(WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&ultra_addr)?.to_string(),
        msg: to_binary(&ultra_token::ExecuteMsg::Burn {
            amount: ultra_amount,
        })?,
        funds: vec![],
    });
    Ok(msgs)
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
