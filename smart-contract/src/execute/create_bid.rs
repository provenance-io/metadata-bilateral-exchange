use crate::storage::bid_order_storage::{get_bid_order_by_id, insert_bid_order};
use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid::Bid;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::util::create_bid_order_utilities::{
    create_bid_order, BidCreationType, BidOrderCreationResponse,
};
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::get_custom_fee_amount_display;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

// create bid entrypoint
pub fn create_bid(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    bid: Bid,
    descriptor: Option<RequestDescriptor>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    if get_bid_order_by_id(deps.storage, bid.get_id()).is_ok() {
        return ContractError::ExistingId {
            id: bid.get_id().to_string(),
            id_type: "bid".to_string(),
        }
        .to_err();
    }
    let BidOrderCreationResponse {
        bid_order,
        bid_fee_msg,
    } = create_bid_order(&deps, &env, &info, bid, descriptor, BidCreationType::New)?;
    insert_bid_order(deps.storage, &bid_order)?;
    let mut response = Response::new()
        .add_attribute("action", "create_bid")
        .add_attribute("bid_id", &bid_order.id)
        .set_data(to_binary(&bid_order)?);
    if let Some(bid_fee_msg) = bid_fee_msg {
        response = response
            .add_attribute(
                "bid_fee_charged",
                get_custom_fee_amount_display(&bid_fee_msg)?,
            )
            .add_message(bid_fee_msg);
    }
    response.to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::execute;
    use crate::execute::create_bid::create_bid;
    use crate::storage::bid_order_storage::{get_bid_order_by_id, insert_bid_order};
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::{
        default_instantiate, test_instantiate, TestInstantiate, DEFAULT_ADMIN_ADDRESS,
    };
    use crate::test::mock_marker::{MockMarker, DEFAULT_MARKER_ADDRESS, DEFAULT_MARKER_DENOM};
    use crate::test::mock_scope::DEFAULT_SCOPE_ADDR;
    use crate::test::request_helpers::mock_bid_order;
    use crate::types::core::error::ContractError;
    use crate::types::core::msg::ExecuteMsg;
    use crate::types::request::bid_types::bid::Bid;
    use crate::types::request::bid_types::bid_collateral::BidCollateral;
    use crate::types::request::bid_types::bid_order::BidOrder;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use crate::util::constants::NHASH;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary, CosmosMsg, Response, Storage, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{MsgFeesMsgParams, ProvenanceMsg, ProvenanceMsgParams};

    #[test]
    fn test_coin_trade_with_valid_data() {
        do_valid_data_coin_trade(None);
    }

    #[test]
    fn test_coin_trade_with_valid_data_and_bid_fee() {
        do_valid_data_coin_trade(Some(100));
    }

    #[test]
    fn test_marker_trade_with_valid_data() {
        do_valid_data_marker_trade(None);
    }

    #[test]
    fn test_marker_trade_with_valid_data_and_bid_fee() {
        do_valid_data_marker_trade(Some(1500));
    }

    #[test]
    fn test_marker_share_sale_with_valid_data() {
        do_valid_data_marker_share_sale(None);
    }

    #[test]
    fn test_marker_share_sale_with_valid_data_and_bid_fee() {
        do_valid_data_marker_share_sale(Some(150));
    }

    #[test]
    fn test_scope_trade_with_valid_data() {
        do_valid_data_scope_trade(None);
    }

    #[test]
    fn test_scope_trade_with_valid_data_and_bid_fee() {
        do_valid_data_scope_trade(Some(12));
    }

    #[test]
    fn test_new_bid_is_rejected_for_existing_id() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let bid_order = mock_bid_order(BidCollateral::coin_trade(&[], &[]));
        assert_eq!(
            "bid_id", bid_order.id,
            "sanity check: mock bid order should have the correct id"
        );
        insert_bid_order(deps.as_mut().storage, &bid_order)
            .expect("expected bid order insert to succeed");
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
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
    fn test_coin_trade_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());

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
            create_bid_msg.clone(),
        )
        .expect_err("expected an error for a missing quote on a bid");

        // verify execute create bid response returns ContractError::InvalidFundsProvided
        match create_bid_response {
            ContractError::InvalidFundsProvided { message } => {
                assert_eq!(
                    "coin trade bid requests should include enough funds for a quote",
                    message,
                );
            }
            e => panic!(
                "unexpected error when no funds provided to create bid: {:?}",
                e
            ),
        };
    }

    #[test]
    fn test_marker_trade_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_marker_trade("", "somedenom", None),
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
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_marker_trade("bid_id", "", None),
            None,
        )
        .expect_err("an error should occur when the bid has a blank denom");
        match err {
            ContractError::MissingField { field } => {
                assert_eq!(
                    "marker_denom", field,
                    "expected the denom field to be the missing field",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &[]),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM, None),
            None,
        )
        .expect_err("an error should occur when no quote funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds provided error should occur when the bidder provides no funds, but got: {:?}",
            err,
        );
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM, None),
            None,
        )
        .expect_err("an error should occur when no marker is found");
        assert!(
            matches!(err, ContractError::Std(_)),
            "an std error should occur when the marker cannot be found, but got: {:?}",
            err,
        );
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM, None),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::none::<String>(&[]),
            )),
        )
        .expect_err("an invalid state (attribute requirement) should trigger an error");
        assert!(
            matches!(err, ContractError::ValidationError { .. }),
            "validation errors in the produced BidOrder should trigger a validation error, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_marker_share_sale_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
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
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_marker_share_sale("bid_id", "", 100),
            None,
        )
        .expect_err("an error should  occur when the denom is blank");
        match err {
            ContractError::MissingField { field } => {
                assert_eq!(
                    "marker_denom", field,
                    "the missing field should be the denom",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
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
            mock_env(),
            mock_info("bidder", &[]),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 100),
            None,
        )
        .expect_err("an error should occur when no funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when a bid is created without a quote, but got: {:?}",
            err,
        );
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 100),
            None,
        )
        .expect_err("an error should occur when no marker is found");
        assert!(
            matches!(err, ContractError::Std(_)),
            "an std error should be returned when no marker can be found by the given denom, but got: {:?}",
            err,
        );
        let invalid_marker = MockMarker {
            coins: vec![],
            ..MockMarker::new_owned_mock_marker("asker")
        }
        .to_marker();
        deps.querier.with_markers(vec![invalid_marker]);
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 100),
            None,
        )
        .expect_err("an error should occur when the marker does not have a proper coin holding");
        assert!(
            matches!(err, ContractError::InvalidMarker { .. }),
            "an invalid marker error should be returned when the marker does not hold its own coin, but got: {:?}",
            err,
        );
        let invalid_marker = MockMarker {
            coins: coins(9, DEFAULT_MARKER_DENOM),
            ..MockMarker::new_owned_mock_marker("asker")
        }
        .to_marker();
        deps.querier.with_markers(vec![invalid_marker]);
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
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
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
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
    fn test_scope_trade_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_scope_trade("", DEFAULT_SCOPE_ADDR),
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
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
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
            mock_env(),
            mock_info("bidder", &[]),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
            None,
        )
        .expect_err("an error should occur when no quote funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should be produced when no quote funds are sent for the bid, but got: {:?}",
            err,
        );
        let err = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
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
        expected_bid_fee: Option<u128>,
        descriptor: &RequestDescriptor,
    ) -> BidOrder {
        if let Some(ref expected_bid_fee) = &expected_bid_fee {
            assert_eq!(
                1,
                response.messages.len(),
                "the correct number of messages should be sent",
            );
            match &response.messages.first().unwrap().msg {
                CosmosMsg::Custom(ProvenanceMsg {
                    params:
                        ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee {
                            amount,
                            from,
                            name,
                            recipient,
                        }),
                    ..
                }) => {
                    assert_eq!(
                        *expected_bid_fee,
                        amount.amount.u128(),
                        "the correct fee amount should be sent",
                    );
                    assert_eq!(
                        NHASH, amount.denom,
                        "the fee should always be paid in nhash"
                    );
                    assert_eq!(
                        MOCK_CONTRACT_ADDR,
                        from.as_str(),
                        "the fee msg should always be built with the contract's address",
                    );
                    assert_eq!(
                        "bid creation nhash fee",
                        name.as_ref().expect("the name value should be set"),
                        "the name for the fee should be formatted correctly",
                    );
                    assert_eq!(
                        DEFAULT_ADMIN_ADDRESS,
                        recipient
                            .as_ref()
                            .expect("the recipient should be set")
                            .as_str(),
                        "the admin should always receive the fee",
                    );
                }
                msg => panic!("unexpected message found for bid fee: {:?}", msg),
            }
            assert_eq!(
                format!("{}{}", expected_bid_fee, NHASH),
                single_attribute_for_key(response, "bid_fee_charged"),
                "expected the correct bid_fee_charged attribute value",
            );
        } else {
            assert!(
                response.messages.is_empty(),
                "no bid creation responses without fees should send messages"
            );
        }
        assert_eq!(
            2 + if expected_bid_fee.is_some() { 1 } else { 0 },
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

    fn do_valid_data_coin_trade(bid_fee: Option<u128>) {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(
            deps.as_mut(),
            TestInstantiate {
                create_bid_nhash_fee: bid_fee.map(Uint128::new).clone(),
                ..TestInstantiate::default()
            },
        );

        let request_descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["nou.pb", "yesu.pio"]),
        );

        // create bid data
        let create_bid_msg = ExecuteMsg::CreateBid {
            bid: Bid::new_coin_trade("bid_id", &coins(100, "base_1")),
            descriptor: Some(request_descriptor.clone()),
        };

        let bidder_info = mock_info("bidder", &coins(2, "mark_2"));

        // execute create bid
        let response = execute(
            deps.as_mut(),
            mock_env(),
            bidder_info.clone(),
            create_bid_msg.clone(),
        )
        .expect("expected a valid response to be produced");

        let bid_order = assert_valid_response(
            deps.as_ref().storage,
            &response,
            RequestType::CoinTrade,
            bid_fee,
            &request_descriptor,
        );

        let collateral = bid_order.collateral.unwrap_coin_trade();
        assert_eq!(
            coins(2, "mark_2"),
            collateral.quote,
            "the correct quote should be listed in the collateral",
        );
        assert_eq!(
            coins(100, "base_1"),
            collateral.base,
            "the correct base should be listed in the collateral",
        );
    }

    fn do_valid_data_marker_trade(bid_fee: Option<u128>) {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(
            deps.as_mut(),
            TestInstantiate {
                create_bid_nhash_fee: bid_fee.map(Uint128::new).clone(),
                ..TestInstantiate::default()
            },
        );
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::none(&["none.pb", "of.pb", "these.pb"]),
        );
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        let response = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(100, NHASH)),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM, None),
            Some(descriptor.clone()),
        )
        .expect("expected bid creation to succeed");
        let bid_order = assert_valid_response(
            deps.as_ref().storage,
            &response,
            RequestType::MarkerTrade,
            bid_fee,
            &descriptor,
        );
        let collateral = bid_order.collateral.unwrap_marker_trade();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS, collateral.marker_address,
            "the correct marker address should be set on the collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be set on the collateral",
        );
        assert_eq!(
            coins(100, NHASH),
            collateral.quote,
            "the correct quote should be set on the collateral",
        );
    }

    fn do_valid_data_marker_share_sale(bid_fee: Option<u128>) {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(
            deps.as_mut(),
            TestInstantiate {
                create_bid_nhash_fee: bid_fee.map(Uint128::new).clone(),
                ..TestInstantiate::default()
            },
        );
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::all(&["attribute.pio", "attribute2.pb"]),
        );
        let response = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(1000, NHASH)),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 10),
            Some(descriptor.clone()),
        )
        .expect("expected the marker share sale bid order to be created successfully");
        let bid_order = assert_valid_response(
            deps.as_ref().storage,
            &response,
            RequestType::MarkerShareSale,
            bid_fee,
            &descriptor,
        );
        let collateral = bid_order.collateral.unwrap_marker_share_sale();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be set in the collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be set in the collateral",
        );
        assert_eq!(
            10,
            collateral.share_count.u128(),
            "the correct share count should be set in the collateral",
        );
        assert_eq!(
            coins(1000, NHASH),
            collateral.quote,
            "the correct quote should be set in the collateral",
        );
    }

    fn do_valid_data_scope_trade(bid_fee: Option<u128>) {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(
            deps.as_mut(),
            TestInstantiate {
                create_bid_nhash_fee: bid_fee.map(Uint128::new).clone(),
                ..TestInstantiate::default()
            },
        );
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["attr.pb", "other.pio"]),
        );
        let response = create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(150, NHASH)),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
            Some(descriptor.clone()),
        )
        .expect("expected the scope trade to successfully execute");
        let bid_order = assert_valid_response(
            deps.as_ref().storage,
            &response,
            RequestType::ScopeTrade,
            bid_fee,
            &descriptor,
        );
        let collateral = bid_order.collateral.unwrap_scope_trade();
        assert_eq!(
            DEFAULT_SCOPE_ADDR, collateral.scope_address,
            "the correct scope address should be set in the bid collateral",
        );
        assert_eq!(
            coins(150, NHASH),
            collateral.quote,
            "the correct quote should be set in the bid collateral",
        );
    }
}
