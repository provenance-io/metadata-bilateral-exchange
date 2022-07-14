use crate::storage::bid_order_storage::{get_bid_order_by_id, update_bid_order};
use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid::Bid;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::util::create_bid_order_utilities::{create_bid_order, BidCreationType};
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

pub fn update_bid(
    deps: DepsMut<ProvenanceQuery>,
    info: MessageInfo,
    bid: Bid,
    descriptor: Option<RequestDescriptor>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let existing_bid_order = get_bid_order_by_id(deps.storage, bid.get_id())?;
    if info.sender != existing_bid_order.owner {
        return ContractError::unauthorized().to_err();
    }
    let refunded_quote = existing_bid_order.collateral.get_quote();
    let new_bid_order = create_bid_order(
        &deps,
        &info,
        bid,
        descriptor,
        BidCreationType::Update {
            existing_bid_order: Box::new(existing_bid_order),
        },
    )?
    .bid_order;
    update_bid_order(deps.storage, &new_bid_order)?;
    Response::new()
        .add_attribute("action", "update_bid")
        .add_attribute("bid_id", &new_bid_order.id)
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: refunded_quote,
        }))
        .set_data(to_binary(&new_bid_order)?)
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::execute::create_bid::create_bid;
    use crate::execute::update_bid::update_bid;
    use crate::storage::bid_order_storage::get_bid_order_by_id;
    use crate::test::cosmos_type_helpers::{single_attribute_for_key, MockOwnedDeps};
    use crate::test::error_helpers::{assert_missing_field_error, assert_validation_error_message};
    use crate::test::mock_instantiate::default_instantiate;
    use crate::test::mock_marker::{MockMarker, DEFAULT_MARKER_ADDRESS, DEFAULT_MARKER_DENOM};
    use crate::test::mock_scope::DEFAULT_SCOPE_ADDR;
    use crate::types::core::error::ContractError;
    use crate::types::request::bid_types::bid::Bid;
    use crate::types::request::bid_types::bid_order::BidOrder;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{coins, from_binary, BankMsg, Coin, CosmosMsg, Response};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::ProvenanceMsg;

    #[test]
    fn test_invalid_update_for_missing_bid() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_coin_trade("bid_id", &coins(100, "base")),
            None,
        )
        .expect_err("an error should occur when the target bid does not exist");
        assert!(
            matches!(err, ContractError::StorageError { .. }),
            "a storage error should occur when an existing bid is not found by id, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_invalid_update_for_different_owner() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_coin_trade("bid_id", &coins(100, "base")),
            None,
        )
        .expect("the bid should be created");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder2", &coins(150, "quote")),
            Bid::new_coin_trade("bid_id", &coins(150, "base")),
            None,
        )
        .expect_err("an error should occur when the wrong sender tries to update a bid");
        assert!(
            matches!(err, ContractError::Unauthorized),
            "the bid owner must update a bid",
        );
    }

    #[test]
    fn test_valid_coin_trade_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "quote")),
            Bid::new_coin_trade("bid_id", &coins(150, "base")),
            None,
        )
        .expect("expected the bid to be created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::all(&["my.pb", "attributes.pb"]),
        );
        let response = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_coin_trade("bid_id", &coins(100, "base")),
            Some(descriptor.clone()),
        )
        .expect("the bid update should be successful");
        let bid_order = assert_valid_response(
            &deps,
            &response,
            RequestType::CoinTrade,
            &coins(150, "quote"),
            Some(descriptor),
        );
        let collateral = bid_order.collateral.unwrap_coin_trade();
        assert_eq!(
            coins(100, "quote"),
            collateral.quote,
            "the correct quote should be set on the bid",
        );
        assert_eq!(
            coins(100, "base"),
            collateral.base,
            "the correct base should be set on the bid",
        );
    }

    #[test]
    fn test_invalid_coin_trade_update_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_coin_trade("bid_id", &coins(100, "base")),
            None,
        )
        .expect("the bid should be created successfully");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "quote")),
            Bid::new_coin_trade("bid_id", &[]),
            None,
        )
        .expect_err("an error should occur when the bid supplies no base funds");
        assert_missing_field_error(err, "base");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            Bid::new_coin_trade("bid_id", &coins(150, "base")),
            None,
        )
        .expect_err("an error should occur when the bid supplies no quote funds");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when no quote funds are provided, but got: {:?}",
            err,
        );
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_scope_trade("bid_id_2", DEFAULT_SCOPE_ADDR),
            None,
        )
        .expect("the bid should be created successfully");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "quote")),
            Bid::new_coin_trade("bid_id_2", &coins(100, "base")),
            None,
        )
        .expect_err("an error should occur when trying to change bid type");
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    explanation,
                    "bid with id [bid_id_2] cannot change bid type from [scope_trade] to [coin_trade]",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "quote")),
            Bid::new_coin_trade("bid_id", &coins(150, "base")),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all::<String>(&[]),
            )),
        )
        .expect_err("an error should occur when an invalid BidOrder is produced");
        assert_validation_error_message(err, "BidOrder [bid_id] specified RequiredAttributes, but the value included no attributes to check");
    }

    #[test]
    fn test_valid_marker_trade_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        // Create a new contract-owned marker to ensure the creation functions see it
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(1400, "quote")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            None,
        )
        .expect("expected the bid to be created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::all(&["my.pb", "attributes.pb"]),
        );
        let response = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(200, "quote")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            Some(descriptor.clone()),
        )
        .expect("the bid update should be successful");
        let bid_order = assert_valid_response(
            &deps,
            &response,
            RequestType::MarkerTrade,
            &coins(1400, "quote"),
            Some(descriptor),
        );
        let collateral = bid_order.collateral.unwrap_marker_trade();
        assert_eq!(
            coins(200, "quote"),
            collateral.quote,
            "the correct quote should be set on the bid",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be set on the bid",
        );
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be set on the bid",
        );
    }

    #[test]
    fn test_invalid_marker_trade_update_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        // Create a new contract-owned marker to ensure the creation functions see it
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(10, "quote")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            None,
        )
        .expect("the bid should be created successfully");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_trade("bid_id", ""),
            None,
        )
        .expect_err("an error should occur when the marker denom is empty");
        assert_missing_field_error(err, "marker_denom");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            None,
        )
        .expect_err("an error should occur when the bid supplies no quote funds");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when no quote funds are provided, but got: {:?}",
            err,
        );
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "quote")),
            Bid::new_coin_trade("bid_id_2", &coins(150, "base")),
            None,
        )
        .expect("expected the second bid to be created successfully");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "quote")),
            Bid::new_marker_trade("bid_id_2", DEFAULT_MARKER_DENOM),
            None,
        )
        .expect_err("an error should occur when trying to change the bid type");
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    explanation,
                    "bid with id [bid_id_2] cannot change bid type from [coin_trade] to [marker_trade]",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_trade("bid_id", "some other denom"),
            None,
        )
        .expect_err(
            "an error should occur when attempting to change to a new marker that does not exist",
        );
        assert!(
            matches!(err, ContractError::Std(_)),
            "an std error should occur when attempting to target a marker that does not exist, but got: {:?}",
            err,
        );
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(45332, "quote")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all::<String>(&[]),
            )),
        )
        .expect_err("an error should occur when an invalid BidOrder is produced");
        assert_validation_error_message(err, "BidOrder [bid_id] specified RequiredAttributes, but the value included no attributes to check");
    }

    #[test]
    fn test_valid_marker_share_sale_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        // Create a new contract-owned marker to ensure the creation functions see it
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(10, "quote")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 15),
            None,
        )
        .expect("expected the bid to be created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::all(&["my.pb", "attributes.pb"]),
        );
        let response = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(444, "quote")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 25),
            Some(descriptor.clone()),
        )
        .expect("the bid update should be successful");
        let bid_order = assert_valid_response(
            &deps,
            &response,
            RequestType::MarkerShareSale,
            &coins(10, "quote"),
            Some(descriptor),
        );
        let collateral = bid_order.collateral.unwrap_marker_share_sale();
        assert_eq!(
            coins(444, "quote"),
            collateral.quote,
            "the correct quote should be set on the bid",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be set on the bid",
        );
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be set on the bid",
        );
        assert_eq!(
            25,
            collateral.share_count.u128(),
            "the correct share count should be set on the bid",
        );
    }

    #[test]
    fn test_invalid_marker_share_sale_update_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        // Set up a valid marker, alongside some bad markers for testing
        deps.querier.with_markers(vec![
            MockMarker::new_marker(),
            MockMarker {
                denom: "nocoin".to_string(),
                coins: vec![],
                ..MockMarker::default()
            }
            .to_marker(),
            MockMarker {
                denom: "fewshares".to_string(),
                coins: coins(1, "fewshares"),
                ..MockMarker::default()
            }
            .to_marker(),
        ]);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 10),
            None,
        )
        .expect("expected the bid to be created");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_share_sale("bid_id", "", 10),
            None,
        )
        .expect_err("an error should occur when the bid attempts to use an empty marker denom");
        assert_missing_field_error(err, "marker_denom");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 0),
            None,
        )
        .expect_err("an error should occur when the share count is zero");
        assert_validation_error_message(
            err,
            "share count must be at least one for a marker share sale",
        );
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 10),
            None,
        )
        .expect_err("an error should occur when no quote funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when no quote funds are provided, but got: {:?}",
            err,
        );
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_share_sale("bid_id", "some random marker", 10),
            None,
        )
        .expect_err(
            "an error should occur when attempting to change to a new marker that does not exist",
        );
        assert!(
            matches!(err, ContractError::Std(_)),
            "an std error should occur when attempting to target a marker that does not exist, buut got: {:?}",
            err,
        );
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_share_sale("bid_id", "nocoin", 10),
            None,
        ).expect_err("an error should occur when updating to a new marker that does not hold any of its own denom");
        assert!(
            matches!(err, ContractError::InvalidMarker { .. }),
            "an invalid marker error should occur when updating to a new marker that does not hold its own coin, but got: {:?}",
            err,
        );
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_share_sale("bid_id", "fewshares", 2),
            None,
        )
        .expect_err(
            "an error should occur when sending in a share sale bid that specifies too few shares",
        );
        assert_validation_error_message(
            err,
            "share count [2] must be less than or equal to remaining [fewshares] shares available [1]",
        );
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_coin_trade("bid_id_2", &coins(100, "base")),
            None,
        )
        .expect("expected the second bid to be created successfully");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_marker_share_sale("bid_id_2", DEFAULT_MARKER_DENOM, 10),
            None,
        )
        .expect_err("an error should occur when trying to change the bid type");
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    explanation,
                    "bid with id [bid_id_2] cannot change bid type from [coin_trade] to [marker_share_sale]",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(10, "quote")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 10),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all::<String>(&[]),
            )),
        )
        .expect_err("an error should occur when an invalid BidOrder is produced");
        assert_validation_error_message(err, "BidOrder [bid_id] specified RequiredAttributes, but the value included no attributes to check");
    }

    #[test]
    fn test_valid_scope_trade_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(11, "quote")),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
            None,
        )
        .expect("expected the bid to be created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::all(&["my.pb", "attributes.pb"]),
        );
        let response = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(111, "quote")),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
            Some(descriptor.clone()),
        )
        .expect("the bid update should be successful");
        let bid_order = assert_valid_response(
            &deps,
            &response,
            RequestType::ScopeTrade,
            &coins(11, "quote"),
            Some(descriptor),
        );
        let collateral = bid_order.collateral.unwrap_scope_trade();
        assert_eq!(
            coins(111, "quote"),
            collateral.quote,
            "the correct quote should be set on the bid",
        );
        assert_eq!(
            DEFAULT_SCOPE_ADDR, collateral.scope_address,
            "the correct scope address should be set on the bid",
        );
    }

    #[test]
    fn test_invalid_scope_trade_update_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
            None,
        )
        .expect("the bid should be created successfully");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_scope_trade("bid_id", ""),
            None,
        )
        .expect_err("an error should occur when the scope address is not set");
        assert_missing_field_error(err, "scope_address");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
            None,
        )
        .expect_err("an error should occur when no quote funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds provided error should occur when no quote funds are provided, but got: {:?}",
            err,
        );
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_coin_trade("bid_id_2", &coins(100, "base")),
            None,
        )
        .expect("the second bid should be created successfully");
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_scope_trade("bid_id_2", DEFAULT_SCOPE_ADDR),
            None,
        )
        .expect_err("an error should occur when trying to change the bid type");
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    explanation,
                    "bid with id [bid_id_2] cannot change bid type from [coin_trade] to [scope_trade]",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(123, "quote")),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all::<String>(&[]),
            )),
        )
        .expect_err("an error should occur when an invalid BidOrder is produced");
        assert_validation_error_message(err, "BidOrder [bid_id] specified RequiredAttributes, but the value included no attributes to check");
    }

    fn assert_valid_response(
        deps: &MockOwnedDeps,
        response: &Response<ProvenanceMsg>,
        expected_bid_type: RequestType,
        expected_old_quote: &[Coin],
        expected_descriptor: Option<RequestDescriptor>,
    ) -> BidOrder {
        assert_eq!(
            2,
            response.attributes.len(),
            "the correct number of response attributes should be sent",
        );
        assert_eq!(
            "update_bid",
            single_attribute_for_key(response, "action"),
            "the correct action attribute value should be sent",
        );
        assert_eq!(
            "bid_id",
            single_attribute_for_key(response, "bid_id"),
            "the correct bid_id attribute value should be sent",
        );
        assert_eq!(
            1,
            response.messages.len(),
            "the correct number of response messages should be sent",
        );
        match &response.messages.first().unwrap().msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "bidder", to_address,
                    "the quote refund should always be sent to the bidder",
                );
                assert_eq!(
                    expected_old_quote, amount,
                    "the old quote amount should be refunded to the bidder",
                );
            }
            msg => panic!("unexpected message encountered: {:?}", msg),
        }
        let bid_order = get_bid_order_by_id(deps.as_ref().storage, "bid_id")
            .expect("the bid order should be available in storage");
        assert_eq!(
            "bidder",
            bid_order.owner.as_str(),
            "the bidder should remain the owner of the bid order",
        );
        assert_eq!(
            expected_bid_type, bid_order.bid_type,
            "the correct bid type should be set on the updated bid",
        );
        assert_eq!(
            expected_descriptor, bid_order.descriptor,
            "the bid order's descriptor should be the expected value",
        );
        let data_bid_order = if let Some(ref data) = &response.data {
            from_binary::<BidOrder>(data)
                .expect("the response data should deserialize as a bid order")
        } else {
            panic!("the response data should be set");
        };
        assert_eq!(
            bid_order, data_bid_order,
            "the updated bid order should be included in the response data",
        );
        bid_order
    }
}
