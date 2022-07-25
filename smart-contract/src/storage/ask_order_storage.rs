use crate::storage::order_indices::OrderIndices;
use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::Storage;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex, UniqueIndex};

const NAMESPACE_ASK_PK: &str = "ask";
const NAMESPACE_COLLATERAL_IDX: &str = "ask__collateral";
const NAMESPACE_OWNER_IDX: &str = "ask__owner";
const NAMESPACE_TYPE_IDX: &str = "ask__type";

pub struct AskOrderIndices<'a> {
    pub collateral_index: UniqueIndex<'a, String, AskOrder>,
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
        collateral_index: UniqueIndex::new(
            |ask: &AskOrder| ask.get_collateral_index(),
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

pub fn may_get_ask_order_by_collateral_id<S: Into<String>>(
    storage: &dyn Storage,
    collateral_id: S,
) -> Option<AskOrder> {
    ask_orders()
        .idx
        .collateral_index
        .item(storage, collateral_id.into())
        .map(|record| record.map(|(_, option)| option))
        .unwrap_or(None)
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
        delete_ask_order_by_id, get_ask_order_by_id, insert_ask_order,
        may_get_ask_order_by_collateral_id, may_get_ask_order_by_id, update_ask_order,
    };
    use crate::test::mock_marker::DEFAULT_MARKER_DENOM;
    use crate::test::request_helpers::{
        mock_ask_marker_share_sale, mock_ask_marker_trade, mock_ask_order, mock_ask_scope_trade,
    };
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
        insert_ask_order(deps.as_mut().storage, &order).expect_err(
            "expected a secondary insert to be rejected because the marker denoms match",
        );
        if let AskCollateral::MarkerTrade(ref mut collateral) = order.collateral {
            collateral.marker_address = Addr::unchecked("marker-address-2");
        }
        insert_ask_order(deps.as_mut().storage, &order)
            .expect("expected the insert to succeed when the ask did not violate any indices");
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
    fn test_may_get_ask_order_by_collateral_id() {
        let mut deps = mock_dependencies(&[]);
        let coin_trade_order = AskOrder {
            id: "coin_trade_ask".to_string(),
            ..mock_ask_order(AskCollateral::coin_trade(&[], &[]))
        };
        let mut test_collateral_id = |ask_order: AskOrder, expected_id: &str, ask_type: &str| {
            assert_eq!(
                None,
                may_get_ask_order_by_collateral_id(deps.as_ref().storage, expected_id),
                "expected the ask order to not be found by collateral id before it is inserted",
            );
            insert_ask_order(deps.as_mut().storage, &ask_order)
                .unwrap_or_else(|_| panic!("expected the {}'s insert to succeed", ask_type));
            assert_eq!(
                ask_order,
                may_get_ask_order_by_collateral_id(deps.as_ref().storage, expected_id)
                    .unwrap_or_else(|| panic!(
                        "expected the {}'s collateral id search to respond without error",
                        ask_type
                    )),
                "expected {}'s collateral id to be available ask the collateral id",
                ask_type,
            );
        };
        test_collateral_id(coin_trade_order, "coin_trade_ask", "coin trade ask");
        let marker_trade_order = AskOrder {
            id: "marker_trade_ask".to_string(),
            ..mock_ask_order(mock_ask_marker_trade(
                "marker_trade_address",
                DEFAULT_MARKER_DENOM,
                100,
                &[],
            ))
        };
        test_collateral_id(
            marker_trade_order,
            "marker_trade_address",
            "marker trade ask",
        );
        let marker_share_sale_order = AskOrder {
            id: "marker_share_sale".to_string(),
            ..mock_ask_order(mock_ask_marker_share_sale(
                "marker_share_sale_address",
                DEFAULT_MARKER_DENOM,
                100,
                50,
                &[],
                ShareSaleType::MultipleTransactions,
            ))
        };
        test_collateral_id(
            marker_share_sale_order,
            "marker_share_sale_address",
            "marker share sale ask",
        );
        let scope_trade_order = AskOrder {
            id: "scope_trade".to_string(),
            ..mock_ask_order(mock_ask_scope_trade("scope_trade_address", &[]))
        };
        test_collateral_id(scope_trade_order, "scope_trade_address", "scope trade ask");
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
