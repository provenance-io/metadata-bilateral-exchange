use crate::storage::ask_order_storage::{
    delete_ask_order_by_id, get_ask_order_by_id, get_ask_orders_by_collateral_id, update_ask_order,
};
use crate::storage::bid_order_storage::{
    delete_bid_order_by_id, get_bid_order_by_id, update_bid_order,
};
use crate::storage::contract_info::get_contract_info;
use crate::types::core::error::ContractError;
use crate::types::request::admin_match_options::{AdminMatchOptions, OverrideQuoteSource};
use crate::types::request::ask_types::ask_collateral::{
    AskCollateral, CoinTradeAskCollateral, MarkerShareSaleAskCollateral, MarkerTradeAskCollateral,
    ScopeTradeAskCollateral,
};
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::types::request::bid_types::bid_collateral::{
    BidCollateral, CoinTradeBidCollateral, MarkerShareSaleBidCollateral, MarkerTradeBidCollateral,
    ScopeTradeBidCollateral,
};
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::share_sale_type::ShareSaleType;
use crate::util::coin_utilities::{
    calculate_marker_share_sale_bid_totals, multiply_coins_by_amount, MSSBidTotalsCalc,
};
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::{release_marker_from_contract, replace_scope_owner};
use crate::validation::execute_match_validation::validate_match;
use crate::validation::validation_handler::ValidationHandler;
use cosmwasm_std::{BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128};
use provwasm_std::{
    withdraw_coins, write_scope, ProvenanceMsg, ProvenanceQuerier, ProvenanceQuery,
};

// match and execute an ask and bid order
pub fn execute_match(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    ask_id: String,
    bid_id: String,
    admin_match_options: Option<AdminMatchOptions>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let handler = ValidationHandler::new();
    if ask_id.is_empty() {
        handler.push("ask id must not be empty");
    }
    if bid_id.is_empty() {
        handler.push("bid id must not be empty");
    }
    let contract_info = get_contract_info(deps.storage)?;
    // return error if either ids are badly formed
    handler.handle()?;
    // return error if funds sent
    if !info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "funds should not be provided during match execution".to_string(),
        }
        .to_err();
    }
    let ask_order = get_ask_order_by_id(deps.storage, &ask_id)?;
    let bid_order = get_bid_order_by_id(deps.storage, &bid_id)?;
    // only the admin or the asker may execute matches
    if info.sender != ask_order.owner && info.sender != contract_info.admin {
        return ContractError::Unauthorized.to_err();
    }
    // Ensure match is viable before trying to actually execute the match
    validate_match(&deps.as_ref(), &ask_order, &bid_order, &admin_match_options)?;
    let execute_result = match &ask_order.collateral {
        AskCollateral::CoinTrade(collateral) => execute_coin_trade(
            deps,
            &ask_order,
            &bid_order,
            collateral,
            bid_order.collateral.get_coin_trade()?,
        )?,
        AskCollateral::MarkerTrade(collateral) => execute_marker_trade(
            deps,
            &env,
            &ask_order,
            &bid_order,
            collateral,
            bid_order.collateral.get_marker_trade()?,
        )?,
        AskCollateral::MarkerShareSale(collateral) => execute_marker_share_sale(
            deps,
            &env,
            &ask_order,
            &bid_order,
            collateral,
            bid_order.collateral.get_marker_share_sale()?,
            admin_match_options.and_then(|opt| match opt {
                AdminMatchOptions::MarkerShareSale {
                    override_quote_source,
                } => override_quote_source,
                _ => None,
            }),
        )?,
        AskCollateral::ScopeTrade(collateral) => execute_scope_trade(
            deps,
            &env,
            &ask_order,
            &bid_order,
            collateral,
            bid_order.collateral.get_scope_trade()?,
        )?,
    };
    Response::new()
        .add_messages(execute_result.messages)
        .add_attribute("action", "execute")
        .add_attribute("ask_id", &ask_order.id)
        .add_attribute("bid_id", &bid_order.id)
        .add_attribute("ask_deleted", execute_result.ask_deleted.to_string())
        .add_attribute("bid_deleted", execute_result.bid_deleted.to_string())
        .add_attribute(
            "collateral_released",
            execute_result.collateral_released.to_string(),
        )
        .to_ok()
}

struct ExecuteResults {
    pub messages: Vec<CosmosMsg<ProvenanceMsg>>,
    pub ask_deleted: bool,
    pub bid_deleted: bool,
    pub collateral_released: bool,
}

fn execute_coin_trade(
    deps: DepsMut<ProvenanceQuery>,
    ask_order: &AskOrder,
    bid_order: &BidOrder,
    ask_collateral: &CoinTradeAskCollateral,
    bid_collateral: &CoinTradeBidCollateral,
) -> Result<ExecuteResults, ContractError> {
    // Remove ask and bid - this transaction has concluded
    delete_ask_order_by_id(deps.storage, &ask_order.id)?;
    delete_bid_order_by_id(deps.storage, &bid_order.id)?;
    ExecuteResults {
        messages: vec![
            CosmosMsg::Bank(BankMsg::Send {
                to_address: ask_order.owner.to_string(),
                amount: bid_collateral.quote.to_owned(),
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: bid_order.owner.to_string(),
                amount: ask_collateral.base.to_owned(),
            }),
        ],
        ask_deleted: true,
        bid_deleted: true,
        collateral_released: true,
    }
    .to_ok()
}

fn execute_marker_trade(
    deps: DepsMut<ProvenanceQuery>,
    env: &Env,
    ask_order: &AskOrder,
    bid_order: &BidOrder,
    ask_collateral: &MarkerTradeAskCollateral,
    bid_collateral: &MarkerTradeBidCollateral,
) -> Result<ExecuteResults, ContractError> {
    let mut messages = vec![];
    // Only transfer marker shares to the bidder if the bidder explicitly requested it with a Some(true)
    // value for their withdraw_shares_after_match param during BidOrder creation
    if bid_collateral.withdraw_shares_after_match.unwrap_or(false) {
        messages.push(withdraw_coins(
            &ask_collateral.marker_denom,
            // Withdraw all remaining shares in the marker to the bidder's account upon marker
            // trade completion.  This will cause them to immediately show up in the bidder's wallet.
            ask_collateral.share_count.u128(),
            &ask_collateral.marker_denom,
            bid_order.owner.to_owned(),
        )?);
    }
    if let Some(asker_permissions) = ask_collateral
        .removed_permissions
        .iter()
        .find(|perm| perm.address == ask_order.owner)
    {
        // Now that the match has been made, grant all permissions on the marker to the bidder that
        // the asker once had.  The validation code has already ensured that the asker was an admin
        // of the marker, so the bidder at very least has the permission on the marker to grant
        // themselves any remaining permissions they desire.
        let mut bidder_permissions = asker_permissions.to_owned();
        bidder_permissions.address = bid_order.owner.to_owned();
        messages.append(&mut release_marker_from_contract(
            &ask_collateral.marker_denom,
            &env.contract.address,
            &[bidder_permissions],
        )?);
    } else {
        return ContractError::ValidationError {
            messages: vec![
                "failed to find access permissions in the revoked permissions for the asker"
                    .to_string(),
            ],
        }
        .to_err();
    }
    // Send the entirety of the quote to the asker. They have just effectively sold their
    // marker to the bidder.
    messages.push(CosmosMsg::Bank(BankMsg::Send {
        to_address: ask_order.owner.to_string(),
        amount: bid_collateral.quote.to_owned(),
    }));
    // Remove ask and bid - this transaction has concluded
    delete_ask_order_by_id(deps.storage, &ask_order.id)?;
    delete_bid_order_by_id(deps.storage, &bid_order.id)?;
    ExecuteResults {
        messages,
        ask_deleted: true,
        bid_deleted: true,
        collateral_released: true,
    }
    .to_ok()
}

