use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use std::cell::RefCell;

pub struct ValidationHandler {
    messages: RefCell<Vec<String>>,
}
impl ValidationHandler {
    pub fn new() -> Self {
        Self {
            messages: RefCell::new(vec![]),
        }
    }

    pub fn push<S: Into<String>>(&self, message: S) {
        self.messages.borrow_mut().push(message.into());
    }

    pub fn append<S: Into<String> + Clone>(&self, messages: &[S]) {
        self.messages.borrow_mut().append(
            &mut messages
                .iter()
                .map(|s| s.to_owned().into())
                .collect::<Vec<String>>(),
        );
    }

    pub fn handle(self) -> Result<(), ContractError> {
        let owned_messages = self.messages.take();
        if owned_messages.is_empty() {
            ().to_ok()
        } else {
            ContractError::ValidationError {
                messages: owned_messages,
            }
            .to_err()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValidationHandler;
    use crate::types::core::error::ContractError;

    #[test]
    fn test_empty_handler_does_not_produce_an_error() {
        let handler = ValidationHandler::new();
        handler
            .handle()
            .expect("no error should be returned because no field messages were appended");
    }

    #[test]
    fn test_populated_handler_produces_an_error() {
        let handler = ValidationHandler::new();
        handler.push("error 1");
        handler.push("error 2");
        let error = handler.handle().expect_err("an error should be produced");
        match error {
            ContractError::ValidationError { messages } => {
                assert_eq!(
                    2,
                    messages.len(),
                    "two error messages should be contained in the error",
                );
                assert!(
                    messages.contains(&"error 1".to_string()),
                    "the first error should be in the error messages, but found: {:?}",
                    messages,
                );
                assert!(
                    messages.contains(&"error 2".to_string()),
                    "the second error should be in the error messages, but found: {:?}",
                    messages,
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
    }
}
