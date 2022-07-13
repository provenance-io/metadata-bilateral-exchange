use crate::storage::ask_order_storage::{get_ask_order_by_id, update_ask_order};
use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask::Ask;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::util::create_ask_order_utilities::{
    create_ask_order, AskCreationType, AskOrderCreationResponse,
};
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

pub fn update_ask(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    ask: Ask,
    descriptor: Option<RequestDescriptor>,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let existing_ask_order = get_ask_order_by_id(deps.storage, ask.get_id())?;
    let AskOrderCreationResponse {
        ask_order,
        messages,
        ..
    } = create_ask_order(
        &deps,
        &env,
        &info,
        ask,
        descriptor,
        AskCreationType::Update {
            existing_ask_order: Box::new(existing_ask_order),
        },
    )?;
    update_ask_order(deps.storage, &ask_order)?;
    Response::new()
        .add_attribute("action", "update_ask")
        .add_attribute("ask_id", &ask_order.id)
        .add_messages(messages)
        .set_data(to_binary(&ask_order)?)
        .to_ok()
}