fn execute_marker_share_sale(
    deps: DepsMut<ProvenanceQuery>,
    env: &Env,
    ask_order: &AskOrder,
    bid_order: &BidOrder,
    ask_collateral: &MarkerShareSaleAskCollateral,
    bid_collateral: &MarkerShareSaleBidCollateral,
    override_quote_source: Option<OverrideQuoteSource>,
) -> Result<ExecuteResults, ContractError> {
    let bid_overage_shares = bid_collateral
        .share_count
        .checked_sub(ask_collateral.remaining_shares_in_sale)
        .unwrap_or(Uint128::zero())
        .u128();
    let shares_purchased = bid_collateral.share_count.u128() - bid_overage_shares;
    let quote_paid = if let Some(OverrideQuoteSource::Bid) = override_quote_source {
        multiply_coins_by_amount(&bid_collateral.get_quote_per_share(), shares_purchased)
    } else {
        multiply_coins_by_amount(&ask_collateral.quote_per_share, shares_purchased)
    };
    // Asker gets the quote that the bidder provided from escrow
    // Bidder gets their X marker coins withdrawn to them from the contract-controlled marker
    let mut messages = vec![
        CosmosMsg::Bank(BankMsg::Send {
            to_address: ask_order.owner.to_string(),
            amount: quote_paid.to_owned(),
        }),
        withdraw_coins(
            &ask_collateral.marker_denom,
            shares_purchased,
            &ask_collateral.marker_denom,
            bid_order.owner.to_owned(),
        )?,
    ];
    let mut collateral_released = false;
    let mut terminate_sale = || -> Result<(), ContractError> {
        // Only release the marker if this is the final remaining ask for the given marker.
        // Multiple marker share sales can be created for a single marker while it is held by
        // the contract, so this check ensures that the marker is only relinquished when the
        // final sale is completed.
        if get_ask_orders_by_collateral_id(deps.storage, ask_collateral.marker_address.as_str())
            .len()
            <= 1
        {
            // Marker gets released to the asker.  The sale is effectively over.
            messages.append(&mut release_marker_from_contract(
                &ask_collateral.marker_denom,
                &env.contract.address,
                &ask_collateral.removed_permissions,
            )?);
            collateral_released = true;
        }
        delete_ask_order_by_id(deps.storage, &ask_order.id)?;
        ().to_ok()
    };
    let ask_deleted = match ask_collateral.sale_type {
        // Single transaction sales should always terminate immediately after the sale completes
        ShareSaleType::SingleTransaction => {
            terminate_sale()?;
            true
        }
        ShareSaleType::MultipleTransactions => {
            // Validation will prevent this value from ever becoming less than zero from the sale,
            // so this is a safe operation
            let shares_remaining_after_sale =
                ask_collateral.remaining_shares_in_sale.u128() - shares_purchased;
            // If all listed shares are now sold, terminate the sale
            if shares_remaining_after_sale == 0 {
                terminate_sale()?;
                true
            } else {
                let mut ask_order = ask_order.to_owned();
                let mut ask_collateral = ask_collateral.to_owned();
                ask_collateral.remaining_shares_in_sale = Uint128::new(shares_remaining_after_sale);
                ask_order.collateral = AskCollateral::MarkerShareSale(ask_collateral);
                // Replace the ask order in storage with an updated remaining_shares value
                update_ask_order(deps.storage, &ask_order)?;
                false
            }
        }
    };
    let MSSBidTotalsCalc {
        expected_remaining_bidder_coin,
        bidder_refund,
        ..
    } = calculate_marker_share_sale_bid_totals(bid_collateral, &quote_paid, bid_overage_shares)?;
    // Subtract coins will not add coin values of zero into the vector.  This ensures that a vector
    // that would be the result of perfect subtraction and cause zeroed out values will be empty,
    // allowing this check to ensure a refund message is only populated when actual coin needs to
    // be sent
    if !bidder_refund.is_empty() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: bid_order.owner.to_string(),
            amount: bidder_refund,
        }));
    }
    // If a bid overage occurred, then the bid should remain open with a reduced number of shares
    // and a reset quote.
    let bid_deleted = if bid_overage_shares > 0 {
        let mut updated_bidder_collateral = bid_collateral.to_owned();
        updated_bidder_collateral.share_count = Uint128::new(bid_overage_shares);
        updated_bidder_collateral.quote = expected_remaining_bidder_coin;
        let mut updated_bid_order = bid_order.to_owned();
        updated_bid_order.collateral = BidCollateral::MarkerShareSale(updated_bidder_collateral);
        update_bid_order(deps.storage, &updated_bid_order)?;
        // False = the bid was not deleted because it still wants additional shares
        false
    } else {
        // If no bid overage occurred, then all shares were purchased at the expected amount and the
        // bid should be closed.
        delete_bid_order_by_id(deps.storage, &bid_order.id)?;
        true
    };
    ExecuteResults {
        messages,
        ask_deleted,
        bid_deleted,
        collateral_released,
    }
    .to_ok()
}

