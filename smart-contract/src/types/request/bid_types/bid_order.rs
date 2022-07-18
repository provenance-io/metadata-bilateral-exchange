use crate::types::request::bid_types::bid_collateral::BidCollateral;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BidOrder {
    pub id: String,
    pub bid_type: RequestType,
    pub owner: Addr,
    pub collateral: BidCollateral,
    pub descriptor: Option<RequestDescriptor>,
}
