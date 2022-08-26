use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    ActivePool,
    TroveManager,
    Owner,
    StabilityPool,
    BorrowerOperations,
    DefaultPool,
    CollateralSurplusPool,
    UltraToken,
    PriceFeed,
    SortedTroves,
    RewardPool,
}
