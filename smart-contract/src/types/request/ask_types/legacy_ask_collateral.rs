use crate::types::request::legacy_share_sale_type::LegacyShareSaleType;
use cosmwasm_std::{Addr, Coin, Uint128};
use provwasm_std::AccessGrant;
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LegacyAskCollateral {
    CoinTrade(LegacyCoinTradeAskCollateral),
    MarkerTrade(LegacyMarkerTradeAskCollateral),
    MarkerShareSale(LegacyMarkerShareSaleAskCollateral),
    ScopeTrade(LegacyScopeTradeAskCollateral),
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyCoinTradeAskCollateral {
    pub base: Vec<Coin>,
    pub quote: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyMarkerTradeAskCollateral {
    pub address: Addr,
    pub denom: String,
    pub share_count: Uint128,
    pub quote_per_share: Vec<Coin>,
    pub removed_permissions: Vec<AccessGrant>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyMarkerShareSaleAskCollateral {
    pub address: Addr,
    pub denom: String,
    pub remaining_shares: Uint128,
    pub quote_per_share: Vec<Coin>,
    pub removed_permissions: Vec<AccessGrant>,
    pub sale_type: LegacyShareSaleType,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyScopeTradeAskCollateral {
    pub scope_address: String,
    pub quote: Vec<Coin>,
}
