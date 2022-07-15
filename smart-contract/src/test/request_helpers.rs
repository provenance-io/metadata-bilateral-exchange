use crate::execute::update_settings::update_settings;
use crate::storage::contract_info::get_contract_info;
use crate::test::cosmos_type_helpers::MockOwnedDeps;
use crate::types::request::ask_types::ask_collateral::AskCollateral;
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::types::request::bid_types::bid_collateral::BidCollateral;
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::settings_update::SettingsUpdate;
use crate::types::request::share_sale_type::ShareSaleType;
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{Addr, Coin};
use provwasm_std::{AccessGrant, MarkerAccess};

pub fn set_ask_fee(deps: &mut MockOwnedDeps, ask_fee: Option<Vec<Coin>>) {
    let contract_info =
        get_contract_info(deps.as_ref().storage).expect("expected contract info to load");
    update_settings(
        deps.as_mut(),
        mock_info(contract_info.admin.as_str(), &[]),
        SettingsUpdate {
            new_admin_address: None,
            ask_fee,
            bid_fee: contract_info.bid_fee,
        },
    )
    .expect("expected the settings update to succeed");
}

pub fn set_bid_fee(deps: &mut MockOwnedDeps, bid_fee: Option<Vec<Coin>>) {
    let contract_info =
        get_contract_info(deps.as_ref().storage).expect("expected contract info to load");
    update_settings(
        deps.as_mut(),
        mock_info(contract_info.admin.as_str(), &[]),
        SettingsUpdate {
            new_admin_address: None,
            ask_fee: contract_info.ask_fee,
            bid_fee,
        },
    )
    .expect("expected the settings update to succeed");
}

pub fn replace_ask_quote(ask_order: &mut AskOrder, quote: &[Coin]) {
    match ask_order.collateral {
        AskCollateral::CoinTrade(ref mut collateral) => collateral.quote = quote.to_vec(),
        AskCollateral::MarkerTrade(ref mut collateral) => {
            collateral.quote_per_share = quote.to_vec()
        }
        AskCollateral::MarkerShareSale(ref mut collateral) => {
            collateral.quote_per_share = quote.to_vec()
        }
        AskCollateral::ScopeTrade(ref mut collateral) => collateral.quote = quote.to_vec(),
    };
}

pub fn replace_bid_quote(bid_order: &mut BidOrder, quote: &[Coin]) {
    match bid_order.collateral {
        BidCollateral::CoinTrade(ref mut collateral) => collateral.quote = quote.to_vec(),
        BidCollateral::MarkerTrade(ref mut collateral) => collateral.quote = quote.to_vec(),
        BidCollateral::MarkerShareSale(ref mut collateral) => collateral.quote = quote.to_vec(),
        BidCollateral::ScopeTrade(ref mut collateral) => collateral.quote = quote.to_vec(),
    };
}

pub fn mock_ask_order(collateral: AskCollateral) -> AskOrder {
    AskOrder::new_unchecked("ask_id", Addr::unchecked("asker"), collateral, None)
}

pub fn mock_ask_order_with_descriptor(
    collateral: AskCollateral,
    descriptor: RequestDescriptor,
) -> AskOrder {
    AskOrder::new_unchecked(
        "ask_id",
        Addr::unchecked("asker"),
        collateral,
        Some(descriptor),
    )
}

pub fn mock_ask_marker_trade<S1: Into<String>, S2: Into<String>>(
    addr: S1,
    denom: S2,
    share_count: u128,
    share_quote: &[Coin],
) -> AskCollateral {
    AskCollateral::marker_trade(
        Addr::unchecked(addr),
        denom,
        share_count,
        share_quote,
        &[AccessGrant {
            address: Addr::unchecked("asker"),
            permissions: vec![MarkerAccess::Admin],
        }],
    )
}

pub fn mock_ask_marker_share_sale<S1: Into<String>, S2: Into<String>>(
    addr: S1,
    denom: S2,
    total_shares_to_sell: u128,
    remaining_shares_to_sell: u128,
    share_quote: &[Coin],
    share_sale_type: ShareSaleType,
) -> AskCollateral {
    AskCollateral::marker_share_sale(
        Addr::unchecked(addr),
        denom,
        total_shares_to_sell,
        remaining_shares_to_sell,
        share_quote,
        &[AccessGrant {
            address: Addr::unchecked("asker"),
            permissions: vec![MarkerAccess::Admin],
        }],
        share_sale_type,
    )
}

pub fn mock_ask_scope_trade<S: Into<String>>(scope_address: S, quote: &[Coin]) -> AskCollateral {
    AskCollateral::scope_trade(scope_address, quote)
}

pub fn mock_bid_order(collateral: BidCollateral) -> BidOrder {
    BidOrder::new_unchecked("bid_id", Addr::unchecked("bidder"), collateral, None)
}

pub fn mock_bid_with_descriptor(
    collateral: BidCollateral,
    descriptor: RequestDescriptor,
) -> BidOrder {
    BidOrder::new_unchecked(
        "bid_id",
        Addr::unchecked("bidder"),
        collateral,
        Some(descriptor),
    )
}

pub fn mock_bid_marker_trade<S1: Into<String>, S2: Into<String>>(
    addr: S1,
    denom: S2,
    quote: &[Coin],
    withdraw_shares_after_match: Option<bool>,
) -> BidCollateral {
    BidCollateral::marker_trade(
        Addr::unchecked(addr),
        denom,
        quote,
        withdraw_shares_after_match,
    )
}

pub fn mock_bid_marker_share<S1: Into<String>, S2: Into<String>>(
    addr: S1,
    denom: S2,
    share_count: u128,
    quote: &[Coin],
) -> BidCollateral {
    BidCollateral::marker_share_sale(Addr::unchecked(addr), denom, share_count, quote)
}

pub fn mock_bid_scope_trade<S: Into<String>>(scope_address: S, quote: &[Coin]) -> BidCollateral {
    BidCollateral::scope_trade(scope_address, quote)
}
