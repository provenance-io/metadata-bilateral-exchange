use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MatchReport {
    pub ask_id: String,
    pub bid_id: String,
    pub ask_exists: bool,
    pub bid_exists: bool,
    pub standard_match_possible: bool,
    pub quote_mismatch_match_possible: bool,
    pub error_messages: Vec<String>,
}
