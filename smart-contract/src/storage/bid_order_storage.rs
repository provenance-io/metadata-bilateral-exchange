use crate::storage::order_indices::OrderIndices;
use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::Storage;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};

const NAMESPACE_BID_PK: &str = "bid";
const NAMESPACE_OWNER_IDX: &str = "bid__owner";
const NAMESPACE_TYPE_IDX: &str = "bid__type";

pub struct BidOrderIndices<'a> {
    pub owner_index: MultiIndex<'a, String, BidOrder, String>,
    pub type_index: MultiIndex<'a, String, BidOrder, String>,
}
impl<'a> IndexList<BidOrder> for BidOrderIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<BidOrder>> + '_> {
        let v: Vec<&dyn Index<BidOrder>> = vec![&self.owner_index, &self.type_index];
        Box::new(v.into_iter())
    }
}
impl<'a> OrderIndices<'a, BidOrder> for BidOrderIndices<'a> {
    fn owner_index(&self) -> &MultiIndex<'a, String, BidOrder, String> {
        &self.owner_index
    }

    fn type_index(&self) -> &MultiIndex<'a, String, BidOrder, String> {
        &self.type_index
    }
}

pub fn bid_orders<'a>() -> IndexedMap<'a, &'a [u8], BidOrder, BidOrderIndices<'a>> {
    let indices = BidOrderIndices {
        owner_index: MultiIndex::new(
            |bid: &BidOrder| bid.owner.clone().to_string(),
            NAMESPACE_BID_PK,
            NAMESPACE_OWNER_IDX,
        ),
        type_index: MultiIndex::new(
            |bid: &BidOrder| bid.bid_type.get_name().to_string(),
            NAMESPACE_BID_PK,
            NAMESPACE_TYPE_IDX,
        ),
    };
    IndexedMap::new(NAMESPACE_BID_PK, indices)
}

pub fn insert_bid_order(
    storage: &mut dyn Storage,
    bid_order: &BidOrder,
) -> Result<(), ContractError> {
    let state = bid_orders();
    if let Ok(existing_bid) = state.load(storage, bid_order.get_pk()) {
        return ContractError::storage_error(format!(
            "a bid with id [{}] for owner [{}] already exists",
            existing_bid.id,
            existing_bid.owner.as_str(),
        ))
        .to_err();
    }
    state
        .replace(storage, bid_order.get_pk(), Some(bid_order), None)?
        .to_ok()
}

pub fn get_bid_order_by_id<S: Into<String>>(
    storage: &dyn Storage,
    id: S,
) -> Result<BidOrder, ContractError> {
    let id = id.into();
    bid_orders().load(storage, id.as_bytes()).map_err(|e| {
        ContractError::storage_error(format!("failed to find BidOrder by id [{}]: {:?}", id, e))
    })
}

pub fn delete_bid_order_by_id<S: Into<String>>(
    storage: &mut dyn Storage,
    id: S,
) -> Result<(), ContractError> {
    let id = id.into();
    bid_orders().remove(storage, id.as_bytes()).map_err(|e| {
        ContractError::storage_error(format!("failed to remove BidOrder by id [{}]: {:?}", id, e))
    })?;
    ().to_ok()
}

#[cfg(test)]
mod tests {
    use crate::storage::bid_order_storage::{
        delete_bid_order_by_id, get_bid_order_by_id, insert_bid_order,
    };
    use crate::types::request::bid_types::bid_collateral::BidCollateral;
    use crate::types::request::bid_types::bid_order::BidOrder;
    use cosmwasm_std::{coins, Addr};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_insert_bid_order() {
        let mut deps = mock_dependencies(&[]);
        let mut order = BidOrder::new_unchecked(
            "bid",
            Addr::unchecked("bidder"),
            BidCollateral::coin_trade(&[], &[]),
            None,
        );
        insert_bid_order(deps.as_mut().storage, &order)
            .expect("inserting a bid order should succeed");
        insert_bid_order(deps.as_mut().storage, &order)
            .expect_err("inserting a bid order with a duplicate id should fail");
        order.id = "bid2".to_string();
        insert_bid_order(deps.as_mut().storage, &order)
            .expect("expected a new id to allow a nearly-identical bid order to be inserted");
    }

    #[test]
    fn test_get_bid_order_by_id() {
        let mut deps = mock_dependencies(&[]);
        let order = BidOrder::new_unchecked(
            "bid",
            Addr::unchecked("bidder"),
            BidCollateral::marker_trade(Addr::unchecked("marker"), "marker", &[]),
            None,
        );
        get_bid_order_by_id(deps.as_ref().storage, &order.id).expect_err("expected a get for the bid order by id to faiil when the order does not exist in storage");
        insert_bid_order(deps.as_mut().storage, &order)
            .expect("expected inserting a bid order to succeed");
        let stored_order = get_bid_order_by_id(deps.as_ref().storage, &order.id)
            .expect("expected getting a bid order by id to succeed after it has been stored");
        assert_eq!(
            order,
            stored_order,
            "expected the stored order to be retrieved as an identical copy to the originally stored value",
        );
    }

    #[test]
    fn test_delete_bid_order_by_id() {
        let mut deps = mock_dependencies(&[]);
        let order = BidOrder::new_unchecked(
            "bid",
            Addr::unchecked("bidder"),
            BidCollateral::marker_share_sale(
                Addr::unchecked("marker"),
                "markerdenom",
                100,
                &coins(10000, "nhash"),
            ),
            None,
        );
        insert_bid_order(deps.as_mut().storage, &order)
            .expect("inserting a bid order should succeed");
        get_bid_order_by_id(deps.as_ref().storage, &order.id)
            .expect("sanity check: the order should be available to get from storage");
        delete_bid_order_by_id(deps.as_mut().storage, &order.id)
            .expect("expected deletion to succeed for an existing bid order");
        get_bid_order_by_id(deps.as_ref().storage, &order.id)
            .expect_err("expected getting a bid order after it has been deleted to fail");
    }
}
