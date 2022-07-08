use crate::types::request::ask_types::ask_collateral::{
    AskCollateral, CoinTradeAskCollateral, MarkerShareSaleAskCollateral, MarkerTradeAskCollateral,
    ScopeTradeAskCollateral,
};
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::types::request::ask_types::legacy_ask_collateral::LegacyAskCollateral;
use crate::types::request::legacy_share_sale_type::LegacyShareSaleType;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use crate::types::request::share_sale_type::ShareSaleType;
use cosmwasm_std::{Addr, Uint128};
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LegacyAskOrder {
    pub id: String,
    pub ask_type: RequestType,
    pub owner: Addr,
    pub collateral: LegacyAskCollateral,
    pub descriptor: Option<RequestDescriptor>,
}
impl LegacyAskOrder {
    pub fn get_pk(&self) -> &[u8] {
        self.id.as_bytes()
    }

    pub fn get_collateral_index(&self) -> String {
        match &self.collateral {
            // Coin trades have no metadata involved - just use self.id as a duplicate index
            LegacyAskCollateral::CoinTrade(_) => self.id.clone(),
            // Marker trades include a marker address - only one ask per marker should be created at a time
            LegacyAskCollateral::MarkerTrade(collateral) => collateral.address.to_string(),
            // Marker trades include a marker address - only one ask per marker should be created at a time
            LegacyAskCollateral::MarkerShareSale(collateral) => collateral.address.to_string(),
            // Scope trades include a scope address - only one ask per scope should be created at a time
            LegacyAskCollateral::ScopeTrade(collateral) => collateral.scope_address.to_owned(),
        }
    }

    pub fn to_new_ask_order(self) -> AskOrder {
        AskOrder {
            id: self.id,
            ask_type: self.ask_type,
            owner: self.owner,
            collateral: match self.collateral {
                LegacyAskCollateral::CoinTrade(collateral) => {
                    AskCollateral::CoinTrade(CoinTradeAskCollateral {
                        base: collateral.base,
                        quote: collateral.quote,
                    })
                }
                LegacyAskCollateral::MarkerTrade(collateral) => {
                    AskCollateral::MarkerTrade(MarkerTradeAskCollateral {
                        marker_address: collateral.address,
                        marker_denom: collateral.denom,
                        share_count: collateral.share_count,
                        quote_per_share: collateral.quote_per_share,
                        removed_permissions: collateral.removed_permissions,
                    })
                }
                LegacyAskCollateral::MarkerShareSale(collateral) => {
                    let (share_count, sale_type) = match collateral.sale_type {
                        LegacyShareSaleType::SingleTransaction { share_count } => {
                            (share_count, ShareSaleType::SingleTransaction)
                        }
                        LegacyShareSaleType::MultipleTransactions {
                            remove_sale_share_threshold,
                        } => (
                            Uint128::new(
                                collateral.remaining_shares.u128()
                                    - remove_sale_share_threshold.map(|t| t.u128()).unwrap_or(0),
                            ),
                            ShareSaleType::MultipleTransactions,
                        ),
                    };
                    AskCollateral::MarkerShareSale(MarkerShareSaleAskCollateral {
                        marker_address: collateral.address,
                        marker_denom: collateral.denom,
                        total_shares_in_sale: share_count,
                        remaining_shares_in_sale: share_count,
                        quote_per_share: collateral.quote_per_share,
                        removed_permissions: collateral.removed_permissions,
                        sale_type,
                    })
                }
                LegacyAskCollateral::ScopeTrade(collateral) => {
                    AskCollateral::ScopeTrade(ScopeTradeAskCollateral {
                        scope_address: collateral.scope_address,
                        quote: collateral.quote,
                    })
                }
            },
            descriptor: self.descriptor,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::mock_scope::DEFAULT_SCOPE_ID;
    use crate::types::request::ask_types::ask_order::AskOrder;
    use crate::types::request::ask_types::legacy_ask_collateral::{
        LegacyAskCollateral, LegacyCoinTradeAskCollateral, LegacyMarkerShareSaleAskCollateral,
        LegacyMarkerTradeAskCollateral, LegacyScopeTradeAskCollateral,
    };
    use crate::types::request::ask_types::legacy_ask_order::LegacyAskOrder;
    use crate::types::request::legacy_share_sale_type::LegacyShareSaleType;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use crate::types::request::share_sale_type::ShareSaleType;
    use cosmwasm_std::{coins, Addr, Uint128};
    use provwasm_std::{AccessGrant, MarkerAccess};

    #[test]
    fn test_successful_conversion_for_coin_trade() {
        let legacy_coin_trade = LegacyAskOrder {
            id: "ask_id".to_string(),
            ask_type: RequestType::CoinTrade,
            owner: Addr::unchecked("asker"),
            collateral: LegacyAskCollateral::CoinTrade(LegacyCoinTradeAskCollateral {
                base: coins(100, "base"),
                quote: coins(100, "quote"),
            }),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::any(&["your", "face"]),
            )),
        };
        let new_coin_trade = legacy_coin_trade.clone().to_new_ask_order();
        assert_base_equality_in_ask_orders(&legacy_coin_trade, &new_coin_trade);
        let legacy_collateral = match legacy_coin_trade.collateral {
            LegacyAskCollateral::CoinTrade(collateral) => collateral,
            _ => panic!("unexpected legacy coin trade ask collateral"),
        };
        let new_collateral = new_coin_trade.collateral.unwrap_coin_trade();
        assert_eq!(legacy_collateral.quote, new_collateral.quote);
        assert_eq!(legacy_collateral.base, new_collateral.base);
    }

