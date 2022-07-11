use crate::storage::ask_order_storage::may_get_ask_order_by_collateral_id;
use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, Binary, Deps};
use provwasm_std::ProvenanceQuery;

pub fn query_ask_by_collateral_id(
    deps: Deps<ProvenanceQuery>,
    collateral_id: String,
) -> Result<Binary, ContractError> {
    to_binary(&may_get_ask_order_by_collateral_id(
        deps.storage,
        collateral_id,
    ))?
    .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::query::get_ask_by_collateral_id::query_ask_by_collateral_id;
    use crate::storage::ask_order_storage::{insert_ask_order, may_get_ask_order_by_collateral_id};
    use crate::test::mock_marker::DEFAULT_MARKER_DENOM;
    use crate::test::request_helpers::{mock_ask_marker_trade, mock_ask_order};
    use crate::types::request::ask_types::ask_order::AskOrder;
    use cosmwasm_std::from_binary;
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_get_ask_by_collateral_id() {
        let mut deps = mock_dependencies(&[]);
        assert_eq!(
            None,
            may_get_ask_order_by_collateral_id(deps.as_ref().storage, "some_fake_id"),
            "no ask order should be found by a collateral id that has not been used",
        );
        // The storage function already has testing for all ask order types, so we just need to
        // ensure the binary portion works here to have a fully-tested set of functionality
        let ask_order = mock_ask_order(mock_ask_marker_trade(
            "collateral_id",
            DEFAULT_MARKER_DENOM,
            100,
            &[],
        ));
        insert_ask_order(deps.as_mut().storage, &ask_order)
            .expect("the ask order should insert successfully");
        let binary = query_ask_by_collateral_id(deps.as_ref(), "collateral_id".to_string())
            .expect("expected binary to be properly produced from the query");
        let deserialized_option_ask_order = from_binary::<Option<AskOrder>>(&binary)
            .expect("expected the binary to deserialize to an optional ask order");
        let deserialized_ask_order = deserialized_option_ask_order
            .unwrap_or_else(|| panic!("the ask order should be present in the optional output"));
        assert_eq!(
            ask_order, deserialized_ask_order,
            "expected the inserted value to be identical to the deserialized value",
        );
    }
}
