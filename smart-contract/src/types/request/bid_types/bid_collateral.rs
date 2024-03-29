use crate::types::core::error::ContractError;
use crate::util::coin_utilities::divide_coins_by_amount;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{Addr, Coin, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BidCollateral {
    CoinTrade(CoinTradeBidCollateral),
    MarkerTrade(MarkerTradeBidCollateral),
    MarkerShareSale(MarkerShareSaleBidCollateral),
    ScopeTrade(ScopeTradeBidCollateral),
}
impl BidCollateral {
    pub fn coin_trade(base: &[Coin], quote: &[Coin]) -> Self {
        Self::CoinTrade(CoinTradeBidCollateral::new(base, quote))
    }

    pub fn marker_trade<S: Into<String>>(
        marker_address: Addr,
        marker_denom: S,
        quote: &[Coin],
        withdraw_shares_after_match: Option<bool>,
    ) -> Self {
        Self::MarkerTrade(MarkerTradeBidCollateral::new(
            marker_address,
            marker_denom,
            quote,
            withdraw_shares_after_match,
        ))
    }

    pub fn marker_share_sale<S: Into<String>>(
        marker_address: Addr,
        marker_denom: S,
        share_count: u128,
        quote: &[Coin],
    ) -> Self {
        Self::MarkerShareSale(MarkerShareSaleBidCollateral::new(
            marker_address,
            marker_denom,
            share_count,
            quote,
        ))
    }

    pub fn scope_trade<S: Into<String>>(scope_address: S, quote: &[Coin]) -> Self {
        Self::ScopeTrade(ScopeTradeBidCollateral::new(scope_address, quote))
    }

    pub fn get_coin_trade(&self) -> Result<&CoinTradeBidCollateral, ContractError> {
        match self {
            Self::CoinTrade(collateral) => collateral.to_ok(),
            _ => ContractError::InvalidType {
                explanation: "expected coin trade bid collateral".to_string(),
            }
            .to_err(),
        }
    }

    pub fn get_marker_trade(&self) -> Result<&MarkerTradeBidCollateral, ContractError> {
        match self {
            Self::MarkerTrade(collateral) => collateral.to_ok(),
            _ => ContractError::InvalidType {
                explanation: "expected marker trade bid collateral".to_string(),
            }
            .to_err(),
        }
    }

    pub fn get_marker_share_sale(&self) -> Result<&MarkerShareSaleBidCollateral, ContractError> {
        match self {
            Self::MarkerShareSale(collateral) => collateral.to_ok(),
            _ => ContractError::InvalidType {
                explanation: "expected marker share sale bid collateral".to_string(),
            }
            .to_err(),
        }
    }

    pub fn get_scope_trade(&self) -> Result<&ScopeTradeBidCollateral, ContractError> {
        match self {
            Self::ScopeTrade(collateral) => collateral.to_ok(),
            _ => ContractError::InvalidType {
                explanation: "expected scope trade bid collateral".to_string(),
            }
            .to_err(),
        }
    }

    pub fn get_quote(&self) -> Vec<Coin> {
        match self {
            BidCollateral::CoinTrade(c) => c.quote.to_owned(),
            BidCollateral::MarkerTrade(c) => c.quote.to_owned(),
            BidCollateral::MarkerShareSale(c) => c.quote.to_owned(),
            BidCollateral::ScopeTrade(c) => c.quote.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CoinTradeBidCollateral {
    pub base: Vec<Coin>,
    pub quote: Vec<Coin>,
}
impl CoinTradeBidCollateral {
    pub fn new(base: &[Coin], quote: &[Coin]) -> Self {
        Self {
            base: base.to_owned(),
            quote: quote.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MarkerTradeBidCollateral {
    pub marker_address: Addr,
    pub marker_denom: String,
    pub quote: Vec<Coin>,
    pub withdraw_shares_after_match: Option<bool>,
}
impl MarkerTradeBidCollateral {
    pub fn new<S: Into<String>>(
        marker_address: Addr,
        marker_denom: S,
        quote: &[Coin],
        withdraw_shares_after_match: Option<bool>,
    ) -> Self {
        Self {
            marker_address,
            marker_denom: marker_denom.into(),
            quote: quote.to_owned(),
            withdraw_shares_after_match,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MarkerShareSaleBidCollateral {
    pub marker_address: Addr,
    pub marker_denom: String,
    pub share_count: Uint128,
    pub quote: Vec<Coin>,
}
impl MarkerShareSaleBidCollateral {
    pub fn new<S: Into<String>>(
        marker_address: Addr,
        marker_denom: S,
        share_count: u128,
        quote: &[Coin],
    ) -> Self {
        Self {
            marker_address,
            marker_denom: marker_denom.into(),
            share_count: Uint128::new(share_count),
            quote: quote.to_owned(),
        }
    }

    pub fn get_quote_per_share(&self) -> Vec<Coin> {
        divide_coins_by_amount(&self.quote, self.share_count.u128())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ScopeTradeBidCollateral {
    pub scope_address: String,
    pub quote: Vec<Coin>,
}
impl ScopeTradeBidCollateral {
    pub fn new<S: Into<String>>(scope_address: S, quote: &[Coin]) -> Self {
        Self {
            scope_address: scope_address.into(),
            quote: quote.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::mock_marker::{DEFAULT_MARKER_ADDRESS, DEFAULT_MARKER_DENOM};
    use crate::types::request::bid_types::bid_collateral::MarkerShareSaleBidCollateral;
    use cosmwasm_std::{coin, Addr};

    #[test]
    fn test_marker_share_sale_bid_collateral_get_quote_per_share() {
        let collateral = MarkerShareSaleBidCollateral::new(
            Addr::unchecked(DEFAULT_MARKER_ADDRESS),
            DEFAULT_MARKER_DENOM,
            100,
            &[coin(1000, "quote1"), coin(1500, "quote2")],
        );
        assert_eq!(
            vec![coin(10, "quote1"), coin(15, "quote2")],
            collateral.get_quote_per_share(),
            "the quote per share should be calculated correctly by dividing each coin amount by the share count",
        );
    }
}
