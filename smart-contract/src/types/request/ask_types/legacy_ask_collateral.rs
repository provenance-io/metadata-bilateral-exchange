use crate::types::core::error::ContractError;
use crate::types::request::share_sale_type::ShareSaleType;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{Addr, Coin, Uint128};
use provwasm_std::AccessGrant;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LegacyAskCollateral {
    CoinTrade(LegacyCoinTradeAskCollateral),
    MarkerTrade(LegacyMarkerTradeAskCollateral),
    MarkerShareSale(LegacyMarkerShareSaleAskCollateral),
    ScopeTrade(LegacyScopeTradeAskCollateral),
}
impl LegacyAskCollateral {
    pub fn coin_trade(base: &[Coin], quote: &[Coin]) -> Self {
        Self::CoinTrade(LegacyCoinTradeAskCollateral::new(base, quote))
    }

    pub fn marker_trade<S: Into<String>>(
        address: Addr,
        denom: S,
        share_count: u128,
        quote_per_share: &[Coin],
        removed_permissions: &[AccessGrant],
    ) -> Self {
        Self::MarkerTrade(LegacyMarkerTradeAskCollateral::new(
            address,
            denom,
            share_count,
            quote_per_share,
            removed_permissions,
        ))
    }

    pub fn marker_share_sale<S: Into<String>>(
        address: Addr,
        denom: S,
        remaining_shares: u128,
        quote_per_share: &[Coin],
        removed_permissions: &[AccessGrant],
        sale_type: ShareSaleType,
    ) -> Self {
        Self::MarkerShareSale(LegacyMarkerShareSaleAskCollateral::new(
            address,
            denom,
            remaining_shares,
            quote_per_share,
            removed_permissions,
            sale_type,
        ))
    }

    pub fn scope_trade<S: Into<String>>(scope_address: S, quote: &[Coin]) -> Self {
        Self::ScopeTrade(LegacyScopeTradeAskCollateral::new(scope_address, quote))
    }

    pub fn get_coin_trade(&self) -> Result<&LegacyCoinTradeAskCollateral, ContractError> {
        match self {
            LegacyAskCollateral::CoinTrade(collateral) => collateral.to_ok(),
            _ => ContractError::invalid_type("expected coin trade ask collateral").to_err(),
        }
    }

    pub fn get_marker_trade(&self) -> Result<&LegacyMarkerTradeAskCollateral, ContractError> {
        match self {
            LegacyAskCollateral::MarkerTrade(collateral) => collateral.to_ok(),
            _ => ContractError::invalid_type("expected marker trade ask collateral").to_err(),
        }
    }

    pub fn get_marker_share_sale(
        &self,
    ) -> Result<&LegacyMarkerShareSaleAskCollateral, ContractError> {
        match self {
            LegacyAskCollateral::MarkerShareSale(collateral) => collateral.to_ok(),
            _ => ContractError::invalid_type("expected marker share sale ask collateral").to_err(),
        }
    }

    pub fn get_scope_trade(&self) -> Result<&LegacyScopeTradeAskCollateral, ContractError> {
        match self {
            LegacyAskCollateral::ScopeTrade(collateral) => collateral.to_ok(),
            _ => ContractError::invalid_type("expected scope trade ask collateral").to_err(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyCoinTradeAskCollateral {
    pub base: Vec<Coin>,
    pub quote: Vec<Coin>,
}
impl LegacyCoinTradeAskCollateral {
    fn new(base: &[Coin], quote: &[Coin]) -> Self {
        Self {
            base: base.to_owned(),
            quote: quote.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyMarkerTradeAskCollateral {
    pub address: Addr,
    pub denom: String,
    pub share_count: Uint128,
    pub quote_per_share: Vec<Coin>,
    pub removed_permissions: Vec<AccessGrant>,
}
impl LegacyMarkerTradeAskCollateral {
    fn new<S: Into<String>>(
        address: Addr,
        denom: S,
        share_count: u128,
        quote_per_share: &[Coin],
        removed_permissions: &[AccessGrant],
    ) -> Self {
        Self {
            address,
            denom: denom.into(),
            share_count: Uint128::new(share_count),
            quote_per_share: quote_per_share.to_owned(),
            removed_permissions: removed_permissions.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyMarkerShareSaleAskCollateral {
    pub address: Addr,
    pub denom: String,
    pub remaining_shares: Uint128,
    pub quote_per_share: Vec<Coin>,
    pub removed_permissions: Vec<AccessGrant>,
    pub sale_type: ShareSaleType,
}
impl LegacyMarkerShareSaleAskCollateral {
    fn new<S: Into<String>>(
        address: Addr,
        denom: S,
        remaining_shares: u128,
        quote_per_share: &[Coin],
        removed_permissions: &[AccessGrant],
        sale_type: ShareSaleType,
    ) -> Self {
        Self {
            address,
            denom: denom.into(),
            remaining_shares: Uint128::new(remaining_shares),
            quote_per_share: quote_per_share.to_owned(),
            removed_permissions: removed_permissions.to_owned(),
            sale_type,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyScopeTradeAskCollateral {
    pub scope_address: String,
    pub quote: Vec<Coin>,
}
impl LegacyScopeTradeAskCollateral {
    fn new<S: Into<String>>(scope_address: S, quote: &[Coin]) -> Self {
        Self {
            scope_address: scope_address.into(),
            quote: quote.to_owned(),
        }
    }
}
