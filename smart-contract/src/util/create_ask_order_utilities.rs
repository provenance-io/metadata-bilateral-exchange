use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask::{
    Ask, CoinTradeAsk, MarkerShareSaleAsk, MarkerTradeAsk, ScopeTradeAsk,
};
use crate::types::request::ask_types::ask_collateral::AskCollateral;
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::{check_scope_owners, get_single_marker_coin_holding};
use crate::util::request_fee::generate_request_fee_msg;
use crate::validation::ask_order_validation::validate_ask_order;
use crate::validation::marker_exchange_validation::validate_marker_for_ask;
use cosmwasm_std::{Addr, BankMsg, CosmosMsg, DepsMut, Env, MessageInfo};
use provwasm_std::{
    revoke_marker_access, AccessGrant, Marker, MarkerAccess, ProvenanceMsg, ProvenanceQuerier,
    ProvenanceQuery,
};

pub enum AskCreationType {
    New,
    Update { existing_ask_order: Box<AskOrder> },
}

pub struct AskOrderCreationResponse {
    pub ask_order: AskOrder,
    pub messages: Vec<CosmosMsg<ProvenanceMsg>>,
    pub ask_fee_msg: Option<CosmosMsg<ProvenanceMsg>>,
}

pub fn create_ask_order(
    deps: &DepsMut<ProvenanceQuery>,
    env: &Env,
    info: &MessageInfo,
    ask: Ask,
    descriptor: Option<RequestDescriptor>,
    creation_type: AskCreationType,
) -> Result<AskOrderCreationResponse, ContractError> {
    let ask_fee_msg = match &creation_type {
        AskCreationType::New => generate_request_fee_msg(
            "ask creation",
            &deps.as_ref(),
            env.contract.address.clone(),
            |c| c.create_ask_nhash_fee.u128(),
        )?,
        // Updates do not charge creation fees
        AskCreationType::Update { .. } => None,
    };
    let AskCreationData {
        collateral,
        messages,
    } = match &ask {
        Ask::CoinTrade(coin_ask) => create_coin_trade_ask_collateral(creation_type, info, coin_ask),
        Ask::MarkerTrade(marker_ask) => {
            create_marker_trade_ask_collateral(creation_type, deps, info, env, marker_ask)
        }
        Ask::MarkerShareSale(marker_share_sale) => create_marker_share_sale_ask_collateral(
            creation_type,
            deps,
            info,
            env,
            marker_share_sale,
        ),
        Ask::ScopeTrade(scope_trade) => {
            create_scope_trade_ask_collateral(creation_type, deps, info, env, scope_trade)
        }
    }?;
    let ask_order = AskOrder {
        id: ask.get_id().to_string(),
        ask_type: RequestType::from_ask_collateral(&collateral),
        owner: info.sender.clone(),
        collateral,
        descriptor,
    };
    validate_ask_order(&ask_order)?;
    AskOrderCreationResponse {
        ask_order,
        messages,
        ask_fee_msg,
    }
    .to_ok()
}

struct AskCreationData {
    pub collateral: AskCollateral,
    pub messages: Vec<CosmosMsg<ProvenanceMsg>>,
}

// create ask entrypoint
fn create_coin_trade_ask_collateral(
    creation_type: AskCreationType,
    info: &MessageInfo,
    coin_trade: &CoinTradeAsk,
) -> Result<AskCreationData, ContractError> {
    if info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "coin trade ask requests should include funds for a base".to_string(),
        }
        .to_err();
    }
    if coin_trade.id.is_empty() {
        return ContractError::MissingField {
            field: "id".to_string(),
        }
        .to_err();
    }
    if coin_trade.quote.is_empty() {
        return ContractError::MissingField {
            field: "quote".to_string(),
        }
        .to_err();
    }
    let messages = match creation_type {
        AskCreationType::New => vec![],
        // Refund the original base sent when the ask was initially created, assuming the new quote
        // is the new desired amount
        AskCreationType::Update { existing_ask_order } => {
            check_ask_type(
                &existing_ask_order.id,
                &existing_ask_order.ask_type,
                &RequestType::CoinTrade,
            )?;
            vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: existing_ask_order
                    .collateral
                    .get_coin_trade()?
                    .base
                    .to_owned(),
            })]
        }
    };
    AskCreationData {
        collateral: AskCollateral::coin_trade(&info.funds, &coin_trade.quote),
        messages,
    }
    .to_ok()
}

