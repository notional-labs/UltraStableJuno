
use cosmwasm_std::{Addr, Uint128, Decimal256, Timestamp, Decimal};
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
    // Role Manager function
    UpdateAdmin { 
        admin : Addr 
    }, 
    UpdateRole { 
        role_provider: Addr 
    },

    ///--- Trove Liquidation functions ---
    // Single liquidation function. Closes the trove if its ICR is lower than the minimum collateral ratio.
    Liquidate {
        borrower: String,
    },
    // // Liquidate a sequence of troves. Closes a maximum number of n under-collateralized Troves,
    // // starting from the one with the lowest collateral ratio in the system, and moving upwards
    // LiquidateTroves {
    //     n: Uint128,
    // },
    // // Attempt to liquidate a custom list of troves provided by the caller.
    // BatchLiquidateTroves {},
    // Send ultra_amount $ULTRA to the system and redeem the corresponding amount of collateral from as many Troves
    // as are needed to fill the redemption request.
    RedeemCollateral {
        ultra_amount: Uint128,
        first_redemption_hint: Option<String>,
        upper_partial_redemption_hint: String,
        lower_partial_redemption_hint: String,
        max_iterations: Uint128,
        max_fee_percentage: Decimal256,
    },
    // Add the borrowers's coll and debt rewards earned from redistributions, to their Trove
    ApplyPendingRewards {
        borrower: String,
    },
    // Update borrower's snapshots of L_JUNO and L_ULTRADebt to reflect the current values
    UpdateTroveRewardSnapshots {
        borrower: String,
    },
    // Remove borrower's stake from the totalStakes sum, and set their stake to 0
    RemoveStake {
        borrower: String,
    },
    // Update borrower's stake based on their latest collateral value
    UpdateStakeAndTotalStakes {
        borrower: String,
    },
    // Close a Trove
    CloseTrove {
        borrower: String,
    },
    // Push the owner's address to the Trove owners list, and record the corresponding array index on the Trove struct
    AddTroveOwnerToArray {
        borrower: String,
    },

    /// --- Borrowing fee functions ---
    DecayBaseRateFromBorrowing {},

    /// --- Trove property setters, called by BorrowerOperations ---
    SetTroveStatus {
        borrower: String,
        status: Status,
    },
    IncreaseTroveColl {
        borrower: String,
        coll_increase: Uint128,
    },
    DecreaseTroveColl {
        borrower: String,
        coll_decrease: Uint128,
    },
    IncreaseTroveDebt {
        borrower: String,
        debt_increase: Uint128,
    },
    DecreaseTroveDebt {
        borrower: String,
        debt_decrease: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetParams {},
    GetTroveFromTroveOwnersArray { index: Uint128 },
    GetTroveOwnersCount {},
    GetNominalICR { borrower: String },
    GetCurrentICR { borrower: String, price: Uint128 },
    GetPendingJUNOReward {},
    GetPendingULTRADebtReward {},
    GetEntireDebtAndColl { borrower: String },
    GetTCR {},
    GetBorrowingFee { ultra_debt: Uint128 },
    GetBorrowingFeeWithDecay { ultra_debt: Uint128 },
    GetBorrowingRate {},
    GetBorrowingRateWithDecay {},
    GetRedemptionRate {},
    GetRedemptionRateWithDecay {},
    GetRedemptionFeeWithDecay { juno_drawn: Uint128 },
    GetTroveStatus {},
    GetTroveStake {},
    GetTroveDebt {},
    GetTroveColl {},
    GetBorrowerOperationsAddress {},
    GetTroveManagerAddress {},
    GetActivePoolAddress {},
    GetULTRATokenAddress {},
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

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum Status{
    NonExistent,
    Active,
    Closed
}


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Manager {
    pub trove_owner_count: Uint128,
    pub base_rate : Decimal,
    pub last_fee_operation_time : Timestamp,
    pub total_stake_snapshot: Uint128,
    pub total_collateral_snapshot: Uint128,
    pub total_stake: Uint128,
    pub total_liquidation_juno: Uint128,
    pub total_liquidation_ultra_debt: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct RewardSnapshot {
    pub juno: Uint128,
    pub ultra_debt: Uint128
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Trove {
    pub coll: Uint128,
    pub debt: Uint128,
    pub stake: Uint128,
    pub status: Status,
    pub owner: Addr,
    pub index: Uint128
}

// --- Variable container structs for redemptions ---
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct RedemptionTotals {
    pub remaining_ultra_debt: Uint128,
    pub total_ultra_debt_to_redeem: Uint128,
    pub total_juno_drawn: Uint128,
    pub juno_fee: Uint128,
    pub juno_to_send_to_redeemer: Uint128,
    pub decayed_base_rate: Uint128,
    pub price: Decimal,
    pub total_ultra_debt_supply_at_start: Uint128,
}

impl Default for RedemptionTotals{
    fn default() -> Self{
        Self {
            remaining_ultra_debt: Uint128::zero(),
            total_ultra_debt_to_redeem: Uint128::zero(),
            total_juno_drawn: Uint128::zero(),
            juno_fee: Uint128::zero(),
            juno_to_send_to_redeemer: Uint128::zero(),
            decayed_base_rate: Uint128::zero(),
            price: Decimal::zero(),
            total_ultra_debt_supply_at_start: Uint128::zero(),
        }
    }
}
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct SingleRedemptionValues {
    pub ultra_debt_lot: Uint128,
    pub juno_lot: Uint128,
    pub cancelled_partial: bool
}