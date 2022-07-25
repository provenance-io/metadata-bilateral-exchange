use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SettingsUpdate {
    pub new_admin_address: Option<String>,
    pub new_create_ask_nhash_fee: Option<Uint128>,
    pub new_create_bid_nhash_fee: Option<Uint128>,
}
