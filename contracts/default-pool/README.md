# Default Pool
The Default Pool holds the JUNO and USJ debt (but not USJ tokens) from liquidations that have been redistributed to active troves but not yet "applied", i.e. not yet recorded on a recipient active trove's struct.
When a trove makes an operation that applies its pending JUNO and USJ debt, its pending JUNO and USJ debt is moved from the Default Pool to the Active Pool.