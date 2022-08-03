use crate::storage::order_indices::OrderIndices;
use crate::types::core::constants::DEFAULT_SEARCH_ORDER;
use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::Storage;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};

const NAMESPACE_ASK_PK: &str = "ask";
const NAMESPACE_COLLATERAL_IDX: &str = "ask__collateral_v2";
const NAMESPACE_OWNER_IDX: &str = "ask__owner";
const NAMESPACE_TYPE_IDX: &str = "ask__type";

pub struct AskOrderIndices<'a> {
    pub collateral_index: MultiIndex<'a, String, AskOrder, String>,
    pub owner_index: MultiIndex<'a, String, AskOrder, String>,
    pub type_index: MultiIndex<'a, String, AskOrder, String>,
}
impl<'a> IndexList<AskOrder> for AskOrderIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AskOrder>> + '_> {
        let v: Vec<&dyn Index<AskOrder>> =
            vec![&self.collateral_index, &self.owner_index, &self.type_index];
        Box::new(v.into_iter())
    }
}
impl<'a> OrderIndices<'a, AskOrder> for AskOrderIndices<'a> {
    fn owner_index(&self) -> &MultiIndex<'a, String, AskOrder, String> {
        &self.owner_index
    }

    fn type_index(&self) -> &MultiIndex<'a, String, AskOrder, String> {
        &self.type_index
    }
}

pub fn ask_orders<'a>() -> IndexedMap<'a, &'a [u8], AskOrder, AskOrderIndices<'a>> {
    let indices = AskOrderIndices {
        collateral_index: MultiIndex::new(
            |ask: &AskOrder| ask.get_collateral_index(),
            NAMESPACE_ASK_PK,
            NAMESPACE_COLLATERAL_IDX,
        ),
        owner_index: MultiIndex::new(
            |ask: &AskOrder| ask.owner.clone().to_string(),
            NAMESPACE_ASK_PK,
            NAMESPACE_OWNER_IDX,
        ),
        type_index: MultiIndex::new(
            |ask: &AskOrder| ask.ask_type.get_name().to_string(),
            NAMESPACE_ASK_PK,
            NAMESPACE_TYPE_IDX,
        ),
    };
    IndexedMap::new(NAMESPACE_ASK_PK, indices)
}

pub fn insert_ask_order(
    storage: &mut dyn Storage,
    ask_order: &AskOrder,
) -> Result<(), ContractError> {
    let state = ask_orders();
    if let Ok(existing_ask) = state.load(storage, ask_order.id.as_bytes()) {
        return ContractError::StorageError {
            message: format!(
                "an ask with id [{}] for owner [{}] already exists",
                existing_ask.id,
                existing_ask.owner.as_str(),
            ),
        }
        .to_err();
    }
    store_ask_order(storage, ask_order)
}

pub fn update_ask_order(
    storage: &mut dyn Storage,
    ask_order: &AskOrder,
) -> Result<(), ContractError> {
    let state = ask_orders();
    if state.load(storage, ask_order.id.as_bytes()).is_ok() {
        delete_ask_order_by_id(storage, &ask_order.id)?;
        store_ask_order(storage, ask_order)
    } else {
        ContractError::StorageError {
            message: format!(
                "attempted to replace ask with id [{}] in storage, but no ask with that id existed",
                &ask_order.id
            ),
        }
        .to_err()
    }
}

fn store_ask_order(storage: &mut dyn Storage, ask_order: &AskOrder) -> Result<(), ContractError> {
    ask_orders()
        .replace(storage, ask_order.id.as_bytes(), Some(ask_order), None)?
        .to_ok()
}

pub fn may_get_ask_order_by_id<S: Into<String>>(storage: &dyn Storage, id: S) -> Option<AskOrder> {
    ask_orders()
        .may_load(storage, id.into().as_bytes())
        .unwrap_or(None)
}

pub fn get_ask_order_by_id<S: Into<String>>(
    storage: &dyn Storage,
    id: S,
) -> Result<AskOrder, ContractError> {
    let id = id.into();
    ask_orders()
        .load(storage, id.as_bytes())
        .map_err(|e| ContractError::StorageError {
            message: format!("failed to find AskOrder by id [{}]: {:?}", id, e),
        })
}

