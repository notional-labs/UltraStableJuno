# Active Pool
The Active Pool holds the JUNO collateral and USJ debt (but not USJ tokens) for all active troves.
When a trove is liquidated, it's JUNO and USJ debt are transferred from the Active Pool, to either the Stability Pool, the Default Pool, or both, depending on the liquidation conditions.