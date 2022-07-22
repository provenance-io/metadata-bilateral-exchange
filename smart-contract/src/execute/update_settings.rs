use crate::storage::contract_info::{get_contract_info, set_contract_info};
use crate::types::core::error::ContractError;
use crate::types::request::settings_update::SettingsUpdate;
use crate::util::constants::NHASH;
use crate::util::extensions::ResultExtensions;
use crate::validation::settings_update_validation::validate_settings_update;
use cosmwasm_std::{DepsMut, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

pub fn update_settings(
    deps: DepsMut<ProvenanceQuery>,
    info: MessageInfo,
    update: SettingsUpdate,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    validate_settings_update(&update)?;
    let mut contract_info = get_contract_info(deps.storage)?;
    if info.sender != contract_info.admin {
        return ContractError::Unauthorized.to_err();
    }
    if !info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "funds cannot be provided during a settings update".to_string(),
        }
        .to_err();
    }
    let mut attributes = vec![];
    if let Some(ref new_admin) = &update.new_admin_address {
        contract_info.admin = deps.api.addr_validate(new_admin)?;
        attributes.push(("new_admin_address".to_string(), new_admin.to_string()));
    }
    if let Some(ref new_ask_fee) = &update.new_create_ask_nhash_fee {
        contract_info.create_ask_nhash_fee = new_ask_fee.to_owned();
        attributes.push((
            "new_ask_fee".to_string(),
            format!("{}{}", new_ask_fee.u128(), NHASH),
        ));
    }
    if let Some(ref new_bid_fee) = &update.new_create_bid_nhash_fee {
        contract_info.create_bid_nhash_fee = new_bid_fee.to_owned();
        attributes.push((
            "new_bid_fee".to_string(),
            format!("{}{}", new_bid_fee.u128(), NHASH),
        ));
    }
    // Save changes to the contract information
    set_contract_info(deps.storage, &contract_info)?;
    Response::new()
        .add_attribute("action", "update_settings")
        .add_attributes(attributes)
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::execute::update_settings::update_settings;
    use crate::storage::contract_info::get_contract_info;
    use crate::test::cosmos_type_helpers::single_attribute_for_key;
    use crate::test::mock_instantiate::{
        default_instantiate, test_instantiate, TestInstantiate, DEFAULT_ADMIN_ADDRESS,
    };
    use crate::types::core::error::ContractError;
    use crate::types::request::settings_update::SettingsUpdate;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{coins, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_update_settings_with_invalid_data() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let err = update_settings(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            SettingsUpdate {
                new_admin_address: Some(String::new()),
                new_create_ask_nhash_fee: None,
                new_create_bid_nhash_fee: None,
            },
        )
        .expect_err("an error should occur when invalid data is provided to the settings update");
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(
                    1,
                    messages.len(),
                    "a single error message should be produced in the validation error"
                );
                assert_eq!(
                    "new_admin_address was empty",
                    messages.first().unwrap(),
                    "unexpected error message was sent by validation",
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
        let valid_update = SettingsUpdate {
            new_admin_address: Some("some admin".to_string()),
            new_create_ask_nhash_fee: None,
            new_create_bid_nhash_fee: None,
        };
        let err = update_settings(
            deps.as_mut(),
            mock_info("not admin", &[]),
            valid_update.clone(),
        )
        .expect_err("an error should occur when the admin is not executing the settings update");
        assert!(
            matches!(err, ContractError::Unauthorized),
            "an unauthorized error should be produced when the admin is not the sender, but got: {:?}",
            err,
        );
        let err = update_settings(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &coins(1200090, "nhash")),
            valid_update.clone(),
        )
        .expect_err("an error should occur when funds are sent");
        assert!(
            matches!(err, ContractError::InvalidFundsProvided { .. }),
            "an invalid funds provided error should occur when the sender adds funds, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_update_settings_with_all_values_set() {
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());
        let response = update_settings(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            SettingsUpdate {
                new_admin_address: Some("new_admin".to_string()),
                new_create_ask_nhash_fee: Some(Uint128::new(100)),
                new_create_bid_nhash_fee: Some(Uint128::new(150)),
            },
        )
        .expect("expected a response to be emitted when a valid request is made by the admin");
        assert!(
            response.messages.is_empty(),
            "no messages should be emitted by the settings update",
        );
        assert_eq!(
            4,
            response.attributes.len(),
            "the correct number of attributes should be emitted in the settings update",
        );
        assert_eq!(
            "update_settings",
            single_attribute_for_key(&response, "action"),
            "the correct value should be set for the action attribute",
        );
        assert_eq!(
            "new_admin",
            single_attribute_for_key(&response, "new_admin_address"),
            "the correct value should be set for the new_admin attribute",
        );
        assert_eq!(
            "100nhash",
            single_attribute_for_key(&response, "new_ask_fee"),
            "the correct value should be set for the new_ask_fee attribute",
        );
        assert_eq!(
            "150nhash",
            single_attribute_for_key(&response, "new_bid_fee"),
            "the correct value should be set for the new_bid_fee attribute",
        );
        let contract_info = get_contract_info(deps.as_ref().storage)
            .expect("expected contract info to load correctly");
        assert_eq!(
            "new_admin",
            contract_info.admin.as_str(),
            "the correct admin address should be now set in the contract info",
        );
        assert_eq!(
            100,
            contract_info.create_ask_nhash_fee.u128(),
            "the correct ask fee should be set in contract info",
        );
        assert_eq!(
            150,
            contract_info.create_bid_nhash_fee.u128(),
            "the correct bid fee should be set in contract info",
        );
    }

    #[test]
    fn test_update_settings_with_no_changes() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(
            deps.as_mut(),
            TestInstantiate {
                create_ask_nhash_fee: Some(Uint128::new(100)),
                create_bid_nhash_fee: Some(Uint128::new(101)),
                ..TestInstantiate::default()
            },
        );
        let contract_info_before_update =
            get_contract_info(deps.as_ref().storage).expect("contract info should load");
        assert_eq!(
            100,
            contract_info_before_update.create_ask_nhash_fee.u128(),
            "sanity check: the correct ask fee should be set in the contract info",
        );
        assert_eq!(
            101,
            contract_info_before_update.create_bid_nhash_fee.u128(),
            "sanity check: the correct bid fee should be set in the contract info",
        );
        let response = update_settings(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            SettingsUpdate {
                new_admin_address: None,
                new_create_ask_nhash_fee: None,
                new_create_bid_nhash_fee: None,
            },
        )
        .expect("expected settings update to run with all none values");
        assert!(
            response.messages.is_empty(),
            "settings update should not send any messages",
        );
        assert_eq!(
            1,
            response.attributes.len(),
            "the update should only produce a single attribute when no changes are made",
        );
        assert_eq!(
            "update_settings",
            single_attribute_for_key(&response, "action"),
            "the correct value should be set for the action attribute",
        );
        assert_eq!(
            contract_info_before_update,
            get_contract_info(deps.as_ref().storage)
                .expect("contract info should load after the update"),
            "the contract info should be unmodified by the update",
        );
    }
}
