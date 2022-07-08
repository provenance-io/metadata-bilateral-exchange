use cosmwasm_std::{Addr, Coin, Uint128};
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LegacyBidCollateral {
    CoinTrade(LegacyCoinTradeBidCollateral),
    MarkerTrade(LegacyMarkerTradeBidCollateral),
    MarkerShareSale(LegacyMarkerShareSaleBidCollateral),
    ScopeTrade(LegacyScopeTradeBidCollateral),
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyCoinTradeBidCollateral {
    pub base: Vec<Coin>,
    pub quote: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyMarkerTradeBidCollateral {
    pub address: Addr,
    pub denom: String,
    pub quote: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyMarkerShareSaleBidCollateral {
    pub address: Addr,
    pub denom: String,
    pub share_count: Uint128,
    pub quote: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyScopeTradeBidCollateral {
    pub scope_address: String,
    pub quote: Vec<Coin>,
}
