use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub name: String,
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ProvideToSP {},
    WithdrawFromSP {},
    WithdrawJUNOGainToTrove {},
    RegisterFrontEnd {},
    Offset {},
    SetAddresses {
        borrower_operations_address: String,
        trove_manager_address: String,
        active_pool_address: String,
        ultra_token_address: String,
        sorted_troves_address: String,
        price_feed_address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetParams {},
    GetCurrentEpoch {},
    GetCurrentScale {},
    GetDeposits { input: String },
    GetDepositSnapshot { input: String },
    GetFrontEnds { input: String },
    GetFrontEndStakes { input: String },
    GetFrontEndSnapshots { input: String },
    GetFrontEndRewardGain { frontend: String },
    GetDepositorJUNOGain { depositor: String },
    GetDepositorRewardGain { depositor: String },
    GetLastJUNOErrorOffset {},
    GetLastRewardError {},
    GetLastUltraLossErrorOffset {},
    GetJUNO {},
    GetTotalUltraDeposits {},
    GetCompoundedFrontEndStake {},
    GetCompoundedUltraDeposit {},
    GetBorrowerOperationsAddress {},
    GetTroveManagerAddress {},
    GetActivePoolAddress {},
    GetUltraTokenAddress {},
    GetSortedTrovesAddress {},
    GetPriceFeedAddress {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SudoMsg {
    /// Update the contract parameters
    /// Can only be called by governance
    UpdateParams {
        name: Option<String>,
        owner: Option<Addr>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ParamsResponse {
    pub name: String,
    pub owner: Addr,
}
