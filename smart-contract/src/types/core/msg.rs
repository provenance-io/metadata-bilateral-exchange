use crate::types::request::admin_match_options::AdminMatchOptions;
use crate::types::request::ask_types::ask::Ask;
use crate::types::request::bid_types::bid::Bid;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::search::Search;
use crate::types::request::settings_update::SettingsUpdate;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub bind_name: String,
    pub contract_name: String,
    pub create_ask_nhash_fee: Option<Uint128>,
    pub create_bid_nhash_fee: Option<Uint128>,
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
    UpdateAsk {
        ask: Ask,
        descriptor: Option<RequestDescriptor>,
    },
    CreateBid {
        bid: Bid,
        descriptor: Option<RequestDescriptor>,
    },
    UpdateBid {
        bid: Bid,
        descriptor: Option<RequestDescriptor>,
    },
    ExecuteMatch {
        ask_id: String,
        bid_id: String,
        admin_match_options: Option<AdminMatchOptions>,
    },
    UpdateSettings {
        update: SettingsUpdate,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetAsk {
        id: String,
    },
    GetAsksByCollateralId {
        collateral_id: String,
    },
    GetBid {
        id: String,
    },
    GetMatchReport {
        ask_id: String,
        bid_id: String,
        admin_match_options: Option<AdminMatchOptions>,
    },
    GetContractInfo {},
    SearchAsks {
        search: Search,
    },
    SearchBids {
        search: Search,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {
    ContractUpgrade {},
}
