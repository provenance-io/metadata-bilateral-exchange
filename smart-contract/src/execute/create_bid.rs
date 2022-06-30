use crate::storage::bid_order_storage::{get_bid_order_by_id, insert_bid_order};
use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid::{
    Bid, CoinTradeBid, MarkerShareSaleBid, MarkerTradeBid, ScopeTradeBid,
};
use crate::types::request::bid_types::bid_collateral::BidCollateral;
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::get_single_marker_coin_holding;
use cosmwasm_std::{to_binary, DepsMut, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuerier, ProvenanceQuery};

// create bid entrypoint
pub fn create_bid(
    deps: DepsMut<ProvenanceQuery>,
    info: MessageInfo,
    bid: Bid,
    descriptor: Option<RequestDescriptor>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    if get_bid_order_by_id(deps.storage, bid.get_id()).is_ok() {
        return ContractError::existing_id("bid", bid.get_id()).to_err();
    }
    let collateral = match &bid {
        Bid::CoinTrade(coin_trade) => create_coin_trade_collateral(&info, coin_trade),
        Bid::MarkerTrade(marker_trade) => {
            create_marker_trade_collateral(&deps, &info, marker_trade)
        }
        Bid::MarkerShareSale(marker_share_sale) => {
            create_marker_share_sale_collateral(&deps, &info, marker_share_sale)
        }
        Bid::ScopeTrade(scope_trade) => create_scope_trade_collateral(&info, scope_trade),
    }?;
    let bid_order = BidOrder::new(bid.get_id(), info.sender, collateral, descriptor)?;
    insert_bid_order(deps.storage, &bid_order)?;
    Response::new()
        .add_attribute("action", "create_bid")
        .add_attribute("bid_id", bid.get_id())
        .set_data(to_binary(&bid_order)?)
        .to_ok()
}

fn create_coin_trade_collateral(
    info: &MessageInfo,
    coin_trade: &CoinTradeBid,
) -> Result<BidCollateral, ContractError> {
    if coin_trade.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if coin_trade.base.is_empty() {
        return ContractError::missing_field("base").to_err();
    }
    if info.funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "coin trade bid requests should include funds",
        )
        .to_err();
    }
    BidCollateral::coin_trade(&coin_trade.base, &info.funds).to_ok()
}

fn create_marker_trade_collateral(
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    marker_trade: &MarkerTradeBid,
) -> Result<BidCollateral, ContractError> {
    if marker_trade.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if marker_trade.denom.is_empty() {
        return ContractError::missing_field("denom").to_err();
    }
    if info.funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "funds must be provided during a marker trade bid to establish a quote",
        )
        .to_err();
    }
    // This grants us access to the marker address, as well as ensuring that the marker is real
    let marker = ProvenanceQuerier::new(&deps.querier).get_marker_by_denom(&marker_trade.denom)?;
    BidCollateral::marker_trade(marker.address, &marker_trade.denom, &info.funds).to_ok()
}

fn create_marker_share_sale_collateral(
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    marker_share_sale: &MarkerShareSaleBid,
) -> Result<BidCollateral, ContractError> {
    if marker_share_sale.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if marker_share_sale.denom.is_empty() {
        return ContractError::missing_field("denom").to_err();
    }
    if marker_share_sale.share_count.is_zero() {
        return ContractError::validation_error(&[
            "share count must be at least one for a marker share sale",
        ])
        .to_err();
    }
    if info.funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "funds must be provided during a marker share trade bid to establish a quote",
        )
        .to_err();
    }
    let marker =
        ProvenanceQuerier::new(&deps.querier).get_marker_by_denom(&marker_share_sale.denom)?;
    let marker_shares_available = get_single_marker_coin_holding(&marker)?.amount.u128();
    if marker_share_sale.share_count.u128() > marker_shares_available {
        return ContractError::validation_error(&[format!(
            "share count [{}] must be less than or equal to remaining [{}] shares available [{}]",
            marker_share_sale.share_count.u128(),
            marker_share_sale.denom,
            marker_shares_available,
        )])
        .to_err();
    }
    BidCollateral::marker_share_sale(
        marker.address,
        &marker_share_sale.denom,
        marker_share_sale.share_count.u128(),
        &info.funds,
    )
    .to_ok()
}

