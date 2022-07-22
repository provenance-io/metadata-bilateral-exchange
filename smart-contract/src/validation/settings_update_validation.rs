use crate::types::core::error::ContractError;
use crate::types::request::settings_update::SettingsUpdate;
use crate::util::extensions::ResultExtensions;

pub fn validate_settings_update(update: &SettingsUpdate) -> Result<(), ContractError> {
    let mut validation_errors: Vec<String> = vec![];
    if let Some(ref new_admin_address) = update.new_admin_address {
        if new_admin_address.is_empty() {
            validation_errors.push("new_admin_address was empty".to_string());
        }
    }
    if !validation_errors.is_empty() {
        ContractError::ValidationError {
            messages: validation_errors,
        }
        .to_err()
    } else {
        ().to_ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::core::error::ContractError;
    use crate::types::request::settings_update::SettingsUpdate;
    use crate::validation::settings_update_validation::validate_settings_update;

    #[test]
    fn test_empty_admin_address() {
        assert_single_error_message(
            "blank admin address provided",
            "new_admin_address was empty",
            SettingsUpdate {
                new_admin_address: Some("".to_string()),
                new_create_ask_nhash_fee: None,
                new_create_bid_nhash_fee: None,
            },
        );
    }

    fn assert_single_error_message<S1: Into<String>, S2: Into<String>>(
        test_description: S1,
        expected_error_message: S2,
        update: SettingsUpdate,
    ) {
        let test_description = test_description.into();
        let error_message = expected_error_message.into();
        let err = validate_settings_update(&update).expect_err(&format!(
            "{}: expected an error to be produced by input",
            test_description
        ));
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(
                    1,
                    messages.len(),
                    "{}: expected only a single message to be produced, but got {:?}",
                    test_description,
                    messages,
                );
                assert_eq!(
                    &error_message,
                    messages.first().unwrap(),
                    "{}: unexpected error message produced",
                    test_description,
                );
            }
            e => panic!("{}: unexpected error: {:?}", test_description, e),
        }
    }
}
