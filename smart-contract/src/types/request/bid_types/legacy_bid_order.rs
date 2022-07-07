use crate::types::request::bid_types::bid_collateral::{
    BidCollateral, CoinTradeBidCollateral, MarkerShareSaleBidCollateral, MarkerTradeBidCollateral,
    ScopeTradeBidCollateral,
};
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::bid_types::legacy_bid_collateral::LegacyBidCollateral;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyBidOrder {
    pub id: String,
    pub bid_type: RequestType,
    pub owner: Addr,
    pub collateral: LegacyBidCollateral,
    pub descriptor: Option<RequestDescriptor>,
}
impl LegacyBidOrder {
    pub fn get_pk(&self) -> &[u8] {
        self.id.as_bytes()
    }

    pub fn to_new_bid_order(self) -> BidOrder {
        BidOrder {
            id: self.id,
            bid_type: self.bid_type,
            owner: self.owner,
            collateral: match self.collateral {
                LegacyBidCollateral::CoinTrade(collateral) => {
                    BidCollateral::CoinTrade(CoinTradeBidCollateral {
                        base: collateral.base,
                        quote: collateral.quote,
                    })
                }
                LegacyBidCollateral::MarkerTrade(collateral) => {
                    BidCollateral::MarkerTrade(MarkerTradeBidCollateral {
                        marker_address: collateral.address,
                        marker_denom: collateral.denom,
                        quote: collateral.quote,
                    })
                }
                LegacyBidCollateral::MarkerShareSale(collateral) => {
                    BidCollateral::MarkerShareSale(MarkerShareSaleBidCollateral {
                        marker_address: collateral.address,
                        marker_denom: collateral.denom,
                        share_count: collateral.share_count,
                        quote: collateral.quote,
                    })
                }
                LegacyBidCollateral::ScopeTrade(collateral) => {
                    BidCollateral::ScopeTrade(ScopeTradeBidCollateral {
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
    use crate::types::request::bid_types::bid_order::BidOrder;
    use crate::types::request::bid_types::legacy_bid_collateral::LegacyBidCollateral;
    use crate::types::request::bid_types::legacy_bid_order::LegacyBidOrder;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use cosmwasm_std::{coins, Addr};

    #[test]
    fn test_successful_conversion_for_coin_trade() {
        let legacy_coin_trade = LegacyBidOrder {
            id: "bid_id".to_string(),
            bid_type: RequestType::CoinTrade,
            owner: Addr::unchecked("bidder"),
            collateral: LegacyBidCollateral::coin_trade(&coins(100, "base"), &coins(100, "quote")),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all(&["some", "stuff"]),
            )),
        };
        let new_coin_trade = legacy_coin_trade.clone().to_new_bid_order();
        assert_base_equality_in_bid_orders(&legacy_coin_trade, &new_coin_trade);
        let legacy_collateral = match legacy_coin_trade.collateral {
            LegacyBidCollateral::CoinTrade(collateral) => collateral,
            _ => panic!("unexpected legacy coin trade bid collateral"),
        };
        let new_collateral = new_coin_trade.collateral.unwrap_coin_trade();
        assert_eq!(legacy_collateral.base, new_collateral.base);
        assert_eq!(legacy_collateral.quote, new_collateral.quote);
    }

    #[test]
    fn test_successful_conversion_for_marker_trade() {
        let legacy_marker_trade = LegacyBidOrder {
            id: "bid_id".to_string(),
            bid_type: RequestType::MarkerTrade,
            owner: Addr::unchecked("bidder"),
            collateral: LegacyBidCollateral::marker_trade(
                Addr::unchecked("marker_address"),
                "markerdenom",
                &coins(100, "quote"),
            ),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all(&["some", "stuff"]),
            )),
        };
        let new_marker_trade = legacy_marker_trade.clone().to_new_bid_order();
        assert_base_equality_in_bid_orders(&legacy_marker_trade, &new_marker_trade);
        let legacy_collateral = match legacy_marker_trade.collateral {
            LegacyBidCollateral::MarkerTrade(collateral) => collateral,
            _ => panic!("unexpected legacy marker trade bid collateral"),
        };
        let new_collateral = new_marker_trade.collateral.unwrap_marker_trade();
        assert_eq!(legacy_collateral.address, new_collateral.marker_address);
        assert_eq!(legacy_collateral.denom, new_collateral.marker_denom);
        assert_eq!(legacy_collateral.quote, new_collateral.quote);
    }

    #[test]
    fn test_successful_conversion_for_marker_share_sale() {
        let legacy_marker_share_sale = LegacyBidOrder {
            id: "bid_id".to_string(),
            bid_type: RequestType::MarkerShareSale,
            owner: Addr::unchecked("bidder"),
            collateral: LegacyBidCollateral::marker_share_sale(
                Addr::unchecked("marker_address"),
                "markerdenom",
                100,
                &coins(100, "quote"),
            ),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all(&["some", "stuff"]),
            )),
        };
        let new_marker_share_sale = legacy_marker_share_sale.clone().to_new_bid_order();
        assert_base_equality_in_bid_orders(&legacy_marker_share_sale, &new_marker_share_sale);
        let legacy_collateral = match legacy_marker_share_sale.collateral {
            LegacyBidCollateral::MarkerShareSale(collateral) => collateral,
            _ => panic!("unexpected legacy marker share sale bid collateral"),
        };
        let new_collateral = new_marker_share_sale.collateral.unwrap_marker_share_sale();
        assert_eq!(legacy_collateral.address, new_collateral.marker_address);
        assert_eq!(legacy_collateral.denom, new_collateral.marker_denom);
        assert_eq!(legacy_collateral.share_count, new_collateral.share_count);
        assert_eq!(legacy_collateral.quote, new_collateral.quote);
    }

    #[test]
    fn test_successful_conversion_for_scope_trade() {
        let legacy_scope_trade = LegacyBidOrder {
            id: "bid_id".to_string(),
            bid_type: RequestType::ScopeTrade,
            owner: Addr::unchecked("bidder"),
            collateral: LegacyBidCollateral::scope_trade("scope address", &coins(100, "quote")),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all(&["some", "stuff"]),
            )),
        };
        let new_scope_trade = legacy_scope_trade.clone().to_new_bid_order();
        assert_base_equality_in_bid_orders(&legacy_scope_trade, &new_scope_trade);
        let legacy_collateral = match legacy_scope_trade.collateral {
            LegacyBidCollateral::ScopeTrade(collateral) => collateral,
            _ => panic!("unexpected legacy scope trade bid collateral"),
        };
        let new_collateral = new_scope_trade.collateral.unwrap_scope_trade();
        assert_eq!(
            legacy_collateral.scope_address,
            new_collateral.scope_address
        );
        assert_eq!(legacy_collateral.quote, new_collateral.quote);
    }

    fn assert_base_equality_in_bid_orders(legacy_order: &LegacyBidOrder, new_order: &BidOrder) {
        assert_eq!(legacy_order.id, new_order.id);
        assert_eq!(legacy_order.bid_type, new_order.bid_type);
        assert_eq!(legacy_order.owner, new_order.owner);
        assert_eq!(legacy_order.descriptor, new_order.descriptor);
    }
}