fn create_marker_trade_ask_collateral(
    creation_type: AskCreationType,
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    env: &Env,
    marker_trade: &MarkerTradeAsk,
) -> Result<AskCreationData, ContractError> {
    if !info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "marker trade ask requests should not include funds".to_string(),
        }
        .to_err();
    }
    let marker =
        ProvenanceQuerier::new(&deps.querier).get_marker_by_denom(&marker_trade.marker_denom)?;
    validate_marker_for_ask(
        &marker,
        match &creation_type {
            // New asks should verify that the sender owns the marker, and then revoke its permissions
            AskCreationType::New => Some(&info.sender),
            // Updates to asks should verify only that the contract still owns the marker
            AskCreationType::Update { .. } => None,
        },
        &env.contract.address,
        &[MarkerAccess::Admin, MarkerAccess::Withdraw],
        None,
    )?;
    let messages = match &creation_type {
        AskCreationType::New => {
            get_marker_permission_revoke_messages(&marker, &env.contract.address)?
        }
        AskCreationType::Update {
            ref existing_ask_order,
        } => {
            check_ask_type(
                &existing_ask_order.id,
                &existing_ask_order.ask_type,
                &RequestType::MarkerTrade,
            )?;
            let existing_marker_denom = get_update_marker_denom(existing_ask_order)?;
            if existing_marker_denom != &marker_trade.marker_denom {
                return ContractError::InvalidUpdate {
                    explanation: format!(
                        "marker trade with id [{}] cannot change marker denom with an update. current denom [{}], proposed new denom [{}]",
                        existing_ask_order.id,
                        existing_marker_denom,
                        marker_trade.marker_denom,
                    )
                }.to_err();
            }
            vec![]
        }
    };
    AskCreationData {
        collateral: AskCollateral::marker_trade(
            marker.address.clone(),
            &marker.denom,
            get_single_marker_coin_holding(&marker)?.amount.u128(),
            &marker_trade.quote_per_share,
            &match &creation_type {
                AskCreationType::New => marker
                    .permissions
                    .into_iter()
                    .filter(|perm| perm.address != env.contract.address)
                    .collect::<Vec<AccessGrant>>(),
                AskCreationType::Update { existing_ask_order } => {
                    get_update_marker_removed_permissions(existing_ask_order)?
                }
            },
        ),
        messages,
    }
    .to_ok()
}

fn create_marker_share_sale_ask_collateral(
    creation_type: AskCreationType,
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    env: &Env,
    marker_share_sale: &MarkerShareSaleAsk,
) -> Result<AskCreationData, ContractError> {
    if !info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "marker share sale ask requests should not include funds".to_string(),
        }
        .to_err();
    }
    let marker = ProvenanceQuerier::new(&deps.querier)
        .get_marker_by_denom(&marker_share_sale.marker_denom)?;
    validate_marker_for_ask(
        &marker,
        match &creation_type {
            // New asks should verify that the sender owns the marker, and then revoke its permissions
            AskCreationType::New => Some(&info.sender),
            // Updates to asks should verify only that the contract still owns the marker
            AskCreationType::Update { .. } => None,
        },
        &env.contract.address,
        &[MarkerAccess::Admin, MarkerAccess::Withdraw],
        Some(marker_share_sale.shares_to_sell.u128()),
    )?;
    let messages = match &creation_type {
        AskCreationType::New => {
            get_marker_permission_revoke_messages(&marker, &env.contract.address)?
        }
        AskCreationType::Update {
            ref existing_ask_order,
        } => {
            check_ask_type(
                &existing_ask_order.id,
                &existing_ask_order.ask_type,
                &RequestType::MarkerShareSale,
            )?;
            let existing_marker_denom = get_update_marker_denom(existing_ask_order)?;
            if existing_marker_denom != &marker_share_sale.marker_denom {
                return ContractError::InvalidUpdate {
                    explanation: format!(
                        "marker share sale with id [{}] cannot change marker denom with an update. current denom [{}], proposed new denom [{}]",
                        existing_ask_order.id,
                        existing_marker_denom,
                        marker_share_sale.marker_denom,
                    )
                }.to_err();
            }
            vec![]
        }
    };
    AskCreationData {
        collateral: AskCollateral::marker_share_sale(
            marker.address.clone(),
            &marker.denom,
            marker_share_sale.shares_to_sell.u128(),
            marker_share_sale.shares_to_sell.u128(),
            &marker_share_sale.quote_per_share,
            &match &creation_type {
                AskCreationType::New => marker
                    .permissions
                    .into_iter()
                    .filter(|perm| perm.address != env.contract.address)
                    .collect::<Vec<AccessGrant>>(),
                AskCreationType::Update { existing_ask_order } => {
                    get_update_marker_removed_permissions(existing_ask_order)?
                }
            },
            marker_share_sale.share_sale_type.to_owned(),
        ),
        messages,
    }
    .to_ok()
}

