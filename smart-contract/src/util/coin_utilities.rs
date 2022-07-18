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

pub fn funds_minus_fees<S: Into<String>>(
    fee_type: S,
    funds: &[Coin],
    fees: &[Coin],
) -> Result<Vec<Coin>, ContractError> {
    let mut owned_funds = funds.to_vec();
    // If no fee value is provided, then the difference is just the funds
    if fees.is_empty() {
        return owned_funds.to_ok();
    }
    owned_funds.sort_by(coin_sort);
    for fee_coin in fees {
        let matching_fund_coin = owned_funds
            .iter_mut()
            .find(|fund_coin| fund_coin.denom == fee_coin.denom);
        if matching_fund_coin.is_none() {
            return ContractError::GenericError {
                message: format!(
                    "{}: unable to find matching coin of denom [{}] in funds. funds: [{}], fees: [{}]",
                    fee_type.into(),
                    fee_coin.denom,
                    format_coin_display(funds),
                    format_coin_display(fees),
                )
            }.to_err();
        }
        let matching_fund_coin = matching_fund_coin.unwrap();
        if matching_fund_coin.amount.u128() < fee_coin.amount.u128() {
            return ContractError::GenericError {
                message: format!(
                    "{}: expected at least [{}{}] to be provided in funds. funds: [{}], fees: [{}]",
                    fee_type.into(),
                    fee_coin.amount.u128(),
                    fee_coin.denom,
                    format_coin_display(funds),
                    format_coin_display(fees),
                ),
            }
            .to_err();
        }
        matching_fund_coin.amount -= fee_coin.amount;
    }
    // Remove all zeroed out values
    owned_funds
        .into_iter()
        .filter(|fund_coin| !fund_coin.amount.is_zero())
        .collect::<Vec<Coin>>()
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::types::core::error::ContractError;
    use crate::util::coin_utilities::{coin_sort, funds_minus_fees};
    use cosmwasm_std::{coin, coins, Coin};

    #[test]
    fn test_funds_minus_fees_with_invalid_data() {
        let err = funds_minus_fees("RIP", &[coin(100, "a")], &[coin(100, "b")])
            .expect_err("an error should occur when the minuend is missing a coin from fees");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: unable to find matching coin of denom [b] in funds. funds: [100a], fees: [100b]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = funds_minus_fees(
            "RIP",
            &[coin(100, "a"), coin(100, "b")],
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
        ).expect_err("an error should occur when the funds have some coins from the fees but not all of them");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: unable to find matching coin of denom [c] in funds. funds: [100a, 100b], fees: [100a, 100b, 100c]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = funds_minus_fees("RIP", &coins(100, "a"), &coins(101, "a"))
            .expect_err("an error should occur when the fees need more coin than the funds have");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: expected at least [101a] to be provided in funds. funds: [100a], fees: [101a]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let err = funds_minus_fees(
            "RIP",
            &[coin(100, "a"), coin(100, "b"), coin(100, "c")],
            &[coin(100, "a"), coin(100, "b"), coin(101, "c")],
        ).expect_err("an error should occur when the fees need more of a single coin after good subtractions");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "RIP: expected at least [101c] to be provided in funds. funds: [100a, 100b, 100c], fees: [100a, 100b, 101c]",
                    message,
                    "unexpected message encountered",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
    }

    #[test]
    fn test_funds_minus_fees_with_valid_input() {
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
            "unrelated coins should be unaffected in funds - full removal",
            &[coin(100, "a"), coin(100, "b")],
            &coins(100, "b"),
            coins(100, "a"),
        );
        assert_minus_result_is_correct(
            "unrelated coins should be unaffected in funds - partial removal",
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
        let mut difference = funds_minus_fees(&test_description, minuend, subtrahend)
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
