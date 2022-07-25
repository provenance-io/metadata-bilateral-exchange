use cosmwasm_std::Coin;

pub fn validate_coin_vector<S: Into<String>>(coin_type: S, coins: &[Coin]) -> Vec<String> {
    let mut invalid_fields = vec![];
    let coin_type = coin_type.into();
    if coins.is_empty() {
        invalid_fields.push(format!("{} was empty", &coin_type));
    } else if coins
        .iter()
        .any(|coin| coin.denom.is_empty() || coin.amount.is_zero())
    {
        invalid_fields.push(format!("{} included invalid coins", &coin_type));
    }
    invalid_fields
}

#[cfg(test)]
mod tests {
    use crate::util::constants::NHASH;
    use crate::validation::generic_validation::validate_coin_vector;
    use cosmwasm_std::{coin, coins, Coin};

    #[test]
    fn test_validate_coin_vector_with_valid_data() {
        assert!(
            validate_coin_vector("some value", &coins(100, NHASH)).is_empty(),
            "valid single coin vector should produce no error messages",
        );
        assert!(
            validate_coin_vector("some value", &[coin(100, "a"), coin(200, "b")]).is_empty(),
            "valid multi coin vector should produce no error messages",
        );
    }

    #[test]
    fn test_validate_coin_vector_with_invalid_data() {
        assert_single_error_message("test_coin", &[], "test_coin was empty");
        assert_single_error_message(
            "test_coin",
            &coins(100, ""),
            "test_coin included invalid coins",
        );
        assert_single_error_message(
            "test_coin",
            &coins(0, "something"),
            "test_coin included invalid coins",
        );
        assert_single_error_message(
            "test_coin",
            &[coin(100, NHASH), coin(1, "")],
            "test_coin included invalid coins",
        );
    }

    fn assert_single_error_message<S1: Into<String>, S2: Into<String>>(
        coin_type: S1,
        coins: &[Coin],
        expected_error_message: S2,
    ) {
        let error_message = expected_error_message.into();
        let error_messages = validate_coin_vector(coin_type, coins);
        assert_eq!(
            1,
            error_messages.len(),
            "expected a single error message of [{}] but got: {:?}",
            error_message,
            error_messages,
        );
        assert_eq!(
            &error_message,
            error_messages.first().unwrap(),
            "expected the correct error message to be produced",
        );
    }
}
