use crate::storage::ask_order_storage::get_ask_orders_by_collateral_id;
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
use crate::validation::marker_exchange_validation::{
    validate_marker_for_ask, ShareSaleValidationDetail,
};
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
            // Marker trades must be uniquely created.  A marker trade cannot be established on top
            // of another marker trade or a marker share sale, because their behaviors conflict in
            // duplicate
            if !get_ask_orders_by_collateral_id(deps.storage, marker.address.as_str()).is_empty() {
                return ContractError::InvalidRequest {
                    message: format!("marker trade asks cannot exist alongside alternate asks for the same marker. marker: [{}]", marker.address.as_str()),
                }.to_err();
            }
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
            // If this update is converting a share sale ask to a marker trade, or even just updating
            // a marker trade, it absolutely cannot cause the marker trade to exist alongside a different
            // ask, because marker trades completely transfer ownership of the marker.  Allowing this
            // scenario would potentially cause a situation where the marker is released from the
            // contract, but another ask still exists that would try to sell shares of a marker that
            // would no longer be controlled.
            if get_ask_orders_by_collateral_id(deps.storage, marker.address.as_str()).len() > 1 {
                return ContractError::InvalidRequest {
                    message: format!("marker trade asks cannot exist alongside alternate asks for the same marker. marker: [{}]", marker.address.as_str()),
                }.to_err();
            }
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
    let existing_related_orders =
        get_ask_orders_by_collateral_id(deps.storage, marker.address.as_str());
    validate_marker_for_ask(
        &marker,
        match &creation_type {
            AskCreationType::New => {
                if existing_related_orders.is_empty() {
                    // New asks should verify that the sender owns the marker, and then revoke its permissions
                    Some(&info.sender)
                } else {
                    // Verify that the sender owns all other ask orders for the given marker. Without
                    // this check, any sender could piggyback an ask order for a marker they never
                    // owned and receive funds
                    if !existing_related_orders
                        .iter()
                        .all(|order| order.owner == info.sender)
                    {
                        return ContractError::InvalidRequest {
                            message: format!(
                                "the sender [{}] is not the owner of all existing marker share sales for the target marker [{}]",
                                info.sender.as_str(),
                                &marker.denom,
                            ),
                        }.to_err();
                    }
                    // If this is a new ask for an already held marker, the marker should no longer
                    // have permissions for the sender
                    None
                }
            }
            // Updates to asks should verify only that the contract still owns the marker
            AskCreationType::Update { .. } => None,
        },
        &env.contract.address,
        &[MarkerAccess::Admin, MarkerAccess::Withdraw],
        Some(ShareSaleValidationDetail {
            shares_sold: marker_share_sale.shares_to_sell.u128(),
            aggregate_shares_listed: existing_related_orders
                .iter()
                // Filter out existing orders with matching ids - if this is an update, this total should
                // not include the old share sale amount in the aggregate.  Only use existing values
                // from other shares sales listed alongside the updated or new value
                .filter(|order| order.id != marker_share_sale.id)
                .fold(0u128, |total, order| {
                    match &order.collateral {
                        // Assumes all related ask orders are marker share sales. Validation below will catch
                        // marker trades before order is fully committed and created
                        AskCollateral::MarkerShareSale(collateral) => {
                            total + collateral.remaining_shares_in_sale.u128()
                        }
                        _ => total,
                    }
                }),
        }),
    )?;
    let messages = match &creation_type {
        AskCreationType::New => {
            if existing_related_orders
                .iter()
                .any(|order| order.ask_type == RequestType::MarkerTrade)
            {
                return ContractError::InvalidRequest {
                    message: format!("marker share sales cannot be created alongside marker trades for marker with address [{}]", marker.address.as_str()),
                }.to_err();
            }
            // Only revoke permissions from the marker if this is the first ask for this marker.
            // Additional asks do not need to revoke permissions because the permissions have already
            // been revoked by the initial ask that sent the marker into the contract's control.
            if existing_related_orders.is_empty() {
                get_marker_permission_revoke_messages(&marker, &env.contract.address)?
            } else {
                vec![]
            }
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
                AskCreationType::New => {
                    // If this is the first ask order for a given marker, then the permissions can
                    // be established by examining the marker itself.
                    // However, if this is not the first marker share sale established for a singular
                    // marker, the permissions currently on the marker itself won't be the permissions
                    // that need to be returned to the owner after all trades have been matched.
                    // Due to this, they must be ported from an existing order.  Using this logic,
                    // all ask orders will include the same permissions, ensuring that the final
                    // order to be matched or cancelled will properly relinquish permissions to the
                    // original owner.
                    if existing_related_orders.is_empty() {
                        marker
                            .permissions
                            .into_iter()
                            .filter(|perm| perm.address != env.contract.address)
                            .collect::<Vec<AccessGrant>>()
                    } else {
                        get_update_marker_removed_permissions(
                            existing_related_orders.first().unwrap(),
                        )?
                    }
                }
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
        AskCreationType::New => {
            if !get_ask_orders_by_collateral_id(deps.storage, &scope_trade.scope_address).is_empty()
            {
                return ContractError::InvalidRequest {
                    message: format!(
                        "only one scope trade can exist at a time for scope [{}]",
                        &scope_trade.scope_address
                    ),
                }
                .to_err();
            }
        }
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
