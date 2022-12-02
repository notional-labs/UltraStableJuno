#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg, to_binary, Uint64};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::ibc::DEFAULT_PACKET_LIFETIME;
use crate::ibc_msg::{GammPacket, SpotPricePacket, PacketMsg, EstimateSwapPacket};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SpotPriceMsg, EstimateSwapMsg, GetPriceResponse};
use crate::state::{CONTRACT_INFO, ContractInfo, CHANNEL, PRICE};


// version info for migration info
const CONTRACT_NAME: &str = "omosis-query";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONTRACT_INFO.save(deps.storage, &ContractInfo{
        token_1: msg.token_1,
        token_2: msg.token_2,
        pool_id: msg.pool_id,
    })?;
    Ok(Response::new()
        .add_attribute("action","instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SpotPrice(msg) => exec_spot_price(deps, env, msg),
        ExecuteMsg::EstimateSwap(msg) => exec_estimate_swap(deps, env, msg),
    }
}


pub fn exec_spot_price(deps: DepsMut, env: Env, msg: SpotPriceMsg) -> Result<Response, ContractError> {
    if CHANNEL.load(deps.storage)? != Some(msg.channel.clone()) {
        return Err(ContractError::InvalidChannel {  })
    }

    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => DEFAULT_PACKET_LIFETIME,
    };
    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(timeout_delta);
    
    let packet;
    // construct a packet to send
    if msg.token_in == contract_info.token_1 {
        packet = PacketMsg {
            client_id: None,
            query: GammPacket::SpotPrice(SpotPricePacket {
                pool: Uint64::from(contract_info.pool_id),
                token_in: msg.token_in,
                token_out: contract_info.token_2,
            }),
        };
    } else if msg.token_in == contract_info.token_2 {
        packet = PacketMsg {
            client_id: None,
            query: GammPacket::SpotPrice(SpotPricePacket {
                pool: Uint64::from(contract_info.pool_id),
                token_in: msg.token_in,
                token_out: contract_info.token_1,
            }),
        };
    } else {
        return Err(ContractError::TokenInNotFound {  })
    }

    let msg = IbcMsg::SendPacket {
        channel_id: msg.channel,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "spot_price");
    Ok(res)
}

pub fn exec_estimate_swap(deps: DepsMut, env: Env, msg: EstimateSwapMsg) -> Result<Response, ContractError> {
    if CHANNEL.load(deps.storage)? != Some(msg.channel.clone()) {
        return Err(ContractError::InvalidChannel {  })
    }

    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    // delta from user is in seconds
    let timeout_delta = match msg.timeout {
        Some(t) => t,
        None => DEFAULT_PACKET_LIFETIME,
    };
    // timeout is in nanoseconds
    let timeout = env.block.time.plus_seconds(timeout_delta);

    let packet;
    // construct a packet to send
    if msg.token_in == contract_info.token_1 {
        packet = PacketMsg {
            client_id: None,
            query: GammPacket::EstimateSwap(EstimateSwapPacket {
                pool: Uint64::from(contract_info.pool_id),
                sender: msg.sender,
                token_in: msg.token_in,
                token_out: contract_info.token_2,
                amount: msg.amount
            }),
        };
    } else if msg.token_in == contract_info.token_2 {
        packet = PacketMsg {
            client_id: None,
            query: GammPacket::EstimateSwap(EstimateSwapPacket {
                pool: Uint64::from(contract_info.pool_id),
                sender: msg.sender,
                token_in: msg.token_in,
                token_out: contract_info.token_1,
                amount: msg.amount
            }),
        };
    } else {
        return Err(ContractError::TokenInNotFound {  })
    }

    let msg = IbcMsg::SendPacket {
        channel_id: msg.channel,
        data: to_binary(&packet)?,
        timeout: timeout.into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "spot_price");
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPrice {  } => to_binary(&query_price(deps)?),
    }
}
fn query_price(deps: Deps) -> StdResult<GetPriceResponse>{
    let price = PRICE.load(deps.storage)?;
    let contract_info = CONTRACT_INFO.load(deps.storage)?;

    Ok(GetPriceResponse {
        token_1: contract_info.token_1,
        token_2: contract_info.token_2,
        token1_by_token2: price.token1_by_token2,
        last_update: price.last_update
    })
}
#[cfg(test)]
mod tests {}
