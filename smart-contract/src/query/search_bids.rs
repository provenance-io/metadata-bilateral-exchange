use crate::storage::bid_order_storage::bid_orders;
use crate::storage::order_search_repository::OrderSearchRepository;
use crate::types::core::error::ContractError;
use crate::types::request::search::Search;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, Binary, Deps};
use provwasm_std::ProvenanceQuery;

pub fn search_bids(deps: Deps<ProvenanceQuery>, search: Search) -> Result<Binary, ContractError> {
    let repository = OrderSearchRepository::new(bid_orders());
    to_binary(&repository.search(deps.storage, search))?.to_ok()
}

#[cfg(test)]
mod tests {
    use crate::query::search_bids::search_bids;
    use crate::storage::bid_order_storage::insert_bid_order;
    use crate::types::core::constants::{
        DEFAULT_SEARCH_PAGE_NUMBER, DEFAULT_SEARCH_PAGE_SIZE, MAX_SEARCH_PAGE_SIZE,
    };
    use crate::types::request::bid_types::bid_collateral::BidCollateral;
    use crate::types::request::bid_types::bid_order::BidOrder;
    use crate::types::request::request_descriptor::RequestDescriptor;
    use crate::types::request::request_type::RequestType;
    use crate::types::request::search::{Search, SearchResult};
    use cosmwasm_std::{from_binary, Addr, Deps};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::ProvenanceQuery;

    #[test]
    fn test_search_all_no_values() {
        let deps = mock_dependencies(&[]);
        let page = search(deps.as_ref(), Search::all(None, None));
        assert!(
            page.results.is_empty(),
            "no results should be returned when no values exist",
        );
        assert_eq!(
            DEFAULT_SEARCH_PAGE_SIZE,
            page.page_size.u128() as usize,
            "the default page size should be used when no value is provided",
        );
        assert_eq!(
            DEFAULT_SEARCH_PAGE_NUMBER,
            page.page_number.u128() as usize,
            "the default page number should be used when no value is provided",
        );
        assert_eq!(
            1,
            page.total_pages.u128(),
            "the total number of pages should be 0, indicating that there is one page of... nothing",
        );
    }

    #[test]
    fn test_search_all_with_values() {
        let mut deps = mock_dependencies(&[]);
        for index in 0..21 {
            insert_bid_order(
                deps.as_mut().storage,
                &BidOrder::new_unchecked(
                    format!("bid_id_{}", index),
                    Addr::unchecked(format!("bidder{}", index)),
                    // Swap between coin and marker for some variety
                    if index % 2 == 0 {
                        BidCollateral::coin_trade(&[], &[])
                    } else {
                        BidCollateral::marker_trade(
                            Addr::unchecked(format!("marker{}", index)),
                            format!("denom{}", index),
                            &[],
                        )
                    },
                    Some(RequestDescriptor::basic(format!("Some bid {}", index))),
                ),
            )
            .expect(&format!(
                "expected bid order {} to be inserted correctly",
                index
            ));
        }
        // Search for everything with a page size of 10 and no offset key
        let first_search = Search::all(Some(10), None);
        let first_page = search(deps.as_ref(), first_search);
        assert_eq!(
            10,
            first_page.results.len(),
            "there are 21 items in storage, so all results should be returned in the 0-10 range",
        );
        assert_eq!(
            1,
            first_page.page_number.u128(),
            "with no page number input, the fist page should be returned"
        );
        assert_eq!(
            10,
            first_page.page_size.u128(),
            "the provided page size should always be returned",
        );
        assert_eq!(
            3,
            first_page.total_pages.u128(),
            "the total page value should be equal to the total number of items divided by results",
        );
        let second_search = Search::all(Some(10), Some(2));
        let second_page = search(deps.as_ref(), second_search);
        assert_eq!(
            10,
            second_page.results.len(),
            "there are 21 items in storage, so all results should be returned in the 10-20 range",
        );
        assert_eq!(
            2,
            second_page.page_number.u128(),
            "the requested page number should be returned in the result",
        );
        assert_eq!(
            3,
            second_page.total_pages.u128(),
            "the total page number should not change unless new values are added",
        );
        let second_page_ids = second_page
            .results
            .iter()
            .map(|bid| &bid.id)
            .collect::<Vec<&String>>();
        assert!(
            !first_page
                .results
                .iter()
                .any(|bid| second_page_ids.contains(&&bid.id)),
            "no ids from the first search should be contained in the second search's results"
        );
        let third_page = search(deps.as_ref(), Search::all(Some(10), Some(3)));
        assert_eq!(
            1,
            third_page.results.len(),
            "the final, 21st result should be returned on the third page",
        );
        assert_eq!(
            3,
            third_page.page_number.u128(),
            "the requested page number should be returned in the result",
        );
        assert_eq!(
            3,
            third_page.total_pages.u128(),
            "the total page number should be unchanging",
        );
        let final_id = third_page.results.first().unwrap().id.to_owned();
        assert!(
            !first_page.results.iter().any(|bid| bid.id == final_id),
            "the third page's resulting id should not be included in the first page's results",
        );
        assert!(
            !second_page_ids.iter().any(|bid_id| bid_id == &&final_id),
            "the third page's resulting id should not be included in the second page's results",
        );
        let fourth_page = search(deps.as_ref(), Search::all(Some(10), Some(4)));
        assert!(
            fourth_page.results.is_empty(),
            "no results should be contained because there are not enough items for a fourth page",
        );
        assert_eq!(
            4,
            fourth_page.page_number.u128(),
            "the requested page number should be returned in the result",
        );
        assert_eq!(
            3, fourth_page.total_pages.u128(),
            "the result should indicate that there are only three pages, revealing how ridiculous the search was in the first place",
        );
        let max_page = search(deps.as_ref(), Search::all(Some(25), None));
        assert_eq!(
            21,
            max_page.results.len(),
            "all items should be returned in a search that requests more items than are available",
        );
        assert_eq!(
            25,
            max_page.page_size.u128(),
            "the provided page size should be returned",
        );
        assert_eq!(
            1,
            max_page.page_number.u128(),
            "the default first page number should be used",
        );
        assert_eq!(
            1,
            max_page.total_pages.u128(),
            "due to there being less results than will fit on a single page, there should be one page",
        );
    }

