use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// These options are to be used in matching by the contract admin.  Requests not made by the admin
/// that include admin match options will be rejected.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdminMatchOptions {
    CoinTrade {
        /// Allows trades with an ask quote that does not match the bid quote to still be executed.
        /// Defaults to false if not specified.
        accept_mismatched_bids: Option<bool>,
    },
    MarkerTrade {
        /// Allows trades with an ask quote that does not match the bid quote to still be executed.
        /// Defaults to false if not specified.
        accept_mismatched_bids: Option<bool>,
    },
    MarkerShareSale {
        /// Allows trades to use either the ask quote or the bid quote to determine the funds allocated
        /// when an ask or bid is executed.  Will never try to use more than the bid quote amount
        /// due to the bid quote being the source of the available funds.
        override_quote_source: Option<OverrideQuoteSource>,
    },
    ScopeTrade {
        /// Allows trades with an ask quote that does not match the bid quote to still be executed.
        /// Defaults to false if not specified.
        accept_mismatched_bids: Option<bool>,
    },
}
#[cfg(test)]
impl AdminMatchOptions {
    pub fn coin_trade_empty() -> Self {
        Self::CoinTrade {
            accept_mismatched_bids: None,
        }
    }

    pub fn coin_trade_options(accept_mismatched_bids: bool) -> Self {
        Self::CoinTrade {
            accept_mismatched_bids: Some(accept_mismatched_bids),
        }
    }

    pub fn marker_trade_empty() -> Self {
        Self::MarkerTrade {
            accept_mismatched_bids: None,
        }
    }

    pub fn marker_trade_options(accept_mismatched_bids: bool) -> Self {
        Self::MarkerTrade {
            accept_mismatched_bids: Some(accept_mismatched_bids),
        }
    }

    pub fn marker_share_sale_empty() -> Self {
        Self::MarkerShareSale {
            override_quote_source: None,
        }
    }

    pub fn marker_share_sale_options(override_quote_source: OverrideQuoteSource) -> Self {
        Self::MarkerShareSale {
            override_quote_source: Some(override_quote_source),
        }
    }

    pub fn scope_trade_empty() -> Self {
        Self::ScopeTrade {
            accept_mismatched_bids: None,
        }
    }

    pub fn scope_trade_options(accept_mismatched_bids: bool) -> Self {
        Self::ScopeTrade {
            accept_mismatched_bids: Some(accept_mismatched_bids),
        }
    }
}

/// Determines the source of the quote to be sent to the asker after a match executes.  By standard,
/// the ask and bid should have matching quotes, but this allows them to not match and for a
/// mismatched ask or bid to drive the amount that is sent, instead.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OverrideQuoteSource {
    Ask,
    Bid,
}
