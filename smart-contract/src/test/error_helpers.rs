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
