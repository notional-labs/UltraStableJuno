#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcQuery, MessageInfo, Order, PortIdResponse,
    Response, StdResult,
};

use cw2::{get_contract_version, set_contract_version};

use crate::error::ContractError;
use crate::ibc::OracleRequestPacket;
use crate::msg::{
    ChannelResponse, ExecuteMsg, InitMsg, ListChannelsResponse, MigrateMsg, OracleMsg,
    PortResponse, QueryMsg,
};
use crate::state::{Config, CHANNEL_INFO, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "band-protocol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = Config {
        default_timeout: msg.default_timeout,
    };
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // ExecuteMsg::ReceiveOracle(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::OracleRequest(msg) => execute_oracle(deps, env, msg),
    }
}

pub fn execute_oracle(deps: DepsMut, env: Env, msg: OracleMsg) -> Result<Response, ContractError> {
    // ensure the requested channel is registered
    // FIXME: add a .has method to map to make this faster
    if CHANNEL_INFO.may_load(deps.storage, &msg.channel)?.is_none() {
        return Err(ContractError::NoSuchChannel { id: msg.channel });
    }

    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => CONFIG.load(deps.storage)?.default_timeout,
    };
    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(timeout_delta);
    let calldata = hex::decode(msg.call_data).expect("Decoding failed");

    // build band packet
    let packet = OracleRequestPacket::new(
        msg.client_id,
        msg.oracle_script_id,
        calldata,
        msg.ask_count,
        msg.min_count,
        msg.denom,
        250000,
        50000,
        300000,
    );
    packet.validate()?;

    // prepare message
    let msg = IbcMsg::SendPacket {
        channel_id: msg.channel,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    };

    // Note: we update local state when we get ack - do not count this transfer towards anything until acked
    // similar event messages like ibctransfer module

    // send response
    let res = Response::new().add_message(msg);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Port {} => to_binary(&query_port(deps)?),
        QueryMsg::ListChannels {} => to_binary(&query_list(deps)?),
        QueryMsg::Channel { id } => to_binary(&query_channel(deps, id)?),
    }
}

fn query_port(deps: Deps) -> StdResult<PortResponse> {
    let query = IbcQuery::PortId {}.into();
    let PortIdResponse { port_id } = deps.querier.query(&query)?;
    Ok(PortResponse { port_id })
}

fn query_list(deps: Deps) -> StdResult<ListChannelsResponse> {
    let channels: StdResult<Vec<_>> = CHANNEL_INFO
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect();
    Ok(ListChannelsResponse {
        channels: channels?,
    })
}

// make public for ibc tests
pub fn query_channel(deps: Deps, id: String) -> StdResult<ChannelResponse> {
    let info = CHANNEL_INFO.load(deps.storage, &id)?;
    // this returns Vec<(outstanding, total)>
    Ok(ChannelResponse { info })
}
