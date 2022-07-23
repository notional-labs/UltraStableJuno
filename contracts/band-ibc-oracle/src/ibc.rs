use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    attr, coins, entry_point, from_binary, to_binary, Binary, Coin, DepsMut, Env, IbcBasicResponse,
    IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcOrder, IbcPacket,
    IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, Reply, Response,
    SubMsgResult,
};

use crate::error::{ContractError, Never};
use crate::state::{ChannelInfo, CHANNEL_INFO};

pub const IBC_VERSION: &str = "bandchain-1";
pub const IBC_ORDERING: IbcOrder = IbcOrder::Unordered;

/// The format for sending an ics20 packet.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/applications/transfer/v1/transfer.proto#L11-L20
/// This is compatible with the JSON serialization

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct OracleRequestPacket {
    // the unique identifier of this oracle request, as specified by the client. This same unique ID will be sent back to the requester with the oracle response.
    pub client_id: String,
    // The unique identifier number assigned to the oracle script when it was first registered on Bandchain
    pub oracle_script_id: i64,
    // The data passed over to the oracle script for the script to use during its execution
    pub calldata: Vec<u8>,
    // The number of validators that are requested to respond to this request
    pub ask_count: i64,
    // The minimum number of validators necessary for the request to proceed to the execution phase
    pub min_count: i64,
    // FeeLimit is the maximum tokens that will be paid to all data source
    pub fee_limit: Vec<Coin>,
    // Prepare Gas is amount of gas to pay to prepare raw requests
    pub prepare_gas: i64,
    // Execute Gas is amount of gas to reserve for executing
    pub execute_gas: i64,
}

impl OracleRequestPacket {
    pub fn new(
        client_id: String,
        oracle_script_id: i64,
        calldata: Vec<u8>,
        ask_count: i64,
        min_count: i64,
        denom: String,
        amount: u128,
        prepare_gas: i64,
        execute_gas: i64,
    ) -> Self {
        let fee_limit = coins(amount, denom);
        OracleRequestPacket {
            client_id,
            oracle_script_id,
            calldata,
            ask_count,
            min_count,
            fee_limit,
            prepare_gas,
            execute_gas,
        }
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct OracleResponsePacket {
    // ClientID is the unique identifier matched with that of the oracle request
    // packet.
    pub client_id: String,
    // RequestID is BandChain's unique identifier for this oracle request.
    pub request_id: u64,
    // AnsCount is the number of validators among to the asked validators that
    // actually responded to this oracle request prior to this oracle request
    // being resolved.
    pub ans_count: u64,
    // RequestTime is the UNIX epoch time at which the request was sent to
    // BandChain.
    pub request_time: i64,
    // ResolveTime is the UNIX epoch time at which the request was resolved to the
    // final result.
    pub resolve_time: i64,
    // ResolveStatus is the status of this oracle request, which can be OK,
    // FAILURE, or EXPIRED.
    pub resolve_status: i32,
    // Result is the final aggregated value encoded in OBI format. Only available
    // if status if OK.
    pub result: Vec<u8>,
}

impl OracleResponsePacket {
    pub fn validate(&self) -> Result<(), ContractError> {
        Ok(())
    }
}

/// This is a generic ICS acknowledgement format.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/core/channel/v1/channel.proto#L141-L147
/// This is compatible with the JSON serialization
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Ics20Ack {
    Result(Binary),
    Error(String),
}

// create a serialized success message
fn _ack_success() -> Binary {
    let res = Ics20Ack::Result(b"1".into());
    to_binary(&res).unwrap()
}

// create a serialized error message
fn ack_fail(err: String) -> Binary {
    let res = Ics20Ack::Error(err);
    to_binary(&res).unwrap()
}

const SEND_TOKEN_ID: u64 = 1337;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    if reply.id != SEND_TOKEN_ID {
        return Err(ContractError::UnknownReplyId { id: reply.id });
    }
    let res = match reply.result {
        SubMsgResult::Ok(_) => Response::new(),
        SubMsgResult::Err(err) => {
            // encode an acknowledgement error
            Response::new().set_data(ack_fail(err))
        }
    };
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioning constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<(), ContractError> {
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// record the channel in CHANNEL_INFO
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // we need to check the counter party version in try and ack (sometimes here)
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel: IbcChannel = msg.into();
    let info = ChannelInfo {
        id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    CHANNEL_INFO.save(deps.storage, &info.id, &info)?;

    Ok(IbcBasicResponse::default())
}

fn enforce_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version != IBC_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(version) = counterparty_version {
        if version != IBC_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                version: version.to_string(),
            });
        }
    }
    if channel.order != IBC_ORDERING {
        return Err(ContractError::OnlyOrderedChannel {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    _channel: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: what to do here?
    // we will have locked funds that need to be returned somehow
    unimplemented!();
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// Check to see if we have any balance here
/// We should not return an error if possible, but rather an acknowledgement of failure
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    let packet = msg.packet;
    let res = match do_ibc_packet_receive(&packet) {
        Ok(msg) => {
            // build attributes first so we don't have to clone msg below
            // similar event messages like ibctransfer module

            // This cannot fail as we parse it in do_ibc_packet_receive. Best to pass the data somehow?

            let attributes = vec![
                attr("action", "receive"),
                attr("msg", msg),
                attr("status", "sucess"),
            ];
            IbcReceiveResponse::new().add_attributes(attributes)
        }
        Err(err) => IbcReceiveResponse::new()
            .set_ack(ack_fail(err.to_string()))
            .add_attributes(vec![
                attr("action", "receive"),
                attr("status", "false"),
                attr("error", err.to_string()),
            ]),
    };

    Ok(res)
}

// this does the work of ibc_packet_receive, we wrap it to turn errors into acknowledgements
fn do_ibc_packet_receive(packet: &IbcPacket) -> Result<String, ContractError> {
    let msg = packet.data.to_string();
    Ok(msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// check if success or failure and update balance, or return funds
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: trap error like in receive?
    let ics20msg: Ics20Ack = from_binary(&msg.acknowledgement.data)?;
    match ics20msg {
        Ics20Ack::Result(_) => on_packet_success(deps, msg.original_packet),
        Ics20Ack::Error(err) => on_packet_failure(deps, msg.original_packet, err),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// return fund to original sender (same as failure in ibc_packet_ack)
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // TODO: trap error like in receive?
    let packet = msg.packet;
    on_packet_failure(deps, packet, "timeout".to_string())
}

// update the balance stored on this (channel, denom) index
fn on_packet_success(_deps: DepsMut, packet: IbcPacket) -> Result<IbcBasicResponse, ContractError> {
    let msg: OracleRequestPacket = from_binary(&packet.data)?;
    // similar event messages like ibctransfer module
    let attributes = vec![
        attr("action", "acknowledge"),
        attr("receiver", &msg.client_id),
    ];
    Ok(IbcBasicResponse::new().add_attributes(attributes))
}

// return the tokens to sender
fn on_packet_failure(
    _deps: DepsMut,
    packet: IbcPacket,
    _err: String,
) -> Result<IbcBasicResponse, ContractError> {
    let _msg: OracleRequestPacket = from_binary(&packet.data)?;
    // similar event messages like ibctransfer module
    let attributes = vec![attr("action", "acknowledge"), attr("status", "fail")];
    Ok(IbcBasicResponse::new().add_attributes(attributes))
}
