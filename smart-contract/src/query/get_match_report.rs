use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{Binary, Deps};
use provwasm_std::ProvenanceQuery;

pub fn get_match_report(
    _deps: Deps<ProvenanceQuery>,
    _ask_id: String,
    _bid_id: String,
) -> Result<Binary, ContractError> {
    // TODO: Re-enable this code once the legacy migration has been completed
    // The code required to convert from legacy ask and bid orders to the new schema causes the output
    // binary to be too large.  Temporarily disabling match reports removes enough code to make the
    // contract deployable.  This is an unfortunate compromise to allow this migration to take place.
    // If the migration occurs quickly enough, though, it shouldn't be an issue...  This functionality
    // is not currently used by any downstream consumers as of the time of writing, so its removal
    // is not damaging to any functionality that relies upon it
    ContractError::generic_error("match reports are currently disabled").to_err()
    // let ask_order_result = get_ask_order_by_id(deps.storage, &ask_id);
    // let bid_order_result = get_bid_order_by_id(deps.storage, &bid_id);
    // if ask_order_result.is_err() || bid_order_result.is_err() {
    //     return MatchReport::new_missing_order(
    //         ask_id,
    //         bid_id,
    //         ask_order_result.is_ok(),
    //         bid_order_result.is_ok(),
    //     )?
    //     .to_binary();
    // }
    // let ask_order = ask_order_result.unwrap();
    // let bid_order = bid_order_result.unwrap();
    // let standard_match_result = validate_match(&deps, &ask_order, &bid_order, false);
    // let quote_mismatch_result = validate_match(&deps, &ask_order, &bid_order, true);
    // let mut error_messages = vec![];
    // if let Err(e) = &standard_match_result {
    //     error_messages.push(format!("Standard match fails due to: {:?}", e))
    // }
    // if let Err(e) = &quote_mismatch_result {
    //     error_messages.push(format!(
    //         "Quote mismatch-enabled match fails due to: {:?}",
    //         e
    //     ))
    // }
    // MatchReport::new_existing_orders(
    //     ask_id,
    //     bid_id,
    //     standard_match_result.is_ok(),
    //     quote_mismatch_result.is_ok(),
    //     &error_messages,
    // )
    // .to_binary()
}

