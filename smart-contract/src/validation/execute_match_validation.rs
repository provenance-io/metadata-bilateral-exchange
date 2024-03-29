use crate::types::core::error::ContractError;
use crate::types::request::admin_match_options::{AdminMatchOptions, OverrideQuoteSource};
use crate::types::request::ask_types::ask_collateral::{
    AskCollateral, CoinTradeAskCollateral, MarkerShareSaleAskCollateral, MarkerTradeAskCollateral,
    ScopeTradeAskCollateral,
};
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::types::request::bid_types::bid_collateral::{
    BidCollateral, CoinTradeBidCollateral, MarkerShareSaleBidCollateral, MarkerTradeBidCollateral,
    ScopeTradeBidCollateral,
};
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::request_descriptor::{AttributeRequirementType, RequestDescriptor};
use crate::types::request::share_sale_type::ShareSaleType;
use crate::util::coin_utilities::{coin_sort, multiply_coins_by_amount};
use crate::util::provenance_utilities::{format_coin_display, get_single_marker_coin_holding};
use crate::validation::validation_handler::ValidationHandler;
use cosmwasm_std::{Addr, Deps};
use provwasm_std::{ProvenanceQuerier, ProvenanceQuery};
use take_if::TakeIf;

pub fn validate_match(
    deps: &Deps<ProvenanceQuery>,
    ask: &AskOrder,
    bid: &BidOrder,
    admin_match_options: &Option<AdminMatchOptions>,
) -> Result<(), ContractError> {
    let handler = ValidationHandler::new();
    let identifiers = format!(
        "Match Validation for AskOrder [{}] and BidOrder [{}]:",
        &ask.id, &bid.id
    );
    if ask.ask_type != bid.bid_type {
        handler.push(format!(
            "{} Ask type [{}] does not match bid type [{}]",
            &identifiers,
            &ask.ask_type.get_name(),
            &bid.bid_type.get_name(),
        ));
    }
    // Verify that the asker has appropriate attributes based on the request descriptor of the bid
    if let Some(validation_err) =
        get_required_attributes_error(deps, &bid.descriptor, &ask.owner, "asker")
    {
        handler.push(validation_err);
    }

    // Verify that the bidder has appropriate attributes based on the request descriptor of the ask
    if let Some(validation_err) =
        get_required_attributes_error(deps, &ask.descriptor, &bid.owner, "bidder")
    {
        handler.push(validation_err);
    }

    match &ask.collateral {
        AskCollateral::CoinTrade(ask_collat) => match &bid.collateral {
            BidCollateral::CoinTrade(bid_collat) => {
                // Reject match requests that send the wrong type of match options to notify the
                // caller that they are attempting an action that is unrelated to the selected match
                if admin_match_options.is_some() && !matches!(admin_match_options, Some(AdminMatchOptions::CoinTrade {.. })) {
                    handler.push(format!(
                        "{} Admin match options were provided, but were not of the coin_trade variant",
                        identifiers,
                    ));
                }
                handler.append(
                    &get_coin_trade_collateral_validation(
                        ask,
                        bid,
                        ask_collat,
                        bid_collat,
                        if let Some(
                            AdminMatchOptions::CoinTrade {
                                accept_mismatched_bids: Some(mismatched_bids)
                            }
                        ) = admin_match_options {
                            *mismatched_bids
                        } else {
                            false
                        }
                    ),
                );
            },
            _ => handler.push(format!(
                "{} Ask collateral was of type coin trade, which did not match bid collateral",
                identifiers
            )),
        },
        AskCollateral::MarkerTrade(ask_collat) => match &bid.collateral {
            BidCollateral::MarkerTrade(bid_collat) => {
                // Reject match requests that send the wrong type of match options to notify the
                // caller that they are attempting an action that is unrelated to the selected match
                if admin_match_options.is_some() && !matches!(admin_match_options, Some(AdminMatchOptions::MarkerTrade {.. })) {
                    handler.push(format!(
                        "{} Admin match options were provided, but were not of the marker_trade variant",
                        identifiers,
                    ));
                }
                handler.append(
                    &get_marker_trade_collateral_validation(
                        deps,
                        ask,
                        bid,
                        ask_collat,
                        bid_collat,
                        if let Some(
                            AdminMatchOptions::MarkerTrade {
                                accept_mismatched_bids: Some(mismatched_bids)
                            }
                        ) = admin_match_options {
                            *mismatched_bids
                        } else {
                            false
                        }
                    ),
                );
            },
            _ => handler.push(format!(
                "{} Ask collateral was of type marker trade, which did not match bid collateral",
                identifiers
            )),
        },
        AskCollateral::MarkerShareSale(ask_collat) => match &bid.collateral {
            BidCollateral::MarkerShareSale(bid_collat) => {
                // Reject match requests that send the wrong type of match options to notify the
                // caller that they are attempting an action that is unrelated to the selected match
                if admin_match_options.is_some() && !matches!(admin_match_options, Some(AdminMatchOptions::MarkerShareSale { .. })) {
                    handler.push(format!(
                        "{} Admin match options were provided, but were not of the marker_share_sale variant",
                        identifiers,
                    ));
                }
                handler.append(
                    &get_marker_share_sale_collateral_validation(
                        deps,
                        ask,
                        bid,
                        ask_collat,
                        bid_collat,
                        if let Some(
                            AdminMatchOptions::MarkerShareSale { override_quote_source }
                        ) = admin_match_options {
                            override_quote_source
                        } else {
                            &None
                        }
                    ),
                );
            },
            _ => handler.push(format!(
                "{} Ask Collateral was of type marker share sale, which did not match bid collateral",
                identifiers,
            )),
        },
        AskCollateral::ScopeTrade(ask_collat) => match &bid.collateral {
            BidCollateral::ScopeTrade(bid_collat) => {
                // Reject match requests that send the wrong type of match options to notify the
                // caller that they are attempting an action that is unrelated to the selected match
                if admin_match_options.is_some() && !matches!(admin_match_options, Some(AdminMatchOptions::ScopeTrade {.. })) {
                    handler.push(format!(
                        "{} Admin match options were provided, but were not of the scope_trade variant",
                        identifiers,
                    ));
                }
                handler.append(
                    &get_scope_trade_collateral_validation(
                        ask,
                        bid,
                        ask_collat,
                        bid_collat,
                        if let Some(
                            AdminMatchOptions::ScopeTrade {
                                accept_mismatched_bids: Some(mismatched_bids)
                            }) = admin_match_options {
                                *mismatched_bids
                        } else {
                            false
                        }
                    ),
                );
            },
            _ => handler.push(format!(
                "{} Ask Collateral was of type scope trade, which did not match bid collateral",
                identifiers,
            )),
        }
    };
    handler.handle()
}

fn get_required_attributes_error<S: Into<String>>(
    deps: &Deps<ProvenanceQuery>,
    descriptor: &Option<RequestDescriptor>,
    target_address: &Addr,
    checked_account_type: S,
) -> Option<String> {
    if let Some(attribute_requirement) = descriptor
        .clone()
        .and_then(|d| d.attribute_requirement)
        .take_if(|ar| !ar.attributes.is_empty())
    {
        let checked_account_type = checked_account_type.into();
        let attribute_response = ProvenanceQuerier::new(&deps.querier)
            .get_attributes(target_address.to_owned(), None::<String>);
        if let Ok(attributes) = attribute_response {
            let account_attribute_names = attributes
                .attributes
                .into_iter()
                .map(|a| a.name)
                .collect::<Vec<String>>();
            let (requirements_met, error_msg) = match attribute_requirement.requirement_type {
                AttributeRequirementType::All => {
                    (
                        attribute_requirement
                            .attributes
                            .iter()
                            .all(|attribute_name| account_attribute_names.contains(attribute_name)),
                        format!(
                            "the [{} account] is required to have all of the following attributes: {:?}",
                            checked_account_type,
                            &attribute_requirement.attributes
                        ),
                    )
                }
                AttributeRequirementType::Any => {
                    (
                        attribute_requirement.attributes
                            .iter()
                            .any(|attribute_name| account_attribute_names.contains(attribute_name)),
                        format!(
                            "the [{} account] did not have any of the following attributes: {:?}",
                            checked_account_type,
                            &attribute_requirement.attributes,
                        ),
                    )
                },
                AttributeRequirementType::None => {
                    (
                        // Negate the .any() to convert it into a .none(), which sadly does not exist
                        !attribute_requirement.attributes
                            .iter()
                            .any(|attribute_name| account_attribute_names.contains(attribute_name)),
                        format!(
                            "the [{} account] is required to not have any of the following attributes: {:?}",
                            checked_account_type,
                            &attribute_requirement.attributes
                        ),
                    )
                }
            };
            return if requirements_met {
                None
            } else {
                Some(error_msg)
            };
        }
        return Some(format!(
            "Failed to fetch account attributes for address [{}]: {:?}",
            target_address.as_str(),
            attribute_response.unwrap_err(),
        ));
    }
    None
}

