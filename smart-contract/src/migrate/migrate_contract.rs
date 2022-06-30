use crate::storage::contract_info::{
    get_contract_info, set_contract_info, ContractInfo, CONTRACT_VERSION,
};
use crate::types::core::error::ContractError;
use crate::types::migrate::migration_options::MigrationOptions;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{attr, to_binary, DepsMut, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};
use semver::Version;

pub fn migrate_contract(
    deps: DepsMut<ProvenanceQuery>,
    options: Option<MigrationOptions>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let mut contract_info = get_contract_info(deps.storage)?;
    check_valid_migration_versioning(&contract_info)?;
    contract_info.contract_version = CONTRACT_VERSION.to_string();
    let mut additional_attributes = vec![];
    if let Some(options) = options {
        if let Some(new_admin_address) = options.new_admin_address {
            contract_info.admin = deps.api.addr_validate(&new_admin_address)?;
            additional_attributes.push(attr("new_admin_address", &new_admin_address));
            set_contract_info(deps.storage, &contract_info)?;
        }
    }
    Response::new()
        .add_attribute("action", "migrate_contract")
        .add_attribute("new_version", CONTRACT_VERSION)
        .add_attributes(additional_attributes)
        .set_data(to_binary(&contract_info)?)
        .to_ok()
}

pub fn check_valid_migration_versioning(contract_info: &ContractInfo) -> Result<(), ContractError> {
    let existing_contract_version = contract_info.contract_version.parse::<Version>()?;
    let new_contract_version = CONTRACT_VERSION.parse::<Version>()?;
    if existing_contract_version > new_contract_version {
        return ContractError::invalid_migration(format!(
            "current contract version [{}] is greater than the migration target version [{}]",
            &contract_info.contract_version, CONTRACT_VERSION,
        ))
        .to_err();
    }
    ().to_ok()
}

#[cfg(test)]
mod tests {
    use crate::migrate::migrate_contract::migrate_contract;
    use crate::storage::contract_info::{get_contract_info, set_contract_info, CONTRACT_VERSION};
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::default_instantiate;
    use crate::types::core::error::ContractError;
    use crate::types::migrate::migration_options::MigrationOptions;
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_successful_migrate_without_options() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let response =
            migrate_contract(deps.as_mut(), None).expect("expected a simple migrate to succeed");
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
    }

    #[test]
    fn test_successful_migrate_with_new_admin_address() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let response = migrate_contract(
            deps.as_mut(),
            Some(MigrationOptions {
                new_admin_address: Some("cooladmin".to_string()),
            }),
        )
        .expect("expected a migration with a new admin to succeed");
        assert!(
            response.messages.is_empty(),
            "migrations should never produce messages",
        );
        assert_eq!(
            3,
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
        assert_eq!(
            "cooladmin",
            single_attribute_for_key(&response, "new_admin_address"),
            "expected the correct new_admin_address attribute to be produced",
        );
        let contract_info = get_contract_info(deps.as_ref().storage)
            .expect("expected contract info to load correctly");
        assert_eq!(
            "cooladmin",
            contract_info.admin.as_str(),
            "expected the new admin address to be stored in contract info",
        );
    }

    #[test]
    fn test_bad_version_rejection() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut().storage);
        let mut contract_info =
            get_contract_info(deps.as_ref().storage).expect("expected contract info to load");
        contract_info.contract_version = "999.999.999".to_string();
        set_contract_info(deps.as_mut().storage, &contract_info)
            .expect("expected contract info to be stored successfully");
        let err = migrate_contract(deps.as_mut(), None)
            .expect_err("an error should be produced if the contract is downgraded");
        match err {
            ContractError::InvalidMigration { message } => {
                assert_eq!(
                    format!("current contract version [999.999.999] is greater than the migration target version [{}]", CONTRACT_VERSION),
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
