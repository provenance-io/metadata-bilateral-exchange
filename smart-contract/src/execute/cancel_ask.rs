use crate::storage::ask_order_storage::{
    delete_ask_order_by_id, get_ask_order_by_id, get_ask_orders_by_collateral_id,
};
use crate::storage::contract_info::get_contract_info;
use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask_collateral::AskCollateral;
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::{release_marker_from_contract, replace_scope_owner};
use cosmwasm_std::{to_binary, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo, Response};
use provwasm_std::{write_scope, ProvenanceMsg, ProvenanceQuerier, ProvenanceQuery};

// cancel ask entrypoint
pub fn cancel_ask(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // return error if id is empty
    if id.is_empty() {
        return ContractError::ValidationError {
            messages: vec!["an id must be provided when cancelling an ask".to_string()],
        }
        .to_err();
    }
    // return error if funds sent
    if !info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "funds should not be provided when cancelling an ask".to_string(),
        }
        .to_err();
    }
    let ask_order = get_ask_order_by_id(deps.storage, &id)?;
    // Only the owner of the ask and the admin can cancel an ask
    if info.sender != ask_order.owner && info.sender != get_contract_info(deps.storage)?.admin {
        return ContractError::Unauthorized.to_err();
    }
    let mut messages: Vec<CosmosMsg<ProvenanceMsg>> = vec![];
    match &ask_order.collateral {
        AskCollateral::CoinTrade(collateral) => {
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: ask_order.owner.to_string(),
                amount: collateral.base.to_owned(),
            }));
        }
        AskCollateral::MarkerTrade(collateral) => {
            messages.append(&mut release_marker_from_contract(
                &collateral.marker_denom,
                &env.contract.address,
                &collateral.removed_permissions,
            )?);
        }
        AskCollateral::MarkerShareSale(collateral) => {
            // Only release the marker if this is the final remaining ask for the given marker.
            // Multiple marker share sales can be created for a single marker while it is held by
            // the contract, so this check ensures that the marker is only relinquished when the
            // final sale is cancelled.
            if get_ask_orders_by_collateral_id(deps.storage, collateral.marker_address.as_str())
                .len()
                <= 1
            {
                messages.append(&mut release_marker_from_contract(
                    &collateral.marker_denom,
                    &env.contract.address,
                    &collateral.removed_permissions,
                )?);
            }
        }
        AskCollateral::ScopeTrade(collateral) => {
            let mut scope =
                ProvenanceQuerier::new(&deps.querier).get_scope(&collateral.scope_address)?;
            scope = replace_scope_owner(scope, ask_order.owner.to_owned());
            messages.push(write_scope(scope, vec![env.contract.address])?);
        }
    }
    delete_ask_order_by_id(deps.storage, &ask_order.id)?;
    Response::new()
        .add_messages(messages)
        .add_attribute("action", "cancel_ask")
        .add_attribute("ask_id", &ask_order.id)
        .set_data(to_binary(&ask_order)?)
        .to_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::execute;
    use crate::execute::create_ask::create_ask;
    use crate::storage::ask_order_storage::insert_ask_order;
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::{default_instantiate, DEFAULT_ADMIN_ADDRESS};
    use crate::test::mock_marker::{MockMarker, DEFAULT_MARKER_DENOM, DEFAULT_MARKER_HOLDINGS};
    use crate::test::mock_scope::{MockScope, DEFAULT_SCOPE_ADDR};
    use crate::test::request_helpers::mock_ask_order;
    use crate::types::core::msg::ExecuteMsg;
    use crate::types::request::ask_types::ask::Ask;
    use crate::types::request::ask_types::ask_order::AskOrder;
    use crate::types::request::share_sale_type::ShareSaleType;
    use crate::util::constants::NHASH;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary, CosmosMsg, Storage};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{MarkerMsgParams, MetadataMsgParams, PartyType, ProvenanceMsgParams};

    #[test]
    fn cancel_coin_ask_as_asker() {
        do_coin_trade_cancel_ask("asker");
    }

    #[test]
    fn test_cancel_ask_as_admin() {
        do_coin_trade_cancel_ask(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_cancel_marker_trade_ask_as_asker() {
        do_marker_trade_cancel_ask("asker");
    }

    #[test]
    fn test_cancel_marker_trade_ask_as_admin() {
        do_marker_trade_cancel_ask(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_cancel_marker_share_sale_ask_as_asker() {
        do_marker_share_sale_cancel_ask("asker");
    }

    #[test]
    fn test_cancel_marker_share_sale_ask_as_admin() {
        do_marker_share_sale_cancel_ask(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_cancel_scope_trade_ask_as_asker() {
        do_scope_trade_cancel_test("asker");
    }

    #[test]
    fn test_cancel_scope_trade_ask_as_admin() {
        do_scope_trade_cancel_test(DEFAULT_ADMIN_ADDRESS);
    }

    #[test]
    fn test_cancel_ask_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let err = cancel_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            String::new(),
        )
        .expect_err("an error should occur when no id is provided");
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(
                    1,
                    messages.len(),
                    "only one validation error message should be produced",
                );
                assert_eq!(
                    "an id must be provided when cancelling an ask",
                    messages.first().unwrap(),
                    "the correct validation error should be produced",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let err = cancel_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &coins(100, NHASH)),
            "ask_id".to_string(),
        )
        .expect_err("an error should occur when funds are provided");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds error should be produced when funds are provided, but got: {:?}",
            err,
        );
        let err = cancel_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            "ask_id".to_string(),
        )
        .expect_err("an error should occur when no ask can be found by the provided id");
        assert!(
            matches!(err, ContractError::StorageError { .. }),
            "a storage error should be produced when no ask can be found by the provided id, but got: {:?}",
            err,
        );
        let ask_order = mock_ask_order(AskCollateral::coin_trade(&[], &[]));
        insert_ask_order(deps.as_mut().storage, &ask_order)
            .expect("inserting an ask order should succeed");
        assert_eq!(
            "ask_id", ask_order.id,
            "sanity check: the mock ask order should have the expected default id",
        );
        let err = cancel_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("impostor", &[]),
            "ask_id".to_string(),
        ).expect_err("an error should be produced when an account other than the asker or admin tries to cancel an ask");
        assert!(
            matches!(err, ContractError::Unauthorized),
            "an unauthorized error should be produced when an invalid account tries to cancel an ask, but got: {:?}",
            err,
        );
    }

    fn assert_cancel_ask_succeeded(storage: &dyn Storage, response: &Response<ProvenanceMsg>) {
        assert_eq!(
            2,
            response.attributes.len(),
            "the response should have the correct number of attributes",
        );
        assert_eq!("cancel_ask", single_attribute_for_key(response, "action"),);
        assert_eq!("ask_id", single_attribute_for_key(response, "ask_id"),);
        let response_data_ask_order = if let Some(ref binary) = response.data {
            from_binary::<AskOrder>(binary).expect("response data deserialize correctly")
        } else {
            panic!("response data should be set");
        };
        get_ask_order_by_id(storage, &response_data_ask_order.id)
            .expect_err("the ask should no longer be available in storage after a cancellation");
    }

    fn do_coin_trade_cancel_ask<S: Into<String>>(sender_address: S) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());

        // create ask data
        let asker_info = mock_info("asker", &coins(200, "base_1"));

        let create_ask_msg = ExecuteMsg::CreateAsk {
            ask: Ask::new_coin_trade("ask_id", &coins(100, "quote_1")),
            descriptor: None,
        };

        // execute create ask
        if let Err(error) = execute(deps.as_mut(), mock_env(), asker_info, create_ask_msg) {
            panic!("unexpected error: {:?}", error)
        }

        // verify ask order stored
        assert!(get_ask_order_by_id(deps.as_ref().storage, "ask_id").is_ok());

        // cancel ask order
        let asker_info = mock_info(&sender_address.into(), &[]);

        let cancel_ask_msg = ExecuteMsg::CancelAsk {
            id: "ask_id".to_string(),
        };
        let response = execute(
            deps.as_mut(),
            mock_env(),
            asker_info.clone(),
            cancel_ask_msg,
        )
        .expect("expected the coin trade ask to be cancelled successfully");
        assert_cancel_ask_succeeded(deps.as_ref().storage, &response);
        assert_eq!(
            1,
            response.messages.len(),
            "expected only a single message to be sent when cancelling a coin trade",
        );
        match &response.messages.first().unwrap().msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    "asker", to_address,
                    "the asker address should be the target of the send message",
                );
                assert_eq!(
                    &coins(200, "base_1"),
                    amount,
                    "the sent amount should be correct",
                );
            }
            msg => panic!("unexpected message in response: {:?}", msg),
        }
    }

    fn do_marker_trade_cancel_ask<S: Into<String>>(sender_address: S) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let ask_id = "ask_id".to_string();
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_trade(&ask_id, DEFAULT_MARKER_DENOM, &coins(150, NHASH)),
            None,
        )
        .expect("marker trade ask should be created without issue");
        get_ask_order_by_id(&mut deps.storage, &ask_id)
            .expect("an ask order should be available in storage");
        let response = cancel_ask(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address.into(), &[]),
            ask_id,
        )
        .expect("cancel ask should succeed");
        assert_cancel_ask_succeeded(deps.as_ref().storage, &response);
        assert_marker_response_sent_proper_messages(&response);
    }

    fn do_marker_share_sale_cancel_ask<S: Into<String>>(sender_address: S) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let ask_id = "ask_id".to_string();
        deps.querier
            .with_markers(vec![MockMarker::new_owned_marker("asker")]);
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_marker_share_sale(
                &ask_id,
                DEFAULT_MARKER_DENOM,
                DEFAULT_MARKER_HOLDINGS,
                &coins(150, NHASH),
                ShareSaleType::SingleTransaction,
            ),
            None,
        )
        .expect("expected the marker share sale to be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, &ask_id)
            .expect("an ask order should be available in storage");
        let response = cancel_ask(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address.into(), &[]),
            ask_id,
        )
        .expect("expected cancel ask to succeed");
        assert_cancel_ask_succeeded(deps.as_ref().storage, &response);
        assert_marker_response_sent_proper_messages(&response);
    }

    fn assert_marker_response_sent_proper_messages(response: &Response<ProvenanceMsg>) {
        assert_eq!(
            2,
            response.messages.len(),
            "two message should be added to the response to properly rewrite the marker to its original owner permissions",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Custom(ProvenanceMsg {
                                  params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess {
                                                                          denom,
                                                                          address,
                                                                          ..
                                                                      }),
                                  ..
                              }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker denom should be referenced in the grant access request",
                );
                assert_eq!(
                    "asker",
                    address.as_str(),
                    "the asker account should be granted its marker access again",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg {
                                  params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess {
                                                                          denom,
                                                                          address,
                                                                      }),
                                  ..
                              }) => {
                assert_eq!(
                    DEFAULT_MARKER_DENOM,
                    denom,
                    "the correct marker denom should be referenced in the revoke access request",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    address.as_str(),
                    "the contract address should be used to revoke its marker access",
                );
            },
            msg => panic!("unexpected message produced when cancelling a marker ask: {:?}", msg),
        });
    }

    fn do_scope_trade_cancel_test<S: Into<String>>(sender_address: S) {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let ask_id = "ask_id".to_string();
        deps.querier
            .with_scope(MockScope::new_with_owner(MOCK_CONTRACT_ADDR));
        create_ask(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]),
            Ask::new_scope_trade(&ask_id, DEFAULT_SCOPE_ADDR, &coins(100, NHASH)),
            None,
        )
        .expect("expected the scope trade to be created successfully");
        get_ask_order_by_id(deps.as_ref().storage, &ask_id)
            .expect("an ask order should be available in storage");
        let response = cancel_ask(
            deps.as_mut(),
            mock_env(),
            mock_info(&sender_address.into(), &[]),
            ask_id,
        )
        .expect("expected cancel ask to succeed");
        assert_cancel_ask_succeeded(deps.as_ref().storage, &response);
        assert_eq!(
            1,
            response.messages.len(),
            "a scope trade cancellation should send the correct number of messages",
        );
        match &response.messages.first().unwrap().msg {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Metadata(MetadataMsgParams::WriteScope { scope, signers }),
                ..
            }) => {
                assert_eq!(
                    "asker",
                    scope.value_owner_address.as_str(),
                    "the asker should be assigned as the scope value owner",
                );
                assert_eq!(
                    1,
                    scope.owners.len(),
                    "there should only be a single scope owner",
                );
                let owner = scope.owners.first().unwrap();
                assert_eq!(
                    "asker",
                    owner.address.as_str(),
                    "the owner address should be the asker",
                );
                assert_eq!(
                    PartyType::Owner,
                    owner.role,
                    "the role of the owner should be PartyType::Owner",
                );
                assert_eq!(
                    1,
                    signers.len(),
                    "there should be a single signer for the scope write",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    signers.first().unwrap().as_str(),
                    "the contract's address should be the signer on the scope write",
                );
            }
            msg => panic!("unexpected message when cancelling scope trade: {:?}", msg),
        };
    }
}
