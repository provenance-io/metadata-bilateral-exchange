use crate::storage::ask_order_storage::{ask_orders, update_ask_order};
use crate::storage::contract_info::{
    get_contract_info, set_contract_info, ContractInfoV2, CONTRACT_TYPE, CONTRACT_VERSION,
};
use crate::types::core::constants::DEFAULT_SEARCH_ORDER;
use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, DepsMut, Response, StdError};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};
use semver::Version;

pub fn migrate_contract(
    mut deps: DepsMut<ProvenanceQuery>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let mut contract_info = get_contract_info(deps.storage)?;
    check_valid_migration_target(&contract_info)?;
    contract_info.contract_version = CONTRACT_VERSION.to_string();
    set_contract_info(deps.storage, &contract_info)?;
    migrate_ask_order_indices(&mut deps)?;
    Response::new()
        .add_attribute("action", "migrate_contract")
        .add_attribute("new_version", CONTRACT_VERSION)
        .set_data(to_binary(&contract_info)?)
        .to_ok()
}

// TODO: Remove this in v1.1.1.  This function rewrites all ask orders with the new collateral index.
// The previous value was a UniqueIndex, but v1.1.0 includes the ability to have multiple ask orders
// for the same marker, which prevents collateral from being unique.
fn migrate_ask_order_indices(deps: &mut DepsMut<ProvenanceQuery>) -> Result<(), ContractError> {
    let ask_order_results = ask_orders()
        .range(deps.storage, None, None, DEFAULT_SEARCH_ORDER)
        .map(|result| result.map(|(_, ask_order)| ask_order))
        .collect::<Vec<Result<AskOrder, StdError>>>();
    for ask_order_result in ask_order_results {
        update_ask_order(deps.storage, &ask_order_result?)?;
    }
    ().to_ok()
}

fn check_valid_migration_target(contract_info: &ContractInfoV2) -> Result<(), ContractError> {
    // Prevent other contracts from being migrated over this one
    if CONTRACT_TYPE != contract_info.contract_type {
        return ContractError::InvalidMigration {
            message: format!(
                "target migration contract type [{}] does not match stored contract type [{}]",
                CONTRACT_TYPE, contract_info.contract_type,
            ),
        }
        .to_err();
    }
    let existing_contract_version = contract_info.contract_version.parse::<Version>()?;
    let new_contract_version = CONTRACT_VERSION.parse::<Version>()?;
    // Ensure only new contract versions are migrated to
    if existing_contract_version >= new_contract_version {
        return ContractError::InvalidMigration {
            message: format!(
                "target migration contract version [{}] is too low to use. stored contract version is [{}]",
                CONTRACT_VERSION, &contract_info.contract_version,
            ),
        }
        .to_err();
    }
    ().to_ok()
}

