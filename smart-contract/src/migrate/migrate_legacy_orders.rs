use crate::migrate::migrate_contract::migrate_contract;
use crate::storage::ask_order_storage::insert_ask_order;
use crate::storage::bid_order_storage::insert_bid_order;
use crate::storage::legacy_ask_order_storage::{
    get_converted_legacy_ask_orders, legacy_ask_orders,
};
use crate::storage::legacy_bid_order_storage::{
    get_converted_legacy_bid_orders, legacy_bid_orders,
};
use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{DepsMut, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

pub fn migrate_legacy_orders(
    deps: DepsMut<ProvenanceQuery>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let migration_response = migrate_contract(&deps)?;
    let converted_ask_orders = get_converted_legacy_ask_orders(deps.storage)?;
    for ask_order in &converted_ask_orders {
        // Remove ask order using the legacy storage, avoiding any schema mismatches between old and new
        legacy_ask_orders().remove(deps.storage, ask_order.id.as_bytes())?;
        // Insert the converted order using the new storage, ensuring new schema is adopted
        insert_ask_order(deps.storage, ask_order)?;
    }
    let converted_bid_orders = get_converted_legacy_bid_orders(deps.storage)?;
    for bid_order in &converted_bid_orders {
        // Remove ask order using the legacy storage, avoiding any schema mismatches between old and new
        legacy_bid_orders().remove(deps.storage, bid_order.id.as_bytes())?;
        // Insert the converted order using the new storage, ensuring new schema is adopted
        insert_bid_order(deps.storage, bid_order)?;
    }
    migration_response
        .add_attribute(
            "converted_ask_orders",
            converted_ask_orders.len().to_string(),
        )
        .add_attribute(
            "converted_bid_orders",
            converted_bid_orders.len().to_string(),
        )
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::migrate::migrate_legacy_orders::migrate_legacy_orders;
    use crate::storage::ask_order_storage::get_ask_order_by_id;
    use crate::storage::bid_order_storage::get_bid_order_by_id;
    use crate::storage::contract_info::CONTRACT_VERSION;
    use crate::storage::legacy_ask_order_storage::legacy_ask_orders;
    use crate::storage::legacy_bid_order_storage::legacy_bid_orders;
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::default_instantiate;
    use crate::types::request::ask_types::legacy_ask_collateral::LegacyAskCollateral;
    use crate::types::request::ask_types::legacy_ask_order::LegacyAskOrder;
    use crate::types::request::bid_types::legacy_bid_collateral::LegacyBidCollateral;
    use crate::types::request::bid_types::legacy_bid_order::LegacyBidOrder;
    use crate::types::request::legacy_share_sale_type::LegacyShareSaleType;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use cosmwasm_std::{coins, Addr, Storage, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_successful_simple_batch_conversion() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let legacy_ask = LegacyAskOrder {
            id: "ask_id".to_string(),
            ask_type: RequestType::MarkerTrade,
            owner: Addr::unchecked("asker"),
            collateral: LegacyAskCollateral::marker_trade(
                Addr::unchecked("marker address"),
                "marker denom",
                100,
                &coins(100, "quote"),
                &[],
            ),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::all(&["something.pb"]),
            )),
        };
        insert_legacy_ask_order(deps.as_mut().storage, &legacy_ask);
        let legacy_bid = LegacyBidOrder {
            id: "bid_id".to_string(),
            bid_type: RequestType::MarkerTrade,
            owner: Addr::unchecked("bidder"),
            collateral: LegacyBidCollateral::marker_trade(
                Addr::unchecked("marker address"),
                "marker denom",
                &coins(100, "quote"),
            ),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "description",
                AttributeRequirement::none(&["NONE.PIO"]),
            )),
        };
        insert_legacy_bid_order(deps.as_mut().storage, &legacy_bid);
        get_ask_order_by_id(deps.as_ref().storage, &legacy_ask.id)
            .expect_err("trying to load the legacy ask order before conversion should fail");
        get_bid_order_by_id(deps.as_ref().storage, &legacy_bid.id)
            .expect_err("trying to load the legacy bid order before conversion should fail");
        let response =
            migrate_legacy_orders(deps.as_mut()).expect("expected the migration to succeed");
        assert!(
            response.messages.is_empty(),
            "migrations should never emit messages",
        );
        assert_eq!(
            4,
            response.attributes.len(),
            "all proper attributes should be emitted by the migration",
        );
        assert_eq!(
            "migrate_contract",
            single_attribute_for_key(&response, "action"),
            "the action attribute should have the correct value",
        );
        assert_eq!(
            CONTRACT_VERSION,
            single_attribute_for_key(&response, "new_version"),
            "the new_version attribute should have the correct value",
        );
        assert_eq!(
            "1",
            single_attribute_for_key(&response, "converted_ask_orders"),
            "the converted_ask_orders attribute should show that a single ask order was converted",
        );
        assert_eq!(
            "1",
            single_attribute_for_key(&response, "converted_bid_orders"),
            "the converted_bid_orders attribute should show that a single bid order was converted",
        );
        let new_ask_order = get_ask_order_by_id(deps.as_mut().storage, &legacy_ask.id)
            .expect("the new ask order should load from storage");
        assert_eq!(
            legacy_ask.to_new_ask_order(),
            new_ask_order,
            "the new ask order should be converted from the old value",
        );
        let new_bid_order = get_bid_order_by_id(deps.as_mut().storage, &legacy_bid.id)
            .expect("the new bid order should load from storage");
        assert_eq!(
            legacy_bid.to_new_bid_order(),
            new_bid_order,
            "the new bid order should be converted from the old value",
        );
    }

    #[test]
    fn test_large_batch_conversion() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let mut legacy_ask_orders = vec![];
        for ask_id in 0..100 {
            let legacy_ask = LegacyAskOrder {
                id: format!("ask_id_{}", ask_id),
                ask_type: RequestType::MarkerShareSale,
                owner: Addr::unchecked(format!("asker_{}", ask_id)),
                collateral: LegacyAskCollateral::marker_share_sale(
                    Addr::unchecked(format!("marker_{}", ask_id)),
                    format!("denom_{}", ask_id),
                    100,
                    &coins(100, "quote"),
                    &[],
                    if ask_id % 2 == 0 {
                        LegacyShareSaleType::SingleTransaction {
                            share_count: Uint128::new(50),
                        }
                    } else {
                        LegacyShareSaleType::MultipleTransactions {
                            remove_sale_share_threshold: Some(Uint128::new(10)),
                        }
                    },
                ),
                descriptor: Some(RequestDescriptor::new_populated_attributes(
                    "description",
                    AttributeRequirement::all(&["something.pb"]),
                )),
            };
            insert_legacy_ask_order(deps.as_mut().storage, &legacy_ask);
            legacy_ask_orders.push(legacy_ask);
        }
        let mut legacy_bid_orders = vec![];
        for bid_id in 0..100 {
            let legacy_bid = LegacyBidOrder {
                id: format!("bid_id_{}", bid_id),
                bid_type: RequestType::MarkerShareSale,
                owner: Addr::unchecked(format!("bidder_{}", bid_id)),
                collateral: LegacyBidCollateral::marker_share_sale(
                    Addr::unchecked(format!("marker_{}", bid_id)),
                    format!("denom_{}", bid_id),
                    100,
                    &coins(100, "quote"),
                ),
                descriptor: Some(RequestDescriptor::new_populated_attributes(
                    "description",
                    AttributeRequirement::none(&["NONE.PIO"]),
                )),
            };
            insert_legacy_bid_order(deps.as_mut().storage, &legacy_bid);
            legacy_bid_orders.push(legacy_bid);
        }
        let response = migrate_legacy_orders(deps.as_mut()).expect("migration should succeed");
        assert!(
            response.messages.is_empty(),
            "migrations should never emit messages",
        );
        assert_eq!(
            4,
            response.attributes.len(),
            "all proper attributes should be emitted by the migration",
        );
        assert_eq!(
            "migrate_contract",
            single_attribute_for_key(&response, "action"),
            "the action attribute should have the correct value",
        );
        assert_eq!(
            CONTRACT_VERSION,
            single_attribute_for_key(&response, "new_version"),
            "the new_version attribute should have the correct value",
        );
        assert_eq!(
            "100",
            single_attribute_for_key(&response, "converted_ask_orders"),
            "the converted_ask_orders attribute should show that all ask orders were converted",
        );
        assert_eq!(
            "100",
            single_attribute_for_key(&response, "converted_bid_orders"),
            "the converted_bid_orders attribute should show that all bid orders were converted",
        );
        for legacy_ask in legacy_ask_orders {
            let new_ask_order = get_ask_order_by_id(deps.as_mut().storage, &legacy_ask.id)
                .unwrap_or_else(|e| {
                    panic!(
                        "the new ask order with id [{}] should load from storage, got error: {:?}",
                        legacy_ask.id, e,
                    )
                });
            assert_eq!(
                legacy_ask.to_new_ask_order(),
                new_ask_order,
                "the new ask order with id [{}] should be converted from the old value",
                new_ask_order.id,
            );
        }
        for legacy_bid in legacy_bid_orders {
            let new_bid_order = get_bid_order_by_id(deps.as_mut().storage, &legacy_bid.id)
                .unwrap_or_else(|e| {
                    panic!(
                        "the new bid order with id [{}] should load from storage, got error: {:?}",
                        legacy_bid.id, e
                    )
                });
            assert_eq!(
                legacy_bid.to_new_bid_order(),
                new_bid_order,
                "the new bid order with id [{}] should be converted from the old value",
                new_bid_order.id,
            );
        }
    }

    fn insert_legacy_ask_order(storage: &mut dyn Storage, ask_order: &LegacyAskOrder) {
        let state = legacy_ask_orders();
        if let Ok(existing_ask) = state.load(storage, ask_order.get_pk()) {
            panic!(
                "an ask with id [{}] for owner [{}] already exists",
                existing_ask.id,
                existing_ask.owner.as_str(),
            );
        }
        legacy_ask_orders()
            .replace(storage, ask_order.get_pk(), Some(ask_order), None)
            .expect("legacy ask order insert should succeed");
        let inserted_ask_order = state
            .load(storage, ask_order.id.as_bytes())
            .expect("expected legacy ask order to be available in storage");
        assert_eq!(
            &inserted_ask_order, ask_order,
            "expected the inserted value to equate to the value fetched",
        );
    }

    fn insert_legacy_bid_order(storage: &mut dyn Storage, bid_order: &LegacyBidOrder) {
        let state = legacy_bid_orders();
        if let Ok(existing_bid) = state.load(storage, bid_order.get_pk()) {
            panic!(
                "a bid with id [{}] for owner [{}] already exists",
                existing_bid.id,
                existing_bid.owner.as_str()
            );
        }
        legacy_bid_orders()
            .replace(storage, bid_order.get_pk(), Some(bid_order), None)
            .expect("legacy bid order insert should succeed");
        let inserted_bid_order = state
            .load(storage, bid_order.id.as_bytes())
            .expect("expected legacy bid order to be available in storage");
        assert_eq!(
            &inserted_bid_order, bid_order,
            "expected the inserted value to equate to the value fetched",
        );
    }
}