pub fn get_ask_orders_by_collateral_id<S: Into<String>>(
    storage: &dyn Storage,
    collateral_id: S,
) -> Vec<AskOrder> {
    ask_orders()
        .idx
        .collateral_index
        .prefix(collateral_id.into())
        .range(storage, None, None, DEFAULT_SEARCH_ORDER)
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap().1)
        .collect()
}

pub fn delete_ask_order_by_id<S: Into<String>>(
    storage: &mut dyn Storage,
    id: S,
) -> Result<(), ContractError> {
    let id = id.into();
    ask_orders()
        .remove(storage, id.as_bytes())
        .map_err(|e| ContractError::StorageError {
            message: format!("failed to remove AskOrder by id [{}]: {:?}", id, e),
        })?;
    ().to_ok()
}

#[cfg(test)]
mod tests {
    use crate::storage::ask_order_storage::{
        delete_ask_order_by_id, get_ask_order_by_id, get_ask_orders_by_collateral_id,
        insert_ask_order, may_get_ask_order_by_id, update_ask_order,
    };
    use crate::test::mock_marker::{DEFAULT_MARKER_ADDRESS, DEFAULT_MARKER_DENOM};
    use crate::types::request::ask_types::ask_collateral::AskCollateral;
    use crate::types::request::ask_types::ask_order::AskOrder;
    use crate::types::request::share_sale_type::ShareSaleType;
    use crate::util::constants::NHASH;
    use cosmwasm_std::{coins, Addr};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_insert_ask_order() {
        let mut deps = mock_dependencies(&[]);
        let mut order = AskOrder::new_unchecked(
            "ask1",
            Addr::unchecked("asker1"),
            AskCollateral::marker_trade(
                Addr::unchecked("marker-address"),
                "marker-denom",
                100,
                &coins(100, NHASH),
                &[],
            ),
            None,
        );
        insert_ask_order(deps.as_mut().storage, &order).expect("expected the insert to succeed");
        insert_ask_order(deps.as_mut().storage, &order)
            .expect_err("expected a secondary insert to be rejected because the ask ids match");
        order.id = "ask2".to_string();
        insert_ask_order(deps.as_mut().storage, &order)
            .expect("expected a secondary insert to succeed because the ids do not match");
    }

    #[test]
    fn test_update_ask_order() {
        let mut deps = mock_dependencies(&[]);
        let mut order = AskOrder::new_unchecked(
            "ask",
            Addr::unchecked("asker"),
            AskCollateral::scope_trade("scope", &coins(100, NHASH)),
            None,
        );
        update_ask_order(deps.as_mut().storage, &order)
            .expect_err("expected an update to fail when the ask does not yet exist in storage");
        insert_ask_order(deps.as_mut().storage, &order)
            .expect("expected inserting an ask order to succeed");
        update_ask_order(deps.as_mut().storage, &order)
            .expect("expected updating an ask order to itself to succeed");
        order.id = "ask2".to_string();
        update_ask_order(deps.as_mut().storage, &order).expect_err("expected updating an ask order after changing its id to fail because it no longer has the same PK");
    }

    #[test]
    fn test_get_ask_order_by_id() {
        let mut deps = mock_dependencies(&[]);
        let order = AskOrder::new_unchecked(
            "ask",
            Addr::unchecked("asker"),
            AskCollateral::scope_trade("scope", &coins(100, NHASH)),
            None,
        );
        get_ask_order_by_id(deps.as_ref().storage, &order.id).expect_err("expected a get for the ask order by id to fail when the order does not yet exist in storage");
        insert_ask_order(deps.as_mut().storage, &order)
            .expect("expected inserting an ask order to succeed");
        let stored_order = get_ask_order_by_id(deps.as_ref().storage, &order.id)
            .expect("expected getting an ask order by id to succeed after it has been stored");
        assert_eq!(
            order,
            stored_order,
            "expected the stored order to be retrieved as an identical copy to the originally stored value",
        );
    }