#[cfg(test)]
mod tests {
    use crate::migrate::migrate_contract::migrate_contract;
    use crate::storage::ask_order_storage::ask_orders;
    use crate::storage::contract_info::{
        get_contract_info, set_contract_info, CONTRACT_TYPE, CONTRACT_VERSION,
    };
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::legacy_ask_order_storage::insert_legacy_ask_order;
    use crate::test::mock_instantiate::default_instantiate;
    use crate::types::core::constants::DEFAULT_SEARCH_ORDER;
    use crate::types::core::error::ContractError;
    use crate::types::request::ask_types::ask_collateral::AskCollateral;
    use crate::types::request::ask_types::ask_order::AskOrder;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use crate::types::request::share_sale_type::ShareSaleType;
    use cosmwasm_std::Addr;
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_successful_migrate_without_options() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let mut contract_info =
            get_contract_info(deps.as_ref().storage).expect("contract info should load");
        contract_info.contract_version = "0.0.1".to_string();
        set_contract_info(deps.as_mut().storage, &contract_info)
            .expect("contract info should be stored");
        assert_eq!(
            "0.0.1",
            get_contract_info(deps.as_ref().storage)
                .expect("contract info should load")
                .contract_version,
            "sanity check: expected contract version change to be persisted",
        );
        let response =
            migrate_contract(deps.as_mut()).expect("expected a simple migrate to succeed");
        assert!(
            response.messages.is_empty(),
            "migrations should never produce messages",
        );
        assert_eq!(
            2,
            response.attributes.len(),
            "expected the correct number of attributes to be produced",
        );
        assert_eq!(
            "migrate_contract",
            single_attribute_for_key(&response, "action"),
            "expected the correct action attribute value to be produced",
        );
        assert_eq!(
            CONTRACT_VERSION,
            single_attribute_for_key(&response, "new_version"),
            "expected the correct new_version attribute value to be produced",
        );
        let contract_info =
            get_contract_info(deps.as_ref().storage).expect("contract info should load");
        assert_eq!(
            CONTRACT_VERSION, contract_info.contract_version,
            "the migration should change the contract version",
        );
    }

    #[test]
    fn test_invalid_migration_scenarios() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let mut contract_info =
            get_contract_info(deps.as_ref().storage).expect("expected contract info to load");
        contract_info.contract_type = "faketype".to_string();
        set_contract_info(deps.as_mut().storage, &contract_info)
            .expect("expected contract info to be stored correctly");
        let err = migrate_contract(deps.as_mut())
            .expect_err("an error should occur when migrating from a different contract type");
        match err {
            ContractError::InvalidMigration { message } => {
                assert_eq!(
                    format!("target migration contract type [{}] does not match stored contract type [faketype]", CONTRACT_TYPE),
                    message,
                    "expected the correct error message to be produced when a migration is rejected for bad typing",
                );
            }
            e => panic!(
                "unexpected error occurred with a bad contract type: {:?}",
                e
            ),
        };
        contract_info.contract_type = CONTRACT_TYPE.to_string();
        contract_info.contract_version = "999.999.999".to_string();
        set_contract_info(deps.as_mut().storage, &contract_info)
            .expect("expected contract info to be stored successfully");
        let err = migrate_contract(deps.as_mut())
            .expect_err("an error should be produced if the contract is downgraded");
        match err {
            ContractError::InvalidMigration { message } => {
                assert_eq!(
                    format!("target migration contract version [{}] is too low to use. stored contract version is [999.999.999]", CONTRACT_VERSION),
                    message,
                    "expected the correct error message to be produced when a migration is rejected for bad versioning",
                );
            }
            e => panic!(
                "unexpected error occurred with a bad contract version: {:?}",
                e,
            ),
        };
    }

    // TODO: Delete this test in v1.1.1
    #[test]
    fn test_migrate_legacy_ask_order_storage() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let mut contract_info =
            get_contract_info(deps.as_ref().storage).expect("contract info should load");
        contract_info.contract_version = "0.0.1".to_string();
        set_contract_info(deps.as_mut().storage, &contract_info)
            .expect("contract info should be stored");
        assert_eq!(
            "0.0.1",
            get_contract_info(deps.as_ref().storage)
                .expect("contract info should load")
                .contract_version,
            "sanity check: expected contract version change to be persisted",
        );
        let coin_ask_order = AskOrder {
            id: "coin_ask".to_string(),
            ask_type: RequestType::CoinTrade,
            owner: Addr::unchecked("coin_trade_asker"),
            collateral: AskCollateral::coin_trade(&[], &[]),
            descriptor: None,
        };
        let marker_trade_ask_order = AskOrder {
            id: "marker_trade_ask".to_string(),
            ask_type: RequestType::MarkerTrade,
            owner: Addr::unchecked("marker_trade_asker"),
            collateral: AskCollateral::marker_trade(
                Addr::unchecked("marker_trade_marker"),
                "marker_trade_denom",
                100,
                &[],
                &[],
            ),
            descriptor: None,
        };
        let marker_share_sale_ask_order = AskOrder {
            id: "marker_share_sale_ask".to_string(),
            ask_type: RequestType::MarkerShareSale,
            owner: Addr::unchecked("marker_share_sale_asker"),
            collateral: AskCollateral::marker_share_sale(
                Addr::unchecked("marker_share_sale_marker"),
                "marker_share_sale_denom",
                100,
                50,
                &[],
                &[],
                ShareSaleType::MultipleTransactions,
            ),
            descriptor: Some(RequestDescriptor::new_populated_attributes(
                "desc",
                AttributeRequirement::all(&["some", "attributes"]),
            )),
        };
        let scope_trade_ask_order = AskOrder {
            id: "scope_trade_ask".to_string(),
            ask_type: RequestType::ScopeTrade,
            owner: Addr::unchecked("scope_trade_asker"),
            collateral: AskCollateral::scope_trade("scope_address", &[]),
            descriptor: None,
        };
        insert_legacy_ask_order(deps.as_mut().storage, &coin_ask_order)
            .expect("expected the coin trade ask order to be inserted without error");
        insert_legacy_ask_order(deps.as_mut().storage, &marker_trade_ask_order)
            .expect("expected the marker trade ask order to be inserted without error");
        insert_legacy_ask_order(deps.as_mut().storage, &marker_share_sale_ask_order)
            .expect("expected the marker share sale ask order to be inserted without error");
        insert_legacy_ask_order(deps.as_mut().storage, &scope_trade_ask_order)
            .expect("expected the scope trade ask order to be inserted without error");
        migrate_contract(deps.as_mut())
            .expect("expected the migration for legacy ask orders to succeed");
        let ask_orders_from_search = ask_orders()
            .range(deps.as_ref().storage, None, None, DEFAULT_SEARCH_ORDER)
            .map(|result| {
                result
                    .expect("ask order should unwrap without error from range search")
                    .1
            })
            .collect::<Vec<AskOrder>>();
        assert_eq!(
            4,
            ask_orders_from_search.len(),
            "all stored ask orders should be produced in the range search",
        );
        assert!(
            ask_orders_from_search
                .iter()
                .any(|order| order.id == coin_ask_order.id),
            "the coin trade ask order should still remain in ask order storage",
        );
        assert!(
            ask_orders_from_search
                .iter()
                .any(|order| order.id == marker_trade_ask_order.id),
            "the marker trade ask order should still remain in ask order storage",
        );
        assert!(
            ask_orders_from_search
                .iter()
                .any(|order| order.id == marker_share_sale_ask_order.id),
            "the marker share sale ask order should still remain in ask order storage",
        );
        assert!(
            ask_orders_from_search
                .iter()
                .any(|order| order.id == scope_trade_ask_order.id),
            "the scope trade ask order should still remain in ask order storage",
        );
    }
}
