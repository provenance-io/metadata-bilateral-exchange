use crate::storage::ask_order_storage::may_get_ask_order_by_id;
use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, Binary, Deps};
use provwasm_std::ProvenanceQuery;

pub fn query_ask(deps: Deps<ProvenanceQuery>, id: String) -> Result<Binary, ContractError> {
    to_binary(&may_get_ask_order_by_id(deps.storage, id))?.to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::query;
    use crate::storage::ask_order_storage::insert_ask_order;
    use crate::test::mock_instantiate::default_instantiate;
    use crate::types::core::msg::QueryMsg;
    use crate::types::request::ask_types::ask_collateral::AskCollateral;
    use crate::types::request::ask_types::ask_order::AskOrder;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{coins, from_binary, Addr};
    use provwasm_mocks::mock_dependencies;

    #[test]
    pub fn query_with_valid_data() {
        // setup
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());

        // store valid ask order
        let ask_order = AskOrder {
            id: "ask_id".into(),
            ask_type: RequestType::CoinTrade,
            owner: Addr::unchecked("asker"),
            collateral: AskCollateral::coin_trade(&coins(200, "base_1"), &coins(100, "quote_1")),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "a very nice description",
                AttributeRequirement::all(&["some.attribute.pb"]),
            )),
        };

        if let Err(error) = insert_ask_order(deps.as_mut().storage, &ask_order) {
            panic!("unexpected error: {:?}", error)
        };

        // query for ask order
        let query_ask_response = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetAsk {
                id: ask_order.id.clone(),
            },
        )
        .expect("expected the query to execute successfully");

        let deserialized_ask_order = from_binary::<Option<AskOrder>>(&query_ask_response)
            .expect("binary deserialization to an optional ask order should succeed")
            .expect("the optional ask order should successfully unwrap to an ask order");
        assert_eq!(
            ask_order, deserialized_ask_order,
            "the deserialized value should equate to the inserted value",
        );
    }
}