fn create_scope_trade_ask_collateral(
    creation_type: AskCreationType,
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    env: &Env,
    scope_trade: &ScopeTradeAsk,
) -> Result<AskCreationData, ContractError> {
    if !info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "scope trade ask requests should not include funds".to_string(),
        }
        .to_err();
    }
    check_scope_owners(
        &ProvenanceQuerier::new(&deps.querier).get_scope(&scope_trade.scope_address)?,
        Some(&env.contract.address),
        Some(&env.contract.address),
    )?;
    match creation_type {
        AskCreationType::New => {}
        AskCreationType::Update { existing_ask_order } => {
            check_ask_type(
                &existing_ask_order.id,
                &existing_ask_order.ask_type,
                &RequestType::ScopeTrade,
            )?;
            let existing_collateral = existing_ask_order.collateral.get_scope_trade()?;
            if existing_collateral.scope_address != scope_trade.scope_address {
                return ContractError::InvalidUpdate {
                    explanation: format!(
                        "scope trade with id [{}] cannot change scope address with an update. current address [{}], proposed new address [{}]",
                        existing_ask_order.id,
                        existing_collateral.scope_address,
                        scope_trade.scope_address,
                    )
                }.to_err();
            }
        }
    }
    AskCreationData {
        collateral: AskCollateral::scope_trade(&scope_trade.scope_address, &scope_trade.quote),
        messages: vec![],
    }
    .to_ok()
}

fn get_marker_permission_revoke_messages(
    marker: &Marker,
    contract_address: &Addr,
) -> Result<Vec<CosmosMsg<ProvenanceMsg>>, ContractError> {
    let mut messages: Vec<CosmosMsg<ProvenanceMsg>> = vec![];
    for permission in marker
        .permissions
        .iter()
        .filter(|perm| &perm.address != contract_address)
    {
        messages.push(revoke_marker_access(
            &marker.denom,
            permission.address.clone(),
        )?);
    }
    messages.to_ok()
}

fn check_ask_type<S: Into<String>>(
    ask_id: S,
    existing_ask_type: &RequestType,
    expected_ask_type: &RequestType,
) -> Result<(), ContractError> {
    let valid_update = match existing_ask_type {
        RequestType::MarkerTrade | RequestType::MarkerShareSale => {
            expected_ask_type == &RequestType::MarkerTrade
                || expected_ask_type == &RequestType::MarkerShareSale
        }
        ask_type => ask_type == expected_ask_type,
    };
    if !valid_update {
        ContractError::InvalidUpdate {
            explanation: format!(
                "ask with id [{}] cannot change ask type from [{}] to [{}]",
                ask_id.into(),
                existing_ask_type.get_name(),
                expected_ask_type.get_name(),
            ),
        }
        .to_err()
    } else {
        ().to_ok()
    }
}

fn get_update_marker_denom(ask_order: &AskOrder) -> Result<&String, ContractError> {
    match &ask_order.collateral {
        AskCollateral::MarkerTrade(ref c) => &c.marker_denom,
        AskCollateral::MarkerShareSale(ref c) => &c.marker_denom,
        _ => {
            return ContractError::InvalidUpdate {
                explanation: format!(
                    "update for ask [{}] of type [{}] attempted to use marker collateral",
                    &ask_order.id,
                    ask_order.ask_type.get_name(),
                ),
            }
            .to_err();
        }
    }
    .to_ok()
}

fn get_update_marker_removed_permissions(
    ask_order: &AskOrder,
) -> Result<Vec<AccessGrant>, ContractError> {
    match &ask_order.collateral {
        AskCollateral::MarkerTrade(ref c) => c.removed_permissions.to_owned(),
        AskCollateral::MarkerShareSale(ref c) => c.removed_permissions.to_owned(),
        _ => {
            return ContractError::InvalidUpdate {
                explanation: format!(
                    "update for ask [{}] of type [{}] attempted to use marker collateral",
                    &ask_order.id,
                    ask_order.ask_type.get_name(),
                ),
            }
            .to_err();
        }
    }
    .to_ok()
}
