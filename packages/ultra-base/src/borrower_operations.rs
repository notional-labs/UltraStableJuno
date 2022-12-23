use cosmwasm_std::{Addr, Decimal256, Uint128};
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
    UpdateAdmin {
        admin: Addr,
    },
    UpdateRole {
        role_provider: Addr,
    },
    /// Send JUNO as collateral to a trove
    AddColl {
        upper_hint: String,
        lower_hint: String,
    },
    /// Alongside a debt change, this function can perform either a collateral top-up or a collateral withdrawal.
    AdjustTrove {
        borrower: String,
        coll_withdrawal: Uint128,
        ultra_change: Uint128,
        is_debt_increase: bool,
        max_fee_percentage: Decimal256,
        upper_hint: String,
        lower_hint: String,
    },
    /// Claim remaining collateral from a redemption or from a liquidation with ICR > MCR in Recovery Mode
    ClaimCollateral {},
    CloseTrove {},
    /// Send JUNO as collateral to a trove. Called by only the Stability Pool.
    MoveJUNOGainToTrove {
        borrower: String,
        upper_hint: String,
        lower_hint: String,
    },
    OpenTrove {
        max_fee_percentage: Decimal256,
        ultra_amount: Uint128,
        upper_hint: String,
        lower_hint: String,
    },
    RepayULTRA {
        ultra_amount: Uint128,
        upper_hint: String,
        lower_hint: String,
    },
    /// Withdraw JUNO collateral from a trove
    WithdrawColl {
        coll_amount: Uint128,
        upper_hint: String,
        lower_hint: String,
    },
    /// Withdraw ULTRA tokens from a trove: mint new ULTRA tokens to the owner, and increase the trove's debt accordingly
    WithdrawULTRA {
        max_fee_percentage: Decimal256,
        ultra_amount: Uint128,
        upper_hint: String,
        lower_hint: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetParams {},
    GetEntireSystemColl {
        active_pool_addr: Addr,
        default_pool_addr: Addr,
    },
    GetEntireSystemDebt {
        active_pool_addr: Addr,
        default_pool_addr: Addr,
    },
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
