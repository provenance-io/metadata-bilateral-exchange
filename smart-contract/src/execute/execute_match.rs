use crate::storage::ask_order_storage::{
    delete_ask_order_by_id, get_ask_order_by_id, update_ask_order,
};
use crate::storage::bid_order_storage::{delete_bid_order_by_id, get_bid_order_by_id};
use crate::storage::contract_info::get_contract_info;
use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask_collateral::{
    AskCollateral, CoinTradeAskCollateral, MarkerShareSaleAskCollateral, MarkerTradeAskCollateral,
    ScopeTradeAskCollateral,
};
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::types::request::bid_types::bid_collateral::{
    CoinTradeBidCollateral, MarkerShareSaleBidCollateral, MarkerTradeBidCollateral,
    ScopeTradeBidCollateral,
};
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::share_sale_type::ShareSaleType;
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::{release_marker_from_contract, replace_scope_owner};
use crate::validation::execute_match_validation::validate_match;
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
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // only the admin may execute matches
    if info.sender != get_contract_info(deps.storage)?.admin {
        return ContractError::unauthorized().to_err();
    }
    // return error if funds sent
    if !info.funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "funds should not be provided during match execution",
        )
        .to_err();
    }
    let mut invalid_fields: Vec<String> = vec![];
    if ask_id.is_empty() {
        invalid_fields.push("ask id must not be empty".to_string());
    }
    if bid_id.is_empty() {
        invalid_fields.push("bid id must not be empty".to_string());
    }
    // return error if either ids are badly formed
    if !invalid_fields.is_empty() {
        return ContractError::validation_error(&invalid_fields).to_err();
    }

    let ask_order = get_ask_order_by_id(deps.storage, ask_id)?;
    let bid_order = get_bid_order_by_id(deps.storage, bid_id)?;

    validate_match(&deps, &ask_order, &bid_order)?;

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
        .to_ok()
}

struct ExecuteResults {
    pub messages: Vec<CosmosMsg<ProvenanceMsg>>,
}
impl ExecuteResults {
    fn new(messages: Vec<CosmosMsg<ProvenanceMsg>>) -> Self {
        Self { messages }
    }
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
    ExecuteResults::new(vec![
        CosmosMsg::Bank(BankMsg::Send {
            to_address: ask_order.owner.to_string(),
            amount: ask_collateral.quote.to_owned(),
        }),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: bid_order.owner.to_string(),
            amount: bid_collateral.base.to_owned(),
        }),
    ])
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
            &ask_collateral.denom,
            &env.contract.address,
            &[bidder_permissions],
        )?);
    } else {
        return ContractError::validation_error(&[
            "failed to find access permissions in the revoked permissions for the asker"
                .to_string(),
        ])
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
    ExecuteResults::new(messages).to_ok()
}

