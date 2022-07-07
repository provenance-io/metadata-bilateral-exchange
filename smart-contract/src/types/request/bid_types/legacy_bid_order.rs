use crate::types::request::bid_types::legacy_bid_collateral::LegacyBidCollateral;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyBidOrder {
    pub id: String,
    pub bid_type: RequestType,
    pub owner: Addr,
    pub collateral: LegacyBidCollateral,
    pub descriptor: Option<RequestDescriptor>,
}
impl LegacyBidOrder {
    pub fn get_pk(&self) -> &[u8] {
        self.id.as_bytes()
    }
}
