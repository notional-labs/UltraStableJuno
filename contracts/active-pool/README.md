# Active Pool
The Active Pool holds the JUNO collateral and ULTRA debt (but not ULTRA tokens) for all active troves.
When a trove is liquidated, it's JUNO and ULTRA debt are transferred from the Active Pool, to either the Stability Pool, the Default Pool, or both, depending on the liquidation conditions.