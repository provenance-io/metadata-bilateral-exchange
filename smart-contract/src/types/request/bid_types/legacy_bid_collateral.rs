use cosmwasm_std::{Addr, Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LegacyBidCollateral {
    CoinTrade(LegacyCoinTradeBidCollateral),
    MarkerTrade(LegacyMarkerTradeBidCollateral),
    MarkerShareSale(LegacyMarkerShareSaleBidCollateral),
    ScopeTrade(LegacyScopeTradeBidCollateral),
}
impl LegacyBidCollateral {
    pub fn coin_trade(base: &[Coin], quote: &[Coin]) -> Self {
        Self::CoinTrade(LegacyCoinTradeBidCollateral::new(base, quote))
    }

    pub fn marker_trade<S: Into<String>>(address: Addr, denom: S, quote: &[Coin]) -> Self {
        Self::MarkerTrade(LegacyMarkerTradeBidCollateral::new(address, denom, quote))
    }

    pub fn marker_share_sale<S: Into<String>>(
        address: Addr,
        denom: S,
        share_count: u128,
        quote: &[Coin],
    ) -> Self {
        Self::MarkerShareSale(LegacyMarkerShareSaleBidCollateral::new(
            address,
            denom,
            share_count,
            quote,
        ))
    }

    pub fn scope_trade<S: Into<String>>(scope_address: S, quote: &[Coin]) -> Self {
        Self::ScopeTrade(LegacyScopeTradeBidCollateral::new(scope_address, quote))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyCoinTradeBidCollateral {
    pub base: Vec<Coin>,
    pub quote: Vec<Coin>,
}
impl LegacyCoinTradeBidCollateral {
    pub fn new(base: &[Coin], quote: &[Coin]) -> Self {
        Self {
            base: base.to_owned(),
            quote: quote.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyMarkerTradeBidCollateral {
    pub address: Addr,
    pub denom: String,
    pub quote: Vec<Coin>,
}
impl LegacyMarkerTradeBidCollateral {
    pub fn new<S: Into<String>>(address: Addr, denom: S, quote: &[Coin]) -> Self {
        Self {
            address,
            denom: denom.into(),
            quote: quote.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyMarkerShareSaleBidCollateral {
    pub address: Addr,
    pub denom: String,
    pub share_count: Uint128,
    pub quote: Vec<Coin>,
}
impl LegacyMarkerShareSaleBidCollateral {
    pub fn new<S: Into<String>>(
        address: Addr,
        denom: S,
        share_count: u128,
        quote: &[Coin],
    ) -> Self {
        Self {
            address,
            denom: denom.into(),
            share_count: Uint128::new(share_count),
            quote: quote.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyScopeTradeBidCollateral {
    pub scope_address: String,
    pub quote: Vec<Coin>,
}
impl LegacyScopeTradeBidCollateral {
    pub fn new<S: Into<String>>(scope_address: S, quote: &[Coin]) -> Self {
        Self {
            scope_address: scope_address.into(),
            quote: quote.to_owned(),
        }
    }
}