    #[test]
    fn test_successful_conversion_for_marker_trade() {
        let legacy_marker_trade = LegacyAskOrder {
            id: "ask_id".to_string(),
            ask_type: RequestType::MarkerTrade,
            owner: Addr::unchecked("asker"),
            collateral: LegacyAskCollateral::MarkerTrade(LegacyMarkerTradeAskCollateral {
                address: Addr::unchecked("markeraddress"),
                denom: "markerdenom".to_string(),
                share_count: Uint128::new(10),
                quote_per_share: coins(100, "quote"),
                removed_permissions: vec![AccessGrant {
                    permissions: vec![MarkerAccess::Admin],
                    address: Addr::unchecked("someperson"),
                }],
            }),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::any(&["your", "face"]),
            )),
        };
        let new_marker_trade = legacy_marker_trade.clone().to_new_ask_order();
        assert_base_equality_in_ask_orders(&legacy_marker_trade, &new_marker_trade);
        let legacy_collateral = match legacy_marker_trade.collateral {
            LegacyAskCollateral::MarkerTrade(collateral) => collateral,
            _ => panic!("unexpected legacy marker trade ask collateral"),
        };
        let new_collateral = new_marker_trade.collateral.unwrap_marker_trade();
        assert_eq!(legacy_collateral.address, new_collateral.marker_address);
        assert_eq!(legacy_collateral.denom, new_collateral.marker_denom);
        assert_eq!(legacy_collateral.share_count, new_collateral.share_count);
        assert_eq!(
            legacy_collateral.quote_per_share,
            new_collateral.quote_per_share
        );
        assert_eq!(
            legacy_collateral.removed_permissions,
            new_collateral.removed_permissions
        );
    }

    #[test]
    fn test_successful_conversion_for_marker_share_sale_single_tx() {
        let legacy_marker_share_sale = LegacyAskOrder {
            id: "ask_id".to_string(),
            ask_type: RequestType::MarkerShareSale,
            owner: Addr::unchecked("asker"),
            collateral: LegacyAskCollateral::MarkerShareSale(LegacyMarkerShareSaleAskCollateral {
                address: Addr::unchecked("markeraddress"),
                denom: "markerdenom".to_string(),
                remaining_shares: Uint128::new(10),
                quote_per_share: coins(100, "quote"),
                removed_permissions: vec![AccessGrant {
                    permissions: vec![MarkerAccess::Admin],
                    address: Addr::unchecked("someperson"),
                }],
                sale_type: LegacyShareSaleType::SingleTransaction {
                    share_count: Uint128::new(10),
                },
            }),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::any(&["your", "face"]),
            )),
        };
        let new_marker_share_sale = legacy_marker_share_sale.clone().to_new_ask_order();
        assert_base_equality_in_ask_orders(&legacy_marker_share_sale, &new_marker_share_sale);
        let legacy_collateral = match legacy_marker_share_sale.collateral {
            LegacyAskCollateral::MarkerShareSale(collateral) => collateral,
            _ => panic!("unexpected legacy marker share sale ask collateral"),
        };
        let new_collateral = new_marker_share_sale.collateral.unwrap_marker_share_sale();
        assert_eq!(legacy_collateral.address, new_collateral.marker_address);
        assert_eq!(legacy_collateral.denom, new_collateral.marker_denom);
        assert_eq!(10, new_collateral.total_shares_in_sale.u128());
        assert_eq!(10, new_collateral.remaining_shares_in_sale.u128());
        assert_eq!(
            legacy_collateral.quote_per_share,
            new_collateral.quote_per_share
        );
        assert_eq!(
            legacy_collateral.removed_permissions,
            new_collateral.removed_permissions
        );
        assert_eq!(ShareSaleType::SingleTransaction, new_collateral.sale_type);
    }

    #[test]
    fn test_successful_conversion_for_marker_share_sale_multi_tx_with_threshold() {
        let legacy_marker_share_sale = LegacyAskOrder {
            id: "ask_id".to_string(),
            ask_type: RequestType::MarkerShareSale,
            owner: Addr::unchecked("asker"),
            collateral: LegacyAskCollateral::MarkerShareSale(LegacyMarkerShareSaleAskCollateral {
                address: Addr::unchecked("markeraddress"),
                denom: "markerdenom".to_string(),
                remaining_shares: Uint128::new(10),
                quote_per_share: coins(100, "quote"),
                removed_permissions: vec![AccessGrant {
                    permissions: vec![MarkerAccess::Admin],
                    address: Addr::unchecked("someperson"),
                }],
                sale_type: LegacyShareSaleType::MultipleTransactions {
                    remove_sale_share_threshold: Some(Uint128::new(7)),
                },
            }),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::any(&["your", "face"]),
            )),
        };
        let new_marker_share_sale = legacy_marker_share_sale.clone().to_new_ask_order();
        assert_base_equality_in_ask_orders(&legacy_marker_share_sale, &new_marker_share_sale);
        let legacy_collateral = match legacy_marker_share_sale.collateral {
            LegacyAskCollateral::MarkerShareSale(collateral) => collateral,
            _ => panic!("unexpected legacy marker share sale ask collateral"),
        };
        let new_collateral = new_marker_share_sale.collateral.unwrap_marker_share_sale();
        assert_eq!(legacy_collateral.address, new_collateral.marker_address);
        assert_eq!(legacy_collateral.denom, new_collateral.marker_denom);
        assert_eq!(3, new_collateral.total_shares_in_sale.u128());
        assert_eq!(3, new_collateral.remaining_shares_in_sale.u128());
        assert_eq!(
            legacy_collateral.quote_per_share,
            new_collateral.quote_per_share
        );
        assert_eq!(
            legacy_collateral.removed_permissions,
            new_collateral.removed_permissions
        );
        assert_eq!(
            ShareSaleType::MultipleTransactions,
            new_collateral.sale_type
        );
    }

    #[test]
    fn test_successful_conversion_for_marker_share_sale_multi_tx_without_threshold() {
        let legacy_marker_share_sale = LegacyAskOrder {
            id: "ask_id".to_string(),
            ask_type: RequestType::MarkerShareSale,
            owner: Addr::unchecked("asker"),
            collateral: LegacyAskCollateral::MarkerShareSale(LegacyMarkerShareSaleAskCollateral {
                address: Addr::unchecked("markeraddress"),
                denom: "markerdenom".to_string(),
                remaining_shares: Uint128::new(15),
                quote_per_share: coins(100, "quote"),
                removed_permissions: vec![AccessGrant {
                    permissions: vec![MarkerAccess::Admin],
                    address: Addr::unchecked("someperson"),
                }],
                sale_type: LegacyShareSaleType::MultipleTransactions {
                    remove_sale_share_threshold: None,
                },
            }),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::any(&["your", "face"]),
            )),
        };
        let new_marker_share_sale = legacy_marker_share_sale.clone().to_new_ask_order();
        assert_base_equality_in_ask_orders(&legacy_marker_share_sale, &new_marker_share_sale);
        let legacy_collateral = match legacy_marker_share_sale.collateral {
            LegacyAskCollateral::MarkerShareSale(collateral) => collateral,
            _ => panic!("unexpected legacy marker share sale ask collateral"),
        };
        let new_collateral = new_marker_share_sale.collateral.unwrap_marker_share_sale();
        assert_eq!(legacy_collateral.address, new_collateral.marker_address);
        assert_eq!(legacy_collateral.denom, new_collateral.marker_denom);
        assert_eq!(15, new_collateral.total_shares_in_sale.u128());
        assert_eq!(15, new_collateral.remaining_shares_in_sale.u128());
        assert_eq!(
            legacy_collateral.quote_per_share,
            new_collateral.quote_per_share
        );
        assert_eq!(
            legacy_collateral.removed_permissions,
            new_collateral.removed_permissions
        );
        assert_eq!(
            ShareSaleType::MultipleTransactions,
            new_collateral.sale_type
        );
    }

    #[test]
    fn test_successful_conversion_for_scope_trade() {
        let legacy_scope_trade = LegacyAskOrder {
            id: "ask_id".to_string(),
            ask_type: RequestType::ScopeTrade,
            owner: Addr::unchecked("asker"),
            collateral: LegacyAskCollateral::ScopeTrade(LegacyScopeTradeAskCollateral {
                scope_address: DEFAULT_SCOPE_ID.to_string(),
                quote: coins(100, "quote"),
            }),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::any(&["your", "face"]),
            )),
        };
        let new_scope_trade = legacy_scope_trade.clone().to_new_ask_order();
        assert_base_equality_in_ask_orders(&legacy_scope_trade, &new_scope_trade);
        let legacy_collateral = match legacy_scope_trade.collateral {
            LegacyAskCollateral::ScopeTrade(collateral) => collateral,
            _ => panic!("unexpected legacy scope trade ask collateral"),
        };
        let new_collateral = new_scope_trade.collateral.unwrap_scope_trade();
        assert_eq!(
            legacy_collateral.scope_address,
            new_collateral.scope_address
        );
        assert_eq!(legacy_collateral.quote, new_collateral.quote);
    }

    fn assert_base_equality_in_ask_orders(legacy_order: &LegacyAskOrder, new_order: &AskOrder) {
        assert_eq!(legacy_order.id, new_order.id);
        assert_eq!(legacy_order.ask_type, new_order.ask_type);
        assert_eq!(legacy_order.owner, new_order.owner);
        assert_eq!(legacy_order.descriptor, new_order.descriptor);
    }
}
