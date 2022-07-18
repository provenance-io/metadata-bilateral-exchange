use crate::types::core::error::ContractError;
use crate::types::core::msg::InstantiateMsg;
use crate::util::extensions::ResultExtensions;
use crate::validation::generic_validation::validate_coin_vector;

pub fn validate_instantiate_msg(msg: &InstantiateMsg) -> Result<(), ContractError> {
    let mut invalid_fields = vec![];
    if msg.bind_name.is_empty() {
        invalid_fields.push("bind_name value was empty".to_string());
    }
    if msg.contract_name.is_empty() {
        invalid_fields.push("contract_name value was empty".to_string());
    }
    if let Some(ref ask_fee) = &msg.ask_fee {
        invalid_fields.append(&mut validate_coin_vector("ask_fee", ask_fee));
    }
    if let Some(ref bid_fee) = &msg.bid_fee {
        invalid_fields.append(&mut validate_coin_vector("bid_fee", bid_fee));
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
    use cosmwasm_std::{coin, coins};

    #[test]
    fn test_invalid_bind_name() {
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: String::new(),
                contract_name: "some name".to_string(),
                ask_fee: None,
                bid_fee: None,
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
                ask_fee: None,
                bid_fee: None,
            },
            "Empty contract_name provided",
            "contract_name value was empty",
        );
    }

    #[test]
    fn test_invalid_ask_fee() {
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "bind_name".to_string(),
                contract_name: "contract_name".to_string(),
                ask_fee: Some(vec![]),
                bid_fee: None,
            },
            "empty ask_fee vector provided",
            "ask_fee was empty",
        );
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "bind_name".to_string(),
                contract_name: "contract_name".to_string(),
                ask_fee: Some(coins(100, "")),
                bid_fee: None,
            },
            "ask_fee with blank denom provided",
            "ask_fee included invalid coins",
        );
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "bind_name".to_string(),
                contract_name: "contract_name".to_string(),
                ask_fee: Some(coins(0, "nhash")),
                bid_fee: None,
            },
            "ask_fee with zero coin amount provided",
            "ask_fee included invalid coins",
        );
        // Proves that some valid and some invalid case is detected
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "bind_name".to_string(),
                contract_name: "contract_name".to_string(),
                ask_fee: Some(vec![coin(100, "nhash"), coin(100, "")]),
                bid_fee: None,
            },
            "ask_fee with zero coin amount provided",
            "ask_fee included invalid coins",
        );
    }

    #[test]
    fn test_invalid_bid_fee() {
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "bind_name".to_string(),
                contract_name: "contract_name".to_string(),
                ask_fee: None,
                bid_fee: Some(vec![]),
            },
            "empty bid_fee vector provided",
            "bid_fee was empty",
        );
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "bind_name".to_string(),
                contract_name: "contract_name".to_string(),
                ask_fee: None,
                bid_fee: Some(coins(100, "")),
            },
            "bid_fee with blank denom provided",
            "bid_fee included invalid coins",
        );
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "bind_name".to_string(),
                contract_name: "contract_name".to_string(),
                ask_fee: None,
                bid_fee: Some(coins(0, "nhash")),
            },
            "bid_fee with zero coin amount provided",
            "bid_fee included invalid coins",
        );
        // Proves that some valid and some invalid case is detected
        assert_expected_validation_error(
            InstantiateMsg {
                bind_name: "bind_name".to_string(),
                contract_name: "contract_name".to_string(),
                ask_fee: None,
                bid_fee: Some(vec![coin(100, "nhash"), coin(100, "")]),
            },
            "bid_fee with zero coin amount provided",
            "bid_fee included invalid coins",
        );
    }

    #[test]
    fn test_valid_input() {
        validate_instantiate_msg(&InstantiateMsg {
            bind_name: "name.pb".to_string(),
            contract_name: "some contract".to_string(),
            ask_fee: Some(coins(10000, "nhash")),
            bid_fee: Some(coins(109009000, "nhash")),
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