fn get_coin_trade_collateral_validation(
    ask: &AskOrder,
    bid: &BidOrder,
    ask_collateral: &CoinTradeAskCollateral,
    bid_collateral: &CoinTradeBidCollateral,
    accept_mismatched_bids: bool,
) -> Vec<String> {
    let mut validation_messages: Vec<String> = vec![];
    let identifiers = format!(
        "COIN TRADE Match Validation for AskOrder [{}] and BidOrder [{}]:",
        &ask.id, &bid.id
    );
    let mut ask_base = ask_collateral.base.to_owned();
    let mut bid_base = bid_collateral.base.to_owned();
    ask_base.sort_by(coin_sort);
    bid_base.sort_by(coin_sort);
    if ask_base != bid_base {
        validation_messages.push(format!(
            "{} Ask base [{}] does not match bid base [{}]",
            &identifiers,
            format_coin_display(&ask_base),
            format_coin_display(&bid_base)
        ));
    }
    if !accept_mismatched_bids {
        let mut ask_quote = ask_collateral.quote.to_owned();
        let mut bid_quote = bid_collateral.quote.to_owned();
        ask_quote.sort_by(coin_sort);
        bid_quote.sort_by(coin_sort);
        if ask_quote != bid_quote {
            validation_messages.push(format!(
                "{} Ask quote [{}] does not match bid quote [{}]",
                &identifiers,
                format_coin_display(&ask_quote),
                format_coin_display(&bid_quote),
            ));
        }
    }
    validation_messages
}

fn get_marker_trade_collateral_validation(
    deps: &Deps<ProvenanceQuery>,
    ask: &AskOrder,
    bid: &BidOrder,
    ask_collateral: &MarkerTradeAskCollateral,
    bid_collateral: &MarkerTradeBidCollateral,
    accept_mismatched_bids: bool,
) -> Vec<String> {
    let mut validation_messages: Vec<String> = vec![];
    let identifiers = format!(
        "MARKER TRADE Match Validation for AskOrder [{}] and BidOrder [{}]:",
        &ask.id, &bid.id
    );
    if ask_collateral.marker_denom != bid_collateral.marker_denom {
        validation_messages.push(format!(
            "{} Ask marker denom [{}] does not match bid marker denom [{}]",
            &identifiers, &ask_collateral.marker_denom, &bid_collateral.marker_denom
        ));
    }
    if ask_collateral.marker_address.as_str() != bid_collateral.marker_address.as_str() {
        validation_messages.push(format!(
            "{} Ask marker address [{}] does not match bid marker address [{}]",
            &identifiers,
            &ask_collateral.marker_address.as_str(),
            &bid_collateral.marker_address.as_str()
        ));
    }
    // If a denom or address mismatch exists between the ask and bid, no other sane checks can be
    // made because each refers to a different marker
    if !validation_messages.is_empty() {
        return validation_messages;
    }
    let marker = match ProvenanceQuerier::new(&deps.querier)
        .get_marker_by_denom(&ask_collateral.marker_denom)
    {
        Ok(marker) => marker,
        // Exit early if the marker does not appear to be available in the Provenance Blockchain
        // system.  No marker means the remaining checks are meaningless.
        Err(_) => {
            validation_messages.push(format!(
                "{} Failed to find marker for denom [{}]",
                &identifiers, &ask_collateral.marker_denom
            ));
            return validation_messages;
        }
    };
    let marker_share_count = if let Ok(marker_coin) = get_single_marker_coin_holding(&marker) {
        if marker_coin.amount.u128() != ask_collateral.share_count.u128() {
            validation_messages.push(
                format!(
                    "{} Marker share count was [{}] but the original value when added to the contract was [{}]",
                    &identifiers,
                    marker_coin.amount.u128(),
                    ask_collateral.share_count.u128(),
                )
            );
        }
        marker_coin.amount.u128()
    } else {
        validation_messages.push(format!(
            "{} Marker had invalid coin holdings for match: [{}]. Expected a single instance of coin [{}]",
            &identifiers,
            format_coin_display(&marker.coins),
            &ask_collateral.marker_denom,
        ));
        return validation_messages;
    };
    if !accept_mismatched_bids {
        let mut ask_quote =
            multiply_coins_by_amount(&ask_collateral.quote_per_share, marker_share_count);
        let mut bid_quote = bid_collateral.quote.to_owned();
        ask_quote.sort_by(coin_sort);
        bid_quote.sort_by(coin_sort);
        if ask_quote != bid_quote {
            validation_messages.push(format!(
                "{} Ask quote [{}] did not match bid quote [{}]",
                &identifiers,
                format_coin_display(&ask_quote),
                format_coin_display(&bid_quote),
            ));
        }
    }
    validation_messages
}

fn get_marker_share_sale_collateral_validation(
    deps: &Deps<ProvenanceQuery>,
    ask: &AskOrder,
    bid: &BidOrder,
    ask_collateral: &MarkerShareSaleAskCollateral,
    bid_collateral: &MarkerShareSaleBidCollateral,
    override_quote_source: &Option<OverrideQuoteSource>,
) -> Vec<String> {
    let mut validation_messages: Vec<String> = vec![];
    let identifiers = format!(
        "MARKER SHARE SALE Match Validation for AskOrder [{}] and BidOrder [{}]:",
        &ask.id, &bid.id,
    );
    if ask_collateral.marker_denom != bid_collateral.marker_denom {
        validation_messages.push(format!(
            "{} Ask marker denom [{}] does not match bid marker denom [{}]",
            &identifiers, &ask_collateral.marker_denom, &bid_collateral.marker_denom,
        ));
    }
    if ask_collateral.marker_address.as_str() != bid_collateral.marker_address.as_str() {
        validation_messages.push(format!(
            "{} Ask marker address [{}] does not match bid marker address [{}]",
            &identifiers,
            &ask_collateral.marker_address.as_str(),
            &bid_collateral.marker_address.as_str()
        ));
    }
    // If a denom or address mismatch exists between the ask and bid, no other sane checks can be
    // made because each refers to a different marker
    if !validation_messages.is_empty() {
        return validation_messages;
    }
    match ask_collateral.sale_type {
        ShareSaleType::SingleTransaction => {
            if bid_collateral.share_count.u128() < ask_collateral.total_shares_in_sale.u128() {
                validation_messages.push(format!(
                    "{} Ask requested that [{}] shares be purchased, but bid wanted too few [{}]",
                    &identifiers,
                    ask_collateral.total_shares_in_sale.u128(),
                    bid_collateral.share_count.u128(),
                ));
            }
        }
        ShareSaleType::MultipleTransactions => {}
    }
    let marker = match ProvenanceQuerier::new(&deps.querier)
        .get_marker_by_denom(&ask_collateral.marker_denom)
    {
        Ok(marker) => marker,
        // Exit early if the marker does not appear to be available in the Provenance Blockchain
        // system.  No marker means the remaining checks are meaningless.
        Err(_) => {
            validation_messages.push(format!(
                "{} Failed to find marker for denom [{}]",
                &identifiers, &ask_collateral.marker_denom
            ));
            return validation_messages;
        }
    };
    if let Ok(marker_coin) = get_single_marker_coin_holding(&marker) {
        if marker_coin.amount.u128() < ask_collateral.remaining_shares_in_sale.u128() {
            validation_messages.push(format!(
                "{} Marker is not synced with the contract! Marker had [{}] shares remaining, which is less than the listed available share count of [{}]",
                &identifiers,
                marker_coin.amount.u128(),
                ask_collateral.remaining_shares_in_sale.u128(),
            ));
        }
    } else {
        validation_messages.push(format!(
            "{} Marker had invalid coin holdings for match: [{}]. Expected a single instance of coin [{}]",
            &identifiers,
            format_coin_display(&marker.coins),
            &ask_collateral.marker_denom,
        ));
        return validation_messages;
    }
    let mut ask_quote_per_share = ask_collateral.quote_per_share.to_owned();
    ask_quote_per_share.sort_by(coin_sort);
    let mut bid_quote_per_share = bid_collateral.get_quote_per_share();
    bid_quote_per_share.sort_by(coin_sort);
    match override_quote_source {
        Some(_) => {
            // Regardless of if the ask or bid is used as the quote source, we must verify that they
            // both contain the same coin types.  Having a bad alignment on quote coin types is prone
            // to produce odd values after match execution
            if ask_quote_per_share.len() != bid_quote_per_share.len() {
                validation_messages.push(format!(
                    "{} Ask quote per share [{}] had a different amount of specified coin types than bid quote per share [{}]",
                    &identifiers,
                    format_coin_display(&ask_quote_per_share),
                    format_coin_display(&bid_quote_per_share),
                ));
            } else {
                // Verify that each ask coin's denom matches a bid coin denom.  This, in tandem with
                // the length verification above, ensures that each quote contains the same coin
                // types.
                for ask_coin in &ask_quote_per_share {
                    if let Some(bid_coin) = bid_quote_per_share
                        .iter()
                        .find(|bc| bc.denom == ask_coin.denom)
                    {
                        // The bid coin must be at least equal to each ask coin for a match to execute.
                        // This ensures that the funds stored for the bid will never be exceeded when
                        // executing the match.
                        if bid_coin.amount.u128() < ask_coin.amount.u128() {
                            validation_messages.push(format!(
                                "{} Ask quote per share [{}] required at least [{}{}] but bid quote per share [{}] only specified [{}{}]",
                                &identifiers,
                                format_coin_display(&ask_quote_per_share),
                                ask_coin.amount.u128(),
                                &ask_coin.denom,
                                format_coin_display(&bid_quote_per_share),
                                bid_coin.amount.u128(),
                                &bid_coin.denom,
                            ));
                        }
                    } else {
                        validation_messages.push(format!(
                            "{} Ask quote per share [{}] contained coin denom [{}] but bid quote per share [{}] did not",
                            &identifiers,
                            format_coin_display(&ask_quote_per_share),
                            ask_coin.denom,
                            format_coin_display(&bid_quote_per_share),
                        ));
                        continue;
                    }
                }
            }
        }
        None => {
            if ask_quote_per_share != bid_quote_per_share {
                validation_messages.push(format!(
                    "{} Ask quote per share [{}] did not equal bid quote per share [{}]",
                    &identifiers,
                    format_coin_display(&ask_collateral.quote_per_share),
                    format_coin_display(&bid_collateral.get_quote_per_share()),
                ));
            }
        }
    }
    validation_messages
}

