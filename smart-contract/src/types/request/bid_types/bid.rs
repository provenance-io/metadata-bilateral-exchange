use cosmwasm_std::{Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Bid {
    CoinTrade(CoinTradeBid),
    MarkerTrade(MarkerTradeBid),
    MarkerShareSale(MarkerShareSaleBid),
    ScopeTrade(ScopeTradeBid),
}
impl Bid {
    pub fn new_coin_trade<S: Into<String>>(id: S, base: &[Coin]) -> Self {
        Self::CoinTrade(CoinTradeBid::new(id, base))
    }

    pub fn new_marker_trade<S1: Into<String>, S2: Into<String>>(
        id: S1,
        marker_denom: S2,
        withdraw_shares_after_match: Option<bool>,
    ) -> Self {
        Self::MarkerTrade(MarkerTradeBid::new(
            id,
            marker_denom,
            withdraw_shares_after_match,
        ))
    }

    pub fn new_marker_share_sale<S1: Into<String>, S2: Into<String>>(
        id: S1,
        marker_denom: S2,
        share_count: u128,
    ) -> Self {
        Self::MarkerShareSale(MarkerShareSaleBid::new(id, marker_denom, share_count))
    }

    pub fn new_scope_trade<S1: Into<String>, S2: Into<String>>(id: S1, scope_address: S2) -> Self {
        Self::ScopeTrade(ScopeTradeBid::new(id, scope_address))
    }

    pub fn get_id(&self) -> &str {
        match self {
            Self::CoinTrade(trade) => &trade.id,
            Self::MarkerTrade(trade) => &trade.id,
            Self::MarkerShareSale(sale) => &sale.id,
            Self::ScopeTrade(trade) => &trade.id,
        }
    }

    pub fn get_storage_key(&self) -> &[u8] {
        self.get_id().as_bytes()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinTradeBid {
    pub id: String,
    pub base: Vec<Coin>,
}
impl CoinTradeBid {
    pub fn new<S: Into<String>>(id: S, base: &[Coin]) -> Self {
        Self {
            id: id.into(),
            base: base.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MarkerTradeBid {
    pub id: String,
    pub marker_denom: String,
    pub withdraw_shares_after_match: Option<bool>,
}
impl MarkerTradeBid {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        id: S1,
        marker_denom: S2,
        withdraw_shares_after_match: Option<bool>,
    ) -> Self {
        Self {
            id: id.into(),
            marker_denom: marker_denom.into(),
            withdraw_shares_after_match,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MarkerShareSaleBid {
    pub id: String,
    pub marker_denom: String,
    pub share_count: Uint128,
}
impl MarkerShareSaleBid {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        id: S1,
        marker_denom: S2,
        share_count: u128,
    ) -> Self {
        Self {
            id: id.into(),
            marker_denom: marker_denom.into(),
            share_count: Uint128::new(share_count),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ScopeTradeBid {
    pub id: String,
    pub scope_address: String,
}
impl ScopeTradeBid {
    pub fn new<S1: Into<String>, S2: Into<String>>(id: S1, scope_address: S2) -> Self {
        Self {
            id: id.into(),
            scope_address: scope_address.into(),
        }
    }
}
