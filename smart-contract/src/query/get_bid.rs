use crate::storage::bid_order_storage::may_get_bid_order_by_id;
use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, Binary, Deps};
use provwasm_std::ProvenanceQuery;

pub fn query_bid(deps: Deps<ProvenanceQuery>, id: String) -> Result<Binary, ContractError> {
    to_binary(&may_get_bid_order_by_id(deps.storage, id))?.to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::query;
    use crate::storage::bid_order_storage::insert_bid_order;
    use crate::test::mock_instantiate::default_instantiate;
    use crate::types::core::msg::QueryMsg;
    use crate::types::request::bid_types::bid_collateral::BidCollateral;
    use crate::types::request::bid_types::bid_order::BidOrder;
    use crate::types::request::request_descriptor::RequestDescriptor;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{coins, from_binary, Addr};
    use provwasm_mocks::mock_dependencies;

    #[test]
    pub fn query_with_valid_data() {
        // setup
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);

        // store valid bid order
        let bid_order = BidOrder::new_unchecked(
            "bid_id",
            Addr::unchecked("bidder"),
            BidCollateral::coin_trade(&coins(100, "base_1"), &coins(100, "quote_1")),
            Some(RequestDescriptor::basic("description words")),
        );

        if let Err(error) = insert_bid_order(deps.as_mut().storage, &bid_order) {
            panic!("unexpected error: {:?}", error);
        };

        // query for bid order
        let query_bid_response = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetBid {
                id: bid_order.id.clone(),
            },
        )
        .expect("expected the query to execute successfully");

        let deserialized_bid_order = from_binary::<Option<BidOrder>>(&query_bid_response)
            .expect("the binary result should successfully deserialize to an optional ask order")
            .expect("the option should successfully unwrap to an ask order");

        assert_eq!(
            bid_order, deserialized_bid_order,
            "the deserialized value should equate to the inserted value",
        );
    }
}
