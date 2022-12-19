use std::marker::PhantomData;

use cosmwasm_std::{Addr, Uint128, Storage, StdResult, StdError, Order};
use cw_storage_plus::{UniqueIndex, MultiIndex, IndexList, Index, IndexedMap};
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
    // // Send ultra_amount $ULTRA to the system and redeem the corresponding amount of collateral from as many Troves
    // // as are needed to fill the redemption request.
    // RedeemCollateral {
    //     ultra_amount: Uint128,
    //     first_redemption_hint: String,
    //     upper_partial_redemption_hint: String,
    //     lower_partial_redemption_hint: String,
    //     max_iterations: Uint128,
    //     max_fee_percentage: Uint128,
    // },
    // // Add the borrowers's coll and debt rewards earned from redistributions, to their Trove
    // ApplyPendingRewards {
    //     borrower: String,
    // },
    // // Update borrower's snapshots of L_JUNO and L_ULTRADebt to reflect the current values
    // UpdateTroveRewardSnapshots {
    //     borrower: String,
    // },
    // // Remove borrower's stake from the totalStakes sum, and set their stake to 0
    // RemoveStake {
    //     borrower: String,
    // },
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
pub struct Trove {
    pub coll: Uint128,
    pub debt: Uint128,
    pub stake: Uint128,
    pub status: Status,
    pub owner: Addr,
    pub index: Uint128
}
