use crate::types::request::ask_types::legacy_ask_collateral::LegacyAskCollateral;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: Remove this after type migrations have occurred
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LegacyAskOrder {
    pub id: String,
    pub ask_type: RequestType,
    pub owner: Addr,
    pub collateral: LegacyAskCollateral,
    pub descriptor: Option<RequestDescriptor>,
}
impl LegacyAskOrder {
    pub fn get_pk(&self) -> &[u8] {
        self.id.as_bytes()
    }

    pub fn get_collateral_index(&self) -> String {
        match &self.collateral {
            // Coin trades have no metadata involved - just use self.id as a duplicate index
            LegacyAskCollateral::CoinTrade(_) => self.id.clone(),
            // Marker trades include a marker address - only one ask per marker should be created at a time
            LegacyAskCollateral::MarkerTrade(collateral) => collateral.address.to_string(),
            // Marker trades include a marker address - only one ask per marker should be created at a time
            LegacyAskCollateral::MarkerShareSale(collateral) => collateral.address.to_string(),
            // Scope trades include a scope address - only one ask per scope should be created at a time
            LegacyAskCollateral::ScopeTrade(collateral) => collateral.scope_address.to_owned(),
        }
    }
}
