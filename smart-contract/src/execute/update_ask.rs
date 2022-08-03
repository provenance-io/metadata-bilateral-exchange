use crate::storage::ask_order_storage::{get_ask_order_by_id, update_ask_order};
use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask::Ask;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::util::create_ask_order_utilities::{
    create_ask_order, AskCreationType, AskOrderCreationResponse,
};
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

pub fn update_ask(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    ask: Ask,
    descriptor: Option<RequestDescriptor>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let existing_ask_order = get_ask_order_by_id(deps.storage, ask.get_id())?;
    if info.sender != existing_ask_order.owner {
        return ContractError::Unauthorized.to_err();
    }
    let AskOrderCreationResponse {
        ask_order,
        messages,
        ..
    } = create_ask_order(
        &deps,
        &env,
        &info,
        ask,
        descriptor,
        AskCreationType::Update {
            existing_ask_order: Box::new(existing_ask_order),
        },
    )?;
    update_ask_order(deps.storage, &ask_order)?;
    Response::new()
        .add_attribute("action", "update_ask")
        .add_attribute("ask_id", &ask_order.id)
        .add_messages(messages)
        .set_data(to_binary(&ask_order)?)
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::execute::create_ask::create_ask;
    use crate::execute::update_ask::update_ask;
    use crate::storage::ask_order_storage::get_ask_order_by_id;
    use crate::test::cosmos_type_helpers::{single_attribute_for_key, MockOwnedDeps};
    use crate::test::error_helpers::{assert_missing_field_error, assert_validation_error_message};
    use crate::test::mock_instantiate::default_instantiate;
    use crate::test::mock_marker::{
        MockMarker, DEFAULT_MARKER_ADDRESS, DEFAULT_MARKER_DENOM, DEFAULT_MARKER_HOLDINGS,
    };
    use crate::test::mock_scope::{MockScope, DEFAULT_SCOPE_ADDR};
    use crate::types::core::error::ContractError;
    use crate::types::request::ask_types::ask::Ask;
    use crate::types::request::ask_types::ask_order::AskOrder;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use crate::types::request::share_sale_type::ShareSaleType;
    use crate::util::constants::NHASH;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary, Addr, BankMsg, CosmosMsg, Response};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::ProvenanceMsg;

    #[test]
    fn test_invalid_update_for_missing_ask() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id", &coins(100, "quote")),
            None,
        )
        .expect_err("an error should occur when the target ask does not exist");
        assert!(
            matches!(err, ContractError::StorageError { .. }),
            "a storage error should occur when an existing ask is not found by id, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_invalid_update_for_different_owner() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id", &coins(100, "quote")),
            None,
        )
        .expect("the ask should be created");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker2", &coins(150, "base")),
            Ask::new_coin_trade("ask_id", &coins(150, "quote")),
            None,
        )
        .expect_err("an error should occur when the wrong sender tries to update an ask");
        assert!(
            matches!(err, ContractError::Unauthorized),
            "the ask owner must update an ask",
        );
    }

    #[test]
    fn test_valid_coin_trade_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id", &coins(100, "quote")),
            None,
        )
        .expect("expected the ask to be created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["this.pb", "that.pio"]),
        );
        let response = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(150, "base")),
            Ask::new_coin_trade("ask_id", &coins(150, "quote")),
            Some(descriptor.clone()),
        )
        .expect("the ask update should be successful");
        let ask_order =
            assert_valid_response(&deps, &response, RequestType::CoinTrade, Some(descriptor));
        assert_eq!(
            1,
            response.messages.len(),
            "expected the correct number of messages to be sent"
        );
        match &response.messages.first().unwrap().msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker",
                    to_address.as_str(),
                    "the asker should receive the base refund",
                );
                assert_eq!(
                    &coins(100, "base"),
                    amount,
                    "the original ask's base should be refunded to the asker",
                );
            }
            msg => panic!("unexpected message produced: {:?}", msg),
        };
        let collateral = ask_order.collateral.unwrap_coin_trade();
        assert_eq!(
            coins(150, "base"),
            collateral.base,
            "the correct base should be set in the updated ask collateral",
        );
        assert_eq!(
            coins(150, "quote"),
            collateral.quote,
            "the correct quote should be set in the updated ask collateral",
        );
    }

    #[test]
    fn test_invalid_coin_trade_update_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id", &coins(100, "quote")),
            None,
        )
        .expect("the ask should be created successfully");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_coin_trade("ask_id", &coins(100, "quote")),
            None,
        )
        .expect_err("an error should occur when no base funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when no base funds are provided, but got: {:?}",
            err,
        );
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id", &[]),
            None,
        )
        .expect_err("an error should occur when no quote is provided");
        assert_missing_field_error(err, "quote");
        deps.querier
            .with_scope(MockScope::new_with_owner(MOCK_CONTRACT_ADDR));
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id_2", DEFAULT_SCOPE_ADDR, &coins(100, "quote")),
            None,
        )
        .expect("the second ask should be created successfully");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id_2", &coins(100, "quote")),
            None,
        )
        .expect_err("an error should occur when trying to change the ask type");
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    "ask with id [ask_id_2] cannot change ask type from [scope_trade] to [coin_trade]",
                    explanation,
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id", &coins(100, "quote")),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::none::<String>(&[]),
            )),
        )
        .expect_err("an error should occur when an invalid AskOrder is produced");
        assert_validation_error_message(err, "AskOrder [ask_id] specified RequiredAttributes, but the value included no attributes to check");
    }

    #[test]
    fn test_valid_marker_trade_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(100, "quote")),
            None,
        )
        .expect("expected the ask to be created");
        let ask_order_before_update = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("the ask order should be available by id after being created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["this.pb", "that.pio"]),
        );
        // Update the marker to simulate a successful ask creation and marker control change to the
        // contract
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        let response = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(250, "quote")),
            Some(descriptor.clone()),
        )
        .expect("the ask update should be successful");
        let ask_order =
            assert_valid_response(&deps, &response, RequestType::MarkerTrade, Some(descriptor));
        assert!(
            response.messages.is_empty(),
            "no messages should be sent for a marker trade update",
        );
        let collateral = ask_order.collateral.unwrap_marker_trade();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_HOLDINGS,
            collateral.share_count.u128(),
            "the correct marker share count should be included in the updated collateral",
        );
        assert_eq!(
            coins(250, "quote"),
            collateral.quote_per_share,
            "the updated quote per share should be included in the updated collateral",
        );
        assert_eq!(
            ask_order_before_update.collateral.unwrap_marker_trade().removed_permissions,
            collateral.removed_permissions,
            "the original permissions revoked in the marker trade should be maintained in the update",
        );
    }

    #[test]
    fn test_valid_marker_trade_update_to_share_sale() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(100, "quote")),
            None,
        )
        .expect("expected the ask to be created");
        let ask_order_before_update = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("the ask order should be available by id after being created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["this.pb", "that.pio"]),
        );
        // Update the marker to simulate a successful ask creation and marker control change to the
        // contract
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        let response = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                50,
                &coins(250, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            Some(descriptor.clone()),
        )
        .expect("the ask update should be successful");
        let ask_order = assert_valid_response(
            &deps,
            &response,
            RequestType::MarkerShareSale,
            Some(descriptor),
        );
        assert!(
            response.messages.is_empty(),
            "no messages should be sent for a marker trade to share sale update",
        );
        let collateral = ask_order.collateral.unwrap_marker_share_sale();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be included in the updated collateral",
        );
        assert_eq!(
            50,
            collateral.total_shares_in_sale.u128(),
            "the correct total shares in sale should be included in the updated collateral",
        );
        assert_eq!(
            50,
            collateral.remaining_shares_in_sale.u128(),
            "the correct remaining shares in sale should be included in the updated collateral",
        );
        assert_eq!(
            coins(250, "quote"),
            collateral.quote_per_share,
            "the updated quote per share should be included in the updated collateral",
        );
        assert_eq!(
            ask_order_before_update.collateral.unwrap_marker_trade().removed_permissions,
            collateral.removed_permissions,
            "the original permissions revoked in the marker trade should be maintained in the update",
        );
        assert_eq!(
            ShareSaleType::SingleTransaction,
            collateral.sale_type,
            "the sale type value should be set to the correct value in the updated collateral",
        );
    }

    #[test]
    fn test_invalid_marker_trade_update_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(100, "quote")),
            None,
        )
        .expect("the ask should be created successfully");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(100, "quote")),
            None,
        )
        .expect_err("an error should occur when base funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when base funds are provided, but got: {:?}",
            err,
        );
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", "some other denom", &coins(100, "quote")),
            None,
        )
        .expect_err("an error should occur when a marker denom is referenced that does not exist");
        assert!(
            matches!(err, ContractError::Std(_)),
            "an std error should occur when the target marker does not exist, but got: {:?}",
            err,
        );
        deps.querier.with_markers(vec![MockMarker {
            permissions: vec![],
            ..MockMarker::default()
        }
        .to_marker()]);
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(100, "quote")),
            None,
        )
        .expect_err("an error should occur if the target marker is invalid");
        assert!(
            matches!(err, ContractError::InvalidMarker { .. }),
            "an invalid marker error should occur when the marker does not have permissions, but got: {:?}",
            err,
        );
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id_2", &coins(100, "quote")),
            None,
        )
        .expect("the second ask should be created successfully");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id_2", DEFAULT_MARKER_DENOM, &coins(100, "quote")),
            None,
        )
        .expect_err(
            "an error should occur when trying to change the ask type to a non-marker type",
        );
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    "ask with id [ask_id_2] cannot change ask type from [coin_trade] to [marker_trade]",
                    explanation,
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(100, "quote")),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all::<String>(&[]),
            )),
        )
        .expect_err("an error should occur when an invalid AskOrder is produced");
        assert_validation_error_message(err, "AskOrder [ask_id] specified RequiredAttributes, but the value included no attributes to check");
    }

    #[test]
    fn test_multiple_marker_share_sales_cannot_update_to_become_a_marker_trade() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                10,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("expected the first ask to be created");
        // Update the marker to simulate a successful ask creation and marker control change to the
        // contract
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id_2",
                DEFAULT_MARKER_DENOM,
                30,
                &coins(300, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("expected the second ask to be created");
        let mut test_update_ask = |ask_id: &str| {
            let err = update_ask(
                deps.as_mut(),
                mock_env(),
                mock_info("asker", &[]),
                Ask::new_marker_trade(
                    ask_id,
                    DEFAULT_MARKER_DENOM,
                    &coins(450, NHASH),
                ),
                None,
            ).expect_err("a marker share sale should not be able to update to a marker trade if multiple marker share sales exist");
            match err {
                ContractError::InvalidRequest { message } => {
                    assert!(
                        message.contains("marker trade asks cannot exist alongside alternate asks for the same marker"),
                        "unexpected invalid request message content: {}",
                        message,
                    );
                }
                e => panic!("unexpected error: {:?}", e),
            };
        };
        // Verify that updating either ask will produce the same issue
        test_update_ask("ask_id");
        test_update_ask("ask_id_2");
    }

    #[test]
    fn test_valid_marker_share_sale_single_tx_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                10,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("expected the ask to be created");
        let ask_order_before_update = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("the ask order should be available by id after being created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["this.pb", "that.pio"]),
        );
        // Update the marker to simulate a successful ask creation and marker control change to the
        // contract
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        let response = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                25,
                &coins(500, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            Some(descriptor.clone()),
        )
        .expect("the ask update should be successful");
        let ask_order = assert_valid_response(
            &deps,
            &response,
            RequestType::MarkerShareSale,
            Some(descriptor),
        );
        assert!(
            response.messages.is_empty(),
            "no messages should be sent for a marker share sale update",
        );
        let collateral = ask_order.collateral.unwrap_marker_share_sale();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be included in the updated collateral",
        );
        assert_eq!(
            25,
            collateral.total_shares_in_sale.u128(),
            "the total shares in sale should be updated to the new value in the updated collateral",
        );
        assert_eq!(
            25,
            collateral.remaining_shares_in_sale.u128(),
            "the remaining shares in sale should be updated to the new value in the updated collateral",
        );
        assert_eq!(
            coins(500, "quote"),
            collateral.quote_per_share,
            "the updated quote per share should be included in the updated collateral",
        );
        assert_eq!(
            ask_order_before_update.collateral.unwrap_marker_share_sale().removed_permissions,
            collateral.removed_permissions,
            "the original permissions revoked in the marker trade should be maintained in the update",
        );
        assert_eq!(
            ShareSaleType::SingleTransaction,
            collateral.sale_type,
            "the sale type value should stay the same in the updated collateral",
        );
    }

    #[test]
    fn test_valid_marker_share_sale_single_tx_update_to_marker_trade() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                10,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("expected the ask to be created");
        let ask_order_before_update = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("the ask order should be available by id after being created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["this.pb", "that.pio"]),
        );
        // Update the marker to simulate a successful ask creation and marker control change to the
        // contract
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        let response = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(500, "quote")),
            Some(descriptor.clone()),
        )
        .expect("the ask update should be successful");
        let ask_order =
            assert_valid_response(&deps, &response, RequestType::MarkerTrade, Some(descriptor));
        assert!(
            response.messages.is_empty(),
            "no messages should be sent for a marker share sale to marker trade update",
        );
        let collateral = ask_order.collateral.unwrap_marker_trade();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_HOLDINGS,
            collateral.share_count.u128(),
            "the correct marker share count should be included in the updated collateral",
        );
        assert_eq!(
            coins(500, "quote"),
            collateral.quote_per_share,
            "the updated quote per share should be included in the updated collateral",
        );
        assert_eq!(
            ask_order_before_update.collateral.unwrap_marker_share_sale().removed_permissions,
            collateral.removed_permissions,
            "the original permissions revoked in the marker share sale should be maintained in the update",
        );
    }

    #[test]
    fn test_valid_marker_share_sale_multiple_tx_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                50,
                &coins(100, "quote"),
                ShareSaleType::MultipleTransactions,
            ),
            None,
        )
        .expect("expected the ask to be created");
        let ask_order_before_update = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("the ask order should be available by id after being created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["this.pb", "that.pio"]),
        );
        // Update the marker to simulate a successful ask creation and marker control change to the
        // contract
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        let response = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                70,
                &coins(500, "quote"),
                ShareSaleType::MultipleTransactions,
            ),
            Some(descriptor.clone()),
        )
        .expect("the ask update should be successful");
        let ask_order = assert_valid_response(
            &deps,
            &response,
            RequestType::MarkerShareSale,
            Some(descriptor),
        );
        assert!(
            response.messages.is_empty(),
            "no messages should be sent for a marker share sale update",
        );
        let collateral = ask_order.collateral.unwrap_marker_share_sale();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be included in the updated collateral",
        );
        assert_eq!(
            70,
            collateral.total_shares_in_sale.u128(),
            "the total shares in sale should be updated to the new value in the updated collateral",
        );
        assert_eq!(
            70,
            collateral.remaining_shares_in_sale.u128(),
            "the remaining shares in sale should be updated to the new value in the updated collateral",
        );
        assert_eq!(
            coins(500, "quote"),
            collateral.quote_per_share,
            "the updated quote per share should be included in the updated collateral",
        );
        assert_eq!(
            ask_order_before_update.collateral.unwrap_marker_share_sale().removed_permissions,
            collateral.removed_permissions,
            "the original permissions revoked in the marker trade should be maintained in the update",
        );
        assert_eq!(
            ShareSaleType::MultipleTransactions,
            collateral.sale_type,
            "the sale type value should stay the same in the updated collateral",
        );
    }

    #[test]
    fn test_valid_marker_share_sale_multiple_tx_update_to_marker_trade() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                50,
                &coins(100, "quote"),
                ShareSaleType::MultipleTransactions,
            ),
            None,
        )
        .expect("expected the ask to be created");
        let ask_order_before_update = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("the ask order should be available by id after being created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["this.pb", "that.pio"]),
        );
        // Update the marker to simulate a successful ask creation and marker control change to the
        // contract
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        let response = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(500, "quote")),
            Some(descriptor.clone()),
        )
        .expect("the ask update should be successful");
        let ask_order =
            assert_valid_response(&deps, &response, RequestType::MarkerTrade, Some(descriptor));
        assert!(
            response.messages.is_empty(),
            "no messages should be sent for a marker share sale to marker trade update",
        );
        let collateral = ask_order.collateral.unwrap_marker_trade();
        assert_eq!(
            DEFAULT_MARKER_ADDRESS,
            collateral.marker_address.as_str(),
            "the correct marker address should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_DENOM, collateral.marker_denom,
            "the correct marker denom should be included in the updated collateral",
        );
        assert_eq!(
            DEFAULT_MARKER_HOLDINGS,
            collateral.share_count.u128(),
            "the correct marker share count should be included in the updated collateral",
        );
        assert_eq!(
            coins(500, "quote"),
            collateral.quote_per_share,
            "the updated quote per share should be included in the updated collateral",
        );
        assert_eq!(
            ask_order_before_update.collateral.unwrap_marker_share_sale().removed_permissions,
            collateral.removed_permissions,
            "the original permissions revoked in the marker share sale should be maintained in the update",
        );
    }

    #[test]
    fn test_marker_share_sale_update_with_multiple_share_sales_considers_correct_amount() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                10,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("expected the first ask to be created");
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id_2",
                DEFAULT_MARKER_DENOM,
                DEFAULT_MARKER_HOLDINGS - 10,
                &coins(250, "quote"),
                ShareSaleType::MultipleTransactions,
            ),
            None,
        )
        .expect("expected the second ask to be created");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                11,
                &coins(2000, "quote"),
                ShareSaleType::MultipleTransactions,
            ),
            None,
        )
        .expect_err("an error should occur when trying to sell more shares than are available");
        match err {
            ContractError::InvalidMarker { message } => {
                assert_eq!(
                    format!(
                        "expected marker [{}] to have enough shares to sell. it had [{}], which is less than proposed sale amount [{}] + shares already listed for sale [{}] = [{}]",
                        DEFAULT_MARKER_DENOM,
                        DEFAULT_MARKER_HOLDINGS,
                        11,
                        DEFAULT_MARKER_HOLDINGS - 10,
                        DEFAULT_MARKER_HOLDINGS + 1,
                    ),
                    message,
                    "unexpected message content for invalid marker error",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id_2",
                DEFAULT_MARKER_DENOM,
                DEFAULT_MARKER_HOLDINGS - 9,
                &coins(12345, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect_err("an error should occur when trying to sell more shares than are available");
        match err {
            ContractError::InvalidMarker { message } => {
                assert_eq!(
                    format!(
                        "expected marker [{}] to have enough shares to sell. it had [{}], which is less than proposed sale amount [{}] + shares already listed for sale [{}] = [{}]",
                        DEFAULT_MARKER_DENOM,
                        DEFAULT_MARKER_HOLDINGS,
                        DEFAULT_MARKER_HOLDINGS - 9,
                        10,
                        DEFAULT_MARKER_HOLDINGS + 1,
                    ),
                    message,
                    "unexpected message content for invalid marker error",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id_2",
                DEFAULT_MARKER_DENOM,
                5,
                &coins(12345, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("the second ask should be updated to be a much smaller amount of shares sold");
        update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                DEFAULT_MARKER_HOLDINGS - 5,
                &coins(11, "quote"),
                ShareSaleType::MultipleTransactions,
            ),
            None,
        )
        .expect("the first ask should be updated to have a much larger amount");
        // Now, both asks have been updated and their totals encompass all the markers coin.  Any
        // new ask should be rejected
        let err = create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id_3",
                DEFAULT_MARKER_DENOM,
                1,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect_err("an error should occur when trying to sell more shares than are available");
        match err {
            ContractError::InvalidMarker { message } => {
                assert_eq!(
                    format!(
                        "expected marker [{}] to have enough shares to sell. it had [{}], which is less than proposed sale amount [{}] + shares already listed for sale [{}] = [{}]",
                        DEFAULT_MARKER_DENOM,
                        DEFAULT_MARKER_HOLDINGS,
                        1,
                        DEFAULT_MARKER_HOLDINGS,
                        DEFAULT_MARKER_HOLDINGS + 1,
                    ),
                    message,
                    "unexpected message content for invalid marker error",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
    }

    #[test]
    fn test_invalid_marker_share_sale_update_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier.with_markers(vec![
            MockMarker::new_owned_marker("asker"),
            MockMarker {
                denom: "othervalidmarker".to_string(),
                coins: coins(DEFAULT_MARKER_HOLDINGS, "othervalidmarker"),
                ..MockMarker::default()
            }
            .to_marker(),
        ]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                10,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("the ask should be created successfully");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                10,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect_err("an error should occur when base funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when base funds are provided, but got: {:?}",
            err,
        );
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                "some rando marker",
                10,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect_err("an error should occur when a marker is targeted that does not exist");
        assert!(
            matches!(err, ContractError::Std(_)),
            "an std error should occur when the target marker is not found, but got: {:?}",
            err,
        );
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale("ask_id", DEFAULT_MARKER_DENOM, DEFAULT_MARKER_HOLDINGS + 1, &coins(100, "quote"), ShareSaleType::SingleTransaction),
            None,
        ).expect_err("an error should occur when the update attempts to sell more shares than the marker owns");
        match err {
            ContractError::InvalidMarker { message } => {
                assert_eq!(
                    message,
                    format!(
                        "expected marker [{}] to have enough shares to sell. it had [{}], which is less than proposed sale amount [{}] + shares already listed for sale [{}] = [{}]",
                        DEFAULT_MARKER_DENOM,
                        DEFAULT_MARKER_HOLDINGS,
                        DEFAULT_MARKER_HOLDINGS + 1,
                        0,
                        DEFAULT_MARKER_HOLDINGS + 1,
                    ),
                    "unexpected message from invalid marker error when too many shares were attempted for sale",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id_2", &coins(100, "quote")),
            None,
        )
        .expect("expected thee second ask to be created");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id_2",
                DEFAULT_MARKER_DENOM,
                1,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect_err(
            "an error should occur when trying to change the ask type to a non-marker type",
        );
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    "ask with id [ask_id_2] cannot change ask type from [coin_trade] to [marker_share_sale]",
                    explanation,
                    "unexpected invalid update error explanation",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                "othervalidmarker",
                10,
                &coins(100, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect_err("an error should occur when trying to change the marker denom");
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    format!("marker share sale with id [ask_id] cannot change marker denom with an update. current denom [{}], proposed new denom [othervalidmarker]", DEFAULT_MARKER_DENOM),
                    explanation,
                    "unexpected invalid update error explanation",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                99,
                &coins(150, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::any::<String>(&[]),
            )),
        )
        .expect_err("an error should occur when an invalid AskOrder is produced");
        assert_validation_error_message(err, "AskOrder [ask_id] specified RequiredAttributes, but the value included no attributes to check");
    }

    #[test]
    fn test_valid_scope_trade_update() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_scope(MockScope::new_with_owner(MOCK_CONTRACT_ADDR));
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id", DEFAULT_SCOPE_ADDR, &coins(99, "quote")),
            None,
        )
        .expect("expected the ask to be created");
        let descriptor = RequestDescriptor::new_populated_attributes(
            "description",
            AttributeRequirement::any(&["this.pb", "that.pio"]),
        );
        let response = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id", DEFAULT_SCOPE_ADDR, &coins(12345, "quote")),
            Some(descriptor.clone()),
        )
        .expect("the ask update should be successful");
        let ask_order =
            assert_valid_response(&deps, &response, RequestType::ScopeTrade, Some(descriptor));
        assert!(
            response.messages.is_empty(),
            "no messages should be sent during a scope trade update",
        );
        let collateral = ask_order.collateral.unwrap_scope_trade();
        assert_eq!(
            coins(12345, "quote"),
            collateral.quote,
            "the correct quote should be set in the updated ask collateral",
        );
        assert_eq!(
            DEFAULT_SCOPE_ADDR, collateral.scope_address,
            "the scope address should be unchanged by the update",
        );
    }

    #[test]
    fn test_invalid_scope_trade_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_scope(MockScope::new_with_owner(MOCK_CONTRACT_ADDR));
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id", DEFAULT_SCOPE_ADDR, &coins(100, "quote")),
            None,
        )
        .expect("expected the ask to be created");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_scope_trade("ask_id", DEFAULT_SCOPE_ADDR, &coins(1000, "quote")),
            None,
        )
        .expect_err("an error should occur when base funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should occur when base funds are provided, but got: {:?}",
            err,
        );
        deps.querier.with_scope(
            MockScope {
                value_owner_address: Addr::unchecked("some other person"),
                ..MockScope::new_mock_scope_with_owner(MOCK_CONTRACT_ADDR)
            }
            .to_scope(),
        );
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id", DEFAULT_SCOPE_ADDR, &coins(111, "quote")),
            None,
        )
        .expect_err("an error should occur when the scope has invalid owners");
        assert!(
            matches!(err, ContractError::InvalidScopeOwner { .. }),
            "an invalid scope owners error should occur, but got: {:?}",
            err,
        );
        deps.querier
            .with_scope(MockScope::new_with_owner(MOCK_CONTRACT_ADDR));
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id_2", &coins(100, "quote")),
            None,
        )
        .expect("the second ask should be created");
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id_2", DEFAULT_SCOPE_ADDR, &coins(100, "quote")),
            None,
        )
        .expect_err("an error should occur when trying to change the ask type");
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    "ask with id [ask_id_2] cannot change ask type from [coin_trade] to [scope_trade]",
                    explanation,
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        deps.querier.with_scope(
            MockScope {
                scope_id: "some other scope address".to_string(),
                ..MockScope::new_mock_scope_with_owner(MOCK_CONTRACT_ADDR)
            }
            .to_scope(),
        );
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id", "some other scope address", &coins(100, "quote")),
            None,
        )
        .expect_err("an error should occur when the update tries to change the scope address");
        match err {
            ContractError::InvalidUpdate { explanation } => {
                assert_eq!(
                    format!(
                        "scope trade with id [ask_id] cannot change scope address with an update. current address [{}], proposed new address [some other scope address]",
                        DEFAULT_SCOPE_ADDR,
                    ),
                    explanation,
                    "unexpected invalid update explanation",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        deps.querier
            .with_scope(MockScope::new_with_owner(MOCK_CONTRACT_ADDR));
        let err = update_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id", DEFAULT_SCOPE_ADDR, &coins(12345, "quote")),
            Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::any::<String>(&[]),
            )),
        )
        .expect_err("an error should occur when an invalid AskOrder is produced");
        assert_validation_error_message(err, "AskOrder [ask_id] specified RequiredAttributes, but the value included no attributes to check");
    }

    fn assert_valid_response(
        deps: &MockOwnedDeps,
        response: &Response<ProvenanceMsg>,
        expected_ask_type: RequestType,
        expected_descriptor: Option<RequestDescriptor>,
    ) -> AskOrder {
        assert_eq!(
            2,
            response.attributes.len(),
            "the correct number of response attributes should be sent",
        );
        assert_eq!(
            "update_ask",
            single_attribute_for_key(response, "action"),
            "the correct action attribute value should be sent",
        );
        assert_eq!(
            "ask_id",
            single_attribute_for_key(response, "ask_id"),
            "the correct ask_id attribute value should be sent",
        );
        let ask_order = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("the ask order should be available in storage");
        assert_eq!(
            "asker",
            ask_order.owner.as_str(),
            "the asker should remain the owner of the bid order",
        );
        assert_eq!(
            expected_ask_type, ask_order.ask_type,
            "the correct ask type should be set on the updated ask",
        );
        assert_eq!(
            expected_descriptor, ask_order.descriptor,
            "the ask order's descriptor should be the expected value",
        );
        let data_ask_order = if let Some(ref data) = &response.data {
            from_binary::<AskOrder>(data)
                .expect("the response data should deserialize as an ask order")
        } else {
            panic!("the response data should be set");
        };
        assert_eq!(
            ask_order, data_ask_order,
            "the updated ask order should be included in the response data",
        );
        ask_order
    }
}
