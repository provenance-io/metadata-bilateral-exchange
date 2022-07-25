use crate::types::core::error::ContractError;
use crate::types::core::msg::InstantiateMsg;
use crate::util::extensions::ResultExtensions;

pub fn validate_instantiate_msg(msg: &InstantiateMsg) -> Result<(), ContractError> {
    let mut invalid_fields = vec![];
    if msg.bind_name.is_empty() {
        invalid_fields.push("bind_name value was empty".to_string());
    }
    if msg.contract_name.is_empty() {
        invalid_fields.push("contract_name value was empty".to_string());
    }
    if !invalid_fields.is_empty() {
        ContractError::ValidationError {
            messages: invalid_fields,
        }
        .to_err()
    } else {
        ().to_ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::core::error::ContractError;
    use crate::types::core::msg::InstantiateMsg;
    use crate::validation::instantiation_validation::validate_instantiate_msg;
    use cosmwasm_std::Uint128;

    #[test]
    fn test_invalid_bind_name() {
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: String::new(),
                contract_name: "some name".to_string(),
                create_ask_nhash_fee: None,
                create_bid_nhash_fee: None,
            },
            "Empty bind_name provided",
            "bind_name value was empty",
        );
    }

    #[test]
    fn test_invalid_contract_name() {
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "somename.pb".to_string(),
                contract_name: String::new(),
                create_ask_nhash_fee: None,
                create_bid_nhash_fee: None,
            },
            "Empty contract_name provided",
            "contract_name value was empty",
        );
    }

    #[test]
    fn test_valid_input() {
        validate_instantiate_msg(&InstantiateMsg {
            bind_name: "name.pb".to_string(),
            contract_name: "some contract".to_string(),
            create_ask_nhash_fee: Some(Uint128::new(10000)),
            create_bid_nhash_fee: Some(Uint128::new(109009000)),
        })
        .expect("fully populated instantiate message should pass validation");
    }

    fn assert_expected_validation_error<S1: Into<String>, S2: Into<String>>(
        msg: InstantiateMsg,
        test_description: S1,
        expected_error_message: S2,
    ) {
        let test_description = test_description.into();
        let error_message = expected_error_message.into();
        let err = validate_instantiate_msg(&msg).expect_err(&format!(
            "{}: expected instantiate message to produce an error",
            test_description
        ));
        match err {
            ContractError::ValidationError { messages } => {
                assert_eq!(
                    1,
                    messages.len(),
                    "{}: expected a single error message ({}) to be produced in the validation error, but got: {:?}",
                    test_description,
                    error_message,
                    messages,
                );
                assert_eq!(
                    &error_message,
                    messages.first().unwrap(),
                    "{}: unexpected erorr message encountered",
                    test_description,
                );
            }
            e => panic!(
                "{}: expected a validation error, but got: {:?}",
                test_description, e
            ),
        };
    }
}
