use crate::types::core::error::ContractError;

pub fn assert_validation_error_message<S: Into<String>>(err: ContractError, expected_message: S) {
    let expected_message = expected_message.into();
    match err {
        ContractError::ValidationError { messages } => {
            assert_eq!(
                1,
                messages.len(),
                "expected only a single validation error message, but got: {:?}",
                messages,
            );
            assert_eq!(
                &expected_message,
                messages.first().unwrap(),
                "expected the correct validation message text",
            );
        }
        e => panic!("unexpected error received: {:?}", e),
    };
}

pub fn assert_missing_field_error<S: Into<String>>(err: ContractError, expected_missing_field: S) {
    let expected_missing_field = expected_missing_field.into();
    match err {
        ContractError::MissingField { field } => {
            assert_eq!(
                expected_missing_field, field,
                "the expected missing field was not specified",
            );
        }
        e => panic!("unexpected error received: {:?}", e),
    };
}
