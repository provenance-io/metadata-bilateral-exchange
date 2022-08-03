use crate::storage::ask_order_storage::get_ask_orders_by_collateral_id;
use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, Binary, Deps};
use provwasm_std::ProvenanceQuery;

pub fn query_asks_by_collateral_id(
    deps: Deps<ProvenanceQuery>,
    collateral_id: String,
) -> Result<Binary, ContractError> {
    to_binary(&get_ask_orders_by_collateral_id(
        deps.storage,
        collateral_id,
    ))?
    .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::query::get_asks_by_collateral_id::query_asks_by_collateral_id;
    use crate::storage::ask_order_storage::insert_ask_order;
    use crate::test::mock_marker::{DEFAULT_MARKER_ADDRESS, DEFAULT_MARKER_DENOM};
    use crate::test::request_helpers::mock_ask_marker_share_sale;
    use crate::types::request::ask_types::ask_order::AskOrder;
    use crate::types::request::share_sale_type::ShareSaleType;
    use cosmwasm_std::{from_binary, Addr};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_get_ask_by_collateral_id_no_results() {
        let deps = mock_dependencies(&[]);
        let empty_binary =
            query_asks_by_collateral_id(deps.as_ref(), DEFAULT_MARKER_ADDRESS.to_string())
                .expect("the result should be returned");
        let empty_ask_order_vector = from_binary::<Vec<AskOrder>>(&empty_binary)
            .expect("result should properly deserialize from binary");
        assert!(
            empty_ask_order_vector.is_empty(),
            "the result should be an empty vector",
        );
    }

    #[test]
    fn test_get_ask_by_collateral_id_with_results() {
        let mut deps = mock_dependencies(&[]);
        let ask_order = AskOrder::new_unchecked(
            "ask_id_1",
            Addr::unchecked("asker"),
            mock_ask_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                50,
                25,
                &[],
                ShareSaleType::MultipleTransactions,
            ),
            None,
        );
        insert_ask_order(deps.as_mut().storage, &ask_order)
            .expect("the ask order should be inserted without error");
        let binary = query_asks_by_collateral_id(deps.as_ref(), DEFAULT_MARKER_ADDRESS.to_string())
            .expect("a binary result should be returned");
        let ask_order_vector = from_binary::<Vec<AskOrder>>(&binary)
            .expect("the result should properly deserialize from binary");
        assert_eq!(
            1,
            ask_order_vector.len(),
            "the ask order vector should contain only a single result",
        );
        assert_eq!(
            &ask_order,
            ask_order_vector.first().unwrap(),
            "expected the ask order to be contained within the vector",
        );
        let second_ask_order = AskOrder::new_unchecked(
            "ask_id_2",
            Addr::unchecked("asker"),
            mock_ask_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                30,
                30,
                &[],
                ShareSaleType::SingleTransaction,
            ),
            None,
        );
        insert_ask_order(deps.as_mut().storage, &second_ask_order)
            .expect("the second ask order should be inserted without error");
        let binary = query_asks_by_collateral_id(deps.as_ref(), DEFAULT_MARKER_ADDRESS.to_string())
            .expect("a binary result should be returned");
        let ask_order_vector = from_binary::<Vec<AskOrder>>(&binary)
            .expect("the result should properly deserialize from binary");
        assert_eq!(
            2,
            ask_order_vector.len(),
            "the ask order vector should contain both related results",
        );
        assert!(
            ask_order_vector.contains(&ask_order),
            "the first ask order should be contained in the results",
        );
        assert!(
            ask_order_vector.contains(&second_ask_order),
            "the second ask order should be contained in the results",
        );
    }
}
