use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::format_coin_display;
use cosmwasm_std::Coin;
use std::cmp::Ordering;

pub fn coin_sort(first: &Coin, second: &Coin) -> Ordering {
    first
        .denom
        .cmp(&second.denom)
        .then_with(|| first.amount.cmp(&second.amount))
}

pub fn get_coin_difference<S: Into<String>>(
    difference_calc_reason: S,
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
        let matching_minuend = owned_minuend
            .iter_mut()
            .find(|min_coin| min_coin.denom == sub_coin.denom);
        if matching_minuend.is_none() {
            return ContractError::generic_error(
                format!(
                    "{}: unable to find matching coin of denom [{}] in minuend. Minuend: [{}], subtrahend: [{}]",
                    difference_calc_reason.into(),
                    sub_coin.denom,
                    format_coin_display(minuend),
                    format_coin_display(subtrahend),
                )
            ).to_err();
        }
        let matching_minuend: &mut Coin = matching_minuend.unwrap();
        if matching_minuend.amount.u128() < sub_coin.amount.u128() {
            return ContractError::generic_error(format!(
                "{}: expected at least [{}{}] to be provided in minuend. Minuend: [{}], subtrahend: [{}]",
                difference_calc_reason.into(),
                sub_coin.amount.u128(),
                sub_coin.denom,
                format_coin_display(minuend),
                format_coin_display(subtrahend),
            ))
            .to_err();
        }
        matching_minuend.amount -= sub_coin.amount;
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
    use crate::util::coin_utilities::{coin_sort, get_coin_difference};
    use cosmwasm_std::{coin, coins, Coin};

    #[test]
    fn test_get_coin_difference_with_invalid_data() {
        let err = get_coin_difference("RIP", &[coin(100, "a")], &[coin(100, "b")]).expect_err(
            "an error should occur when the minuend is missing a coin from the subtrahend",
        );
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: unable to find matching coin of denom [b] in minuend. Minuend: [100a], subtrahend: [100b]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = get_coin_difference(
            "RIP",
            &[coin(100, "a"), coin(100, "b")],
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
        ).expect_err("an error should occur when the minuend has some coin from the subtrahend but not all of it");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: unable to find matching coin of denom [c] in minuend. Minuend: [100a, 100b], subtrahend: [100a, 100b, 100c]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = get_coin_difference("RIP", &coins(100, "a"), &coins(101, "a")).expect_err(
            "an error should occur when the subtrahend needs more coin than the minuend has",
        );
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: expected at least [101a] to be provided in minuend. Minuend: [100a], subtrahend: [101a]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = get_coin_difference(
            "RIP",
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
            &[coin(100, "a"), coin(100, "b"), coin(101, "c")],
        ).expect_err("an error should occur when the subtrahend needs more of a single coin after good subtractions");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: expected at least [101c] to be provided in minuend. Minuend: [100a, 100b, 100c], subtrahend: [100a, 100b, 101c]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
    }

    #[test]
    fn test_get_coin_difference_with_valid_input() {
        assert_correct_coin_difference(
            "identical single coin input should produce an empty result",
            &coins(100, "a"),
            &coins(100, "a"),
            vec![],
        );
        assert_correct_coin_difference(
            "identical multiple coin input should produce an empty result",
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
            &[coin(100, "c"), coin(100, "a"), coin(100, "b")],
            vec![],
        );
        assert_correct_coin_difference(
            "single coin difference should calculate correctly",
            &coins(100, "a"),
            &coins(40, "a"),
            coins(60, "a"),
        );
        assert_correct_coin_difference(
            "multiple coin difference should calculate correctly",
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
            &[coin(1, "c"), coin(2, "b"), coin(55, "a")],
            vec![coin(45, "a"), coin(98, "b"), coin(99, "c")],
        );
        assert_correct_coin_difference(
            "unrelated coins should be unaffected in minuend - full removal",
            &[coin(100, "a"), coin(100, "b")],
            &coins(100, "b"),
            coins(100, "a"),
        );
        assert_correct_coin_difference(
            "unrelated coins should be unaffected in minuend - partial removal",
            &[coin(100, "a"), coin(100, "b")],
            &coins(5, "b"),
            vec![coin(100, "a"), coin(95, "b")],
        );
    }

    fn assert_correct_coin_difference<S: Into<String>>(
        test_description: S,
        minuend: &[Coin],
        subtrahend: &[Coin],
        mut expected_difference: Vec<Coin>,
    ) {
        let test_description = test_description.into();
        let mut difference = get_coin_difference(&test_description, minuend, subtrahend)
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
