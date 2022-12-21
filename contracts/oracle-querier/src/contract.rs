use std::{ops::Deref, vec};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr,to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo,
    QuerierWrapper, QueryRequest, Response, StdResult
};

use cw2::set_contract_version;

use crate::{error::ContractError, state::RATE};
use ultra_base::oracle::{
    ExchangeRateResponse, ExecuteMsg, InstantiateMsg, OracleQuery, QueryMsg, UltraQuery,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oracle-querier";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::GetExchangeRate { denom } => get_exchange_rate(deps, denom),
    }
}

pub fn get_exchange_rate(deps: DepsMut, denom: String) -> Result<Response, ContractError> {
    let query: UltraQuery = UltraQuery::Oracle(OracleQuery::ExchangeRate {
        denom: denom.clone(),
    });
    let request: QueryRequest<UltraQuery> = UltraQuery::into(query);
    let querier: QuerierWrapper<UltraQuery> =
        QuerierWrapper::<UltraQuery>::new(deps.querier.deref());
    let exchangerate: ExchangeRateResponse = querier.query(&request)?;
    RATE.save(deps.storage, denom.clone(), &exchangerate.rate)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "get_rate"),
        attr("denom", denom),
        attr("rate", exchangerate.rate.to_string()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ExchangeRate { denom } => to_binary(&query_exchange_rate(deps, denom)?),
    }
}

pub fn query_exchange_rate(deps: Deps, denom: String) -> StdResult<Decimal> {
    let rate = RATE.load(deps.storage, denom)?;
    Ok(rate)
}
