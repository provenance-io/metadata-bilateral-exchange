use crate::types::core::error::ContractError;
use crate::types::request::share_sale_type::ShareSaleType;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{Addr, Coin, Uint128};
use provwasm_std::AccessGrant;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AskCollateral {
    CoinTrade(CoinTradeAskCollateral),
    MarkerTrade(MarkerTradeAskCollateral),
    MarkerShareSale(MarkerShareSaleAskCollateral),
    ScopeTrade(ScopeTradeAskCollateral),
}
impl AskCollateral {
    pub fn coin_trade(base: &[Coin], quote: &[Coin]) -> Self {
        Self::CoinTrade(CoinTradeAskCollateral::new(base, quote))
    }

    pub fn marker_trade<S: Into<String>>(
        marker_address: Addr,
        marker_denom: S,
        share_count: u128,
        quote_per_share: &[Coin],
        removed_permissions: &[AccessGrant],
    ) -> Self {
        Self::MarkerTrade(MarkerTradeAskCollateral::new(
            marker_address,
            marker_denom,
            share_count,
            quote_per_share,
            removed_permissions,
        ))
    }

    pub fn marker_share_sale<S: Into<String>>(
        marker_address: Addr,
        marker_denom: S,
        total_shares_in_sale: u128,
        remaining_shares_in_sale: u128,
        quote_per_share: &[Coin],
        removed_permissions: &[AccessGrant],
        sale_type: ShareSaleType,
    ) -> Self {
        Self::MarkerShareSale(MarkerShareSaleAskCollateral::new(
            marker_address,
            marker_denom,
            total_shares_in_sale,
            remaining_shares_in_sale,
            quote_per_share,
            removed_permissions,
            sale_type,
        ))
    }

    pub fn scope_trade<S: Into<String>>(scope_address: S, quote: &[Coin]) -> Self {
        Self::ScopeTrade(ScopeTradeAskCollateral::new(scope_address, quote))
    }

    pub fn get_coin_trade(&self) -> Result<&CoinTradeAskCollateral, ContractError> {
        match self {
            AskCollateral::CoinTrade(collateral) => collateral.to_ok(),
            _ => ContractError::InvalidType {
                explanation: "expected coin trade ask collateral".to_string(),
            }
            .to_err(),
        }
    }

    pub fn get_marker_trade(&self) -> Result<&MarkerTradeAskCollateral, ContractError> {
        match self {
            AskCollateral::MarkerTrade(collateral) => collateral.to_ok(),
            _ => ContractError::InvalidType {
                explanation: "expected marker trade ask collateral".to_string(),
            }
            .to_err(),
        }
    }

    pub fn get_marker_share_sale(&self) -> Result<&MarkerShareSaleAskCollateral, ContractError> {
        match self {
            AskCollateral::MarkerShareSale(collateral) => collateral.to_ok(),
            _ => ContractError::InvalidType {
                explanation: "expected marker share sale ask collateral".to_string(),
            }
            .to_err(),
        }
    }

    pub fn get_scope_trade(&self) -> Result<&ScopeTradeAskCollateral, ContractError> {
        match self {
            AskCollateral::ScopeTrade(collateral) => collateral.to_ok(),
            _ => ContractError::InvalidType {
                explanation: "expected scope trade ask collateral".to_string(),
            }
            .to_err(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinTradeAskCollateral {
    pub base: Vec<Coin>,
    pub quote: Vec<Coin>,
}
impl CoinTradeAskCollateral {
    fn new(base: &[Coin], quote: &[Coin]) -> Self {
        Self {
            base: base.to_owned(),
            quote: quote.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MarkerTradeAskCollateral {
    pub marker_address: Addr,
    pub marker_denom: String,
    pub share_count: Uint128,
    pub quote_per_share: Vec<Coin>,
    pub removed_permissions: Vec<AccessGrant>,
}
impl MarkerTradeAskCollateral {
    fn new<S: Into<String>>(
        marker_address: Addr,
        marker_denom: S,
        share_count: u128,
        quote_per_share: &[Coin],
        removed_permissions: &[AccessGrant],
    ) -> Self {
        Self {
            marker_address,
            marker_denom: marker_denom.into(),
            share_count: Uint128::new(share_count),
            quote_per_share: quote_per_share.to_owned(),
            removed_permissions: removed_permissions.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MarkerShareSaleAskCollateral {
    pub marker_address: Addr,
    pub marker_denom: String,
    pub total_shares_in_sale: Uint128,
    pub remaining_shares_in_sale: Uint128,
    pub quote_per_share: Vec<Coin>,
    pub removed_permissions: Vec<AccessGrant>,
    pub sale_type: ShareSaleType,
}
impl MarkerShareSaleAskCollateral {
    fn new<S: Into<String>>(
        marker_address: Addr,
        marker_denom: S,
        total_shares_in_sale: u128,
        remaining_shares_in_sale: u128,
        quote_per_share: &[Coin],
        removed_permissions: &[AccessGrant],
        sale_type: ShareSaleType,
    ) -> Self {
        Self {
            marker_address,
            marker_denom: marker_denom.into(),
            total_shares_in_sale: Uint128::new(total_shares_in_sale),
            remaining_shares_in_sale: Uint128::new(remaining_shares_in_sale),
            quote_per_share: quote_per_share.to_owned(),
            removed_permissions: removed_permissions.to_owned(),
            sale_type,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ScopeTradeAskCollateral {
    pub scope_address: String,
    pub quote: Vec<Coin>,
}
impl ScopeTradeAskCollateral {
    fn new<S: Into<String>>(scope_address: S, quote: &[Coin]) -> Self {
        Self {
            scope_address: scope_address.into(),
            quote: quote.to_owned(),
        }
    }
}
