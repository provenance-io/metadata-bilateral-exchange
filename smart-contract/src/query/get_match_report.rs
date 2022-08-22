use crate::storage::ask_order_storage::get_ask_order_by_id;
use crate::storage::bid_order_storage::get_bid_order_by_id;
use crate::types::core::error::ContractError;
use crate::types::request::admin_match_options::AdminMatchOptions;
use crate::types::request::match_report::MatchReport;
use crate::util::extensions::ResultExtensions;
use crate::validation::execute_match_validation::validate_match;
use cosmwasm_std::{to_binary, Binary, Deps};
use provwasm_std::ProvenanceQuery;

pub fn get_match_report(
    deps: Deps<ProvenanceQuery>,
    ask_id: String,
    bid_id: String,
    admin_match_options: Option<AdminMatchOptions>,
) -> Result<Binary, ContractError> {
    let ask_order_result = get_ask_order_by_id(deps.storage, &ask_id);
    let bid_order_result = get_bid_order_by_id(deps.storage, &bid_id);
    if ask_order_result.is_err() || bid_order_result.is_err() {
        let error_message = if ask_order_result.is_ok() {
            format!("BidOrder [{}] was missing from contract storage", &bid_id)
        } else if bid_order_result.is_ok() {
            format!("AskOrder [{}] was missing from contract storage", &ask_id)
        } else {
            format!(
                "AskOrder [{}] and BidOrder [{}] were missing from contract storage",
                &ask_id, &bid_id
            )
        };
        return to_binary(&MatchReport {
            ask_id,
            bid_id,
            ask_exists: ask_order_result.is_ok(),
            bid_exists: bid_order_result.is_ok(),
            match_possible: false,
            error_message: Some(error_message),
        })?
        .to_ok();
    }
    let ask_order = ask_order_result.unwrap();
    let bid_order = bid_order_result.unwrap();
    let match_result = validate_match(&deps, &ask_order, &bid_order, &admin_match_options);
    to_binary(&MatchReport {
        ask_id,
        bid_id,
        ask_exists: true,
        bid_exists: true,
        match_possible: match_result.is_ok(),
        error_message: if let Err(e) = &match_result {
            Some(format!("Match fails due to: {:?}", e))
        } else {
            None
        },
    })?
    .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::query::get_match_report::get_match_report;
    use crate::storage::ask_order_storage::insert_ask_order;
    use crate::storage::bid_order_storage::insert_bid_order;
    use crate::test::cosmos_type_helpers::MockOwnedDeps;
    use crate::test::mock_scope::DEFAULT_SCOPE_ADDR;
    use crate::test::request_helpers::{mock_ask_order, mock_bid_order, mock_bid_scope_trade};
    use crate::types::request::admin_match_options::AdminMatchOptions;
    use crate::types::request::ask_types::ask_collateral::AskCollateral;
    use crate::types::request::bid_types::bid_collateral::BidCollateral;
    use crate::types::request::match_report::MatchReport;
    use cosmwasm_std::{coins, from_binary};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_missing_orders_report() {
        let mut deps = mock_dependencies(&[]);
        let missing_both_report = deserialize_report(&deps, "ask_id", "bid_id", None);
        assert!(
            !missing_both_report.ask_exists,
            "the ask should not be marked as existing",
        );
        assert!(
            !missing_both_report.bid_exists,
            "the bid should not be marked as existing",
        );
        assert_report_contains_error_message(
            &missing_both_report,
            "AskOrder [ask_id] and BidOrder [bid_id] were missing from contract storage",
        );
        let ask_order = mock_ask_order(AskCollateral::coin_trade(&[], &[]));
        insert_ask_order(deps.as_mut().storage, &ask_order)
            .expect("expected the ask order to be inserted");
        let bid_order = mock_bid_order(BidCollateral::coin_trade(&[], &[]));
        insert_bid_order(deps.as_mut().storage, &bid_order)
            .expect("expected the bid order to be inserted");
        let missing_ask_report = deserialize_report(&deps, "not_ask", "bid_id", None);
        assert!(
            !missing_ask_report.ask_exists,
            "the ask should not be marked as existing"
        );
        assert!(
            missing_ask_report.bid_exists,
            "the bid should be marked as existing"
        );
        assert_report_contains_error_message(
            &missing_ask_report,
            "AskOrder [not_ask] was missing from contract storage",
        );
        let missing_bid_report = deserialize_report(&deps, "ask_id", "not_bid", None);
        assert!(
            missing_bid_report.ask_exists,
            "the ask should be marked as existing",
        );
        assert!(
            !missing_bid_report.bid_exists,
            "the bid should not be marked as existing",
        );
        assert_report_contains_error_message(
            &missing_bid_report,
            "BidOrder [not_bid] was missing from contract storage",
        );
    }

    #[test]
    fn test_match_failure_without_match_options() {
        let mut deps = mock_dependencies(&[]);
        let base = coins(100, "base");
        let ask_order = mock_ask_order(AskCollateral::coin_trade(&base, &coins(100, "quote")));
        insert_ask_order(deps.as_mut().storage, &ask_order).expect("ask should be inserted");
        let bid_order = mock_bid_order(BidCollateral::coin_trade(&base, &coins(99, "quote")));
        insert_bid_order(deps.as_mut().storage, &bid_order).expect("bid should be inserted");
        let report = deserialize_report(&deps, "ask_id", "bid_id", None);
        assert!(report.ask_exists, "the ask should be marked as existing",);
        assert!(report.bid_exists, "the bid should be marked as existing",);
        assert!(
            !report.match_possible,
            "the match possible report param should indicate false",
        );
        let error_message = report.error_message.expect("an error should be returned");
        assert!(
            error_message.contains("Match fails due to:"),
            "the failing message should contain a match failure prefix",
        );
    }

    #[test]
    fn test_failure_with_match_options() {
        let mut deps = mock_dependencies(&[]);
        let ask_order = mock_ask_order(AskCollateral::coin_trade(
            &coins(100, "base"),
            &coins(100, "quote"),
        ));
        insert_ask_order(deps.as_mut().storage, &ask_order).expect("ask should be inserted");
        let bid_order = mock_bid_order(mock_bid_scope_trade(
            DEFAULT_SCOPE_ADDR,
            &coins(100, "quote"),
        ));
        insert_bid_order(deps.as_mut().storage, &bid_order).expect("bid should be inserted");
        let report = deserialize_report(
            &deps,
            "ask_id",
            "bid_id",
            Some(AdminMatchOptions::coin_trade_options(true)),
        );
        assert!(report.ask_exists, "the ask should be marked as existing");
        assert!(report.bid_exists, "the bid should be marked as existing");
        assert!(
            !report.match_possible,
            "the match possible report param should indicate false",
        );
        let error_message = report
            .error_message
            .expect("an error message should be returned");
        assert!(
            error_message.contains("Match fails due to:"),
            "the failing message should contain a match failure prefix",
        );
    }

    #[test]
    fn test_successful_match_report() {
        let mut deps = mock_dependencies(&[]);
        let base = coins(100, "base");
        let quote = coins(100, "quote");
        let ask_order = mock_ask_order(AskCollateral::coin_trade(&base, &quote));
        insert_ask_order(deps.as_mut().storage, &ask_order).expect("ask should be inserted");
        let bid_order = mock_bid_order(BidCollateral::coin_trade(&base, &quote));
        insert_bid_order(deps.as_mut().storage, &bid_order).expect("bid should be inserted");
        let report = deserialize_report(
            &deps,
            "ask_id",
            "bid_id",
            Some(AdminMatchOptions::coin_trade_options(true)),
        );
        assert!(report.ask_exists, "the ask should be marked as existing");
        assert!(report.bid_exists, "the bid should be marked as existing");
        assert!(
            report.match_possible,
            "the report should indicate that a match is possible",
        );
        assert!(
            report.error_message.is_none(),
            "the report should not include any error message",
        );
    }

    fn deserialize_report<A: Into<String>, B: Into<String>>(
        deps: &MockOwnedDeps,
        ask_id: A,
        bid_id: B,
        match_options: Option<AdminMatchOptions>,
    ) -> MatchReport {
        let ask_id = ask_id.into();
        let bid_id = bid_id.into();
        let binary = get_match_report(deps.as_ref(), ask_id.clone(), bid_id.clone(), match_options)
            .expect("expected a report to be produced");
        let report = from_binary::<MatchReport>(&binary)
            .expect("expected the binary to deserialize to a match report");
        assert_eq!(
            ask_id, report.ask_id,
            "sanity check: ask id should match the input value",
        );
        assert_eq!(
            bid_id, report.bid_id,
            "sanity check: bid id should match the input value",
        );
        report
    }

    fn assert_report_contains_error_message<S: Into<String>>(
        report: &MatchReport,
        expected_error_message: S,
    ) {
        let error_message = report
            .error_message
            .to_owned()
            .expect("an error message should be included in the report");
        assert_eq!(
            expected_error_message.into(),
            error_message,
            "expected the error message in the report to be the correct text",
        );
    }
}