    #[test]
    fn test_search_value_type_no_values() {
        let mut deps = mock_dependencies(&[]);
        // Insert 10 coin bids, ensuring that a marker search should yield nothing
        for index in 0..10 {
            insert_bid_order(
                deps.as_mut().storage,
                &BidOrder::new_unchecked(
                    format!("bid_id_{}", index),
                    Addr::unchecked(format!("bidder{}", index)),
                    BidCollateral::coin_trade(&[], &[]),
                    Some(RequestDescriptor::basic(format!("Some bid {}", index))),
                ),
            )
            .expect(&format!(
                "expected bid order {} to be inserted correctly",
                index
            ));
        }
        let marker_page = search(
            deps.as_ref(),
            Search::value_type(RequestType::MarkerTrade.get_name(), None, None),
        );
        assert!(
            marker_page.results.is_empty(),
            "no results should be yielded for a marker search when all values are of type coin",
        );
        assert_eq!(
            DEFAULT_SEARCH_PAGE_SIZE,
            marker_page.page_size.u128() as usize,
            "the provided page size should be returned",
        );
        assert_eq!(
            DEFAULT_SEARCH_PAGE_NUMBER,
            marker_page.page_number.u128() as usize,
            "the default first page number should be used",
        );
        assert_eq!(
            1,
            marker_page.total_pages.u128(),
            "due to there being no results, a single page should be returned",
        );
    }

    #[test]
    fn test_search_value_type_with_values() {
        let mut deps = mock_dependencies(&[]);
        // Insert 25 bid orders, which should equate to 13 coin and 12 marker
        for index in 0..25 {
            insert_bid_order(
                deps.as_mut().storage,
                &BidOrder::new_unchecked(
                    format!("bid_id_{}", index),
                    Addr::unchecked(format!("bidder{}", index)),
                    // Swap between coin and marker for some variety
                    if index % 2 == 0 {
                        BidCollateral::coin_trade(&[], &[])
                    } else {
                        BidCollateral::marker_trade(
                            Addr::unchecked(format!("marker{}", index)),
                            format!("denom{}", index),
                            &[],
                        )
                    },
                    Some(RequestDescriptor::basic(format!("Some bid {}", index))),
                ),
            )
            .expect(&format!(
                "expected bid order {} to be inserted correctly",
                index
            ));
        }
        let coin_page = search(
            deps.as_ref(),
            Search::value_type(RequestType::CoinTrade.get_name(), Some(15), None),
        );
        assert_eq!(
            13,
            coin_page.results.len(),
            "13 results of type coin should be returned",
        );
        assert!(
            coin_page
                .results
                .iter()
                .all(|bid| bid.bid_type == RequestType::CoinTrade),
            "all returned results should be coin results",
        );
        let marker_page = search(
            deps.as_ref(),
            Search::value_type(RequestType::MarkerTrade.get_name(), Some(15), None),
        );
        assert_eq!(
            12,
            marker_page.results.len(),
            "12 results of type marker should be returned",
        );
        assert!(
            marker_page
                .results
                .iter()
                .all(|bid| bid.bid_type == RequestType::MarkerTrade),
            "all returned results should be marker results",
        );
    }

