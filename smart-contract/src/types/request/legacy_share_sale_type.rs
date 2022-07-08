use cosmwasm_std::Uint128;
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LegacyShareSaleType {
    /// Indicates that only a single transaction will be made after an ask of this share type is made.
    /// Ex: Asker indicates they want to sell  80 shares of their marker at a certain quote.  The
    /// bidder must buy exactly that many shares.
    SingleTransaction { share_count: Uint128 },
    /// Indicates that multiple transactions can be made after an ask of this share type is made.
    /// Optionally allows the sale to be withdrawn after a certain share count is met.  This
    /// ensures that shares can be purchased many times from the marker, but never more shares than
    /// would reduce the marker's share count below the specified threshold.  The ask is automatically
    /// deleted after the threshold is hit.  If the value is not specified, a default of zero will
    /// be used.
    /// Ex: Asker indicates they want to sell shares of their marker until there are only 10
    /// remaining.  Multiple bids can come in and incrementally buy shares from the marker.  Once
    /// the threshold of 10 remaining shares is hit, the ask will be automatically deleted.
    MultipleTransactions {
        remove_sale_share_threshold: Option<Uint128>,
    },
}