fn get_scope_trade_collateral_validation(
    ask: &AskOrder,
    bid: &BidOrder,
    ask_collateral: &ScopeTradeAskCollateral,
    bid_collateral: &ScopeTradeBidCollateral,
    accept_mismatched_bids: bool,
) -> Vec<String> {
    let mut validation_messages: Vec<String> = vec![];
    let identifiers = format!(
        "SCOPE TRADE Match Validation for AskOrder [{}] and BidOrder [{}]:",
        &ask.id, &bid.id,
    );
    if ask_collateral.scope_address != bid_collateral.scope_address {
        validation_messages.push(format!(
            "{} Ask scope address [{}] does not match bid scope address [{}]",
            &identifiers, &ask_collateral.scope_address, &bid_collateral.scope_address,
        ));
    }
    if !accept_mismatched_bids {
        let mut ask_quote = ask_collateral.quote.to_owned();
        let mut bid_quote = bid_collateral.quote.to_owned();
        ask_quote.sort_by(coin_sort);
        bid_quote.sort_by(coin_sort);
        if ask_quote != bid_quote {
            validation_messages.push(format!(
                "{} Ask quote [{}] does not match bid quote [{}]",
                &identifiers,
                format_coin_display(&ask_quote),
                format_coin_display(&bid_quote),
            ));
        }
    }
    validation_messages
}

#[cfg(test)]
mod tests {
    use crate::test::mock_marker::{MockMarker, DEFAULT_MARKER_ADDRESS, DEFAULT_MARKER_DENOM};
    use crate::test::mock_scope::DEFAULT_SCOPE_ADDR;
    use crate::test::request_helpers::{
        mock_ask_marker_share_sale, mock_ask_marker_trade, mock_ask_order,
        mock_ask_order_with_descriptor, mock_ask_scope_trade, mock_bid_marker_share_sale,
        mock_bid_marker_trade, mock_bid_order, mock_bid_scope_trade, mock_bid_with_descriptor,
        replace_ask_quote, replace_bid_quote,
    };
    use crate::types::core::error::ContractError;
    use crate::types::request::admin_match_options::{AdminMatchOptions, OverrideQuoteSource};
    use crate::types::request::ask_types::ask_collateral::AskCollateral;
    use crate::types::request::ask_types::ask_order::AskOrder;
    use crate::types::request::bid_types::bid_collateral::BidCollateral;
    use crate::types::request::bid_types::bid_order::BidOrder;
    use crate::types::request::request_descriptor::{AttributeRequirement, RequestDescriptor};
    use crate::types::request::request_type::RequestType;
    use crate::types::request::share_sale_type::ShareSaleType;
    use crate::util::constants::NHASH;
    use crate::validation::ask_order_validation::validate_ask_order;
    use crate::validation::bid_order_validation::validate_bid_order;
    use crate::validation::execute_match_validation::{
        get_required_attributes_error, validate_match,
    };
    use cosmwasm_std::{coin, coins, Addr, Deps};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{AccessGrant, MarkerAccess, ProvenanceQuery};

