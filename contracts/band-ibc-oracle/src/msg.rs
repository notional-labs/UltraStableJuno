use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::ChannelInfo;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct InitMsg {
    pub default_timeout: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    // ReceiveOracle(OracleReceiveMsg),
    ///This allow ibc request bandd
    OracleRequest(OracleMsg),
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OracleMsg {
    /// The local channel to send the packets on
    pub channel: String,
    /// How long the packet lives in seconds. If not specified, use default_timeout
    pub timeout: Option<u64>,
    // the unique identifier of this oracle request, as specified by the client. This same unique ID will be sent back to the requester with the oracle response.
    pub client_id: String,
    // The unique identifier number assigned to the oracle script when it was first registered on Bandchain
    pub oracle_script_id: i64,
    // The data passed over to the oracle script for the script to use during its execution
    pub call_data: String,
    // The number of validators that are requested to respond to this request
    pub ask_count: i64,
    // The minimum number of validators necessary for the request to proceed to the execution phase
    pub min_count: i64,

    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return the port ID bound by this contract. Returns PortResponse
    Port {},
    /// Show all channels we have connected to. Return type is ListChannelsResponse.
    ListChannels {},
    /// Returns the details of the name channel, error if not created.
    /// Return type: ChannelResponse.
    Channel { id: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListChannelsResponse {
    pub channels: Vec<ChannelInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ChannelResponse {
    /// Information on the channel's connection
    pub info: ChannelInfo,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PortResponse {
    pub port_id: String,
}