    #[test]
    fn test_search_id_no_values() {
        let mut deps = mock_dependencies(&[]);
        insert_bid_order(
            deps.as_mut().storage,
            &BidOrder::new_unchecked(
                "bid_id_0",
                Addr::unchecked("bidder"),
                BidCollateral::coin_trade(&[], &[]),
                Some(RequestDescriptor::basic("Some bid")),
            ),
        )
        .expect("expected the bid order to be inserted correctly");
        let id_page = search(deps.as_ref(), Search::id("bid_id_1", None, None));
        assert!(
            id_page.results.is_empty(),
            "expected no results to be returned because the id requested does not exist",
        );
        assert_eq!(
            1,
            id_page.page_number.u128(),
            "the first page should always be returned when no page is requested",
        );
        assert_eq!(
            DEFAULT_SEARCH_PAGE_SIZE,
            id_page.page_size.u128() as usize,
            "the default page size should be used when no value is provided",
        );
        assert_eq!(
            1,
            id_page.total_pages.u128(),
            "one total page should be returned when there are no results",
        );
    }

    #[test]
    fn test_search_id_with_values() {
        let mut deps = mock_dependencies(&[]);
        // Insert some bids, ensuring there are results to return
        for index in 0..10 {
            insert_bid_order(
                deps.as_mut().storage,
                &BidOrder::new_unchecked(
                    format!("bid_id_{}", index),
                    Addr::unchecked(format!("bidder{}", index)),
                    // Swap between coin and marker for some variety
                    if index % 2 == 0 {
                        BidCollateral::coin_trade(&[], &[])
                    } else {
                        BidCollateral::marker_trade(
                            Addr::unchecked(format!("marker{}", index)),
                            format!("denom{}", index),
                            &[],
                        )
                    },
                    Some(RequestDescriptor::basic(format!("Some bid {}", index))),
                ),
            )
            .expect(&format!(
                "expected bid order {} to be inserted correctly",
                index
            ));
        }
        let bid_0_page = search(deps.as_ref(), Search::id("bid_id_0", None, None));
        assert_eq!(
            1,
            bid_0_page.results.len(),
            "a single result should be returned when the id PK matches",
        );
        assert_eq!(
            "bid_id_0",
            bid_0_page.results.first().unwrap().id,
            "the result should have the correct id",
        );
        let bid_1_page = search(
            deps.as_ref(),
            Search::id("bid_id_1", Some(MAX_SEARCH_PAGE_SIZE as u128), Some(150)),
        );
        assert_eq!(
            1,
            bid_1_page.results.len(),
            "a single result should be returned when the id PK matches",
        );
        assert_eq!(
            "bid_id_1",
            bid_1_page.results.first().unwrap().id,
            "the result should have the correct id",
        );
        assert_eq!(
            MAX_SEARCH_PAGE_SIZE,
            bid_1_page.page_size.u128() as usize,
            "page size should be returned as defined",
        );
        assert_eq!(
            150,
            bid_1_page.page_number.u128(),
            "page number should be returned as defined",
        );
        assert_eq!(
            1,
            bid_1_page.total_pages.u128(),
            "total pages should show 1, indicating that there will always be 1 page for id searches",
        );
    }

    fn search(deps: Deps<ProvenanceQuery>, search: Search) -> SearchResult<BidOrder> {
        let bin = search_bids(deps, search).expect("expected the result to succeed");
        from_binary(&bin)
            .expect("expected binary deserialization to the appropriate type to succeed")
    }
}
