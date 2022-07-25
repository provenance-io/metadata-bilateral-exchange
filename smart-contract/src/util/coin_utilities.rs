use cosmwasm_std::Coin;
use std::cmp::Ordering;

pub fn coin_sort(first: &Coin, second: &Coin) -> Ordering {
    first
        .denom
        .cmp(&second.denom)
        .then_with(|| first.amount.cmp(&second.amount))
}