fn create_scope_trade_collateral(
    info: &MessageInfo,
    scope_trade: &ScopeTradeBid,
) -> Result<BidCollateral, ContractError> {
    if scope_trade.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if scope_trade.scope_address.is_empty() {
        return ContractError::missing_field("scope_address").to_err();
    }
    if info.funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "funds must be provided during a scope trade bid to establish a quote",
        )
        .to_err();
    }
    BidCollateral::scope_trade(&scope_trade.scope_address, &info.funds).to_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::execute;
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::default_instantiate;
    use crate::test::mock_marker::{MockMarker, DEFAULT_MARKER_ADDRESS, DEFAULT_MARKER_DENOM};
    use crate::test::mock_scope::DEFAULT_SCOPE_ID;
    use crate::test::request_helpers::mock_bid_order;
    use crate::types::core::msg::ExecuteMsg;
    use crate::types::request::request_descriptor::AttributeRequirement;
    use crate::types::request::request_type::RequestType;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Storage};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_new_bid_is_rejected_for_existing_id() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let bid_order = mock_bid_order(BidCollateral::coin_trade(&[], &[]));
        assert_eq!(
            "bid_id", bid_order.id,
            "sanity check: mock bid order should have the correct id"
        );
        insert_bid_order(deps.as_mut().storage, &bid_order)
            .expect("expected bid order insert to succeed");
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            Bid::new_coin_trade(
                // Matches the mock bid id, which should trigger the error
                "bid_id",
                &[],
            ),
            None,
        )
        .expect_err("an error should occur");
        assert!(
            matches!(err, ContractError::ExistingId { .. }),
            "an existing id error should be returned, indicating that a bid with this id exists",
        )
    }

    #[test]
    fn create_coin_trade_bid_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);

        // create bid data
        let create_bid_msg = ExecuteMsg::CreateBid {
            bid: Bid::new_coin_trade("bid_id", &coins(100, "base_1")),
            descriptor: None,
        };

        let bidder_info = mock_info("bidder", &coins(2, "mark_2"));

        // execute create bid
        let create_bid_response = execute(
            deps.as_mut(),
            mock_env(),
            bidder_info.clone(),
            create_bid_msg.clone(),
        );

        // verify execute create bid response
        match create_bid_response {
            Ok(response) => {
                assert_eq!(response.attributes.len(), 2);
                assert_eq!("create_bid", single_attribute_for_key(&response, "action"));
                assert_eq!("bid_id", single_attribute_for_key(&response, "bid_id"));
            }
            Err(error) => {
                panic!("failed to create bid: {:?}", error)
            }
        }

        // verify bid order stored
        if let ExecuteMsg::CreateBid {
            bid: Bid::CoinTrade(CoinTradeBid { id, base }),
            descriptor: None,
        } = create_bid_msg
        {
            match get_bid_order_by_id(deps.as_ref().storage, "bid_id") {
                Ok(stored_order) => {
                    assert_eq!(
                        stored_order,
                        BidOrder {
                            id,
                            bid_type: RequestType::CoinTrade,
                            owner: bidder_info.sender,
                            collateral: BidCollateral::coin_trade(&base, &bidder_info.funds),
                            descriptor: None,
                        }
                    )
                }
                _ => {
                    panic!("bid order was not found in storage")
                }
            }
        } else {
            panic!("bid_message is not a CreateBid type. this is bad.")
        }
    }

    #[test]
    fn create_coin_bid_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);

        // create bid missing id
        let create_bid_msg = ExecuteMsg::CreateBid {
            bid: Bid::new_coin_trade("", &coins(100, "base_1")),
            descriptor: None,
        };

        // execute create bid
        let create_bid_response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, "quote_1")),
            create_bid_msg,
        );

        // verify execute create bid response returns ContractError::MissingField { id }
        match create_bid_response {
            Ok(_) => panic!("expected error, but create_bid_response ok"),
            Err(error) => match error {
                ContractError::MissingField { field } => {
                    assert_eq!(field, "id")
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }

        // create bid missing base
        let create_bid_msg = ExecuteMsg::CreateBid {
            bid: Bid::new_coin_trade("id", &[]),
            descriptor: None,
        };

        // execute create bid
        let create_bid_response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, "quote_1")),
            create_bid_msg,
        );

        // verify execute create bid response returns ContractError::MissingField { base }
        match create_bid_response {
            Ok(_) => panic!("expected error, but create_bid_response ok"),
            Err(error) => match error {
                ContractError::MissingField { field } => {
                    assert_eq!(field, "base")
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }

        // create bid missing quote
        let create_bid_msg = ExecuteMsg::CreateBid {
            bid: Bid::new_coin_trade("id", &coins(100, "base_1")),
            descriptor: None,
        };

        // execute create bid
        let create_bid_response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &[]),
            create_bid_msg,
        )
        .expect_err("expected an error for a missing quote on a bid");

        // verify execute create bid response returns ContractError::BidMissingQuote
        match create_bid_response {
            ContractError::InvalidFundsProvided { message } => {
                assert_eq!("coin trade bid requests should include funds", message,);
            }
            e => panic!(
                "unexpected error when no funds provided to create bid: {:?}",
                e
            ),
        }
    }

    #[test]
    fn test_create_marker_trade_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::none(&["none.pb", "of.pb", "these.pb"]),
        );
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        let response = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            Some(descriptor.clone()),
        )
        .expect("expected bid creation to succeed");
        let bid_order = assert_valid_response(
            deps.as_ref().storage,
            &response,
            RequestType::MarkerTrade,
            &descriptor,
        );
        let collateral = bid_order.collateral.unwrap_marker_trade();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS, collateral.address,
            "the correct marker address should be set on the collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.denom,
            "the correct marker denom should be set on the collateral",
        );
        assert_eq!(
            coins(100, "nhash"),
            collateral.quote,
            "the correct quote should be set on the collateral",
        );
    }

    #[test]
    fn test_create_marker_trade_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_trade("", "somedenom"),
            None,
        )
        .expect_err("an error should occur when the bid has a blank id");
        match err {
            ContractError::MissingField { field } => {
                assert_eq!("id", field, "expected the id field to be the missing field",);
            }
            e => panic!("unexpected error: {:?}", e),
        }
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_trade("bid_id", ""),
            None,
        )
        .expect_err("an error should occur when the bid has a blank denom");
        match err {
            ContractError::MissingField { field } => {
                assert_eq!(
                    "denom", field,
                    "expected the denom field to be the missing field",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            None,
        )
        .expect_err("an error should occur when no quote funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds provided error should occur when the bidder provides no funds",
        );
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            None,
        )
        .expect_err("an error should occur when no marker is found");
        assert!(
            matches!(err, ContractError::Std(_)),
            "an std error should occur when the marker cannot be found",
        );
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::none::<String>(&[]),
            )),
        )
        .expect_err("an invalid state (attribute requirement) should trigger an error");
        assert!(
            matches!(err, ContractError::ValidationError { .. }),
            "validation errors in the produced BidOrder should trigger a validation error",
        );
    }

    #[test]
    fn test_marker_share_sale_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::all(&["attribute.pio", "attribute2.pb"]),
        );
        let response = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(1000, "nhash")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 10),
            Some(descriptor.clone()),
        )
        .expect("expected the marker share sale bid order to be created successfully");
        let bid_order = assert_valid_response(
            deps.as_ref().storage,
            &response,
            RequestType::MarkerShareSale,
            &descriptor,
        );
        let collateral = bid_order.collateral.unwrap_marker_share_sale();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.address.as_str(),
            "the correct marker address should be set in the collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.denom,
            "the correct marker denom should be set in the collateral",
        );
        assert_eq!(
            10,
            collateral.share_count.u128(),
            "the correct share count should be set in the collateral",
        );
        assert_eq!(
            coins(1000, "nhash"),
            collateral.quote,
            "the correct quote should be set in the collateral",
        );
    }

    #[test]
    fn test_marker_share_sale_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_share_sale("", DEFAULT_MARKER_DENOM, 100),
            None,
        )
        .expect_err("an error should occur when the id is blank");
        match err {
            ContractError::MissingField { field } => {
                assert_eq!("id", field, "the missing field should be the id",);
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_share_sale("bid_id", "", 100),
            None,
        )
        .expect_err("an error should  occur when the denom is blank");
        match err {
            ContractError::MissingField { field } => {
                assert_eq!("denom", field, "the missing field should be the denom",);
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 0),
            None,
        )
        .expect_err("an error should occur when the share count is zero");
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(
                    1,
                    messages.len(),
                    "there should be a single validation error",
                );
                assert_eq!(
                    "share count must be at least one for a marker share sale",
                    messages.first().unwrap(),
                    "the correct validation error should be produced",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 100),
            None,
        )
        .expect_err("an error should occur when no funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when a bid is created without a quote",
        );
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 100),
            None,
        )
        .expect_err("an error should occur when no marker is found");
        assert!(
            matches!(err, ContractError::Std(_)),
            "an std error should be returned when no marker can be found by the given denom",
        );
        let invalid_marker = MockMarker {
            coins: vec![],
            ..MockMarker::new_owned_mock_marker("asker")
        }
        .to_marker();
        deps.querier.with_markers(vec![invalid_marker]);
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 100),
            None,
        )
        .expect_err("an error should occur when the marker does not have a proper coin holding");
        assert!(
            matches!(err, ContractError::InvalidMarker { .. }),
            "an invalid marker error should be returned when the marker does not hold its own coin",
        );
        let invalid_marker = MockMarker {
            coins: coins(9, DEFAULT_MARKER_DENOM),
            ..MockMarker::new_owned_mock_marker("asker")
        }
        .to_marker();
        deps.querier.with_markers(vec![invalid_marker]);
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 10),
            None,
        )
        .expect_err(
            "an error should occur when the bid wants to buy more coin than the marker has",
        );
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(
                    1,
                    messages.len(),
                    "expected only a single validation error message",
                );
                assert_eq!(
                    &format!("share count [10] must be less than or equal to remaining [{}] shares available [9]", DEFAULT_MARKER_DENOM),
                    messages.first().unwrap(),
                    "expected the correct error message to be produced",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 10),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all::<String>(&[]),
            )),
        )
        .expect_err("a missing attribute requirement attributes values should produce an error");
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(1, messages.len(), "only one error should occur",);
                assert_eq!(
                    "BidOrder [bid_id] specified RequiredAttributes, but the value included no attributes to check",
                    messages.first().unwrap(),
                    "the correct error message should be produced",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
    }

    #[test]
    fn test_create_scope_trade_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["attr.pb", "other.pio"]),
        );
        let response = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "nhash")),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ID),
            Some(descriptor.clone()),
        )
        .expect("expected the scope trade to successfully execute");
        let bid_order = assert_valid_response(
            deps.as_ref().storage,
            &response,
            RequestType::ScopeTrade,
            &descriptor,
        );
        let collateral = bid_order.collateral.unwrap_scope_trade();
        assert_eq!(
            DEFAULT_SCOPE_ID, collateral.scope_address,
            "the correct scope address should be set in the bid collateral",
        );
        assert_eq!(
            coins(150, "nhash"),
            collateral.quote,
            "the correct quote should be set in the bid collateral",
        );
    }

    #[test]
    fn test_scope_trade_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_scope_trade("", DEFAULT_SCOPE_ID),
            None,
        )
        .expect_err("an error should occur when the bid id is missing");
        match err {
            ContractError::MissingField { field } => {
                assert_eq!("id", field, "the id field should be missing",);
            }
            e => panic!("unexpected error: {:?}", e),
        }
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_scope_trade("bid_id", ""),
            None,
        )
        .expect_err("an error should occur when the scope address is missing");
        match err {
            ContractError::MissingField { field } => {
                assert_eq!(
                    "scope_address", field,
                    "the scope_address field should be missing",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ID),
            None,
        )
        .expect_err("an error should occur when no quote funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should be produced when no quote funds are sent for the bid",
        );
        let err = create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "nhash")),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ID),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all::<String>(&[]),
            )),
        )
        .expect_err("a missing attribute requirement attributes values should produce an error");
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(1, messages.len(), "only one error should occur",);
                assert_eq!(
                    "BidOrder [bid_id] specified RequiredAttributes, but the value included no attributes to check",
                    messages.first().unwrap(),
                    "the correct error message should be produced",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
    }

    fn assert_valid_response(
        storage: &dyn Storage,
        response: &Response<ProvenanceMsg>,
        expected_bid_type: RequestType,
        descriptor: &RequestDescriptor,
    ) -> BidOrder {
        assert!(
            response.messages.is_empty(),
            "no bid creation responses should send messages"
        );
        assert_eq!(
            2,
            response.attributes.len(),
            "the correct number of attributes should be sent in a bid response",
        );
        assert_eq!(
            "create_bid",
            single_attribute_for_key(response, "action"),
            "the correct action attribute should be sent",
        );
        assert_eq!(
            "bid_id",
            single_attribute_for_key(response, "bid_id"),
            "the correct bid_id attribute should be sent",
        );
        let bid_order: BidOrder = if let Some(bid_order_binary) = &response.data {
            from_binary(bid_order_binary)
                .expect("expected bid order to deserialize properly from response")
        } else {
            panic!("expected data to be properly set after a successful response")
        };
        let storage_order =
            get_bid_order_by_id(storage, &bid_order.id).expect("bid order should be found by id");
        assert_eq!(
            bid_order, storage_order,
            "the bid order produced in the data should be the same value in storage",
        );
        assert_eq!(
            "bid_id", bid_order.id,
            "the correct id should be set on the bid order",
        );
        assert_eq!(
            expected_bid_type, bid_order.bid_type,
            "the correct bid type should be set on the bid order",
        );
        assert_eq!(
            "bidder",
            bid_order.owner.as_str(),
            "the correct owner should be set on the bid order",
        );
        if let Some(ref found_descriptor) = &bid_order.descriptor {
            assert_eq!(
                descriptor, found_descriptor,
                "expected the descriptor to be properly set on the bid order",
            );
        } else {
            panic!("expected a descriptor to be set on the bid order");
        }
        bid_order
    }
}
