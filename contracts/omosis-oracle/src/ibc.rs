use cosmwasm_std::{
    entry_point, from_slice, DepsMut, Env, IbcBasicResponse,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, StdResult, IbcOrder, from_binary, Ibc3ChannelOpenResponse,
};

use crate::{state::{CHANNEL, PRICE}, ContractError, ibc_msg::{PacketMsg, PacketAck, GammPacket, SpotPriceAck, EstimateSwapAck}};

pub const GAMM_ORDER: IbcOrder = IbcOrder::Unordered;
pub const GAMM_VERSION: &str = "osmosis-price-v1";
pub const DEFAULT_PACKET_LIFETIME: u64 = 60 * 60;

#[entry_point]
pub fn ibc_channel_open(
    _deps: DepsMut, 
    _env: Env, 
    msg: IbcChannelOpenMsg
) -> Result<Option<Ibc3ChannelOpenResponse>, ContractError>{
    let channel = msg.channel();

    if channel.order != GAMM_ORDER{
        return Err(ContractError::InvalidChannelOrder {})
    }

    if channel.version.as_str() != GAMM_VERSION {
        return Err(ContractError::InvalidChannelVersion( GAMM_VERSION));
    }
    
    if let Some(version) = msg.counterparty_version() {
        if version != GAMM_VERSION {
            return Err(ContractError::InvalidCounterpartyVersion( GAMM_VERSION));
        }
    }

    Ok(None)
}


#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let channel_id = &channel.endpoint.channel_id;

    CHANNEL.save(deps.storage, &Some(channel_id.to_string()))?;
    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
pub fn ibc_channel_close(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let channel_id = &channel.endpoint.channel_id;
    CHANNEL.save(deps.storage, &None)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    _packet: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    unimplemented!();
}

#[entry_point]
pub fn ibc_packet_ack(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {
    // which local channel was this packet send from
    let caller = msg.original_packet.src.channel_id;
    // we need to parse the ack based on our request
    let packet: PacketMsg = from_slice(&msg.original_packet.data)?;
    let ack: PacketAck = from_binary(&msg.acknowledgement.data)?;
    match packet.query {
        GammPacket::SpotPrice(_) => ack_spot_price_result(deps, env, caller, ack),
        GammPacket::EstimateSwap(_) => ack_estimate_swap_result(deps, env, caller, ack),
    }
}

fn ack_spot_price_result(
    deps: DepsMut,
    env: Env,
    _caller: String,
    ack: PacketAck,
) -> StdResult<IbcBasicResponse> {
    let result: SpotPriceAck = match ack {
        PacketAck::Result(data) => from_binary(&data)?,
        PacketAck::Error(e) => {
            return Ok(IbcBasicResponse::new()
                .add_attribute("action", "receive_spot_price")
                .add_attribute("error", e))
        }
    };
    PRICE.update(deps.storage, |mut price| -> StdResult<_> {
        price.last_update = env.block.time.nanos();
        price.token1_by_token2 = result.price.to_string();
        Ok(price)
    })?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "receive_spot_price")
        .add_attribute("amount", result.price.to_string()))
}

fn ack_estimate_swap_result(
    deps: DepsMut,
    env: Env,
    _caller: String,
    ack: PacketAck,
) -> StdResult<IbcBasicResponse> {
    let result: EstimateSwapAck = match ack {
        PacketAck::Result(data) => from_binary(&data)?,
        PacketAck::Error(e) => {
            return Ok(IbcBasicResponse::new()
                .add_attribute("action", "receive_estimate_swap")
                .add_attribute("error", e))
        }
    };
    PRICE.update(deps.storage, |mut price| -> StdResult<_> {
        price.last_update = env.block.time.nanos();
        price.token1_by_token2 = result.amount.to_string();
        Ok(price)
    })?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "receive_estimate_swap")
        .add_attribute("amount", result.amount))
}


#[entry_point]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_packet_timeout"))
}