#[cfg(test)]
mod tests {
    // use crate::query::get_match_report::get_match_report;
    // use crate::storage::ask_order_storage::insert_ask_order;
    // use crate::storage::bid_order_storage::insert_bid_order;
    // use crate::test::cosmos_type_helpers::MockOwnedDeps;
    // use crate::test::mock_scope::DEFAULT_SCOPE_ID;
    // use crate::test::request_helpers::{mock_ask_order, mock_bid_order, mock_bid_scope_trade};
    // use crate::types::request::ask_types::ask_collateral::AskCollateral;
    // use crate::types::request::bid_types::bid_collateral::BidCollateral;
    // use crate::types::request::match_report::MatchReport;
    // use cosmwasm_std::{coins, from_binary};
    // use provwasm_mocks::mock_dependencies;
    //
    // #[test]
    // fn test_missing_orders_report() {
    //     let mut deps = mock_dependencies(&[]);
    //     let missing_both_report = deserialize_report(&deps, "ask_id", "bid_id");
    //     assert!(
    //         !missing_both_report.ask_exists,
    //         "the ask should not be marked as existing",
    //     );
    //     assert!(
    //         !missing_both_report.bid_exists,
    //         "the bid should not be marked as existing",
    //     );
    //     assert_report_includes_single_error(
    //         &missing_both_report,
    //         "AskOrder [ask_id] and BidOrder [bid_id] were missing from contract storage",
    //     );
    //     let ask_order = mock_ask_order(AskCollateral::coin_trade(&[], &[]));
    //     insert_ask_order(deps.as_mut().storage, &ask_order)
    //         .expect("expected the ask order to be inserted");
    //     let bid_order = mock_bid_order(BidCollateral::coin_trade(&[], &[]));
    //     insert_bid_order(deps.as_mut().storage, &bid_order)
    //         .expect("expected the bid order to be inserted");
    //     let missing_ask_report = deserialize_report(&deps, "not_ask", "bid_id");
    //     assert!(
    //         !missing_ask_report.ask_exists,
    //         "the ask should not be marked as existing"
    //     );
    //     assert!(
    //         missing_ask_report.bid_exists,
    //         "the bid should be marked as existing"
    //     );
    //     assert_report_includes_single_error(
    //         &missing_ask_report,
    //         "AskOrder [not_ask] was missing from contract storage",
    //     );
    //     let missing_bid_report = deserialize_report(&deps, "ask_id", "not_bid");
    //     assert!(
    //         missing_bid_report.ask_exists,
    //         "the ask should be marked as existing",
    //     );
    //     assert!(
    //         !missing_bid_report.bid_exists,
    //         "the bid should not be marked as existing",
    //     );
    //     assert_report_includes_single_error(
    //         &missing_bid_report,
    //         "BidOrder [not_bid] was missing from contract storage",
    //     );
    // }
    //
    // #[test]
    // fn test_standard_match_failure_only() {
    //     let mut deps = mock_dependencies(&[]);
    //     let base = coins(100, "base");
    //     let ask_order = mock_ask_order(AskCollateral::coin_trade(&base, &coins(100, "quote")));
    //     insert_ask_order(deps.as_mut().storage, &ask_order).expect("ask should be inserted");
    //     let bid_order = mock_bid_order(BidCollateral::coin_trade(&base, &coins(99, "quote")));
    //     insert_bid_order(deps.as_mut().storage, &bid_order).expect("bid should be inserted");
    //     let report = deserialize_report(&deps, "ask_id", "bid_id");
    //     assert!(report.ask_exists, "the ask should be marked as existing",);
    //     assert!(report.bid_exists, "the bid should be marked as existing",);
    //     assert!(
    //         !report.standard_match_possible,
    //         "the standard match possible report param should indicate false",
    //     );
    //     assert!(
    //         report.quote_mismatch_match_possible,
    //         "the quote mismatch match possible report param should indicate true",
    //     );
    //     assert_eq!(
    //         1,
    //         report.error_messages.len(),
    //         "the report should have a single error message",
    //     );
    //     assert!(
    //         report
    //             .error_messages
    //             .first()
    //             .unwrap()
    //             .contains("Standard match fails due to"),
    //         "the failing message should indicate the reason for the mismatched standard match",
    //     );
    // }
    //
    // #[test]
    // fn test_both_match_types_failure() {
    //     let mut deps = mock_dependencies(&[]);
    //     let ask_order = mock_ask_order(AskCollateral::coin_trade(
    //         &coins(100, "base"),
    //         &coins(100, "quote"),
    //     ));
    //     insert_ask_order(deps.as_mut().storage, &ask_order).expect("ask should be inserted");
    //     let bid_order =
    //         mock_bid_order(mock_bid_scope_trade(DEFAULT_SCOPE_ID, &coins(100, "quote")));
    //     insert_bid_order(deps.as_mut().storage, &bid_order).expect("bid should be inserted");
    //     let report = deserialize_report(&deps, "ask_id", "bid_id");
    //     assert!(report.ask_exists, "the ask should be marked as existing");
    //     assert!(report.bid_exists, "the bid should be marked as existing");
    //     assert!(
    //         !report.standard_match_possible,
    //         "the standard match possible report param should indicate false",
    //     );
    //     assert!(
    //         !report.quote_mismatch_match_possible,
    //         "the quote mismatch match possible report param should indicate false",
    //     );
    //     assert_eq!(
    //         2,
    //         report.error_messages.len(),
    //         "the report should include two error messages",
    //     );
    //     assert!(
    //         report
    //             .error_messages
    //             .iter()
    //             .any(|message| message.contains("Standard match fails due to")),
    //         "a message should indicate the reason that a standard match cannot be completed",
    //     );
    //     assert!(
    //         report
    //             .error_messages
    //             .iter()
    //             .any(|message| message.contains("Quote mismatch-enabled match fails due to")),
    //         "a message should indicate the reason that a quote mismatch match cannot be completed",
    //     );
    // }
    //
    // #[test]
    // fn test_successful_match_report() {
    //     let mut deps = mock_dependencies(&[]);
    //     let base = coins(100, "base");
    //     let quote = coins(100, "quote");
    //     let ask_order = mock_ask_order(AskCollateral::coin_trade(&base, &quote));
    //     insert_ask_order(deps.as_mut().storage, &ask_order).expect("ask should be inserted");
    //     let bid_order = mock_bid_order(BidCollateral::coin_trade(&base, &quote));
    //     insert_bid_order(deps.as_mut().storage, &bid_order).expect("bid should be inserted");
    //     let report = deserialize_report(&deps, "ask_id", "bid_id");
    //     assert!(report.ask_exists, "the ask should be marked as existing");
    //     assert!(report.bid_exists, "the bid should be marked as existing");
    //     assert!(
    //         report.standard_match_possible,
    //         "the report should indicate that a standard match is possible",
    //     );
    //     assert!(
    //         report.quote_mismatch_match_possible,
    //         "the report should indicate that a quote mismatch match is possible",
    //     );
    //     assert!(
    //         report.error_messages.is_empty(),
    //         "the report should not include any error messages",
    //     );
    // }
    //
    // fn deserialize_report<A: Into<String>, B: Into<String>>(
    //     deps: &MockOwnedDeps,
    //     ask_id: A,
    //     bid_id: B,
    // ) -> MatchReport {
    //     let ask_id = ask_id.into();
    //     let bid_id = bid_id.into();
    //     let binary = get_match_report(deps.as_ref(), ask_id.clone(), bid_id.clone())
    //         .expect("expected a report to be produced");
    //     let report = from_binary::<MatchReport>(&binary)
    //         .expect("expected the binary to deserialize to a match report");
    //     assert_eq!(
    //         ask_id, report.ask_id,
    //         "sanity check: ask id should match the input value",
    //     );
    //     assert_eq!(
    //         bid_id, report.bid_id,
    //         "sanity check: bid id should match the input value",
    //     );
    //     report
    // }
    //
    // fn assert_report_includes_single_error<S: Into<String>>(
    //     report: &MatchReport,
    //     expected_error_message: S,
    // ) {
    //     assert_eq!(
    //         1,
    //         report.error_messages.len(),
    //         "expected only a single error message to be included in the match report",
    //     );
    //     assert_eq!(
    //         &expected_error_message.into(),
    //         report.error_messages.first().unwrap(),
    //         "expected the error message in the report to be the correct text",
    //     );
    // }
}
