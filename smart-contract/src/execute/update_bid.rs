use crate::storage::bid_order_storage::{get_bid_order_by_id, update_bid_order};
use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid::Bid;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::util::create_bid_order_utilities::{create_bid_order, BidCreationType};
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::format_coin_display;
use cosmwasm_std::{to_binary, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

pub fn update_bid(
    deps: DepsMut<ProvenanceQuery>,
    info: MessageInfo,
    bid: Bid,
    descriptor: Option<RequestDescriptor>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let existing_bid_order = get_bid_order_by_id(deps.storage, bid.get_id())?;
    let refunded_quote = existing_bid_order.collateral.get_quote();
    let new_bid_order = create_bid_order(
        &deps,
        &info,
        bid,
        descriptor,
        BidCreationType::Update {
            existing_bid_order: Box::new(existing_bid_order),
        },
    )?
    .bid_order;
    update_bid_order(deps.storage, &new_bid_order)?;
    Response::new()
        .add_attribute("action", "update_bid")
        .add_attribute("bid_id", &new_bid_order.id)
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: refunded_quote,
        }))
        .set_data(to_binary(&new_bid_order)?)
        .to_ok()
}
