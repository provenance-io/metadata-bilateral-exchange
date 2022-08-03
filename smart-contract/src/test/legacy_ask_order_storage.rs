use crate::storage::order_indices::OrderIndices;
use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::Storage;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex, UniqueIndex};

// TODO: Delete this entire file in v1.1.1

const LEGACY_NAMESPACE_ASK_PK: &str = "ask";
const LEGACY_NAMESPACE_COLLATERAL_IDX: &str = "ask__collateral";
const LEGACY_NAMESPACE_OWNER_IDX: &str = "ask__owner";
const LEGACY_NAMESPACE_TYPE_IDX: &str = "ask__type";

pub struct LegacyAskOrderIndices<'a> {
    pub collateral_index: UniqueIndex<'a, String, AskOrder>,
    pub owner_index: MultiIndex<'a, String, AskOrder, String>,
    pub type_index: MultiIndex<'a, String, AskOrder, String>,
}
impl<'a> IndexList<AskOrder> for LegacyAskOrderIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AskOrder>> + '_> {
        let v: Vec<&dyn Index<AskOrder>> =
            vec![&self.collateral_index, &self.owner_index, &self.type_index];
        Box::new(v.into_iter())
    }
}
impl<'a> OrderIndices<'a, AskOrder> for LegacyAskOrderIndices<'a> {
    fn owner_index(&self) -> &MultiIndex<'a, String, AskOrder, String> {
        &self.owner_index
    }

    fn type_index(&self) -> &MultiIndex<'a, String, AskOrder, String> {
        &self.type_index
    }
}

pub fn legacy_ask_orders<'a>() -> IndexedMap<'a, &'a [u8], AskOrder, LegacyAskOrderIndices<'a>> {
    let indices = LegacyAskOrderIndices {
        collateral_index: UniqueIndex::new(
            |ask: &AskOrder| ask.get_collateral_index(),
            LEGACY_NAMESPACE_COLLATERAL_IDX,
        ),
        owner_index: MultiIndex::new(
            |ask: &AskOrder| ask.owner.clone().to_string(),
            LEGACY_NAMESPACE_ASK_PK,
            LEGACY_NAMESPACE_OWNER_IDX,
        ),
        type_index: MultiIndex::new(
            |ask: &AskOrder| ask.ask_type.get_name().to_string(),
            LEGACY_NAMESPACE_ASK_PK,
            LEGACY_NAMESPACE_TYPE_IDX,
        ),
    };
    IndexedMap::new(LEGACY_NAMESPACE_ASK_PK, indices)
}

pub fn insert_legacy_ask_order(
    storage: &mut dyn Storage,
    ask_order: &AskOrder,
) -> Result<(), ContractError> {
    let state = legacy_ask_orders();
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
    store_legacy_ask_order(storage, ask_order)
}

pub fn update_legacy_ask_order(
    storage: &mut dyn Storage,
    ask_order: &AskOrder,
) -> Result<(), ContractError> {
    let state = legacy_ask_orders();
    if state.load(storage, ask_order.id.as_bytes()).is_ok() {
        delete_legacy_ask_order_by_id(storage, &ask_order.id)?;
        store_legacy_ask_order(storage, ask_order)
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

fn store_legacy_ask_order(
    storage: &mut dyn Storage,
    ask_order: &AskOrder,
) -> Result<(), ContractError> {
    legacy_ask_orders()
        .replace(storage, ask_order.id.as_bytes(), Some(ask_order), None)?
        .to_ok()
}

pub fn may_get_legacy_ask_order_by_id<S: Into<String>>(
    storage: &dyn Storage,
    id: S,
) -> Option<AskOrder> {
    legacy_ask_orders()
        .may_load(storage, id.into().as_bytes())
        .unwrap_or(None)
}

pub fn get_legacy_ask_order_by_id<S: Into<String>>(
    storage: &dyn Storage,
    id: S,
) -> Result<AskOrder, ContractError> {
    let id = id.into();
    legacy_ask_orders()
        .load(storage, id.as_bytes())
        .map_err(|e| ContractError::StorageError {
            message: format!("failed to find AskOrder by id [{}]: {:?}", id, e),
        })
}

pub fn may_get_legacy_ask_order_by_collateral_id<S: Into<String>>(
    storage: &dyn Storage,
    collateral_id: S,
) -> Option<AskOrder> {
    legacy_ask_orders()
        .idx
        .collateral_index
        .item(storage, collateral_id.into())
        .map(|record| record.map(|(_, option)| option))
        .unwrap_or(None)
}

pub fn delete_legacy_ask_order_by_id<S: Into<String>>(
    storage: &mut dyn Storage,
    id: S,
) -> Result<(), ContractError> {
    let id = id.into();
    legacy_ask_orders()
        .remove(storage, id.as_bytes())
        .map_err(|e| ContractError::StorageError {
            message: format!("failed to remove AskOrder by id [{}]: {:?}", id, e),
        })?;
    ().to_ok()
}
