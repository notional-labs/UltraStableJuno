use cosmwasm_std::Decimal;
use cw_storage_plus::Map;



pub const RATE: Map<String, Decimal> = Map::new("rate");