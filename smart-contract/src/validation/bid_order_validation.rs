use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid_collateral::BidCollateral;
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::request_type::RequestType;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::Coin;

pub fn validate_bid_order(bid_order: &BidOrder) -> Result<(), ContractError> {
    let mut invalid_field_messages: Vec<String> = vec![];
    if bid_order.id.is_empty() {
        invalid_field_messages.push("id for BidOrder must not be empty".to_string());
    }
    if bid_order.owner.as_str().is_empty() {
        invalid_field_messages.push("owner for BidOrder must not be empty".to_string());
    }
    if let Some(attribute_requirement) = bid_order
        .descriptor
        .as_ref()
        .and_then(|d| d.attribute_requirement.as_ref())
    {
        if attribute_requirement.attributes.is_empty() {
            invalid_field_messages.push(format!(
                "BidOrder [{}] specified RequiredAttributes, but the value included no attributes to check",
                bid_order.id,
            ));
        }
    }
    match bid_order.bid_type {
        RequestType::CoinTrade => {
            if !matches!(bid_order.collateral, BidCollateral::CoinTrade(_)) {
                invalid_field_messages.push(format!(
                    "bid type [{}] for BidOrder [{}] is invalid. type requires collateral type of BidCollateral::CoinTrade",
                    bid_order.bid_type.get_name(), bid_order.id,
                ));
            }
        }
        RequestType::MarkerTrade => {
            if !matches!(bid_order.collateral, BidCollateral::MarkerTrade(_)) {
                invalid_field_messages.push(format!(
                   "bid type [{}] for BidOrder [{}] is invalid. type requires collateral type of BidCollateral::MarkerTrade",
                   bid_order.bid_type.get_name(), bid_order.id,
               ));
            }
        }
        RequestType::MarkerShareSale => {
            if !matches!(bid_order.collateral, BidCollateral::MarkerShareSale(_)) {
                invalid_field_messages.push(format!(
                    "bid type [{}] for BidOrder [{}] is invalid. type requires collateral type of BidCollateral::MarkerShareSale",
                    bid_order.bid_type.get_name(), bid_order.id,
                ))
            }
        }
        RequestType::ScopeTrade => {
            if !matches!(bid_order.collateral, BidCollateral::ScopeTrade(_)) {
                invalid_field_messages.push(format!(
                    "bid type [{}] for BidOrder [{}] is invalid. type requires collateral type of BidCollateral::ScopeTrade",
                    bid_order.bid_type.get_name(), bid_order.id,
                ))
            }
        }
    };
    let validate_coin = |coin: &Coin, coin_type: &str| {
        let mut messages: Vec<String> = vec![];
        if coin.amount.u128() == 0 {
            messages.push(
                format!(
                    "Zero amounts not allowed on coins. Coin denom [{}] and type [{}] for BidOrder [{}]",
                    &coin.denom,
                    coin_type,
                    &bid_order.id,
                )
            );
        }
        if coin.denom.is_empty() {
            messages.push(
                format!(
                    "Blank denoms not allowed on coins. Coin amount [{}] and type [{}] for BidOrder [{}]",
                    coin.amount.u128(),
                    coin_type,
                    &bid_order.id,
                )
            );
        }
        messages
    };
    match &bid_order.collateral {
        BidCollateral::CoinTrade(collateral) => {
            let prefix = format!("BidOrder [{}] of type coin trade", bid_order.id);
            if collateral.base.is_empty() {
                invalid_field_messages.push(format!("{} must include base funds", prefix));
            }
            invalid_field_messages.append(
                &mut collateral
                    .base
                    .iter()
                    .flat_map(|coin| validate_coin(coin, "BidCollateral Base Coin"))
                    .collect(),
            );
            if collateral.quote.is_empty() {
                invalid_field_messages.push(format!("{} must include quote funds", prefix));
            }
            invalid_field_messages.append(
                &mut collateral
                    .quote
                    .iter()
                    .flat_map(|coin| validate_coin(coin, "BidCollateral Quote Coin"))
                    .collect(),
            );
        }
        BidCollateral::MarkerTrade(collateral) => {
            let prefix = format!("BidOrder [{}] of type marker trade", bid_order.id);
            if collateral.address.as_str().is_empty() {
                invalid_field_messages
                    .push(format!("{} must include a valid marker address", prefix,));
            }
            if collateral.denom.is_empty() {
                invalid_field_messages
                    .push(format!("{} must include a valid marker denom", prefix,));
            }
            if collateral.quote.is_empty() {
                invalid_field_messages
                    .push(format!("{} must include at least one quote coin", prefix));
            }
            invalid_field_messages.append(
                &mut collateral
                    .quote
                    .iter()
                    .flat_map(|coin| validate_coin(coin, "BidCollateral Quote Coin"))
                    .collect(),
            );
        }
        BidCollateral::MarkerShareSale(collateral) => {
            let prefix = format!("BidOrder [{}] of type marker share sale", bid_order.id);
            if collateral.address.as_str().is_empty() {
                invalid_field_messages
                    .push(format!("{} must include a valid marker address", prefix));
            }
            if collateral.denom.is_empty() {
                invalid_field_messages
                    .push(format!("{} must include a valid marker denom", prefix));
            }
            if collateral.share_count.is_zero() {
                invalid_field_messages.push(format!(
                    "{} must request to purchase at least one share",
                    prefix
                ));
            }
            if collateral.quote.is_empty() {
                invalid_field_messages
                    .push(format!("{} must include at least one quote coin", prefix));
            }
            invalid_field_messages.append(
                &mut collateral
                    .quote
                    .iter()
                    .flat_map(|coin| validate_coin(coin, "BidCollateral Quote Coin"))
                    .collect(),
            );
        }
        BidCollateral::ScopeTrade(collateral) => {
            let prefix = format!("BidOrder [{}] of type scope trade", bid_order.id);
            if collateral.scope_address.is_empty() {
                invalid_field_messages
                    .push(format!("{} must include a valid scope address", prefix));
            }
            if collateral.quote.is_empty() {
                invalid_field_messages
                    .push(format!("{} must include at least one quote coin", prefix));
            }
            invalid_field_messages.append(
                &mut collateral
                    .quote
                    .iter()
                    .flat_map(|coin| validate_coin(coin, "BidCollateral Quote Coin"))
                    .collect(),
            );
        }
    }
    if invalid_field_messages.is_empty() {
        ().to_ok()
    } else {
        ContractError::validation_error(&invalid_field_messages).to_err()
    }
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::test::request_helpers::{
        mock_bid, mock_bid_marker_share, mock_bid_marker_trade, mock_bid_scope_trade,
        mock_bid_with_descriptor,
    };
    use crate::types::core::error::ContractError;
    use crate::types::request::bid_types::bid_collateral::BidCollateral;
    use crate::types::request::bid_types::bid_order::BidOrder;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use crate::validation::bid_order_validation::validate_bid_order;
    use cosmwasm_std::{coins, Addr};

    #[test]
    fn test_missing_id() {
        assert_validation_failure(
            "bid order id is empty",
            &BidOrder::new_unchecked(
                "",
                Addr::unchecked("bidder"),
                BidCollateral::coin_trade(&[], &[]),
                None,
            ),
            "id for BidOrder must not be empty",
        );
    }

    #[test]
    fn test_missing_owner_address() {
        assert_validation_failure(
            "bid order address is empty",
            &BidOrder::new_unchecked(
                "ask_id",
                Addr::unchecked(""),
                BidCollateral::coin_trade(&[], &[]),
                None,
            ),
            "owner for BidOrder must not be empty",
        );
    }

    #[test]
    fn test_attribute_requirement_provided_with_empty_attributes_list() {
        assert_validation_failure(
            "bid order provided an empty attributes list for RequiredAttributes",
            &mock_bid_with_descriptor(
                BidCollateral::coin_trade(&[], &[]),
                RequestDescriptor::new_populated_attributes(
                    "descriptor",
                    AttributeRequirement::none::<String>(&[]),
                ),
            ),
            "BidOrder [bid_id] specified RequiredAttributes, but the value included no attributes to check",
        );
    }

    #[test]
    fn test_bid_type_mismatches() {
        let mut bid_order = BidOrder {
            id: "bid_id".to_string(),
            bid_type: RequestType::CoinTrade,
            owner: Addr::unchecked("bidder"),
            collateral: BidCollateral::scope_trade("scope", &[]),
            descriptor: None,
        };
        assert_validation_failure(
            "bid order provided coin_trade request type but wrong collateral type",
            &bid_order,
            "bid type [coin_trade] for BidOrder [bid_id] is invalid. type requires collateral type of BidCollateral::CoinTrade",
        );
        bid_order.bid_type = RequestType::MarkerTrade;
        assert_validation_failure(
            "bid order provided marker_trade request type but wrong collateral type",
            &bid_order,
            "bid type [marker_trade] for BidOrder [bid_id] is invalid. type requires collateral type of BidCollateral::MarkerTrade",
        );
        bid_order.bid_type = RequestType::MarkerShareSale;
        assert_validation_failure(
            "bid order provided marker_share_sale request type but wrong collateral type",
            &bid_order,
            "bid type [marker_share_sale] for BidOrder [bid_id] is invalid. type requires collateral type of BidCollateral::MarkerShareSale",
        );
        bid_order.bid_type = RequestType::ScopeTrade;
        bid_order.collateral = BidCollateral::coin_trade(&[], &[]);
        assert_validation_failure(
            "bid order provided scope_trade request type but wrong collateral type",
            &bid_order,
            "bid type [scope_trade] for BidOrder [bid_id] is invalid. type requires collateral type of BidCollateral::ScopeTrade",
        );
    }

    #[test]
    fn test_coin_trade_empty_base() {
        assert_validation_failure(
            "bid order is missing coin trade base funds",
            &mock_bid(BidCollateral::coin_trade(&[], &coins(100, "nhash"))),
            coin_trade_error("must include base funds"),
        );
    }

    #[test]
    fn test_coin_trade_base_funds_include_invalid_coins() {
        assert_validation_failure(
            "bid order includes base coin with zero amount",
            &mock_bid(BidCollateral::coin_trade(&coins(0, "nhash"), &[])),
            zero_coin_error("nhash", "BidCollateral Base Coin"),
        );
        assert_validation_failure(
            "bid order includes base coin with invalid denom",
            &mock_bid(BidCollateral::coin_trade(&coins(100, ""), &[])),
            blank_denom_error(100, "BidCollateral Base Coin"),
        );
    }

    #[test]
    fn test_coin_trade_empty_quote() {
        assert_validation_failure(
            "bid order is missing coin trade quote funds",
            &mock_bid(BidCollateral::coin_trade(&coins(100, "nhash"), &[])),
            coin_trade_error("must include quote funds"),
        );
    }

    #[test]
    fn test_coin_trade_quote_funds_include_invalid_coins() {
        assert_validation_failure(
            "bid order includes quote coin with zero amount",
            &mock_bid(BidCollateral::coin_trade(&[], &coins(0, "nhash"))),
            zero_coin_error("nhash", "BidCollateral Quote Coin"),
        );
        assert_validation_failure(
            "bid order includes quote coin with invalid denom",
            &mock_bid(BidCollateral::coin_trade(&[], &coins(100, ""))),
            blank_denom_error(100, "BidCollateral Quote Coin"),
        );
    }

    #[test]
    fn test_marker_trade_empty_marker_address() {
        assert_validation_failure(
            "bid order does not include a valid marker address",
            &mock_bid(mock_bid_marker_trade("", "denom", &coins(100, "nhash"))),
            marker_trade_error("must include a valid marker address"),
        );
    }

    #[test]
    fn test_marker_trade_empty_marker_denom() {
        assert_validation_failure(
            "bid order does not include a valid marker denom",
            &mock_bid(mock_bid_marker_trade("marker", "", &coins(100, "nhash"))),
            marker_trade_error("must include a valid marker denom"),
        );
    }

    #[test]
    fn test_marker_trade_empty_quote() {
        assert_validation_failure(
            "bid order does not include quote funds",
            &mock_bid(mock_bid_marker_trade("marker", "denom", &[])),
            marker_trade_error("must include at least one quote coin"),
        );
    }

    #[test]
    fn test_marker_trade_quote_funds_include_invalid_coins() {
        assert_validation_failure(
            "bid order includes quote coin with zero amount",
            &mock_bid(mock_bid_marker_trade("marker", "denom", &coins(0, "nhash"))),
            zero_coin_error("nhash", "BidCollateral Quote Coin"),
        );
        assert_validation_failure(
            "bid order includes quote coin with blank denom",
            &mock_bid(mock_bid_marker_trade("marker", "denom", &coins(100, ""))),
            blank_denom_error(100, "BidCollateral Quote Coin"),
        );
    }

    #[test]
    fn test_marker_share_sale_empty_marker_address() {
        assert_validation_failure(
            "bid order does not include a valid marker address",
            &mock_bid(mock_bid_marker_share(
                "",
                "denom",
                100,
                &coins(100, "nhash"),
            )),
            marker_share_sale_error("must include a valid marker address"),
        );
    }

    #[test]
    fn test_marker_share_sale_empty_marker_denom() {
        assert_validation_failure(
            "bid order does not include a valid marker denom",
            &mock_bid(mock_bid_marker_share(
                "marker",
                "",
                100,
                &coins(100, "nhash"),
            )),
            marker_share_sale_error("must include a valid marker denom"),
        );
    }

    #[test]
    fn test_marker_share_sale_zero_share_count() {
        assert_validation_failure(
            "bid order has a share count of zero",
            &mock_bid(mock_bid_marker_share(
                "marker",
                "denom",
                0,
                &coins(100, "nhash"),
            )),
            marker_share_sale_error("must request to purchase at least one share"),
        );
    }

    #[test]
    fn test_marker_share_sale_empty_quote() {
        assert_validation_failure(
            "bid order does not include quote funds",
            &mock_bid(mock_bid_marker_share("marker", "denom", 100, &[])),
            marker_share_sale_error("must include at least one quote coin"),
        );
    }

    #[test]
    fn test_marker_share_sale_quote_funds_include_invalid_coins() {
        assert_validation_failure(
            "bid order includes quote coin with zero amount",
            &mock_bid(mock_bid_marker_share(
                "marker",
                "denom",
                100,
                &coins(0, "nhash"),
            )),
            zero_coin_error("nhash", "BidCollateral Quote Coin"),
        );
        assert_validation_failure(
            "bid order includes quote coin with blank denom",
            &mock_bid(mock_bid_marker_share(
                "marker",
                "denom",
                50,
                &coins(100, ""),
            )),
            blank_denom_error(100, "BidCollateral Quote Coin"),
        );
    }

    #[test]
    fn test_scope_trade_empty_scope_address() {
        assert_validation_failure(
            "bid order does not include a valid scope address",
            &mock_bid(mock_bid_scope_trade("", &coins(100, "nhash"))),
            scope_trade_error("must include a valid scope address"),
        );
    }

    #[test]
    fn test_scope_trade_empty_quote() {
        assert_validation_failure(
            "bid order does not include quote funds",
            &mock_bid(mock_bid_scope_trade("scope", &[])),
            scope_trade_error("must include at least one quote coin"),
        );
    }

    #[test]
    fn test_scope_trade_quote_funds_include_invalid_coins() {
        assert_validation_failure(
            "bid order includes quote coin with zero amount",
            &mock_bid(mock_bid_scope_trade("scope", &coins(0, "nhash"))),
            zero_coin_error("nhash", "BidCollateral Quote Coin"),
        );
        assert_validation_failure(
            "bid order includes quote coin with blank denom",
            &mock_bid(mock_bid_scope_trade("scope", &coins(100, ""))),
            blank_denom_error(100, "BidCollateral Quote Coin"),
        );
    }

    fn collateral_type_error<S1: Into<String>, S2: Into<String>>(
        collateral_type: S1,
        suffix: S2,
    ) -> String {
        format!(
            "BidOrder [bid_id] of type {} {}",
            collateral_type.into(),
            suffix.into()
        )
    }

    fn coin_trade_error<S: Into<String>>(suffix: S) -> String {
        collateral_type_error("coin trade", suffix)
    }

    fn marker_trade_error<S: Into<String>>(suffix: S) -> String {
        collateral_type_error("marker trade", suffix)
    }

    fn marker_share_sale_error<S: Into<String>>(suffix: S) -> String {
        collateral_type_error("marker share sale", suffix)
    }

    fn scope_trade_error<S: Into<String>>(suffix: S) -> String {
        collateral_type_error("scope trade", suffix)
    }

    fn zero_coin_error<S1: Into<String>, S2: Into<String>>(denom: S1, coin_type: S2) -> String {
        format!(
            "Zero amounts not allowed on coins. Coin denom [{}] and type [{}] for BidOrder [bid_id]", 
            denom.into(),
            coin_type.into(),
        )
    }

    fn blank_denom_error<S: Into<String>>(coin_amount: u128, coin_type: S) -> String {
        format!(
            "Blank denoms not allowed on coins. Coin amount [{}] and type [{}] for BidOrder [bid_id]",
            coin_amount,
            coin_type.into(),
        )
    }

    fn assert_validation_failure<S1: Into<String>, S2: Into<String>>(
        test_name: S1,
        bid_order: &BidOrder,
        expected_error_message: S2,
    ) {
        let test_name = test_name.into();
        let message = expected_error_message.into();
        let messages = match validate_bid_order(&bid_order) {
            Err(e) => match e {
                ContractError::ValidationError { messages } => messages,
                e => panic!(
                    "{}: Expected message [{}], but got unexpected error type instead during validation: {:?}",
                    test_name, message, e,
                ),
            },
            Ok(_) => panic!(
                "{}: Expected message [{}] to be output for input values, but validation passed",
                test_name, message,
            ),
        };
        assert!(
            messages.contains(&message),
            "expected message [{}] to be in result list {:?} for ask order [{}]",
            &message,
            &messages,
            &bid_order.id,
        );
    }
}
