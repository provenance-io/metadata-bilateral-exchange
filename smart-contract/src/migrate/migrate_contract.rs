use crate::storage::contract_info::{
    set_contract_info, ContractInfoV2, CONTRACT_INFO, CONTRACT_TYPE, CONTRACT_VERSION,
};
use crate::types::core::error::ContractError;
use crate::util::constants::NHASH;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, Coin, DepsMut, Response, Uint128};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};
use semver::Version;

// TODO: Replace this code with code that exclusively uses ContractInfoV2 once the migration has completed
pub fn migrate_contract(
    deps: DepsMut<ProvenanceQuery>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let contract_info_v1 = CONTRACT_INFO.load(deps.storage)?;
    // Port the existing contract info over to the new contract info format
    let mut contract_info_v2 = ContractInfoV2 {
        admin: contract_info_v1.admin,
        bind_name: contract_info_v1.bind_name,
        contract_name: contract_info_v1.contract_name,
        contract_type: contract_info_v1.contract_type,
        contract_version: contract_info_v1.contract_version,
        create_ask_nhash_fee: port_fee_to_nhash(&contract_info_v1.ask_fee),
        create_bid_nhash_fee: port_fee_to_nhash(&contract_info_v1.bid_fee),
    };
    CONTRACT_INFO.remove(deps.storage);
    check_valid_migration_target(&contract_info_v2)?;
    contract_info_v2.contract_version = CONTRACT_VERSION.to_string();
    set_contract_info(deps.storage, &contract_info_v2)?;
    Response::new()
        .add_attribute("action", "migrate_contract")
        .add_attribute("new_version", CONTRACT_VERSION)
        .set_data(to_binary(&contract_info_v2)?)
        .to_ok()
}

