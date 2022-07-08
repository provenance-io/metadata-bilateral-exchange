use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::bid_types::legacy_bid_order::LegacyBidOrder;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{Order, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};

const NAMESPACE_BID_PK: &str = "bid";
const NAMESPACE_OWNER_IDX: &str = "bid__owner";
const NAMESPACE_TYPE_IDX: &str = "bid__type";

pub struct LegacyBidOrderIndices<'a> {
    pub owner_index: MultiIndex<'a, String, LegacyBidOrder, String>,
    pub type_index: MultiIndex<'a, String, LegacyBidOrder, String>,
}
impl<'a> IndexList<LegacyBidOrder> for LegacyBidOrderIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<LegacyBidOrder>> + '_> {
        let v: Vec<&dyn Index<LegacyBidOrder>> = vec![&self.owner_index, &self.type_index];
        Box::new(v.into_iter())
    }
}

pub fn legacy_bid_orders<'a>() -> IndexedMap<'a, &'a [u8], LegacyBidOrder, LegacyBidOrderIndices<'a>>
{
    let indices = LegacyBidOrderIndices {
        owner_index: MultiIndex::new(
            |bid: &LegacyBidOrder| bid.owner.clone().to_string(),
            NAMESPACE_BID_PK,
            NAMESPACE_OWNER_IDX,
        ),
        type_index: MultiIndex::new(
            |bid: &LegacyBidOrder| bid.bid_type.get_name().to_string(),
            NAMESPACE_BID_PK,
            NAMESPACE_TYPE_IDX,
        ),
    };
    IndexedMap::new(NAMESPACE_BID_PK, indices)
}

pub fn get_converted_legacy_bid_orders(
    storage: &mut dyn Storage,
) -> Result<Vec<BidOrder>, ContractError> {
    let mut converted_bid_orders = vec![];
    for result in legacy_bid_orders().range(storage, None, None, Order::Descending) {
        converted_bid_orders.push(result?.1.to_new_bid_order());
    }
    converted_bid_orders.to_ok()
}
