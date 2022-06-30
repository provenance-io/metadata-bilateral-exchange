use crate::storage::bid_order_storage::{delete_bid_order_by_id, get_bid_order_by_id};
use crate::storage::contract_info::get_contract_info;
use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid_collateral::BidCollateral;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, BankMsg, DepsMut, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

// cancel bid entrypoint
pub fn cancel_bid(
    deps: DepsMut<ProvenanceQuery>,
    info: MessageInfo,
    id: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // return error if id is empty
    if id.is_empty() {
        return ContractError::validation_error(&["an id must be provided when cancelling a bid"])
            .to_err();
    }

    // return error if funds sent
    if !info.funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "funds should not be provided when cancelling a bid",
        )
        .to_err();
    }
    let bid_order = get_bid_order_by_id(deps.storage, &id)?;
    // Only the owner of the bid and the admin can cancel a bid
    if info.sender != bid_order.owner && info.sender != get_contract_info(deps.storage)?.admin {
        return ContractError::unauthorized().to_err();
    }
    let coin_to_send = match &bid_order.collateral {
        BidCollateral::CoinTrade(collateral) => collateral.quote.to_owned(),
        BidCollateral::MarkerTrade(collateral) => collateral.quote.to_owned(),
        BidCollateral::MarkerShareSale(collateral) => collateral.quote.to_owned(),
        BidCollateral::ScopeTrade(collateral) => collateral.quote.to_owned(),
    };
    // Remove the bid order from storage now that it is no longer needed
    delete_bid_order_by_id(deps.storage, &id)?;
    Response::new()
        .add_message(BankMsg::Send {
            to_address: bid_order.owner.to_string(),
            amount: coin_to_send,
        })
        .add_attribute("action", "cancel_bid")
        .add_attribute("bid_id", &bid_order.id)
        .set_data(to_binary(&bid_order)?)
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::execute::cancel_bid::cancel_bid;
    use crate::execute::create_bid::create_bid;
    use crate::storage::bid_order_storage::{get_bid_order_by_id, insert_bid_order};
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::{default_instantiate, DEFAULT_ADMIN_ADDRESS};
    use crate::test::mock_marker::{MockMarker, DEFAULT_MARKER_DENOM};
    use crate::test::mock_scope::DEFAULT_SCOPE_ID;
    use crate::test::request_helpers::mock_bid_order;
    use crate::types::core::error::ContractError;
    use crate::types::request::bid_types::bid::Bid;
    use crate::types::request::bid_types::bid_collateral::BidCollateral;
    use crate::types::request::bid_types::bid_order::BidOrder;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{coin, coins, from_binary, BankMsg, Coin, CosmosMsg, Response, Storage};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::ProvenanceMsg;

    #[test]
    fn test_cancel_coin_trade_as_bidder() {
        do_coin_trade_cancel_bid("bidder");
    }

    #[test]
    fn test_cancel_coin_trade_as_admin() {
        do_coin_trade_cancel_bid(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_cancel_marker_trade_as_bidder() {
        do_marker_trade_cancel_bid("bidder");
    }

    #[test]
    fn test_cancel_marker_trade_as_admin() {
        do_marker_trade_cancel_bid(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_cancel_marker_share_sale_as_bidder() {
        do_marker_share_sale_cancel_bid("bidder");
    }

    #[test]
    fn test_cancel_marker_share_sale_as_admin() {
        do_marker_share_sale_cancel_bid(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_cancel_scope_trade_as_bidder() {
        do_scope_trade_bid_cancel("bidder");
    }

    #[test]
    fn test_cancel_scope_trade_as_admin() {
        do_scope_trade_bid_cancel(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_cancel_bid_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let err = cancel_bid(deps.as_mut(), mock_info("bidder", &[]), String::new())
            .expect_err("an error should occur when the bid id is missing");
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(
                    1,
                    messages.len(),
                    "expected there to be a single validation error message",
                );
                assert_eq!(
                    "an id must be provided when cancelling a bid",
                    messages.first().unwrap(),
                    "the correct validation message should be produced",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = cancel_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "nhash")),
            "bid_id".to_string(),
        )
        .expect_err("an error should occur when the sender adds funds");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds provided error should occur when the sender adds funds, but got: {:?}",
            err,
        );
        let err = cancel_bid(
            deps.as_mut(),
            mock_info("bidder", &[]),
            "bid_id".to_string(),
        )
        .expect_err("an error should occur when the bid order cannot be found");
        assert!(
            matches!(err, ContractError::StorageError { .. }),
            "a storage error should occur when the bid order cannot be found by the provided id, but got: {:?}",
            err,
        );
        let bid_order = mock_bid_order(BidCollateral::coin_trade(&[], &[]));
        assert_eq!(
            "bid_id", bid_order.id,
            "sanity check: expected the mock bid order id to be the correct value",
        );
        insert_bid_order(deps.as_mut().storage, &bid_order)
            .expect("the bid order should be inserted successfully");
        let err = cancel_bid(
            deps.as_mut(),
            mock_info("impostor", &[]),
            "bid_id".to_string(),
        )
        .expect_err("an error should occur when the sender is not the bidder or the admin");
        assert!(
            matches!(err, ContractError::Unauthorized),
            "an unauthorized error should be produced when the wrong sender is detected, but got: {:?}",
            err,
        );
    }

    fn do_coin_trade_cancel_bid<S: Into<String>>(sender_address: S) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_coin_trade("bid_id", &coins(100, "base")),
            None,
        )
        .expect("expected bid creation to succeed");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id")
            .expect("expected a bid order to exist");
        let response = cancel_bid(
            deps.as_mut(),
            mock_info(&sender_address.into(), &[]),
            "bid_id".to_string(),
        )
        .expect("expected the bid cancel to succeed");
        assert_cancel_bid_succeeded(deps.as_ref().storage, &response, &coins(100, "quote"));
    }

    fn do_marker_trade_cancel_bid<S: Into<String>>(sender_address: S) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(150, "quotecoin")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            None,
        )
        .expect("expected bid creation to succeed");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id")
            .expect("expected a bid order to exist");
        let response = cancel_bid(
            deps.as_mut(),
            mock_info(&sender_address.into(), &[]),
            "bid_id".to_string(),
        )
        .expect("expected the bid cancel to succeed");
        assert_cancel_bid_succeeded(deps.as_ref().storage, &response, &coins(150, "quotecoin"));
    }

    fn do_marker_share_sale_cancel_bid<S: Into<String>>(sender_address: S) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(1000, "coincoin")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 100),
            None,
        )
        .expect("expected bid creation to succeed");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id")
            .expect("expected a bid order to exist");
        let response = cancel_bid(
            deps.as_mut(),
            mock_info(&sender_address.into(), &[]),
            "bid_id".to_string(),
        )
        .expect("expected the bid cancel to succeed");
        assert_cancel_bid_succeeded(deps.as_ref().storage, &response, &coins(1000, "coincoin"));
    }

    fn do_scope_trade_bid_cancel<S: Into<String>>(sender_address: S) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &[coin(10, "bitcoin"), coin(10, "nhash")]),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ID),
            None,
        )
        .expect("expected bid creation to succeed");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id")
            .expect("expected a bid order to exist");
        let response = cancel_bid(
            deps.as_mut(),
            mock_info(&sender_address.into(), &[]),
            "bid_id".to_string(),
        )
        .expect("expected the bid cancel to succeed");
        assert_cancel_bid_succeeded(
            deps.as_ref().storage,
            &response,
            &[coin(10, "bitcoin"), coin(10, "nhash")],
        );
    }

    fn assert_cancel_bid_succeeded(
        storage: &dyn Storage,
        response: &Response<ProvenanceMsg>,
        expected_quote_funds_to_be_returned: &[Coin],
    ) {
        assert_eq!(
            2,
            response.attributes.len(),
            "expected the correct number of attributes to be emitted when cancelling a bid",
        );
        assert_eq!(
            "cancel_bid",
            single_attribute_for_key(response, "action"),
            "the correct value should be set for the cancel bid action attribute",
        );
        assert_eq!(
            "bid_id",
            single_attribute_for_key(response, "bid_id"),
            "the correct value should be set for the cancel bid bid_id attribute",
        );
        let bid_order = if let Some(ref binary) = response.data {
            from_binary::<BidOrder>(binary)
                .expect("expected the bid order to deserialize from the response data")
        } else {
            panic!("expected the response data to be populated");
        };
        get_bid_order_by_id(storage, &bid_order.id).expect_err(
            "expected the bid order to be deleted from storage after the cancel is completed",
        );
        assert_eq!(
            1,
            response.messages.len(),
            "expected the correct number of messages to be sent after a bid cancel",
        );
        match &response.messages.first().unwrap().msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "bidder", to_address,
                    "the to_address on the bank send should always be the bidder",
                );
                assert_eq!(
                    expected_quote_funds_to_be_returned, amount,
                    "the quote funds returned to the bidder was not correct",
                );
            }
            msg => panic!("unexpected message sent: {:?}", msg),
        };
    }
}