fn port_fee_to_nhash(fee_coins: &Option<Vec<Coin>>) -> Uint128 {
    fee_coins
        .to_owned()
        // Get the coin vector or default out to an empty vector
        .unwrap_or_default()
        .into_iter()
        // Get the first result with an nhash denom - this is the only thing that relates to the new format
        .find(|coin| coin.denom.as_str() == NHASH)
        // Just grab the amount from the nhash fee coin.  This will be used for Provenance fees
        .map(|coin| coin.amount)
        // If all else fails, default out to zero, which is what is used when no fee is to be charged
        .unwrap_or_else(Uint128::zero)
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
    use crate::storage::contract_info::{
        get_contract_info, set_contract_info, ContractInfo, ContractInfoV2, CONTRACT_INFO,
        CONTRACT_TYPE, CONTRACT_VERSION,
    };
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::{
        default_instantiate, DEFAULT_ADMIN_ADDRESS, DEFAULT_CONTRACT_BIND_NAME,
        DEFAULT_CONTRACT_NAME,
    };
    use crate::types::core::error::ContractError;
    use crate::util::constants::NHASH;
    use cosmwasm_std::{coin, from_binary, Addr, StdError};
    use provwasm_mocks::mock_dependencies;

    // TODO: Re-enable this test once the migration has completed
    #[ignore]
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

    // TODO: Re-enable this test once the migration has completed
    #[ignore]
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

    // TODO: Delete this test and all below it after the migration has been completed to contract info v2
    #[test]
    fn test_invalid_migration_scenarios_with_contract_info_change() {
        let mut deps = mock_dependencies(&[]);
        let err = migrate_contract(deps.as_mut())
            .expect_err("an error should occur when no contract info v1 is set");
        assert!(
            matches!(err, ContractError::Std(StdError::NotFound { .. })),
            "an std not found error should occur when no contract info is available",
        );
        let contract_info_v1 = ContractInfo {
            admin: Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
            bind_name: DEFAULT_CONTRACT_BIND_NAME.to_string(),
            contract_name: DEFAULT_CONTRACT_NAME.to_string(),
            contract_type: CONTRACT_TYPE.to_string(),
            contract_version: CONTRACT_VERSION.to_string(),
            ask_fee: None,
            bid_fee: None,
        };
        CONTRACT_INFO
            .save(deps.as_mut().storage, &contract_info_v1)
            .expect("contract info v1 should be saved without issue");
        let err = migrate_contract(deps.as_mut())
            .expect_err("an error should occur when the contrat version is too low");
        assert!(
            matches!(err, ContractError::InvalidMigration { .. }),
            "an invalid migration error should occur if the contract info has the wrong version, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_none_fee_equates_to_zero() {
        let contract_info_v2 = test_migration_base(&ContractInfo {
            admin: Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
            bind_name: DEFAULT_CONTRACT_BIND_NAME.to_string(),
            contract_name: DEFAULT_CONTRACT_NAME.to_string(),
            contract_type: CONTRACT_TYPE.to_string(),
            contract_version: "1.0.7".to_string(),
            ask_fee: None,
            bid_fee: None,
        });
        assert_eq!(
            0,
            contract_info_v2.create_ask_nhash_fee.u128(),
            "the new ask fee should be zero because no fee was set in the original contract info",
        );
        assert_eq!(
            0,
            contract_info_v2.create_bid_nhash_fee.u128(),
            "the new bid fee should be zero because no fee was set in the original contract info",
        );
    }

    #[test]
    fn test_no_nhash_fees_equates_to_zero() {
        let contract_info_v2 = test_migration_base(&ContractInfo {
            admin: Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
            bind_name: DEFAULT_CONTRACT_BIND_NAME.to_string(),
            contract_name: DEFAULT_CONTRACT_NAME.to_string(),
            contract_type: CONTRACT_TYPE.to_string(),
            contract_version: "1.0.7".to_string(),
            ask_fee: Some(vec![coin(10, "something"), coin(150, "something_else")]),
            bid_fee: Some(vec![coin(5, "bidthing")]),
        });
        assert_eq!(
            0,
            contract_info_v2.create_ask_nhash_fee.u128(),
            "the new ask fee should be zero because no fees were set in nhash",
        );
        assert_eq!(
            0,
            contract_info_v2.create_bid_nhash_fee.u128(),
            "the new bid fee should be zero because no fees were set in nhash",
        );
    }

    #[test]
    fn test_nhash_fee_is_properly_ported() {
        let contract_info_v2 = test_migration_base(&ContractInfo {
            admin: Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
            bind_name: DEFAULT_CONTRACT_BIND_NAME.to_string(),
            contract_name: DEFAULT_CONTRACT_NAME.to_string(),
            contract_type: CONTRACT_TYPE.to_string(),
            contract_version: "1.0.7".to_string(),
            ask_fee: Some(vec![
                coin(10, "something"),
                coin(150, "something_else"),
                coin(50, NHASH),
            ]),
            bid_fee: Some(vec![coin(5, "bidthing"), coin(100, NHASH)]),
        });
        assert_eq!(
            50,
            contract_info_v2.create_ask_nhash_fee.u128(),
            "the nhash ask fee should be preserved",
        );
        assert_eq!(
            100,
            contract_info_v2.create_bid_nhash_fee.u128(),
            "the nhash bid fee should be preserved",
        );
    }

    fn test_migration_base(contract_info_v1: &ContractInfo) -> ContractInfoV2 {
        let mut deps = mock_dependencies(&[]);
        CONTRACT_INFO
            .save(deps.as_mut().storage, contract_info_v1)
            .expect("expected original contract info to be saved without issue");
        let response =
            migrate_contract(deps.as_mut()).expect("expected migration to succeed without issue");
        assert!(
            response.messages.is_empty(),
            "migrations should not emit messages",
        );
        assert_eq!(
            2,
            response.attributes.len(),
            "the correct number of attribute should be emitted",
        );
        assert_eq!(
            "migrate_contract",
            single_attribute_for_key(&response, "action"),
            "the correct value should be set for the action attribute",
        );
        assert_eq!(
            CONTRACT_VERSION,
            single_attribute_for_key(&response, "new_version"),
            "the correct value should be set for the new_version attribute",
        );
        let contract_info_v2 = get_contract_info(deps.as_ref().storage)
            .expect("contract info v2 should be available after the migration completes");
        let binary_deserialized_contract_info = from_binary::<ContractInfoV2>(
            &response
                .data
                .clone()
                .expect("the response data should be set"),
        )
        .expect("contract info v2 should be included in the data in the response");
        assert_eq!(
            contract_info_v2, binary_deserialized_contract_info,
            "the data set in the response should equate to the new contract info value",
        );
        assert_eq!(
            contract_info_v1.admin, contract_info_v2.admin,
            "the admin value should be properly ported in the new contract info",
        );
        assert_eq!(
            contract_info_v1.bind_name, contract_info_v2.bind_name,
            "the bind_name value should be properly ported in the new contract info",
        );
        assert_eq!(
            contract_info_v1.contract_name, contract_info_v2.contract_name,
            "the contract_name value should be properly ported in the new contract info",
        );
        assert_eq!(
            contract_info_v1.contract_type, contract_info_v2.contract_type,
            "the contract_type value should be properly ported in the new contract info",
        );
        assert_eq!(
            CONTRACT_VERSION, contract_info_v2.contract_version,
            "the new contract info should include the specified contract version",
        );
        contract_info_v2
    }
}