fn execute_scope_trade(
    deps: DepsMut<ProvenanceQuery>,
    env: &Env,
    ask_order: &AskOrder,
    bid_order: &BidOrder,
    ask_collateral: &ScopeTradeAskCollateral,
    bid_collateral: &ScopeTradeBidCollateral,
) -> Result<ExecuteResults, ContractError> {
    // Asker gets the quote that the bidder provided from escrow
    let mut messages = vec![CosmosMsg::Bank(BankMsg::Send {
        to_address: ask_order.owner.to_string(),
        amount: bid_collateral.quote.to_owned(),
    })];
    let scope = ProvenanceQuerier::new(&deps.querier).get_scope(&ask_collateral.scope_address)?;
    // Bidder gets the scope transferred to them
    messages.push(write_scope(
        replace_scope_owner(scope, bid_order.owner.to_owned()),
        vec![env.contract.address.to_owned()],
    )?);
    // Remove the ask and bid orders now that the trade has been finalized
    delete_ask_order_by_id(deps.storage, &ask_order.id)?;
    delete_bid_order_by_id(deps.storage, &bid_order.id)?;
    ExecuteResults {
        messages,
        ask_deleted: true,
        bid_deleted: true,
        collateral_released: true,
    }
    .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::execute::create_ask::create_ask;
    use crate::execute::create_bid::create_bid;
    use crate::execute::execute_match::execute_match;
    use crate::storage::ask_order_storage::{get_ask_order_by_id, insert_ask_order};
    use crate::storage::bid_order_storage::{get_bid_order_by_id, insert_bid_order};
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::error_helpers::assert_validation_error_message;
    use crate::test::mock_instantiate::{default_instantiate, DEFAULT_ADMIN_ADDRESS};
    use crate::test::mock_marker::{MockMarker, DEFAULT_MARKER_DENOM, DEFAULT_MARKER_HOLDINGS};
    use crate::test::mock_scope::{MockScope, DEFAULT_SCOPE_ADDR};
    use crate::test::request_helpers::{mock_ask_order, mock_bid_order, mock_bid_scope_trade};
    use crate::types::core::error::ContractError;
    use crate::types::request::admin_match_options::{AdminMatchOptions, OverrideQuoteSource};
    use crate::types::request::ask_types::ask::Ask;
    use crate::types::request::ask_types::ask_collateral::AskCollateral;
    use crate::types::request::bid_types::bid::Bid;
    use crate::types::request::share_sale_type::ShareSaleType;
    use crate::util::constants::NHASH;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins, BankMsg, Coin, CosmosMsg, Response, Storage, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        MarkerMsgParams, MetadataMsgParams, PartyType, ProvenanceMsg, ProvenanceMsgParams,
    };

    #[test]
    fn test_execute_match_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            String::new(),
            "bid_id".to_string(),
            None,
        )
        .expect_err("an error should occur when the ask id is empty");
        assert_validation_error_message(err, "ask id must not be empty");
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            String::new(),
            None,
        )
        .expect_err("an error should occur when the bid id is empty");
        assert_validation_error_message(err, "bid id must not be empty");
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &coins(100, NHASH)),
            "ask_id".to_string(),
            "bid_id".to_string(),
            None,
        )
        .expect_err("an error should occur due to funds being provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should be returned when funds are provided, but got: {:?}",
            err,
        );
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            None,
        )
        .expect_err("an error should occur when the ask is missing");
        match err {
            ContractError::StorageError { message } => {
                assert!(
                    message.contains("failed to find AskOrder by id"),
                    "unexpected message from storage error for missing ask: {}",
                    message,
                );
            }
            e => panic!("unexpected error: {:?}", e),
        }
        let ask_order = mock_ask_order(AskCollateral::coin_trade(&[], &[]));
        insert_ask_order(deps.as_mut().storage, &ask_order)
            .expect("expected ask order insert to succeed");
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            None,
        )
        .expect_err("an error should occur when the bid is missing");
        match err {
            ContractError::StorageError { message } => {
                assert!(
                    message.contains("failed to find BidOrder by id"),
                    "unexpected message from storage error for missing bid: {}",
                    message,
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let bid_order = mock_bid_order(mock_bid_scope_trade(DEFAULT_SCOPE_ADDR, &[]));
        insert_bid_order(deps.as_mut().storage, &bid_order)
            .expect("expected bid order insert to succeed");
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info("not-admin", &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            None,
        )
        .expect_err("an error should occur due to the admin not being the sender");
        assert!(
            matches!(err, ContractError::Unauthorized),
            "an unauthorized error should be returned when the admin is not used as the sender, but got: {:?}",
            err,
        );
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            None,
        )
        .expect_err("an error should occur when the ask and bid don't match");
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(2, messages.len(), "two error messages should be produced");
                assert!(
                    messages.contains(&"Match Validation for AskOrder [ask_id] and BidOrder [bid_id]: Ask type [coin_trade] does not match bid type [scope_trade]".to_string()),
                    "a message about ask type not matching bid type should be included, but got messages: {:?}",
                    messages,
                );
                assert!(
                    messages.contains(&"Match Validation for AskOrder [ask_id] and BidOrder [bid_id]: Ask collateral was of type coin trade, which did not match bid collateral".to_string()),
                    "a message about ask collateral not matching bid collateral should be included, but got messages: {:?}",
                    messages,
                );
            }
            e => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_execute_coin_trade_from_admin_matching_quote() {
        do_coin_trade_test(DEFAULT_ADMIN_ADDRESS, false);
    }

    #[test]
    fn test_execute_coin_trade_from_admin_mismatched_quote() {
        do_coin_trade_test(DEFAULT_ADMIN_ADDRESS, true);
    }

    #[test]
    fn test_execute_coin_trade_from_asker_matching_quote() {
        do_coin_trade_test("asker", false);
    }

    #[test]
    fn test_execute_marker_trade_from_admin_matching_quote() {
        do_marker_trade_test(DEFAULT_ADMIN_ADDRESS, false, None);
    }

    #[test]
    fn test_execute_marker_trade_from_admin_mismatched_quote() {
        do_marker_trade_test(DEFAULT_ADMIN_ADDRESS, true, None);
    }

    #[test]
    fn test_execute_marker_trade_from_admin_matching_quote_explicit_no_withdraw_coins() {
        do_marker_trade_test(DEFAULT_ADMIN_ADDRESS, false, Some(false));
    }

    #[test]
    fn test_execute_marker_trade_from_admin_matching_quote_and_withdraw_coins() {
        do_marker_trade_test(DEFAULT_ADMIN_ADDRESS, false, Some(true));
    }

    #[test]
    fn test_execute_marker_trade_from_asker_matching_quote() {
        do_marker_trade_test("asker", false, None);
    }

    #[test]
    fn test_execute_marker_trade_from_asker_matching_quote_explicit_no_withdraw_coins() {
        do_marker_trade_test("asker", false, Some(false));
    }

    #[test]
    fn test_execute_marker_trade_from_asker_matching_quote_and_withdraw_coins() {
        do_marker_trade_test("asker", false, Some(true));
    }

    #[test]
    fn test_execute_marker_share_sale_single_tx_from_admin() {
        do_marker_share_sale_single_tx_test(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_execute_marker_share_sale_single_tx_from_asker() {
        do_marker_share_sale_single_tx_test("asker");
    }

    #[test]
    fn test_execute_marker_share_sale_multi_tx_from_admin() {
        do_marker_share_sale_multi_tx_test(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_execute_marker_share_sale_multi_tx_from_asker() {
        do_marker_share_sale_multi_tx_test("asker");
    }

    #[test]
    fn test_execute_multiple_marker_share_sales() {
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
                15,
                &coins(20, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("the first ask should be created");
        // Simulate the marker being moved to be owned by the contract
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id_2",
                DEFAULT_MARKER_DENOM,
                10,
                &coins(40, "quote"),
                ShareSaleType::MultipleTransactions,
            ),
            None,
        )
        .expect("the second ask should be created");
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(300, "quote")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 15),
            None,
        )
        .expect("the first bid should be created");
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(200, "quote")),
            Bid::new_marker_share_sale("bid_id_2", DEFAULT_MARKER_DENOM, 5),
            None,
        )
        .expect("the second bid should be created");
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(120, "quote")),
            Bid::new_marker_share_sale("bid_id_3", DEFAULT_MARKER_DENOM, 3),
            None,
        )
        .expect("the third bid should be created");
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &coins(80, "quote")),
            Bid::new_marker_share_sale("bid_id_4", DEFAULT_MARKER_DENOM, 2),
            None,
        )
        .expect("the fourth bid should be created");
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            None,
        )
        .expect("the first match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response, false);
        assert_response_messages_for_incomplete_marker_share_sale(
            &response,
            &coins(300, "quote"),
            &coin(15, DEFAULT_MARKER_DENOM),
        );
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id_2".to_string(),
            "bid_id_2".to_string(),
            None,
        )
        .expect("the second match should execute successfully");
        assert_match_produced_correct_results_with_extras(
            deps.as_ref().storage,
            &response,
            false,
            true,
            false,
            "ask_id_2",
            "bid_id_2",
        );
        assert_response_messages_for_incomplete_marker_share_sale(
            &response,
            &coins(200, "quote"),
            &coin(5, DEFAULT_MARKER_DENOM),
        );
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id_2".to_string(),
            "bid_id_3".to_string(),
            None,
        )
        .expect("the third match should execute successfully");
        assert_match_produced_correct_results_with_extras(
            deps.as_ref().storage,
            &response,
            false,
            true,
            false,
            "ask_id_2",
            "bid_id_3",
        );
        assert_response_messages_for_incomplete_marker_share_sale(
            &response,
            &coins(120, "quote"),
            &coin(3, DEFAULT_MARKER_DENOM),
        );
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id_2".to_string(),
            "bid_id_4".to_string(),
            None,
        )
        .expect("the fourth match should execute successfully");
        assert_match_produced_correct_results_with_extras(
            deps.as_ref().storage,
            &response,
            true,
            false,
            false,
            "ask_id_2",
            "bid_id_4",
        );
        assert_eq!(
            4,
            response.messages.len(),
            "the correct number of messages should be produced when the final ask is removed for a marker",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker",
                    to_address,
                    "the asker should receive funds from the match",
                );
                assert_eq!(
                    &coins(80, "quote"),
                    amount,
                    "the correct amount of funds should be sent to the bidder",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    marker_denom,
                    "the correct marker should be referenced in the withdraw message",
                );
                assert_eq!(
                    &cosmwasm_std::coin(2, DEFAULT_MARKER_DENOM),
                    coin,
                    "the correct amount of marker funds should be withdrawn",
                );
                assert_eq!(
                    "bidder",
                    recipient.as_str(),
                    "the bidder should receive the marker tokens",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker denom should be referenced in the grant message",
                );
                assert_eq!(
                    "asker",
                    address.as_str(),
                    "the asker should be re-granted its permissions on the marker after the sale",
                );
                assert_eq!(
                    &MockMarker::get_default_owner_permissions(),
                    permissions,
                    "the asker should be returned all of its permissions",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker denom should be referenced in the revoke message",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    address.as_str(),
                    "the contract should have its permissions to the marker revoked after the sale completes",
                );
            },
            msg => panic!("unexpected message: {:?}", msg),
        });
    }

    #[test]
    fn test_execute_marker_share_sale_excess_bid_shares_leftover_no_admin_options() {
        for share_sale_type in vec![
            ShareSaleType::SingleTransaction,
            ShareSaleType::MultipleTransactions,
        ] {
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
                    15,
                    &coins(1, "quote"),
                    share_sale_type.to_owned(),
                ),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the ask should be created successfully",
                    share_sale_type
                )
            });
            create_bid(
                deps.as_mut(),
                mock_env(),
                mock_info("bidder", &coins(20, "quote")),
                Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 20),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the bid should be created successfully",
                    share_sale_type
                )
            });
            let response = execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the match should execute successfully",
                    share_sale_type
                )
            });
            assert_match_produced_correct_results_with_extras(
                deps.as_ref().storage,
                &response,
                true,
                false,
                true,
                "ask_id",
                "bid_id",
            );
            assert_eq!(
                4,
                response.messages.len(),
                "{:?}: the correct number of messages should be produced",
                share_sale_type,
            );
            response.messages.iter().for_each(|msg| match &msg.msg {
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                    assert_eq!(
                        "asker",
                        to_address,
                        "{:?}: the asker should receive funds from the match",
                        share_sale_type,
                    );
                    assert_eq!(
                        &coins(15, "quote"),
                        amount,
                        "{:?}: the asker should get the correct quote funds",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        marker_denom,
                        "{:?}: the correct marker should be referenced in the withdraw message",
                        share_sale_type,
                    );
                    assert_eq!(
                        &cosmwasm_std::coin(15, DEFAULT_MARKER_DENOM),
                        coin,
                        "{:?}: the correct amount of marker funds should be withdrawn",
                        share_sale_type,
                    );
                    assert_eq!(
                        "bidder",
                        recipient.as_str(),
                        "{:?}: the bidder should receive the marker tokens",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the grant message",
                        share_sale_type,
                    );
                    assert_eq!(
                        "asker",
                        address.as_str(),
                        "{:?}: the asker should be re-granted its permissions on the marker after the sale",
                        share_sale_type,
                    );
                    assert_eq!(
                        &MockMarker::get_default_owner_permissions(),
                        permissions,
                        "{:?}: the asker should be returned all of its permissions",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the revoke message",
                        share_sale_type,
                    );
                    assert_eq!(
                        MOCK_CONTRACT_ADDR,
                        address.as_str(),
                        "{:?}: the contract should have its permissions to the marker revoked after the sale completes",
                        share_sale_type,
                    );
                },
                msg => panic!("{:?}: unexpected message: {:?}", share_sale_type, msg),
            });
            let bid_order =
                get_bid_order_by_id(deps.as_ref().storage, "bid_id").unwrap_or_else(|_| {
                    panic!("{:?}: bid order should remain in storage", share_sale_type)
                });
            let bid_collateral = bid_order.collateral.unwrap_marker_share_sale();
            assert_eq!(
                5,
                bid_collateral.share_count.u128(),
                "{:?}: the five unpurchased shares should be indicated in the collateral",
                share_sale_type,
            );
            assert_eq!(
                coins(5, "quote"),
                bid_collateral.quote,
                "{:?}: the quote should reflect the remaining unspent quote coin",
                share_sale_type,
            );
        }
    }

    #[test]
    fn test_execute_marker_share_sale_use_lower_ask_amount_full_sale() {
        for share_sale_type in vec![
            ShareSaleType::SingleTransaction,
            ShareSaleType::MultipleTransactions,
        ] {
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
                    15,
                    &coins(1, "quote"),
                    share_sale_type.to_owned(),
                ),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the ask should be created successfully",
                    share_sale_type
                )
            });
            create_bid(
                deps.as_mut(),
                mock_env(),
                // Quote per share = 3quote
                mock_info("bidder", &coins(45, "quote")),
                Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 15),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the bid should be created successfully",
                    share_sale_type
                )
            });
            let response = execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                Some(AdminMatchOptions::marker_share_sale_options(
                    OverrideQuoteSource::Ask,
                )),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the match should execute successfully",
                    share_sale_type
                )
            });
            assert_match_produced_correct_results(deps.as_ref().storage, &response, true);
            assert_eq!(
                5,
                response.messages.len(),
                "{:?}: the correct number of messages should be produced",
                share_sale_type,
            );
            response.messages.iter().for_each(|msg| match &msg.msg {
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                    match to_address.as_str() {
                        // The asker receives a coin send for their purchased shares
                        "asker" => {
                            assert_eq!(
                                &coins(15, "quote"),
                                amount,
                                "{:?}: the asker should get the correct quote funds",
                                share_sale_type,
                            );
                        },
                        // The bidder receives a refund for the additional quote they didn't spend
                        "bidder" => {
                            assert_eq!(
                                &coins(30, "quote"),
                                amount,
                                "{:?}: the bidder should get a refund for the 30quote they didn't spend",
                                share_sale_type,
                            );
                        },
                        unexpected_addr => panic!("{:?}: unexpected send msg to address: {}", share_sale_type, unexpected_addr),
                    }

                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        marker_denom,
                        "{:?}: the correct marker should be referenced in the withdraw message",
                        share_sale_type,
                    );
                    assert_eq!(
                        &cosmwasm_std::coin(15, DEFAULT_MARKER_DENOM),
                        coin,
                        "{:?}: the correct amount of marker funds should be withdrawn",
                        share_sale_type,
                    );
                    assert_eq!(
                        "bidder",
                        recipient.as_str(),
                        "{:?}: the bidder should receive the marker tokens",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the grant message",
                        share_sale_type,
                    );
                    assert_eq!(
                        "asker",
                        address.as_str(),
                        "{:?}: the asker should be re-granted its permissions on the marker after the sale",
                        share_sale_type,
                    );
                    assert_eq!(
                        &MockMarker::get_default_owner_permissions(),
                        permissions,
                        "{:?}: the asker should be returned all of its permissions",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the revoke message",
                        share_sale_type,
                    );
                    assert_eq!(
                        MOCK_CONTRACT_ADDR,
                        address.as_str(),
                        "{:?}: the contract should have its permissions to the marker revoked after the sale completes",
                        share_sale_type,
                    );
                },
                msg => panic!("{:?}: unexpected message: {:?}", share_sale_type, msg),
            });
        }
    }

    #[test]
    fn test_execute_marker_share_sale_use_lower_ask_amount_bidder_shares_leftover() {
        for share_sale_type in vec![
            ShareSaleType::SingleTransaction,
            ShareSaleType::MultipleTransactions,
        ] {
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
                    15,
                    &coins(1, "quote"),
                    share_sale_type.to_owned(),
                ),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the ask should be created successfully",
                    share_sale_type
                )
            });
            create_bid(
                deps.as_mut(),
                mock_env(),
                // Quote per share = 3quote
                mock_info("bidder", &coins(90, "quote")),
                Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 30),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the bid should be created successfully",
                    share_sale_type
                )
            });
            let response = execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                Some(AdminMatchOptions::marker_share_sale_options(
                    OverrideQuoteSource::Ask,
                )),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the match should execute successfully",
                    share_sale_type
                )
            });
            assert_match_produced_correct_results_with_extras(
                deps.as_ref().storage,
                &response,
                true,
                false,
                true,
                "ask_id",
                "bid_id",
            );
            assert_eq!(
                5,
                response.messages.len(),
                "{:?}: the correct number of messages should be produced",
                share_sale_type,
            );
            response.messages.iter().for_each(|msg| match &msg.msg {
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                    match to_address.as_str() {
                        // The asker receives a coin send for their purchased shares
                        "asker" => {
                            assert_eq!(
                                &coins(15, "quote"),
                                amount,
                                "{:?}: the asker should get the correct quote funds",
                                share_sale_type,
                            );
                        },
                        // The bidder receives a refund for the additional quote they didn't spend
                        "bidder" => {
                            assert_eq!(
                                &coins(30, "quote"),
                                amount,
                                "{:?}: the bidder should get a refund for the 30quote they didn't spend",
                                share_sale_type,
                            );
                        },
                        unexpected_addr => panic!("{:?}: unexpected send msg to address: {}", share_sale_type, unexpected_addr),
                    }

                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        marker_denom,
                        "{:?}: the correct marker should be referenced in the withdraw message",
                        share_sale_type,
                    );
                    assert_eq!(
                        &cosmwasm_std::coin(15, DEFAULT_MARKER_DENOM),
                        coin,
                        "{:?}: the correct amount of marker funds should be withdrawn",
                        share_sale_type,
                    );
                    assert_eq!(
                        "bidder",
                        recipient.as_str(),
                        "{:?}: the bidder should receive the marker tokens",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the grant message",
                        share_sale_type,
                    );
                    assert_eq!(
                        "asker",
                        address.as_str(),
                        "{:?}: the asker should be re-granted its permissions on the marker after the sale",
                        share_sale_type,
                    );
                    assert_eq!(
                        &MockMarker::get_default_owner_permissions(),
                        permissions,
                        "{:?}: the asker should be returned all of its permissions",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the revoke message",
                        share_sale_type,
                    );
                    assert_eq!(
                        MOCK_CONTRACT_ADDR,
                        address.as_str(),
                        "{:?}: the contract should have its permissions to the marker revoked after the sale completes",
                        share_sale_type,
                    );
                },
                msg => panic!("{:?}: unexpected message: {:?}", share_sale_type, msg),
            });
            let bid_order =
                get_bid_order_by_id(deps.as_ref().storage, "bid_id").unwrap_or_else(|_| {
                    panic!("{:?}: bid order should remain in storage", share_sale_type)
                });
            let bid_collateral = bid_order.collateral.unwrap_marker_share_sale();
            assert_eq!(
                15,
                bid_collateral.share_count.u128(),
                "{:?}: the fifteen unpurchased shares should be indicated in the collateral",
                share_sale_type,
            );
            assert_eq!(
                coins(45, "quote"),
                bid_collateral.quote,
                "{:?}: the quote should reflect the remaining unspent quote coin",
                share_sale_type,
            );
        }
    }

    #[test]
    fn test_execute_marker_share_sale_single_tx_use_higher_bid_amount_bidder_shares_leftover() {
        for share_sale_type in vec![
            ShareSaleType::SingleTransaction,
            ShareSaleType::MultipleTransactions,
        ] {
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
                    15,
                    &coins(1, "quote"),
                    share_sale_type.to_owned(),
                ),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the ask should be created successfully",
                    share_sale_type
                )
            });
            create_bid(
                deps.as_mut(),
                mock_env(),
                // Quote per share = 3quote
                mock_info("bidder", &coins(90, "quote")),
                Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 30),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the bid should be created successfully",
                    share_sale_type
                )
            });
            let response = execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                Some(AdminMatchOptions::marker_share_sale_options(
                    OverrideQuoteSource::Bid,
                )),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the match should execute successfully",
                    share_sale_type
                )
            });
            assert_match_produced_correct_results_with_extras(
                deps.as_ref().storage,
                &response,
                true,
                false,
                true,
                "ask_id",
                "bid_id",
            );
            assert_eq!(
                4,
                response.messages.len(),
                "{:?}: the correct number of messages should be produced",
                share_sale_type,
            );
            response.messages.iter().for_each(|msg| match &msg.msg {
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                    assert_eq!(
                        "asker",
                        to_address,
                        "{:?}: the asker should receive funds from the match",
                        share_sale_type,
                    );
                    assert_eq!(
                        &coins(45, "quote"),
                        amount,
                        "{:?}: the asker should get the correct quote funds",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        marker_denom,
                        "{:?}: the correct marker should be referenced in the withdraw message",
                        share_sale_type,
                    );
                    assert_eq!(
                        &cosmwasm_std::coin(15, DEFAULT_MARKER_DENOM),
                        coin,
                        "{:?}: the correct amount of marker funds should be withdrawn",
                        share_sale_type,
                    );
                    assert_eq!(
                        "bidder",
                        recipient.as_str(),
                        "{:?}: the bidder should receive the marker tokens",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the grant message",
                        share_sale_type,
                    );
                    assert_eq!(
                        "asker",
                        address.as_str(),
                        "{:?}: the asker should be re-granted its permissions on the marker after the sale",
                        share_sale_type,
                    );
                    assert_eq!(
                        &MockMarker::get_default_owner_permissions(),
                        permissions,
                        "{:?}: the asker should be returned all of its permissions",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the revoke message",
                        share_sale_type,
                    );
                    assert_eq!(
                        MOCK_CONTRACT_ADDR,
                        address.as_str(),
                        "{:?}: the contract should have its permissions to the marker revoked after the sale completes",
                        share_sale_type,
                    );
                },
                msg => panic!("{:?}: unexpected message: {:?}", share_sale_type, msg),
            });
            let bid_order =
                get_bid_order_by_id(deps.as_ref().storage, "bid_id").unwrap_or_else(|_| {
                    panic!("{:?}: bid order should remain in storage", share_sale_type)
                });
            let bid_collateral = bid_order.collateral.unwrap_marker_share_sale();
            assert_eq!(
                15,
                bid_collateral.share_count.u128(),
                "{:?}: the fifteen unpurchased shares should be indicated in the collateral",
                share_sale_type,
            );
            assert_eq!(
                coins(45, "quote"),
                bid_collateral.quote,
                "{:?}: the quote should reflect the remaining unspent quote coin",
                share_sale_type,
            );
        }
    }

    #[test]
    fn test_execute_marker_share_sale_use_higher_bid_amount_full_sale() {
        for share_sale_type in vec![
            ShareSaleType::SingleTransaction,
            ShareSaleType::MultipleTransactions,
        ] {
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
                    15,
                    &coins(1, "quote"),
                    share_sale_type.to_owned(),
                ),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the ask should be created successfully",
                    share_sale_type
                )
            });
            create_bid(
                deps.as_mut(),
                mock_env(),
                // Quote per share = 3quote
                mock_info("bidder", &coins(45, "quote")),
                Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 15),
                None,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the bid should be created successfully",
                    share_sale_type
                )
            });
            let response = execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                Some(AdminMatchOptions::marker_share_sale_options(
                    OverrideQuoteSource::Bid,
                )),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "{:?}: the match should execute successfully",
                    share_sale_type
                )
            });
            assert_match_produced_correct_results(deps.as_ref().storage, &response, true);
            assert_eq!(
                4,
                response.messages.len(),
                "{:?}: the correct number of messages should be produced",
                share_sale_type,
            );
            response.messages.iter().for_each(|msg| match &msg.msg {
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                    assert_eq!(
                        "asker",
                        to_address,
                        "{:?}: the asker should receive funds from the match",
                        share_sale_type,
                    );
                    assert_eq!(
                        &coins(45, "quote"),
                        amount,
                        "{:?}: the asker should get the correct quote funds",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        marker_denom,
                        "{:?}: the correct marker should be referenced in the withdraw message",
                        share_sale_type,
                    );
                    assert_eq!(
                        &cosmwasm_std::coin(15, DEFAULT_MARKER_DENOM),
                        coin,
                        "{:?}: the correct amount of marker funds should be withdrawn",
                        share_sale_type,
                    );
                    assert_eq!(
                        "bidder",
                        recipient.as_str(),
                        "{:?}: the bidder should receive the marker tokens",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the grant message",
                        share_sale_type,
                    );
                    assert_eq!(
                        "asker",
                        address.as_str(),
                        "{:?}: the asker should be re-granted its permissions on the marker after the sale",
                        share_sale_type,
                    );
                    assert_eq!(
                        &MockMarker::get_default_owner_permissions(),
                        permissions,
                        "{:?}: the asker should be returned all of its permissions",
                        share_sale_type,
                    );
                },
                CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                    assert_eq!(
                        DEFAULT_MARKER_DENOM,
                        denom,
                        "{:?}: the correct marker denom should be referenced in the revoke message",
                        share_sale_type,
                    );
                    assert_eq!(
                        MOCK_CONTRACT_ADDR,
                        address.as_str(),
                        "{:?}: the contract should have its permissions to the marker revoked after the sale completes",
                        share_sale_type,
                    );
                },
                msg => panic!("{:?}: unexpected message: {:?}", share_sale_type, msg),
            });
        }
    }

    #[test]
    fn test_execute_scope_trade_from_admin_with_matching_quote() {
        do_scope_trade_test(DEFAULT_ADMIN_ADDRESS, false);
    }

    #[test]
    fn test_execute_scope_trade_from_admin_with_mismatched_quote() {
        do_scope_trade_test(DEFAULT_ADMIN_ADDRESS, true);
    }

    #[test]
    fn test_execute_scope_trade_from_asker_with_matching_quote() {
        do_scope_trade_test(DEFAULT_ADMIN_ADDRESS, false);
    }

    fn assert_match_produced_correct_results(
        storage: &dyn Storage,
        response: &Response<ProvenanceMsg>,
        expect_collateral_released: bool,
    ) {
        assert_match_produced_correct_results_with_extras(
            storage,
            response,
            expect_collateral_released,
            false,
            false,
            "ask_id",
            "bid_id",
        );
    }

    fn assert_match_produced_correct_results_with_extras<S1: Into<String>, S2: Into<String>>(
        storage: &dyn Storage,
        response: &Response<ProvenanceMsg>,
        expect_collateral_released: bool,
        expect_ask_to_remain_in_storage: bool,
        expect_bid_to_remain_in_storage: bool,
        ask_id: S1,
        bid_id: S2,
    ) {
        let ask_id = ask_id.into();
        let bid_id = bid_id.into();
        assert_eq!(
            6,
            response.attributes.len(),
            "the correct number of attributes should be produced in the response",
        );
        assert_eq!(
            "execute",
            single_attribute_for_key(response, "action"),
            "the correct action attribute value should be produced",
        );
        assert_eq!(
            ask_id,
            single_attribute_for_key(response, "ask_id"),
            "the correct ask_id attribute value should be produced",
        );
        assert_eq!(
            bid_id,
            single_attribute_for_key(response, "bid_id"),
            "the correct bid_id attribute value should be produced",
        );
        assert_eq!(
            (!expect_ask_to_remain_in_storage).to_string(),
            single_attribute_for_key(response, "ask_deleted"),
            "the correct ask_deleted value should be produced",
        );
        assert_eq!(
            (!expect_bid_to_remain_in_storage).to_string(),
            single_attribute_for_key(response, "bid_deleted"),
            "the correct bid_deleted value should be produced",
        );
        assert_eq!(
            expect_collateral_released.to_string(),
            single_attribute_for_key(response, "collateral_released"),
            "the correct collateral_released value should be produced",
        );
        let ask_order_result = get_ask_order_by_id(storage, &ask_id);
        if expect_ask_to_remain_in_storage {
            ask_order_result.expect("ask should remain in storage");
        } else {
            ask_order_result.expect_err("ask should be missing from storage");
        }
        let bid_order_result = get_bid_order_by_id(storage, &bid_id);
        if expect_bid_to_remain_in_storage {
            bid_order_result.expect("bid should remain in storage");
        } else {
            bid_order_result.expect_err("bid should be missing from storage");
        }
    }

    fn assert_response_messages_for_incomplete_marker_share_sale(
        response: &Response<ProvenanceMsg>,
        asker_coin_received: &[Coin],
        withdrawn_funds_for_bidder: &Coin,
    ) {
        assert_eq!(
            2,
            response.messages.len(),
            "the correct number of messages should be sent after the match",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker", to_address,
                    "the asker should receive funds from the match",
                );
                assert_eq!(
                    asker_coin_received, amount,
                    "the asker should receive all the bid coin",
                );
            }
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins {
                        marker_denom,
                        coin,
                        recipient,
                    }),
                ..
            }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM, marker_denom,
                    "the correct marker should be referenced in the withdraw message",
                );
                assert_eq!(
                    withdrawn_funds_for_bidder, coin,
                    "the correct amount of marker funds should be withdrawn",
                );
                assert_eq!(
                    "bidder",
                    recipient.as_str(),
                    "the bidder should receive the marker tokens",
                );
            }
            msg => panic!("unexpected message: {:?}", msg),
        });
    }

    fn do_coin_trade_test<S: Into<String>>(match_sender_address: S, mismatched_quotes: bool) {
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
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        let bid_quote = coins(
            100,
            if mismatched_quotes {
                "notquote"
            } else {
                "quote"
            },
        );
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &bid_quote),
            Bid::new_coin_trade("bid_id", &coins(100, "base")),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let sender_address = match_sender_address.into();
        if mismatched_quotes {
            execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(&sender_address, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                Some(AdminMatchOptions::coin_trade_options(false)),
            )
            .expect_err(
                "an error should be returned when the bid quote does not match the ask quote",
            );
            execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(&sender_address, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                None,
            ).expect_err(
                    "an error should be returned when the bid quote does not match the ask quote and no value is provided in the mismatch flag",
                );
        }
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            // Only the admin may provide admin match options.  Otherwise, the request will be rejected
            if sender_address == DEFAULT_ADMIN_ADDRESS {
                // Only allow invalid matches when the quotes are supposed to have a mismatch
                Some(AdminMatchOptions::coin_trade_options(mismatched_quotes))
            } else {
                None
            },
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response, true);
        assert_eq!(
            2,
            response.messages.len(),
            "the correct number of messages should be produced",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => match to_address.as_str() {
                "asker" => {
                    assert_eq!(
                        &bid_quote,
                        amount,
                        "{}",
                        if mismatched_quotes {
                            "the asker should get the correct mismatched quote funds"
                        } else {
                            "the asker should get the correct quote funds"
                        },
                    );
                }
                "bidder" => {
                    assert_eq!(
                        &coins(100, "base"),
                        amount,
                        "the bidder should get the base funds",
                    );
                }
                other => panic!("unexpected funds receiver: {}", other),
            },
            msg => panic!("unexpected message: {:?}", msg),
        });
    }

    fn do_marker_trade_test<S: Into<String>>(
        match_sender_address: S,
        mismatched_quotes: bool,
        withdraw_shares: Option<bool>,
    ) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade("ask_id", DEFAULT_MARKER_DENOM, &coins(1, "quote")),
            None,
        )
        .expect("the ask should be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        let bid_quote = if mismatched_quotes {
            coins(1500, "fakecoin")
        } else {
            coins(DEFAULT_MARKER_HOLDINGS, "quote")
        };
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &bid_quote),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM, withdraw_shares),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let sender_address = match_sender_address.into();
        if mismatched_quotes {
            execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(&sender_address, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                Some(AdminMatchOptions::marker_trade_options(false)),
            )
            .expect_err(
                "an error should be returned when the bid quote does not match the ask quote",
            );
            execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(&sender_address, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                None,
            ).expect_err(
                "an error should be returned when the bid quote does not match the ask quote and no value is provided in the mismatch flag",
            );
        }
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            // Only the admin may provide admin match options.  Otherwise, the request will be rejected
            if sender_address == DEFAULT_ADMIN_ADDRESS {
                Some(AdminMatchOptions::marker_trade_options(mismatched_quotes))
            } else {
                None
            },
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response, true);
        assert_eq!(
            3 + if withdraw_shares.unwrap_or(false) {
                1
            } else {
                0
            },
            response.messages.len(),
            "the correct number of messages should be produced",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker",
                    to_address,
                    "the asker should receive the funds in the trade",
                );
                assert_eq!(
                    &bid_quote,
                    amount,
                    "{}",
                    if mismatched_quotes {
                        "the asker should get the correct mismatched quote funds"
                    } else {
                        "the asker should get the correct quote funds"
                    },
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker should be targeted in the grant request",
                );
                assert_eq!(
                    "bidder",
                    address.as_str(),
                    "the bidder should be granted marker access",
                );
                assert_eq!(
                    &MockMarker::get_default_owner_permissions(),
                    permissions,
                    "the bidder should be granted all the same permissions that the asker had when they placed the ask",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker should be targeted in the revoked request",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    address.as_str(),
                    "the contract should have its marker permissions removed",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                assert!(
                    withdraw_shares.unwrap_or(false),
                    "a withdraw coins message should only be sent when the bidder requested a withdraw",
                );
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    marker_denom,
                    "the withdrawn marker denom should be the correct type",
                );
                assert_eq!(
                    &Coin { amount: Uint128::new(DEFAULT_MARKER_HOLDINGS), denom: DEFAULT_MARKER_DENOM.to_string() },
                    coin,
                    "the withdrawn marker coin should equate to the entirety of the marker's coin holdings",
                );
                assert_eq!(
                    "bidder",
                    recipient.as_str(),
                    "the bidder should receive all the marker coin",
                );
            }
            msg => panic!("unexpected message: {:?}", msg),
        });
    }

    fn do_marker_share_sale_single_tx_test<S: Into<String>>(match_sender_address: S) {
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
                15,
                &coins(1, "quote"),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("the ask should be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        let bid_quote = coins(15, "quote");
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &bid_quote),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 15),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let sender_address = match_sender_address.into();
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            None,
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response, true);
        assert_eq!(
            4,
            response.messages.len(),
            "the correct number of messages should be produced",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker",
                    to_address,
                    "the asker should receive funds from the match",
                );
                assert_eq!(
                    &bid_quote,
                    amount,
                    "the asker should get the correct quote funds"
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    marker_denom,
                    "the correct marker should be referenced in the withdraw message",
                );
                assert_eq!(
                    &cosmwasm_std::coin(15, DEFAULT_MARKER_DENOM),
                    coin,
                    "the correct amount of marker funds should be withdrawn",
                );
                assert_eq!(
                    "bidder",
                    recipient.as_str(),
                    "the bidder should receive the marker tokens",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker denom should be referenced in the grant message",
                );
                assert_eq!(
                    "asker",
                    address.as_str(),
                    "the asker should be re-granted its permissions on the marker after the sale",
                );
                assert_eq!(
                    &MockMarker::get_default_owner_permissions(),
                    permissions,
                    "the asker should be returned all of its permissions",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker denom should be referenced in the revoke message",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    address.as_str(),
                    "the contract should have its permissions to the marker revoked after the sale completes",
                );
            },
            msg => panic!("unexpected message: {:?}", msg),
        });
    }

    fn do_marker_share_sale_multi_tx_test<S: Into<String>>(match_sender_address: S) {
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
                100,
                &coins(1, "quote"),
                ShareSaleType::MultipleTransactions,
            ),
            None,
        )
        .expect("the ask should be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        let first_bid_quote = coins(50, "quote");
        // Create the first bid that only buys half the shares for sale
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &first_bid_quote),
            Bid::new_marker_share_sale("bid_id_1", DEFAULT_MARKER_DENOM, 50),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id_1").expect("bid order should exist");
        let sender_address = match_sender_address.into();
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address, &[]),
            "ask_id".to_string(),
            "bid_id_1".to_string(),
            None,
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results_with_extras(
            deps.as_ref().storage,
            &response,
            false,
            true,
            false,
            "ask_id",
            "bid_id_1",
        );
        let ask_order = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("ask should remain in storage");
        let collateral = ask_order.collateral.unwrap_marker_share_sale();
        assert_eq!(
            100,
            collateral.total_shares_in_sale.u128(),
            "there should be 100 total shares in sale, indicating the correct original amount",
        );
        assert_eq!(
            50,
            collateral.remaining_shares_in_sale.u128(),
            "there should be 50 remaining shares after the sale completes, down from the original 100",
        );
        get_bid_order_by_id(deps.as_ref().storage, "bid_id")
            .expect_err("bid should be missing from storage");
        assert_eq!(
            2,
            response.messages.len(),
            "the correct number of messages should be produced",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker", to_address,
                    "the asker should receive funds from the match",
                );
                assert_eq!(
                    &first_bid_quote, amount,
                    "the asker should get the correct quote funds",
                );
            }
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins {
                        marker_denom,
                        coin,
                        recipient,
                    }),
                ..
            }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM, marker_denom,
                    "the correct marker should be referenced in the withdraw message",
                );
                assert_eq!(
                    &cosmwasm_std::coin(50, DEFAULT_MARKER_DENOM),
                    coin,
                    "the correct amount of marker funds should be withdrawn",
                );
                assert_eq!(
                    "bidder",
                    recipient.as_str(),
                    "the bidder should receive the marker tokens",
                );
            }
            msg => panic!("unexpected message: {:?}", msg),
        });
        // Messages in unit tests aren't executed, so we need to simulate the marker being updated
        // on the chain by manually duping it with its new lower coin holdings
        let updated_marker = MockMarker {
            coins: coins(50, DEFAULT_MARKER_DENOM),
            ..MockMarker::new_owned_mock_marker("asker")
        }
        .to_marker();
        deps.querier.with_markers(vec![updated_marker]);
        let second_bid_quote = coins(50, "quote");
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &second_bid_quote),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 50),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            None,
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response, true);
        assert_eq!(
            4,
            response.messages.len(),
            "the correct number of messages should be produced",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker",
                    to_address,
                    "the asker should receive funds from the match",
                );
                assert_eq!(
                    &second_bid_quote,
                    amount,
                    "the asker should get the correct quote funds",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::WithdrawCoins { marker_denom, coin, recipient }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    marker_denom,
                    "the correct marker should be referenced in the withdraw message",
                );
                assert_eq!(
                    &cosmwasm_std::coin(50, DEFAULT_MARKER_DENOM),
                    coin,
                    "the correct amount of marker funds should be withdrawn",
                );
                assert_eq!(
                    "bidder",
                    recipient.as_str(),
                    "the bidder should receive the marker tokens",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker denom should be referenced in the grant message",
                );
                assert_eq!(
                    "asker",
                    address.as_str(),
                    "the asker should be re-granted its permissions on the marker after the sale",
                );
                assert_eq!(
                    &MockMarker::get_default_owner_permissions(),
                    permissions,
                    "the asker should be returned all of its permissions",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker denom should be referenced in the revoke message",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    address.as_str(),
                    "the contract should have its permissions to the marker revoked after the sale completes",
                );
            },
            msg => panic!("unexpected message: {:?}", msg),
        });
    }

    fn do_scope_trade_test<S: Into<String>>(match_sender_address: S, mismatched_quotes: bool) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        deps.querier
            .with_scope(MockScope::new_with_owner(MOCK_CONTRACT_ADDR));
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id", DEFAULT_SCOPE_ADDR, &coins(420, "quote")),
            None,
        )
        .expect("the ask should be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        let bid_quote = if mismatched_quotes {
            coins(12345, "countcoin")
        } else {
            coins(420, "quote")
        };
        create_bid(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &bid_quote),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ADDR),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let sender_address = match_sender_address.into();
        if mismatched_quotes {
            execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(&sender_address, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                Some(AdminMatchOptions::scope_trade_options(false)),
            )
            .expect_err(
                "an error should be returned when the bid quote does not match the ask quote",
            );
            execute_match(
                deps.as_mut(),
                mock_env(),
                mock_info(&sender_address, &[]),
                "ask_id".to_string(),
                "bid_id".to_string(),
                None,
            ).expect_err(
                "an error should be returned when the bid quote does not match the ask quote and no value is provided in the mismatch flag",
            );
        }
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
            // Only the admin may provide admin match options.  Otherwise, the request will be rejected
            if sender_address == DEFAULT_ADMIN_ADDRESS {
                Some(AdminMatchOptions::scope_trade_options(mismatched_quotes))
            } else {
                None
            },
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response, true);
        assert_eq!(
            2,
            response.messages.len(),
            "the correct number of messages should be produced",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker",
                    to_address,
                    "the asker should receive funds after the match",
                );
                assert_eq!(
                    &bid_quote,
                    amount,
                    "{}",
                    if mismatched_quotes {
                        "the asker should get the correct mismatched quote funds"
                    } else {
                        "the asker should get the correct quote funds"
                    },
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Metadata(MetadataMsgParams::WriteScope { scope, signers }), .. }) => {
                assert_eq!(
                    "bidder",
                    scope.value_owner_address,
                    "the bidder should be the new value owner of the scope",
                );
                assert_eq!(
                    1,
                    scope.owners.len(),
                    "there should be a single owner listed on the scope",
                );
                let owner = scope.owners.first().unwrap();
                assert_eq!(
                    "bidder",
                    owner.address.as_str(),
                    "the bidder should be listed as the sole owner in the scope's owner vector",
                );
                assert_eq!(
                    PartyType::Owner,
                    owner.role,
                    "the role of the bidder's owner vector entry should be that own PartyType::Owner",
                );
                assert_eq!(
                    1,
                    signers.len(),
                    "the write scope message should include a single signer",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    signers.first().unwrap().as_str(),
                    "the contract should be listed as the signer on the write scope message",
                );
            },
            msg => panic!("unexpected message: {:?}", msg),
        })
    }
}