    #[test]
    fn test_successful_coin_trade_validation() {
        let mut deps = mock_dependencies(&[]);
        let mut ask_order = AskOrder::new(
            "ask_id",
            Addr::unchecked("asker"),
            AskCollateral::coin_trade(&coins(100, NHASH), &coins(250, "othercoin")),
            Some(RequestDescriptor::new_populated_attributes(
                "some description",
                AttributeRequirement::all(&["attribute.pb"]),
            )),
        )
        .expect("expected validation to pass for the new ask order");
        let mut bid_order = BidOrder::new(
            "bid_id",
            Addr::unchecked("bidder"),
            BidCollateral::coin_trade(&coins(100, NHASH), &coins(250, "othercoin")),
            // Provwasm has a limitation - it will only allow one address to have mocked attributes
            // at a time, so we can't simultaneously test the presence of attributes on both asker
            // and bidder.  Testing all and none together is the best we can do
            Some(RequestDescriptor::new_populated_attributes(
                "bid description",
                AttributeRequirement::none(&["otherattribute.pb"]),
            )),
        )
        .expect("expected validation to pass for the new bid order");
        deps.querier
            .with_attributes("bidder", &[("attribute.pb", "value", "string")]);
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None)
            .expect("expected validation to pass for a simple coin to coin trade");
        ask_order.collateral = AskCollateral::coin_trade(
            &[coin(10, "a"), coin(20, "b"), coin(30, "c")],
            &[coin(50, "d"), coin(60, "e"), coin(70, "f")],
        );
        validate_ask_order(&ask_order).expect("expected modified ask order to remain valid");
        bid_order.collateral = BidCollateral::coin_trade(
            &[coin(30, "c"), coin(10, "a"), coin(20, "b")],
            &[coin(50, "d"), coin(70, "f"), coin(60, "e")],
        );
        validate_bid_order(&bid_order).expect("expected modified bid order to remain valid");
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None)
            .expect("expected validation to pass for a complex coin trade with mismatched orders");
    }

    #[test]
    fn test_successful_marker_trade_validation() {
        let mut deps = mock_dependencies(&[]);
        let marker = MockMarker {
            denom: "targetcoin".to_string(),
            coins: coins(10, "targetcoin"),
            ..MockMarker::default()
        }
        .to_marker();
        deps.querier.with_markers(vec![marker.clone()]);
        let mut ask_order = AskOrder::new(
            "ask_id",
            Addr::unchecked("asker"),
            AskCollateral::marker_trade(
                Addr::unchecked("marker"),
                "targetcoin",
                10,
                &coins(100, NHASH),
                &[AccessGrant {
                    address: Addr::unchecked("asker"),
                    permissions: vec![MarkerAccess::Admin],
                }],
            ),
            Some(RequestDescriptor::new_populated_attributes(
                "Best ask ever",
                AttributeRequirement::none(&["badattribute.pio"]),
            )),
        )
        .expect("expected the ask order to be valid");
        deps.querier
            .with_attributes("asker", &[("required.pb", "value", "string")]);
        let mut bid_order = BidOrder::new(
            "bid_id",
            Addr::unchecked("bidder"),
            BidCollateral::marker_trade(
                Addr::unchecked("marker"),
                "targetcoin",
                &coins(1000, NHASH),
                None,
            ),
            Some(RequestDescriptor::new_populated_attributes(
                "Best bid ever",
                AttributeRequirement::all(&["required.pb"]),
            )),
        )
        .expect("expected the bid order to be valid");
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None)
            .expect("expected validation to pass for a single coin quote");
        replace_ask_quote(
            &mut ask_order,
            &[
                coin(10, NHASH),
                coin(5, "otherthing"),
                coin(13, "worstthing"),
            ],
        );
        validate_ask_order(&ask_order)
            .expect("expected the ask order to remain valid after changes");
        replace_bid_quote(
            &mut bid_order,
            &[
                coin(100, NHASH),
                coin(50, "otherthing"),
                coin(130, "worstthing"),
            ],
        );
        validate_bid_order(&bid_order)
            .expect("expected the bid order to remain valid after changes");
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None)
            .expect("expected the validation to pass for a multi-coin quote");
    }

    #[test]
    fn test_successful_marker_share_sale_single_transaction_validation() {
        let mut deps = mock_dependencies(&[]);
        let marker = MockMarker {
            denom: "targetcoin".to_string(),
            coins: coins(10, "targetcoin"),
            ..MockMarker::default()
        }
        .to_marker();
        deps.querier.with_markers(vec![marker.clone()]);
        let mut ask_order = AskOrder::new(
            "ask_id",
            Addr::unchecked("asker"),
            AskCollateral::marker_share_sale(
                Addr::unchecked(DEFAULT_MARKER_ADDRESS),
                "targetcoin",
                5,
                5,
                &coins(100, NHASH),
                &[AccessGrant {
                    address: Addr::unchecked("asker"),
                    permissions: vec![MarkerAccess::Admin],
                }],
                ShareSaleType::SingleTransaction,
            ),
            Some(RequestDescriptor::new_populated_attributes(
                "ask description",
                AttributeRequirement::all(&["required.pb", "required2.pb"]),
            )),
        )
        .expect("expected ask order to pass validation");
        let mut bid_order = BidOrder::new(
            "bid_id",
            Addr::unchecked("bidder"),
            BidCollateral::marker_share_sale(
                Addr::unchecked(DEFAULT_MARKER_ADDRESS),
                "targetcoin",
                5,
                &coins(500, NHASH),
            ),
            Some(RequestDescriptor::new_populated_attributes(
                "bid description",
                AttributeRequirement::none(&["bad.attribute"]),
            )),
        )
        .expect("expected bid order to pass validation");
        deps.querier.with_attributes(
            "bidder",
            &[
                ("required.pb", "value", "string"),
                ("required2.pb", "value2", "string"),
            ],
        );
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None)
            .expect("expected match validation to pass with correct parameters");
        replace_ask_quote(&mut ask_order, &[coin(100, NHASH), coin(250, "yolocoin")]);
        validate_ask_order(&ask_order)
            .expect("expected ask order to pass validation with a multi coin quote per share");
        replace_bid_quote(&mut bid_order, &[coin(500, NHASH), coin(1250, "yolocoin")]);
        validate_bid_order(&bid_order)
            .expect("expected bid order to pass validation with multi coin quote");
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None).expect(
            "expected match validation to pass when ask and bid order used a multi-coin quote",
        );
    }

    #[test]
    fn test_successful_marker_share_sale_multiple_transaction_validation() {
        let mut deps = mock_dependencies(&[]);
        let marker = MockMarker {
            denom: "targetcoin".to_string(),
            coins: coins(10, "targetcoin"),
            ..MockMarker::default()
        }
        .to_marker();
        deps.querier.with_markers(vec![marker.clone()]);
        let mut ask_order = AskOrder::new(
            "ask_id",
            Addr::unchecked("asker"),
            AskCollateral::marker_share_sale(
                Addr::unchecked(DEFAULT_MARKER_ADDRESS),
                "targetcoin",
                5,
                5,
                &coins(100, NHASH),
                &[AccessGrant {
                    address: Addr::unchecked("asker"),
                    permissions: vec![MarkerAccess::Admin],
                }],
                ShareSaleType::MultipleTransactions,
            ),
            Some(RequestDescriptor::new_populated_attributes(
                "ask description",
                AttributeRequirement::none(&["a.pb", "b.pb"]),
            )),
        )
        .expect("expected ask order to pass validation");
        deps.querier
            .with_attributes("asker", &[("second.pb", "value", "string")]);
        let mut bid_order = BidOrder::new(
            "bid_id",
            Addr::unchecked("bidder"),
            BidCollateral::marker_share_sale(
                Addr::unchecked(DEFAULT_MARKER_ADDRESS),
                "targetcoin",
                5,
                &coins(500, NHASH),
            ),
            Some(RequestDescriptor::new_populated_attributes(
                "bid description",
                AttributeRequirement::any(&["first.pb", "second.pb"]),
            )),
        )
        .expect("expected bid order to pass validation");
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None)
            .expect("expected match validation to pass with correct parameters");
        replace_ask_quote(&mut ask_order, &[coin(100, NHASH), coin(250, "yolocoin")]);
        validate_ask_order(&ask_order)
            .expect("expected ask order to pass validation with a multi coin quote per share");
        replace_bid_quote(&mut bid_order, &[coin(500, NHASH), coin(1250, "yolocoin")]);
        validate_bid_order(&bid_order)
            .expect("expected bid order to pass validation with multi coin quote");
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None).expect(
            "expected match validation to pass when ask and bid order used a multi-coin quote",
        );
    }

    #[test]
    fn test_successful_scope_trade_validation() {
        let mut deps = mock_dependencies(&[]);
        let mut ask_order = AskOrder::new(
            "ask_id",
            Addr::unchecked("asker"),
            AskCollateral::scope_trade("scope", &coins(100, NHASH)),
            Some(RequestDescriptor::new_populated_attributes(
                "ask description",
                AttributeRequirement::all(&["a.pb", "b.pb", "c.pb"]),
            )),
        )
        .expect("expected ask order to pass validation");
        let mut bid_order = BidOrder::new(
            "bid_id",
            Addr::unchecked("bidder"),
            BidCollateral::scope_trade("scope", &coins(100, NHASH)),
            Some(RequestDescriptor::new_populated_attributes(
                "bid description",
                AttributeRequirement::none(&["no-u.pio"]),
            )),
        )
        .expect("expected bid order to pass validation");
        deps.querier.with_attributes(
            "bidder",
            &[
                ("a.pb", "value", "string"),
                ("b.pb", "value", "string"),
                ("c.pb", "value", "string"),
            ],
        );
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None)
            .expect("expected match validation to pass for correct scope trade parameters");
        replace_ask_quote(&mut ask_order, &[coin(100, "acoin"), coin(100, "bcoin")]);
        validate_ask_order(&ask_order).expect("multi coin ask order should pass validation");
        replace_bid_quote(&mut bid_order, &[coin(100, "acoin"), coin(100, "bcoin")]);
        validate_bid_order(&bid_order).expect("multi coin bid order should pass validation");
        validate_match(&deps.as_ref(), &ask_order, &bid_order, &None).expect(
            "expected match validation to pass when ask and bid order used a multi-coin quote",
        );
    }

    #[test]
    fn test_mismatched_ask_and_bid_types() {
        let deps = mock_dependencies(&[]);
        RequestType::iterator().for_each(|ask_request_type| {
            let ask_order = AskOrder {
                id: "ask_id".to_string(),
                ask_type: ask_request_type.to_owned(),
                owner: Addr::unchecked("ask_addr"),
                collateral: AskCollateral::coin_trade(&[], &[]),
                descriptor: None,
            };
            RequestType::iterator().for_each(|bid_request_type| {
                // Skip duplicate types - they obviously will match
                if ask_request_type == bid_request_type {
                    return;
                }
                let bid_order = BidOrder {
                    id: "bid_id".to_string(),
                    bid_type: bid_request_type.to_owned(),
                    owner: Addr::unchecked("bid_addr"),
                    collateral: BidCollateral::coin_trade(&[], &[]),
                    descriptor: None,
                };
                assert_validation_failure_with_mismatch_test(
                    format!(
                        "Ask type [{}] and bid type [{}] mismatch",
                        ask_request_type.get_name(),
                        bid_request_type.get_name()
                    ),
                    &deps.as_ref(),
                    &ask_order,
                    &bid_order,
                    expected_error(format!(
                        "Ask type [{}] does not match bid type [{}]",
                        ask_request_type.get_name(),
                        bid_request_type.get_name()
                    )),
                    true,
                );
            });
        });
    }

    #[test]
    fn test_asker_missing_required_attributes() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Ask order is required to have an attribute but it has no attributes",
            &deps.as_ref(),
            &mock_ask_order(AskCollateral::coin_trade(&[], &[])),
            &mock_bid_with_descriptor(
                BidCollateral::coin_trade(&[], &[]),
                RequestDescriptor::new_populated_attributes("description", AttributeRequirement::all(&["myattribute.pb"])),
            ),
            "the [asker account] is required to have all of the following attributes: [\"myattribute.pb\"]",
            true,
        );
    }

    #[test]
    fn test_bidder_missing_required_attributes() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Bid order is required to have an attribute but it has no attributes",
            &deps.as_ref(),
            &mock_ask_order_with_descriptor(
                AskCollateral::coin_trade(&[], &[]),
                RequestDescriptor::new_populated_attributes("description", AttributeRequirement::all(&["attr.pb"])),
            ),
            &mock_bid_order(BidCollateral::coin_trade(&[], &[])),
            "the [bidder account] is required to have all of the following attributes: [\"attr.pb\"]",
            true,
        );
    }

    #[test]
    fn test_get_required_attributes_error_none_scenarios() {
        let deps = mock_dependencies(&[]);
        let address = Addr::unchecked("asker");
        let account_type = "asker";
        assert_eq!(
            None,
            get_required_attributes_error(&deps.as_ref(), &None, &address, account_type,),
            "None should be returned when attribute requirement is not provided",
        );
        assert_eq!(
            None,
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_none()),
                &address,
                account_type,
            ),
            "None should be returned when the request descriptor has no attribute requirement",
        );
        assert_eq!(
            None,
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "description",
                    AttributeRequirement::all::<String>(&[]),
                )),
                &address,
                account_type,
            ),
            "None should be returned when the attribute vector is empty in the attribute requirement",
        );
    }

    #[test]
    fn test_get_required_attributes_error_all_type_scenarios() {
        let mut deps = mock_dependencies(&[]);
        let address = Addr::unchecked("asker");
        let account_type = "asker";
        assert_eq!(
            "the [asker account] is required to have all of the following attributes: [\"a.pb\"]",
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::all(&["a.pb"]),
                )),
                &address,
                account_type,
            )
            .expect("expected a string response with erroneous input"),
            "expected the proper error message when no attributes were found",
        );
        deps.querier
            .with_attributes("asker", &[("a.pb", "value", "string")]);
        assert_eq!(
            None,
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::all(&["a.pb"]),
                )),
                &address,
                account_type,
            ),
            "expected None to be returned when all attributes were held on the account",
        );
        assert_eq!(
            "the [asker account] is required to have all of the following attributes: [\"a.pb\", \"b.pb\"]",
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::all(&["a.pb", "b.pb"]),
                )),
                &address,
                account_type,
            ).expect("expected a string response with erroneous input"),
            "expected an error to occur when the account only has one of many of the needed attributes",
        );
    }

    #[test]
    fn test_get_required_attributes_error_any_type_scenarios() {
        let mut deps = mock_dependencies(&[]);
        let address = Addr::unchecked("bidder");
        let account_type = "bidder";
        assert_eq!(
            "the [bidder account] did not have any of the following attributes: [\"a.pb\"]",
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::any(&["a.pb"]),
                )),
                &address,
                account_type,
            )
            .expect("expected a string response with erroneous input"),
            "expected the proper error message when no attributes were found",
        );
        deps.querier
            .with_attributes("bidder", &[("a.pb", "value", "string")]);
        assert_eq!(
            None,
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::any(&["a.pb"]),
                )),
                &address,
                account_type,
            ),
            "expected None to be returned when one of one attributes were held on the account",
        );
        assert_eq!(
            "the [bidder account] did not have any of the following attributes: [\"b.pb\", \"c.pb\", \"d.pb\"]",
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::any(&["b.pb", "c.pb", "d.pb"]),
                )),
                &address,
                account_type,
            ).expect("expected a string response with erroneous input"),
            "expected an error to be returned when the account did not have any of the required attributes, but had other attributes",
        );
        deps.querier
            .with_attributes("bidder", &[("d.pb", "value", "string")]);
        assert_eq!(
            None,
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::any(&["b.pb", "c.pb", "d.pb"]),
                )),
                &address,
                account_type,
            ),
            "expected None to be returned when the account had one of the required attributes",
        );
    }

    #[test]
    fn test_get_required_attributes_error_none_type_scenarios() {
        let mut deps = mock_dependencies(&[]);
        let address = Addr::unchecked("bidder");
        let account_type = "bidder";
        assert_eq!(
            None,
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::none(&["a.pb"]),
                )),
                &address,
                account_type,
            ),
            "expected None to be returned when the account did not have any of the attributes",
        );
        deps.querier
            .with_attributes("bidder", &[("a.pb", "value", "string")]);
        assert_eq!(
            "the [bidder account] is required to not have any of the following attributes: [\"a.pb\"]",
            get_required_attributes_error(
                &deps.as_ref(),
                &Some(RequestDescriptor::new_populated_attributes(
                    "desc",
                    AttributeRequirement::none(&["a.pb"]),
                )),
                &address,
                account_type,
            ).expect("expected an error string to be returned for erroneous input"),
            "expected the proper error message when a none requirement found related values",
        );
    }

    #[test]
    fn test_mismatched_collateral_types() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Ask collateral coin_trade and bid collateral marker_trade mismatch",
            &deps.as_ref(),
            &mock_ask_order(AskCollateral::coin_trade(&[], &[])),
            &mock_bid_order(mock_bid_marker_trade("marker", "somecoin", &[], None)),
            expected_error(
                "Ask collateral was of type coin trade, which did not match bid collateral",
            ),
            true,
        );
        assert_validation_failure_with_mismatch_test(
            "Ask collateral marker_trade and bid collateral coin_trade mismatch",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_trade("marker", "somecoin", 400, &[])),
            &mock_bid_order(BidCollateral::coin_trade(&[], &[])),
            expected_error(
                "Ask collateral was of type marker trade, which did not match bid collateral",
            ),
            true,
        );
    }

    #[test]
    fn test_mismatched_admin_options_coin_trade() {
        let deps = mock_dependencies(&[]);
        let illegal_variants = vec![
            AdminMatchOptions::marker_trade_options(true),
            AdminMatchOptions::marker_trade_empty(),
            AdminMatchOptions::marker_share_sale_options(OverrideQuoteSource::Ask),
            AdminMatchOptions::marker_share_sale_empty(),
            AdminMatchOptions::scope_trade_options(true),
            AdminMatchOptions::scope_trade_empty(),
        ];
        for illegal_variant in illegal_variants {
            assert_validation_failure_options(
                format!(
                    "Coin trades with illegal admin options variant: {:?}",
                    illegal_variant
                ),
                &deps.as_ref(),
                &mock_ask_order(AskCollateral::coin_trade(&[], &[])),
                &mock_bid_order(BidCollateral::coin_trade(&[], &[])),
                expected_error(
                    "Admin match options were provided, but were not of the coin_trade variant",
                ),
                illegal_variant,
            );
        }
    }

    #[test]
    fn test_mismatched_admin_options_marker_trade() {
        let deps = mock_dependencies(&[]);
        let illegal_variants = vec![
            AdminMatchOptions::coin_trade_options(true),
            AdminMatchOptions::coin_trade_empty(),
            AdminMatchOptions::marker_share_sale_options(OverrideQuoteSource::Ask),
            AdminMatchOptions::marker_share_sale_empty(),
            AdminMatchOptions::scope_trade_options(true),
            AdminMatchOptions::scope_trade_empty(),
        ];
        for illegal_variant in illegal_variants {
            assert_validation_failure_options(
                format!(
                    "Marker trades with illegal admin options variant: {:?}",
                    illegal_variant
                ),
                &deps.as_ref(),
                &mock_ask_order(mock_ask_marker_trade(
                    DEFAULT_MARKER_ADDRESS,
                    DEFAULT_MARKER_DENOM,
                    10,
                    &[],
                )),
                &mock_bid_order(mock_bid_marker_trade(
                    DEFAULT_MARKER_ADDRESS,
                    DEFAULT_MARKER_DENOM,
                    &[],
                    None,
                )),
                expected_error(
                    "Admin match options were provided, but were not of the marker_trade variant",
                ),
                illegal_variant,
            );
        }
    }

    #[test]
    fn test_mismatched_admin_options_marker_share_sale() {
        let deps = mock_dependencies(&[]);
        let illegal_variants = vec![
            AdminMatchOptions::coin_trade_options(true),
            AdminMatchOptions::coin_trade_empty(),
            AdminMatchOptions::marker_trade_options(true),
            AdminMatchOptions::marker_trade_empty(),
            AdminMatchOptions::scope_trade_options(true),
            AdminMatchOptions::scope_trade_empty(),
        ];
        for illegal_variant in illegal_variants {
            assert_validation_failure_options(
                format!(
                    "Marker share sale with illegal admin options variant: {:?}",
                    illegal_variant
                ),
                &deps.as_ref(),
                &mock_ask_order(mock_ask_marker_share_sale(
                    DEFAULT_MARKER_ADDRESS,
                    DEFAULT_MARKER_DENOM,
                    10,
                    10,
                    &[],
                    ShareSaleType::SingleTransaction,
                )),
                &mock_bid_order(mock_bid_marker_share_sale(
                    DEFAULT_MARKER_ADDRESS,
                    DEFAULT_MARKER_DENOM,
                    10,
                    &[],
                )),
                expected_error(
                    "Admin match options were provided, but were not of the marker_share_sale variant",
                ),
                illegal_variant,
            );
        }
    }

    #[test]
    fn test_mismatched_admin_options_scope_trade() {
        let deps = mock_dependencies(&[]);
        let illegal_variants = vec![
            AdminMatchOptions::coin_trade_options(true),
            AdminMatchOptions::coin_trade_empty(),
            AdminMatchOptions::marker_trade_options(true),
            AdminMatchOptions::marker_trade_empty(),
            AdminMatchOptions::marker_share_sale_options(OverrideQuoteSource::Ask),
            AdminMatchOptions::marker_share_sale_empty(),
        ];
        for illegal_variant in illegal_variants {
            assert_validation_failure_options(
                format!(
                    "Scope trades with illegal admin options variant: {:?}",
                    illegal_variant
                ),
                &deps.as_ref(),
                &mock_ask_order(mock_ask_scope_trade(DEFAULT_SCOPE_ADDR, &[])),
                &mock_bid_order(mock_bid_scope_trade(DEFAULT_SCOPE_ADDR, &[])),
                expected_error(
                    "Admin match options were provided, but were not of the scope_trade variant",
                ),
                illegal_variant,
            );
        }
    }

    #[test]
    fn test_mismatched_coin_trade_bases() {
        let deps = mock_dependencies(&[]);
        let mut ask_order = mock_ask_order(AskCollateral::coin_trade(&coins(150, NHASH), &[]));
        let mut bid_order = mock_bid_order(BidCollateral::coin_trade(&coins(100, NHASH), &[]));
        assert_validation_failure_with_mismatch_test(
            "Ask base denoms match but amounts do not match",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            coin_trade_error("Ask base [150nhash] does not match bid base [100nhash]"),
            true,
        );
        ask_order.collateral = AskCollateral::coin_trade(&coins(100, "a"), &[]);
        bid_order.collateral = BidCollateral::coin_trade(&coins(100, "b"), &[]);
        assert_validation_failure_with_mismatch_test(
            "Ask base amounts match but denoms do not match",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            coin_trade_error("Ask base [100a] does not match bid base [100b]"),
            true,
        );
        ask_order.collateral = AskCollateral::coin_trade(&[coin(100, "a"), coin(100, "b")], &[]);
        bid_order.collateral = BidCollateral::coin_trade(&coins(100, "a"), &[]);
        assert_validation_failure_with_mismatch_test(
            "Ask base includes coin not in bid base",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            coin_trade_error("Ask base [100a, 100b] does not match bid base [100a]"),
            true,
        );
        ask_order.collateral = AskCollateral::coin_trade(&coins(100, "a"), &[]);
        bid_order.collateral = BidCollateral::coin_trade(&[coin(100, "a"), coin(100, "b")], &[]);
        assert_validation_failure_with_mismatch_test(
            "Bid base includes coin not in ask base",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            coin_trade_error("Ask base [100a] does not match bid base [100a, 100b]"),
            true,
        );
    }

    #[test]
    fn test_mismatched_coin_trade_quotes() {
        let deps = mock_dependencies(&[]);
        let mut ask_order = mock_ask_order(AskCollateral::coin_trade(&[], &coins(1, NHASH)));
        let mut bid_order = mock_bid_order(BidCollateral::coin_trade(&[], &coins(2, NHASH)));
        assert_validation_failure_with_mismatch_test(
            "Ask quote denoms match but amounts do not match",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            coin_trade_error("Ask quote [1nhash] does not match bid quote [2nhash]"),
            false,
        );
        ask_order.collateral = AskCollateral::coin_trade(&[], &coins(4000, "acoin"));
        bid_order.collateral = BidCollateral::coin_trade(&[], &coins(4000, "bcoin"));
        assert_validation_failure_with_mismatch_test(
            "Ask quote amounts match but denoms do not match",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            coin_trade_error("Ask quote [4000acoin] does not match bid quote [4000bcoin]"),
            false,
        );
        ask_order.collateral =
            AskCollateral::coin_trade(&[], &[coin(200, "acoin"), coin(200, "bcoin")]);
        bid_order.collateral = BidCollateral::coin_trade(&[], &coins(200, "acoin"));
        assert_validation_failure_with_mismatch_test(
            "Ask quote includes coin not in bid quote",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            coin_trade_error("Ask quote [200acoin, 200bcoin] does not match bid quote [200acoin]"),
            false,
        );
        ask_order.collateral = AskCollateral::coin_trade(&[], &coins(200, "acoin"));
        bid_order.collateral =
            BidCollateral::coin_trade(&[], &[coin(200, "acoin"), coin(200, "bcoin")]);
        assert_validation_failure_with_mismatch_test(
            "Bid quote includes coin not in ask quote",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            coin_trade_error("Ask quote [200acoin] does not match bid quote [200acoin, 200bcoin]"),
            false,
        );
        validate_match(
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            &Some(AdminMatchOptions::coin_trade_options(true)),
        )
        .expect("validation should pass when mismatched bids are accepted");
    }

    #[test]
    fn test_marker_trade_mismatched_denoms() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Ask marker denom does not match bid marker denom",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_trade("marker", "firstmarkerdenom", 10, &[])),
            &mock_bid_order(mock_bid_marker_trade("marker", "secondmarkerdenom", &[], None)),
            marker_trade_error("Ask marker denom [firstmarkerdenom] does not match bid marker denom [secondmarkerdenom]"),
            true,
        );
    }

    #[test]
    fn test_marker_trade_mismatched_marker_addresses() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Ask marker address does not match bid marker address",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_trade("marker1", "test", 10, &[])),
            &mock_bid_order(mock_bid_marker_trade("marker2", "test", &[], None)),
            marker_trade_error(
                "Ask marker address [marker1] does not match bid marker address [marker2]",
            ),
            true,
        );
    }

    #[test]
    fn test_marker_trade_missing_marker_in_provland() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "No marker was mocked for target marker address",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_trade("marker", "test", 10, &[])),
            &mock_bid_order(mock_bid_marker_trade("marker", "test", &[], None)),
            marker_trade_error("Failed to find marker for denom [test]"),
            true,
        );
    }

    #[test]
    fn test_marker_trade_unexpected_holdings() {
        let mut deps = mock_dependencies(&[]);
        let mut marker = MockMarker {
            denom: "targetcoin".to_string(),
            coins: vec![coin(100, NHASH), coin(50, "mydenom")],
            ..MockMarker::default()
        }
        .to_marker();
        deps.querier.with_markers(vec![marker.clone()]);
        let ask = mock_ask_order(mock_ask_marker_trade("marker", "targetcoin", 10, &[]));
        let bid = mock_bid_order(mock_bid_marker_trade("marker", "targetcoin", &[], None));
        assert_validation_failure_with_mismatch_test(
            "Marker contained none of its own denom",
            &deps.as_ref(),
            &ask,
            &bid,
            marker_trade_error("Marker had invalid coin holdings for match: [100nhash, 50mydenom]. Expected a single instance of coin [targetcoin]"),
            true,
        );
        marker.coins = vec![];
        deps.querier.with_markers(vec![marker.clone()]);
        assert_validation_failure_with_mismatch_test(
            "Marker contained no coins whatsoever",
            &deps.as_ref(),
            &ask,
            &bid,
            marker_trade_error("Marker had invalid coin holdings for match: []. Expected a single instance of coin [targetcoin]"),
            true,
        );
        marker.coins = vec![coin(10, "targetcoin"), coin(20, "targetcoin")];
        deps.querier.with_markers(vec![marker]);
        assert_validation_failure_with_mismatch_test(
            "Marker contained duplicates of the target coin",
            &deps.as_ref(),
            &ask,
            &bid,
            marker_trade_error("Marker had invalid coin holdings for match: [10targetcoin, 20targetcoin]. Expected a single instance of coin [targetcoin]"),
            true,
        );
    }

    #[test]
    fn test_marker_trade_unexpected_share_count() {
        let mut deps = mock_dependencies(&[]);
        let marker = MockMarker {
            denom: "targetcoin".to_string(),
            coins: coins(50, "targetcoin"),
            ..MockMarker::default()
        }
        .to_marker();
        deps.querier.with_markers(vec![marker]);
        assert_validation_failure_with_mismatch_test(
            "Marker contained a coin count that did not match the value recorded when the ask was made",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_trade("marker", "targetcoin", 49, &[])),
            &mock_bid_order(mock_bid_marker_trade("marker", "targetcoin", &[], None)),
            marker_trade_error("Marker share count was [50] but the original value when added to the contract was [49]"),
            true,
        );
    }

    #[test]
    fn test_marker_trade_mismatched_ask_and_bid_quotes() {
        let mut deps = mock_dependencies(&[]);
        let marker = MockMarker {
            denom: "targetcoin".to_string(),
            coins: coins(10, "targetcoin"),
            ..MockMarker::default()
        }
        .to_marker();
        deps.querier.with_markers(vec![marker]);
        assert_validation_failure_with_mismatch_test(
            "Marker bid had a bad value to match the calculated marker quote",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_trade(
                "marker",
                "targetcoin",
                10,
                &coins(50, NHASH),
            )),
            &mock_bid_order(mock_bid_marker_trade(
                "marker",
                "targetcoin",
                &coins(200, NHASH),
                None,
            )),
            marker_trade_error("Ask quote [500nhash] did not match bid quote [200nhash]"),
            false,
        );
    }

    #[test]
    fn test_marker_share_sale_mismatched_denoms() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Marker ask and bid collaterals refer to different marker denoms",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale(
                "marker",
                "denom1",
                10,
                10,
                &[],
                ShareSaleType::SingleTransaction,
            )),
            &mock_bid_order(mock_bid_marker_share_sale("marker", "denom2", 10, &[])),
            marker_share_sale_error(
                "Ask marker denom [denom1] does not match bid marker denom [denom2]",
            ),
            true,
        );
    }

    #[test]
    fn test_marker_share_sale_mismatched_marker_addresses() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Marker ask and bid addresses refer to different markers",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale(
                "marker1",
                "denom",
                10,
                10,
                &[],
                ShareSaleType::SingleTransaction,
            )),
            &mock_bid_order(mock_bid_marker_share_sale("marker2", "denom", 10, &[])),
            marker_share_sale_error(
                "Ask marker address [marker1] does not match bid marker address [marker2]",
            ),
            true,
        );
    }

    #[test]
    fn test_marker_share_sale_single_tx_mismatched_share_purchase_amount() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Marker ask requires 10 shares to be purchased, but bidder wants 5",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale(
                "marker",
                "denom",
                10,
                10,
                &[],
                ShareSaleType::SingleTransaction,
            )),
            &mock_bid_order(mock_bid_marker_share_sale("marker", "denom", 5, &[])),
            marker_share_sale_error(
                "Ask requested that [10] shares be purchased, but bid wanted too few [5]",
            ),
            true,
        );
    }

    #[test]
    fn test_marker_share_sale_marker_missing() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Marker for ask and bid does not appear to exist",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale(
                "marker",
                "denom",
                10,
                10,
                &[],
                ShareSaleType::SingleTransaction,
            )),
            &mock_bid_order(mock_bid_marker_share_sale("marker", "denom", 10, &[])),
            marker_share_sale_error("Failed to find marker for denom [denom]"),
            true,
        );
    }

    #[test]
    fn test_marker_share_sale_marker_amount_mismatch_with_ask_record() {
        let mut deps = mock_dependencies(&[]);
        let marker = MockMarker {
            denom: "fakecoin".to_string(),
            coins: coins(10, "fakecoin"),
            address: Addr::unchecked("marker"),
            ..MockMarker::default()
        }
        .to_marker();
        deps.querier.with_markers(vec![marker]);
        assert_validation_failure_with_mismatch_test(
            "Marker on chain does not match share count in ask - this would be a security bug if we ever see it",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale("marker", "fakecoin", 15, 15, &[], ShareSaleType::SingleTransaction)),
            &mock_bid_order(mock_bid_marker_share_sale("marker", "fakecoin", 15, &[])),
            marker_share_sale_error("Marker is not synced with the contract! Marker had [10] shares remaining, which is less than the listed available share count of [15]"),
            true,
        );
    }

    #[test]
    fn test_marker_share_sale_marker_missing_its_own_coin_holdings() {
        let mut deps = mock_dependencies(&[]);
        let marker = MockMarker {
            denom: "fakecoin".to_string(),
            coins: coins(10, "lessfakecoin"),
            address: Addr::unchecked("marker"),
            ..MockMarker::default()
        }
        .to_marker();
        deps.querier.with_markers(vec![marker]);
        assert_validation_failure_with_mismatch_test(
            "Marker on chain does not hold any of its own denom anymore somehow - this would be a security bug if we ever see it",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale("marker", "fakecoin", 10, 10, &[], ShareSaleType::MultipleTransactions)),
            &mock_bid_order(mock_bid_marker_share_sale("marker", "fakecoin", 10, &[])),
            marker_share_sale_error("Marker had invalid coin holdings for match: [10lessfakecoin]. Expected a single instance of coin [fakecoin]"),
            true,
        );
    }

    #[test]
    fn test_marker_share_sale_different_quotes_with_override_quote_source() {
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        assert_validation_failure_options(
            "Quote per share does not match for ask and bid in length",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                100,
                100,
                &[coin(10, "quote1"), coin(20, "quote2")],
                ShareSaleType::MultipleTransactions,
            )),
            &mock_bid_order(mock_bid_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                150,
                &coins(1500, "quote1"),
            )),
            marker_share_sale_error("Ask quote per share [10quote1, 20quote2] had a different amount of specified coin types than bid quote per share [10quote1]"),
            AdminMatchOptions::marker_share_sale_options(OverrideQuoteSource::Ask),
        );
    }

    #[test]
    fn test_marker_share_sale_too_high_ask_amount_with_override_quote_source() {
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        assert_validation_failure_options(
            "Ask quote requests more coin than bid quote can offer",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                100,
                100,
                &[coin(10, "quote1"), coin(10, "quote2")],
                ShareSaleType::MultipleTransactions,
            )),
            &mock_bid_order(mock_bid_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                100,
                &[coin(1000, "quote1"), coin(900, "quote2")],
            )),
            marker_share_sale_error("Ask quote per share [10quote1, 10quote2] required at least [10quote2] but bid quote per share [10quote1, 9quote2] only specified [9quote2]"),
            AdminMatchOptions::marker_share_sale_options(OverrideQuoteSource::Ask),
        );
    }

    #[test]
    fn test_marker_share_sale_different_ask_and_bid_coin_types_with_override_quote_source() {
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        assert_validation_failure_options(
            "Ask quote includes different coin than bid quote",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                100,
                100,
                &[coin(10, "quote1"), coin(10, "quote2"), coin(10, "quote3")],
                ShareSaleType::MultipleTransactions,
            )),
            &mock_bid_order(mock_bid_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                100,
                &[coin(1000, "quote1"), coin(1000, "quote2"), coin(1000, "quote4")],
            )),
            marker_share_sale_error("Ask quote per share [10quote1, 10quote2, 10quote3] contained coin denom [quote3] but bid quote per share [10quote1, 10quote2, 10quote4] did not"),
            AdminMatchOptions::marker_share_sale_options(OverrideQuoteSource::Bid),
        );
    }

    #[test]
    fn test_marker_share_sale_mismatched_quote_per_share_no_override_quote_source() {
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_markers(vec![MockMarker::new_marker()]);
        assert_validation_failure(
            "Ask quote does not match bid quote",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                100,
                100,
                &[coin(10, "quote1"), coin(10, "quote2")],
                ShareSaleType::SingleTransaction,
            )),
            &mock_bid_order(mock_bid_marker_share_sale(
                DEFAULT_MARKER_ADDRESS,
                DEFAULT_MARKER_DENOM,
                100,
                &[coin(1000, "quote1"), coin(1000, "quote3")],
            )),
            marker_share_sale_error("Ask quote per share [10quote1, 10quote2] did not equal bid quote per share [10quote1, 10quote3]"),
        );
    }

    #[test]
    fn test_scope_trade_scope_address_mismatch() {
        let deps = mock_dependencies(&[]);
        assert_validation_failure_with_mismatch_test(
            "Ask scope address does not match bid scope address",
            &deps.as_ref(),
            &mock_ask_order(mock_ask_scope_trade("scope1", &[])),
            &mock_bid_order(mock_bid_scope_trade("scope2", &[])),
            scope_trade_error(
                "Ask scope address [scope1] does not match bid scope address [scope2]",
            ),
            true,
        );
    }

    #[test]
    fn test_scope_trade_quote_mismatch() {
        let deps = mock_dependencies(&[]);
        let mut ask_order = mock_ask_order(mock_ask_scope_trade("scope", &coins(100, NHASH)));
        let mut bid_order = mock_bid_order(mock_bid_scope_trade("scope", &coins(99, NHASH)));
        assert_validation_failure_with_mismatch_test(
            "Ask wants 100nhash but bid offers 99nhash",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            scope_trade_error("Ask quote [100nhash] does not match bid quote [99nhash]"),
            false,
        );
        replace_ask_quote(&mut ask_order, &[coin(100, NHASH), coin(20, "bitcoin")]);
        replace_bid_quote(&mut bid_order, &[coin(100, NHASH)]);
        assert_validation_failure_with_mismatch_test(
            "Ask wants 100nhash and 20bitcoin but bid \"forgot\" to add the 20bitcoin",
            &deps.as_ref(),
            &ask_order,
            &bid_order,
            scope_trade_error(
                "Ask quote [20bitcoin, 100nhash] does not match bid quote [100nhash]",
            ),
            false,
        );
    }

    fn assert_validation_failure<S1: Into<String>, S2: Into<String>>(
        test_name: S1,
        deps: &Deps<ProvenanceQuery>,
        ask_order: &AskOrder,
        bid_order: &BidOrder,
        expected_error_message: S2,
    ) {
        assert_validation_failure_full(
            test_name,
            deps,
            ask_order,
            bid_order,
            expected_error_message,
            None,
        );
    }

    fn assert_validation_failure_options<S1: Into<String>, S2: Into<String>>(
        test_name: S1,
        deps: &Deps<ProvenanceQuery>,
        ask_order: &AskOrder,
        bid_order: &BidOrder,
        expected_error_message: S2,
        options: AdminMatchOptions,
    ) {
        assert_validation_failure_full(
            test_name,
            deps,
            ask_order,
            bid_order,
            expected_error_message,
            Some(options),
        );
    }

    fn assert_validation_failure_full<S1: Into<String>, S2: Into<String>>(
        test_name: S1,
        deps: &Deps<ProvenanceQuery>,
        ask_order: &AskOrder,
        bid_order: &BidOrder,
        expected_error_message: S2,
        options: Option<AdminMatchOptions>,
    ) {
        let test_name = test_name.into();
        let message = expected_error_message.into();
        let result = validate_match(deps, ask_order, bid_order, &options);
        let messages = match result {
            Err(e) => match e {
                ContractError::ValidationError { messages } => messages,
                e => panic!(
                    "{}: Expected message [{}], but got unexpected error instead during validation: {:?}",
                    test_name, message, e
                ),
            },
            Ok(_) => panic!(
                "{}: Expected message [{}] to be be output for input values, but validation passed",
                test_name, message,
            ),
        };
        assert!(
            messages.contains(&message),
            "expected message [{}] to be in result list {:?} for ask [{}] and bid [{}]",
            &message,
            &messages,
            &ask_order.id,
            &bid_order.id,
        );
    }

    fn assert_validation_failure_with_mismatch_test<S1: Into<String>, S2: Into<String>>(
        test_name: S1,
        deps: &Deps<ProvenanceQuery>,
        ask_order: &AskOrder,
        bid_order: &BidOrder,
        expected_error_message: S2,
        validation_should_fail_with_mismatched_bids: bool,
    ) {
        let test_name = test_name.into();
        let message = expected_error_message.into();
        let get_admin_options = |accept_mismatched_bids: bool| {
            match &ask_order.ask_type {
                RequestType::CoinTrade => Some(AdminMatchOptions::coin_trade_options(
                    accept_mismatched_bids,
                )),
                RequestType::MarkerTrade => Some(AdminMatchOptions::marker_trade_options(
                    accept_mismatched_bids,
                )),
                RequestType::MarkerShareSale => None, // Mismatched bids is not an option for marker share sale
                RequestType::ScopeTrade => Some(AdminMatchOptions::scope_trade_options(
                    accept_mismatched_bids,
                )),
            }
        };
        let test = |accept_mismatched_bids: bool| {
            let result = validate_match(
                deps,
                ask_order,
                bid_order,
                &get_admin_options(accept_mismatched_bids),
            );
            if !accept_mismatched_bids || validation_should_fail_with_mismatched_bids {
                assert_validation_failure_full(
                    &test_name,
                    deps,
                    ask_order,
                    bid_order,
                    &message,
                    get_admin_options(accept_mismatched_bids),
                );
            } else {
                result.unwrap_or_else(|e| panic!("{}: validation should pass with mismatched bids flag enabled, but got error: {:?}", test_name, e));
            }
        };
        test(false);
        test(true);
    }

    fn expected_error<S: Into<String>>(suffix: S) -> String {
        format!(
            "Match Validation for AskOrder [ask_id] and BidOrder [bid_id]: {}",
            suffix.into()
        )
    }

    fn coin_trade_error<S: Into<String>>(suffix: S) -> String {
        format!(
            "COIN TRADE Match Validation for AskOrder [ask_id] and BidOrder [bid_id]: {}",
            suffix.into()
        )
    }

    fn marker_trade_error<S: Into<String>>(suffix: S) -> String {
        format!(
            "MARKER TRADE Match Validation for AskOrder [ask_id] and BidOrder [bid_id]: {}",
            suffix.into()
        )
    }

    fn marker_share_sale_error<S: Into<String>>(suffix: S) -> String {
        format!(
            "MARKER SHARE SALE Match Validation for AskOrder [ask_id] and BidOrder [bid_id]: {}",
            suffix.into(),
        )
    }

    fn scope_trade_error<S: Into<String>>(suffix: S) -> String {
        format!(
            "SCOPE TRADE Match Validation for AskOrder [ask_id] and BidOrder [bid_id]: {}",
            suffix.into(),
        )
    }
}
