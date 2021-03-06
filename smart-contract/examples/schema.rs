use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use metadata_bilateral_exchange::storage::contract_info::ContractInfoV2;
use metadata_bilateral_exchange::types::core::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use metadata_bilateral_exchange::types::request::ask_types::ask_order::AskOrder;
use metadata_bilateral_exchange::types::request::bid_types::bid_order::BidOrder;
use metadata_bilateral_exchange::types::request::match_report::MatchReport;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();
    export_schema(&schema_for!(AskOrder), &out_dir);
    export_schema(&schema_for!(BidOrder), &out_dir);
    export_schema(&schema_for!(MatchReport), &out_dir);
    export_schema(&schema_for!(ContractInfoV2), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
}
