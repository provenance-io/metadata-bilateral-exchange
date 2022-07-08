use crate::types::request::ask_types::ask::Ask;
use crate::types::request::bid_types::bid::Bid;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::search::Search;
use crate::types::request::settings_update::SettingsUpdate;
use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub bind_name: String,
    pub contract_name: String,
    pub ask_fee: Option<Vec<Coin>>,
    pub bid_fee: Option<Vec<Coin>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CancelAsk {
        id: String,
    },
    CancelBid {
        id: String,
    },
    CreateAsk {
        ask: Ask,
        descriptor: Option<RequestDescriptor>,
    },
    CreateBid {
        bid: Bid,
        descriptor: Option<RequestDescriptor>,
    },
    ExecuteMatch {
        ask_id: String,
        bid_id: String,
        accept_mismatched_bids: Option<bool>,
    },
    UpdateSettings {
        update: SettingsUpdate,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAsk { id: String },
    GetAskByCollateralId { collateral_id: String },
    GetBid { id: String },
    GetMatchReport { ask_id: String, bid_id: String },
    GetContractInfo {},
    SearchAsks { search: Search },
    SearchBids { search: Search },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {
    ContractUpgrade {},
}
