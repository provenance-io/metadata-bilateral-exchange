use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SettingsUpdate {
    pub new_admin_address: Option<String>,
    pub ask_fee: Option<Vec<Coin>>,
    pub bid_fee: Option<Vec<Coin>>,
}
