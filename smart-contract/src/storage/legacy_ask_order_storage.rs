use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::types::request::ask_types::legacy_ask_order::LegacyAskOrder;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{Order, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex, UniqueIndex};

const NAMESPACE_ASK_PK: &str = "ask";
const NAMESPACE_COLLATERAL_IDX: &str = "ask__collateral";
const NAMESPACE_OWNER_IDX: &str = "ask__owner";
const NAMESPACE_TYPE_IDX: &str = "ask__type";

pub struct LegacyAskOrderIndices<'a> {
    pub collateral_index: UniqueIndex<'a, String, LegacyAskOrder>,
    pub owner_index: MultiIndex<'a, String, LegacyAskOrder, String>,
    pub type_index: MultiIndex<'a, String, LegacyAskOrder, String>,
}
impl<'a> IndexList<LegacyAskOrder> for LegacyAskOrderIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<LegacyAskOrder>> + '_> {
        let v: Vec<&dyn Index<LegacyAskOrder>> =
            vec![&self.collateral_index, &self.owner_index, &self.type_index];
        Box::new(v.into_iter())
    }
}

pub fn legacy_ask_orders<'a>() -> IndexedMap<'a, &'a [u8], LegacyAskOrder, LegacyAskOrderIndices<'a>>
{
    let indices = LegacyAskOrderIndices {
        collateral_index: UniqueIndex::new(
            |ask: &LegacyAskOrder| ask.get_collateral_index(),
            NAMESPACE_COLLATERAL_IDX,
        ),
        owner_index: MultiIndex::new(
            |ask: &LegacyAskOrder| ask.owner.clone().to_string(),
            NAMESPACE_ASK_PK,
            NAMESPACE_OWNER_IDX,
        ),
        type_index: MultiIndex::new(
            |ask: &LegacyAskOrder| ask.ask_type.get_name().to_string(),
            NAMESPACE_ASK_PK,
            NAMESPACE_TYPE_IDX,
        ),
    };
    IndexedMap::new(NAMESPACE_ASK_PK, indices)
}

pub fn get_converted_legacy_ask_orders(
    storage: &mut dyn Storage,
) -> Result<Vec<AskOrder>, ContractError> {
    let mut converted_ask_orders = vec![];
    for result in legacy_ask_orders().range(storage, None, None, Order::Descending) {
        converted_ask_orders.push(result?.1.to_new_ask_order());
    }
    converted_ask_orders.to_ok()
}
