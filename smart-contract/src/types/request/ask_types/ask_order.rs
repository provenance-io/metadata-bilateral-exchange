use crate::types::request::ask_types::ask_collateral::AskCollateral;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AskOrder {
    pub id: String,
    pub ask_type: RequestType,
    pub owner: Addr,
    pub collateral: AskCollateral,
    pub descriptor: Option<RequestDescriptor>,
}
impl AskOrder {
    pub fn get_collateral_index(&self) -> String {
        match &self.collateral {
            // Coin trades have no metadata involved - just use self.id as a duplicate index
            AskCollateral::CoinTrade(_) => self.id.clone(),
            // Marker trades include a marker address - only one ask per marker should be created at a time
            AskCollateral::MarkerTrade(collateral) => collateral.marker_address.to_string(),
            // Marker trades include a marker address - only one ask per marker should be created at a time
            AskCollateral::MarkerShareSale(collateral) => collateral.marker_address.to_string(),
            // Scope trades include a scope address - only one ask per scope should be created at a time
            AskCollateral::ScopeTrade(collateral) => collateral.scope_address.to_owned(),
        }
    }
}