    #[test]
    fn test_may_get_ask_order_by_id() {
        let mut deps = mock_dependencies(&[]);
        let order = AskOrder::new_unchecked(
            "ask",
            Addr::unchecked("asker"),
            AskCollateral::scope_trade("scope", &coins(100, NHASH)),
            None,
        );
        assert!(
            may_get_ask_order_by_id(deps.as_ref().storage, &order.id).is_none(),
            "ask order should fail to load because no order exists with the given id",
        );
        insert_ask_order(deps.as_mut().storage, &order)
            .expect("expected inserting an ask order to succeed");
        let stored_order = may_get_ask_order_by_id(deps.as_ref().storage, &order.id)
            .expect("expected getting an ask order by id to succeed after it has been stored");
        assert_eq!(
            order,
            stored_order,
            "expected the stored order to be retrieved as an identical copy to the originally stored value",
        );
    }

    #[test]
    fn test_get_ask_orders_by_collateral_id() {
        let mut deps = mock_dependencies(&[]);
        assert!(
            get_ask_orders_by_collateral_id(deps.as_ref().storage, DEFAULT_MARKER_ADDRESS)
                .is_empty(),
            "no ask orders should be returned because none have been inserted",
        );
        let first_order = AskOrder::new_unchecked(
            "ask",
            Addr::unchecked("asker"),
            AskCollateral::marker_share_sale(
                Addr::unchecked(DEFAULT_MARKER_ADDRESS),
                DEFAULT_MARKER_DENOM,
                10,
                10,
                &coins(100, NHASH),
                &[],
                ShareSaleType::SingleTransaction,
            ),
            None,
        );
        insert_ask_order(deps.as_mut().storage, &first_order)
            .expect("inserting the first ask order should succeed");
        let fetched_ask_orders =
            get_ask_orders_by_collateral_id(deps.as_ref().storage, DEFAULT_MARKER_ADDRESS);
        assert_eq!(
            1,
            fetched_ask_orders.len(),
            "the resulting search should produce a single ask order",
        );
        assert_eq!(
            &first_order,
            fetched_ask_orders.first().unwrap(),
            "the returned value should contain the only ask order using the correct marker address",
        );
        let second_order = AskOrder::new_unchecked(
            "ask2",
            Addr::unchecked("asker"),
            AskCollateral::marker_share_sale(
                Addr::unchecked(DEFAULT_MARKER_ADDRESS),
                DEFAULT_MARKER_DENOM,
                10,
                10,
                &coins(400, NHASH),
                &[],
                ShareSaleType::MultipleTransactions,
            ),
            None,
        );
        insert_ask_order(deps.as_mut().storage, &second_order)
            .expect("inserting the second ask order should succeed");
        let fetched_ask_orders =
            get_ask_orders_by_collateral_id(deps.as_ref().storage, DEFAULT_MARKER_ADDRESS);
        assert_eq!(
            2,
            fetched_ask_orders.len(),
            "the resulting search should produce two ask orders",
        );
        assert!(
            fetched_ask_orders.iter().any(|order| order == &first_order),
            "the first ask order should be produced in the query results",
        );
        assert!(
            fetched_ask_orders
                .iter()
                .any(|order| order == &second_order),
            "the second ask order should be produced in the query results",
        );
    }

    #[test]
    fn test_delete_ask_order_by_id() {
        let mut deps = mock_dependencies(&[]);
        let order = AskOrder::new_unchecked(
            "ask",
            Addr::unchecked("asker"),
            AskCollateral::coin_trade(&[], &coins(100, NHASH)),
            None,
        );
        insert_ask_order(deps.as_mut().storage, &order)
            .expect("inserting an ask order should succeed");
        get_ask_order_by_id(deps.as_ref().storage, &order.id)
            .expect("sanity check: the order should be available to get from storage");
        delete_ask_order_by_id(deps.as_mut().storage, &order.id)
            .expect("expected deletion to succeed for an existing ask order");
        get_ask_order_by_id(deps.as_ref().storage, &order.id)
            .expect_err("expected getting an ask order after it has been deleted to fail");
    }
}
