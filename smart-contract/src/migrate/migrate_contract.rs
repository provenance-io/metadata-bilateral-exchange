use crate::storage::contract_info::{
    get_contract_info, set_contract_info, ContractInfoV2, CONTRACT_TYPE, CONTRACT_VERSION,
};
use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, DepsMut, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};
use semver::Version;

pub fn migrate_contract(
    deps: DepsMut<ProvenanceQuery>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let mut contract_info = get_contract_info(deps.storage)?;
    check_valid_migration_target(&contract_info)?;
    contract_info.contract_version = CONTRACT_VERSION.to_string();
    set_contract_info(deps.storage, &contract_info)?;
    Response::new()
        .add_attribute("action", "migrate_contract")
        .add_attribute("new_version", CONTRACT_VERSION)
        .set_data(to_binary(&contract_info)?)
        .to_ok()
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
        get_contract_info, set_contract_info, CONTRACT_TYPE, CONTRACT_VERSION,
    };
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::default_instantiate;
    use crate::types::core::error::ContractError;
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
}
