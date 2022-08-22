use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::format_coin_display;
use cosmwasm_std::{coin, Coin};
use std::cmp::Ordering;

pub fn coin_sort(first: &Coin, second: &Coin) -> Ordering {
    first
        .denom
        .cmp(&second.denom)
        .then_with(|| first.amount.cmp(&second.amount))
}

pub fn multiply_coins_by_amount(coins: &[Coin], amount: u128) -> Vec<Coin> {
    coins
        .iter()
        .map(|c| coin(c.amount.u128() * amount, &c.denom))
        .collect()
}

pub fn divide_coins_by_amount(coins: &[Coin], amount: u128) -> Vec<Coin> {
    coins
        .iter()
        .map(|c| coin(c.amount.u128() / amount, &c.denom))
        .collect()
}

pub fn subtract_coins<S: Into<String>>(
    error_prefix: S,
    minuend: &[Coin],
    subtrahend: &[Coin],
) -> Result<Vec<Coin>, ContractError> {
    let mut owned_minuend = minuend.to_vec();
    // If no subtrahend value is provided, then the difference is just the minuend
    if subtrahend.is_empty() {
        return owned_minuend.to_ok();
    }
    owned_minuend.sort_by(coin_sort);
    for sub_coin in subtrahend {
        let matching_min_coin = owned_minuend
            .iter_mut()
            .find(|min_coin| min_coin.denom == sub_coin.denom);
        if matching_min_coin.is_none() {
            return ContractError::GenericError {
                message: format!(
                    "{}: unable to find matching coin of denom [{}] in minuend. minuend: [{}], subtrahend: [{}]",
                    error_prefix.into(),
                    sub_coin.denom,
                    format_coin_display(minuend),
                    format_coin_display(subtrahend),
            ),
            }
            .to_err();
        }
        let matching_min_coin = matching_min_coin.unwrap();
        if matching_min_coin.amount.u128() < sub_coin.amount.u128() {
            return ContractError::GenericError {
                message: format!(
                    "{}: expected at least [{}{}] to be provided in minuend. minuend: [{}], subtrahend: [{}]",
                    error_prefix.into(),
                    sub_coin.amount.u128(),
                    sub_coin.denom,
                    format_coin_display(minuend),
                    format_coin_display(subtrahend),
                ),
            }
            .to_err();
        }
        matching_min_coin.amount -= sub_coin.amount;
    }
    // Remove all zeroed out values
    owned_minuend
        .into_iter()
        .filter(|min_coin| !min_coin.amount.is_zero())
        .collect::<Vec<Coin>>()
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::types::core::error::ContractError;
    use crate::util::coin_utilities::{
        coin_sort, divide_coins_by_amount, multiply_coins_by_amount, subtract_coins,
    };
    use crate::util::constants::NHASH;
    use cosmwasm_std::{coin, coins, Coin};

    #[test]
    fn test_multiply_no_coins() {
        assert!(
            multiply_coins_by_amount(&[], 10).is_empty(),
            "the result of multiplying no coins by an amount should equate to an empty vector",
        );
    }

    #[test]
    fn test_multiply_single_coin() {
        assert_eq!(
            vec![coin(50, NHASH)],
            multiply_coins_by_amount(&[coin(10, NHASH)], 5),
            "multiplying a single coin should result in a single coin vector with the correct amount",
        );
    }

    #[test]
    fn test_multiply_multiple_coins() {
        assert_eq!(
            vec![coin(10, NHASH), coin(50, "fakecoin")],
            multiply_coins_by_amount(&[coin(1, NHASH), coin(5, "fakecoin")], 10),
            "multiplying multiple coins should result in a vector containing all results",
        );
    }

    #[test]
    fn test_divide_no_coins() {
        assert!(
            divide_coins_by_amount(&[], 10).is_empty(),
            "the result of dividing no coins by an amount should equate to an empty vector",
        );
    }

    #[test]
    fn test_divide_single_coin() {
        assert_eq!(
            vec![coin(13, NHASH)],
            divide_coins_by_amount(&[coin(39, NHASH)], 3),
            "dividing a single coin should result in a single coin vector with the correct amount",
        );
    }

    #[test]
    fn test_divide_multiple_coins() {
        assert_eq!(
            vec![coin(10, NHASH), coin(50, "ahash")],
            divide_coins_by_amount(&[coin(100, NHASH), coin(500, "ahash")], 10),
            "dividing multiple coins should result in a vector containing all results",
        );
    }

    #[test]
    fn test_subtract_coins_with_invalid_data() {
        let err = subtract_coins("RIP", &[coin(100, "a")], &[coin(100, "b")])
            .expect_err("an error should occur when the minuend is missing a coin from subtrahend");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: unable to find matching coin of denom [b] in minuend. minuend: [100a], subtrahend: [100b]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = subtract_coins(
            "RIP",
            &[coin(100, "a"), coin(100, "b")],
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
        ).expect_err("an error should occur when the minuend have some coins from the subtrahend but not all of them");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: unable to find matching coin of denom [c] in minuend. minuend: [100a, 100b], subtrahend: [100a, 100b, 100c]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = subtract_coins("RIP", &coins(100, "a"), &coins(101, "a")).expect_err(
            "an error should occur when the subtrahend need more coin than the minuend have",
        );
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: expected at least [101a] to be provided in minuend. minuend: [100a], subtrahend: [101a]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = subtract_coins(
            "RIP",
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
            &[coin(100, "a"), coin(100, "b"), coin(101, "c")],
        ).expect_err("an error should occur when the subtrahend need more of a single coin after good subtractions");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: expected at least [101c] to be provided in minuend. minuend: [100a, 100b, 100c], subtrahend: [100a, 100b, 101c]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
    }

    #[test]
    fn test_subtract_coins_with_valid_input() {
        assert_minus_result_is_correct(
            "identical single coin input should produce an empty result",
            &coins(100, "a"),
            &coins(100, "a"),
            vec![],
        );
        assert_minus_result_is_correct(
            "identical multiple coin input should produce an empty result",
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
            &[coin(100, "c"), coin(100, "a"), coin(100, "b")],
            vec![],
        );
        assert_minus_result_is_correct(
            "single coin difference should calculate correctly",
            &coins(100, "a"),
            &coins(40, "a"),
            coins(60, "a"),
        );
        assert_minus_result_is_correct(
            "multiple coin difference should calculate correctly",
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
            &[coin(1, "c"), coin(2, "b"), coin(55, "a")],
            vec![coin(45, "a"), coin(98, "b"), coin(99, "c")],
        );
        assert_minus_result_is_correct(
            "unrelated coins should be unaffected in minuend - full removal",
            &[coin(100, "a"), coin(100, "b")],
            &coins(100, "b"),
            coins(100, "a"),
        );
        assert_minus_result_is_correct(
            "unrelated coins should be unaffected in minuend - partial removal",
            &[coin(100, "a"), coin(100, "b")],
            &coins(5, "b"),
            vec![coin(100, "a"), coin(95, "b")],
        );
    }

    fn assert_minus_result_is_correct<S: Into<String>>(
        test_description: S,
        minuend: &[Coin],
        subtrahend: &[Coin],
        mut expected_difference: Vec<Coin>,
    ) {
        let test_description = test_description.into();
        let mut difference = subtract_coins(&test_description, minuend, subtrahend)
            .unwrap_or_else(|e| {
                panic!(
                    "{}: expected the difference to be calculated without error, but got error: {:?}",
                    test_description, e
                )
            });
        difference.sort_by(coin_sort);
        expected_difference.sort_by(coin_sort);
        assert_eq!(
            expected_difference, difference,
            "{}: expected difference value to equate to the correct values",
            test_description,
        );
    }
}
