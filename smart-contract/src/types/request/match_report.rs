use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, Binary};
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
impl MatchReport {
    pub fn new_missing_order<A: Into<String>, B: Into<String>>(
        ask_id: A,
        bid_id: B,
        ask_exists: bool,
        bid_exists: bool,
    ) -> Result<Self, ContractError> {
        let ask_id = ask_id.into();
        let bid_id = bid_id.into();
        let error_message = if ask_exists && bid_exists {
            return ContractError::validation_error(&["created missing order MatchReport with both ask and bid existing. this is a contract bug"]).to_err();
        } else if ask_exists {
            format!("BidOrder [{}] was missing from contract storage", &bid_id)
        } else if bid_exists {
            format!("AskOrder [{}] was missing from contract storage", &ask_id)
        } else {
            format!(
                "AskOrder [{}] and BidOrder [{}] were missing from contract storage",
                &ask_id, &bid_id
            )
        };
        Self {
            ask_id,
            bid_id,
            ask_exists,
            bid_exists,
            standard_match_possible: false,
            quote_mismatch_match_possible: false,
            error_messages: vec![error_message],
        }
        .to_ok()
    }

    pub fn new_existing_orders<A: Into<String>, B: Into<String>, E: Into<String>>(
        ask_id: A,
        bid_id: B,
        standard_match_possible: bool,
        quote_mismatch_match_possible: bool,
        error_messages: &[E],
    ) -> Self
    where
        E: Clone,
    {
        Self {
            ask_id: ask_id.into(),
            bid_id: bid_id.into(),
            ask_exists: true,
            bid_exists: true,
            standard_match_possible,
            quote_mismatch_match_possible,
            error_messages: error_messages.iter().cloned().map(|s| s.into()).collect(),
        }
    }

    pub fn to_binary(&self) -> Result<Binary, ContractError> {
        to_binary(self)?.to_ok()
    }
}
