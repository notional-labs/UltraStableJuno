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
    /// Send JUNO as collateral to a trove
    AddColl {
        upper_hint: Addr,
        lower_hint: Addr,
    },
    /// Alongside a debt change, this function can perform either a collateral top-up or a collateral withdrawal.
    AdjustTrove {
        borrower: Addr,
        coll_withdrawal: Uint128,
        ultra_change: Uint128,
        is_debt_increase: bool,
        max_fee_percentage: Decimal256,
        upper_hint: Addr,
        lower_hint: Addr,
    },
    /// Claim remaining collateral from a redemption or from a liquidation with ICR > MCR in Recovery Mode
    ClaimCollateral {},
    CloseTrove {},
    /// Send JUNO as collateral to a trove. Called by only the Stability Pool.
    MoveJUNOGainToTrove {
        borrower: Addr,
        upper_hint: Addr,
        lower_hint: Addr,
    },
    OpenTrove {
        max_fee_percentage: Decimal256,
        ultra_amount: Uint128,
        upper_hint: Addr,
        lower_hint: Addr,
    },
    /// Burn the specified amount of ULTRA from `account` and decreases the total active debt
    RepayULTRA {
        active_pool_addr: Addr,
        ultra_token_addr: Addr,
        account: Addr,
        ultra_amount: Uint128,
        upper_hint: Addr,
        lower_hint: Addr,
    },
    /// Withdraw JUNO collateral from a trove
    WithdrawColl {
        coll_amount: Uint128,
        upper_hint: Addr,
        lower_hint: Addr,
    },
    /// Withdraw ULTRA tokens from a trove: mint new ULTRA tokens to the owner, and increase the trove's debt accordingly
    WithdrawULTRA {
        max_fee_percentage: Uint128,
        ultra_amount: Uint128,
        upper_hint: Addr,
        lower_hint: Addr,
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