fn execute_marker_share_sale(
    deps: DepsMut<ProvenanceQuery>,
    env: &Env,
    ask_order: &AskOrder,
    bid_order: &BidOrder,
    ask_collateral: &MarkerShareSaleAskCollateral,
    bid_collateral: &MarkerShareSaleBidCollateral,
) -> Result<ExecuteResults, ContractError> {
    // Asker gets the quote that the bidder provided from escrow
    // Bidder gets their X marker coins withdrawn to them from the contract-controlled marker
    let mut messages = vec![
        CosmosMsg::Bank(BankMsg::Send {
            to_address: ask_order.owner.to_string(),
            amount: bid_collateral.quote.to_owned(),
        }),
        withdraw_coins(
            &ask_collateral.denom,
            bid_collateral.share_count.u128(),
            &ask_collateral.denom,
            bid_order.owner.to_owned(),
        )?,
    ];
    let mut terminate_sale = || -> Result<(), ContractError> {
        // Marker gets released to the asker.  The sale is effectively over.
        messages.append(&mut release_marker_from_contract(
            &ask_collateral.denom,
            &env.contract.address,
            &ask_collateral.removed_permissions,
        )?);
        delete_ask_order_by_id(deps.storage, &ask_order.id)?;
        ().to_ok()
    };
    match ask_collateral.sale_type {
        ShareSaleType::SingleTransaction { .. } => terminate_sale()?,
        ShareSaleType::MultipleTransactions {
            remove_sale_share_threshold,
        } => {
            let share_threshold = remove_sale_share_threshold.map(|t| t.u128()).unwrap_or(0);
            let shares_remaining_after_sale =
                ask_collateral.remaining_shares.u128() - bid_collateral.share_count.u128();
            // Validation will prevent this code from being executed if shares_remaining_after_sale
            // is ever less than share_threshold, so only an equality check is necessary
            if share_threshold == shares_remaining_after_sale {
                terminate_sale()?;
            } else {
                let mut ask_order = ask_order.to_owned();
                let mut ask_collateral = ask_collateral.to_owned();
                ask_collateral.remaining_shares = Uint128::new(shares_remaining_after_sale);
                ask_order.collateral = AskCollateral::MarkerShareSale(ask_collateral);
                // Replace the ask order in storage with an updated remaining_shares value
                update_ask_order(deps.storage, &ask_order)?;
            }
        }
    }
    // Regardless of sale type scenario, the bid is always deleted after successful execution
    delete_bid_order_by_id(deps.storage, &bid_order.id)?;
    ExecuteResults::new(messages).to_ok()
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
        amount: ask_collateral.quote.to_owned(),
    })];
    let scope = ProvenanceQuerier::new(&deps.querier).get_scope(&bid_collateral.scope_address)?;
    // Bidder gets the scope transferred to them
    messages.push(write_scope(
        replace_scope_owner(scope, bid_order.owner.to_owned()),
        vec![env.contract.address.to_owned()],
    )?);
    // Remove the ask and bid orders now that the trade has been finalized
    delete_ask_order_by_id(deps.storage, &ask_order.id)?;
    delete_bid_order_by_id(deps.storage, &bid_order.id)?;
    ExecuteResults::new(messages).to_ok()
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
    use crate::test::mock_scope::{MockScope, DEFAULT_SCOPE_ID};
    use crate::test::request_helpers::{mock_ask_order, mock_bid_order, mock_bid_scope_trade};
    use crate::types::core::error::ContractError;
    use crate::types::request::ask_types::ask::Ask;
    use crate::types::request::ask_types::ask_collateral::AskCollateral;
    use crate::types::request::bid_types::bid::Bid;
    use crate::types::request::share_sale_type::ShareSaleType;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, BankMsg, CosmosMsg, Response, Storage};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        MarkerMsgParams, MetadataMsgParams, PartyType, ProvenanceMsg, ProvenanceMsgParams,
    };

    #[test]
    fn test_execute_match_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info("not-admin", &[]),
            "ask".to_string(),
            "bid".to_string(),
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
            mock_info(DEFAULT_ADMIN_ADDRESS, &coins(100, "nhash")),
            "ask_id".to_string(),
            "bid_id".to_string(),
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
            String::new(),
            "bid_id".to_string(),
        )
        .expect_err("an error should occur when the ask id is empty");
        assert_validation_error_message(err, "ask id must not be empty");
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            String::new(),
        )
        .expect_err("an error should occur when the bid id is empty");
        assert_validation_error_message(err, "bid id must not be empty");
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
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
        let bid_order = mock_bid_order(mock_bid_scope_trade(DEFAULT_SCOPE_ID, &[]));
        insert_bid_order(deps.as_mut().storage, &bid_order)
            .expect("expected bid order insert to succeed");
        let err = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
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
    fn test_execute_coin_trade_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, "base")),
            Ask::new_coin_trade("ask_id", &coins(100, "quote")),
            None,
        )
        .expect("the ask should be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(100, "quote")),
            Bid::new_coin_trade("bid_id", &coins(100, "base")),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response);
        assert_eq!(
            2,
            response.messages.len(),
            "the correct number of messages should be produced",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => match to_address.as_str() {
                "asker" => {
                    assert_eq!(
                        &coins(100, "quote"),
                        amount,
                        "the asker should get the quote funds",
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

    #[test]
    fn test_execute_marker_trade_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
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
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(DEFAULT_MARKER_HOLDINGS, "quote")),
            Bid::new_marker_trade("bid_id", DEFAULT_MARKER_DENOM),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response);
        assert_eq!(
            3,
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
                    &coins(DEFAULT_MARKER_HOLDINGS, "quote"),
                    amount,
                    "the correct quote funds should be sent to the asker",
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
            msg => panic!("unexpected message: {:?}", msg),
        });
    }

    #[test]
    fn test_marker_share_sale_single_tx_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                &coins(1, "quote"),
                ShareSaleType::single(15),
            ),
            None,
        )
        .expect("the ask should be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(15, "quote")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 15),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response);
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
                    &coins(15, "quote"),
                    amount,
                    "the asker should receive the correct amount of quote funds",
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

    #[test]
    fn test_marker_share_sale_multiple_tx_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                "ask_id",
                DEFAULT_MARKER_DENOM,
                &coins(1, "quote"),
                ShareSaleType::multiple(None),
            ),
            None,
        )
        .expect("the ask should be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        // Create the first bid that only buys half the shares for sale
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(50, "quote")),
            Bid::new_marker_share_sale("bid_id_1", DEFAULT_MARKER_DENOM, 50),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id_1").expect("bid order should exist");
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id_1".to_string(),
        )
        .expect("the match should execute successfully");
        assert_eq!(
            3,
            response.attributes.len(),
            "the correct number of attributes should be produced in the response",
        );
        assert_eq!(
            "execute",
            single_attribute_for_key(&response, "action"),
            "the correct action attribute value should be produced",
        );
        assert_eq!(
            "ask_id",
            single_attribute_for_key(&response, "ask_id"),
            "the correct ask_id attribute value should be produced",
        );
        assert_eq!(
            "bid_id_1",
            single_attribute_for_key(&response, "bid_id"),
            "the correct bid_id attribute value should be produced",
        );
        let ask_order = get_ask_order_by_id(deps.as_ref().storage, "ask_id")
            .expect("ask should remain in storage");
        let collateral = ask_order.collateral.unwrap_marker_share_sale();
        assert_eq!(
            50,
            collateral.remaining_shares.u128(),
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
                    &coins(50, "quote"),
                    amount,
                    "the asker should receive the correct amount of quote funds",
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
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(50, "quote")),
            Bid::new_marker_share_sale("bid_id", DEFAULT_MARKER_DENOM, 50),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response);
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
                    &coins(50, "quote"),
                    amount,
                    "the asker should receive the correct amount of quote funds",
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

    #[test]
    fn test_scope_trade_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        deps.querier
            .with_scope(MockScope::new_with_owner(MOCK_CONTRACT_ADDR));
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade("ask_id", DEFAULT_SCOPE_ID, &coins(420, "quote")),
            None,
        )
        .expect("the ask should be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, "ask_id").expect("ask order should exist");
        create_bid(
            deps.as_mut(),
            mock_info("bidder", &coins(420, "quote")),
            Bid::new_scope_trade("bid_id", DEFAULT_SCOPE_ID),
            None,
        )
        .expect("the bid should be created successfully");
        get_bid_order_by_id(deps.as_ref().storage, "bid_id").expect("bid order should exist");
        let response = execute_match(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            "ask_id".to_string(),
            "bid_id".to_string(),
        )
        .expect("the match should execute successfully");
        assert_match_produced_correct_results(deps.as_ref().storage, &response);
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
                    &coins(420, "quote"),
                    amount,
                    "the asker should receive the correct number of funds",
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

    fn assert_match_produced_correct_results(
        storage: &dyn Storage,
        response: &Response<ProvenanceMsg>,
    ) {
        assert_eq!(
            3,
            response.attributes.len(),
            "the correct number of attributes should be produced in the response",
        );
        assert_eq!(
            "execute",
            single_attribute_for_key(response, "action"),
            "the correct action attribute value should be produced",
        );
        assert_eq!(
            "ask_id",
            single_attribute_for_key(response, "ask_id"),
            "the correct ask_id attribute value should be produced",
        );
        assert_eq!(
            "bid_id",
            single_attribute_for_key(response, "bid_id"),
            "the correct bid_id attribute value should be produced",
        );
        get_ask_order_by_id(storage, "ask_id").expect_err("ask should be missing from storage");
        get_bid_order_by_id(storage, "bid_id").expect_err("bid should be missing from storage");
    }
}
