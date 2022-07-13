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
use crate::util::request_fee::generate_request_fee;
use crate::validation::marker_exchange_validation::validate_marker_for_ask;
use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo};
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
    let (fee_send_msg, funds_after_fee) = match &creation_type {
        AskCreationType::New => {
            // TODO: Refactor this after provwasm fees are available. That will deeply simplify the fee charge process
            let resp = generate_request_fee("ask fee", &deps.as_ref(), info, |c| &c.ask_fee)?;
            (resp.fee_send_msg, resp.funds_after_fee)
        }
        AskCreationType::Update { .. } => (None, info.funds.to_owned()),
    };
    let AskCreationData {
        collateral,
        messages,
    } = match &ask {
        Ask::CoinTrade(coin_ask) => {
            create_coin_trade_ask_collateral(creation_type, &info, &funds_after_fee, coin_ask)
        }
        Ask::MarkerTrade(marker_ask) => create_marker_trade_ask_collateral(
            creation_type,
            deps,
            info,
            env,
            &funds_after_fee,
            marker_ask,
        ),
        Ask::MarkerShareSale(marker_share_sale) => create_marker_share_sale_ask_collateral(
            creation_type,
            deps,
            info,
            env,
            &funds_after_fee,
            marker_share_sale,
        ),
        Ask::ScopeTrade(scope_trade) => create_scope_trade_ask_collateral(
            creation_type,
            deps,
            env,
            &funds_after_fee,
            scope_trade,
        ),
    }?;
    let ask_order = AskOrder::new(ask.get_id(), info.sender.clone(), collateral, descriptor)?;
    AskOrderCreationResponse {
        ask_order,
        messages,
        ask_fee_msg: fee_send_msg,
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
    base_funds: &[Coin],
    coin_trade: &CoinTradeAsk,
) -> Result<AskCreationData, ContractError> {
    if base_funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "coin trade ask requests should include enough funds for ask fee + base",
        )
        .to_err();
    }
    if coin_trade.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if coin_trade.quote.is_empty() {
        return ContractError::missing_field("quote").to_err();
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
        collateral: AskCollateral::coin_trade(base_funds, &coin_trade.quote),
        messages,
    }
    .to_ok()
}

fn create_marker_trade_ask_collateral(
    creation_type: AskCreationType,
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    env: &Env,
    base_funds: &[Coin],
    marker_trade: &MarkerTradeAsk,
) -> Result<AskCreationData, ContractError> {
    if !base_funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "marker trade ask requests should not include funds greater than the amount needed for ask fees",
        )
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
        &[MarkerAccess::Admin],
        None,
    )?;
    let messages = match creation_type {
        AskCreationType::New => {
            get_marker_permission_revoke_messages(&marker, &env.contract.address)?
        }
        AskCreationType::Update { existing_ask_order } => {
            check_ask_type(
                &existing_ask_order.id,
                &existing_ask_order.ask_type,
                &RequestType::MarkerTrade,
            )?;
            let existing_collateral = existing_ask_order.collateral.get_marker_trade()?;
            if existing_collateral.marker_denom != marker_trade.marker_denom {
                return ContractError::invalid_update(
                    format!(
                        "marker trade with id [{}] cannot change marker denom with an update. current denom [{}], proposed new denom [{}]", 
                        existing_ask_order.id,
                        existing_collateral.marker_denom,
                        marker_trade.marker_denom
                    )
                ).to_err();
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
            &marker
                .permissions
                .into_iter()
                .filter(|perm| perm.address != env.contract.address)
                .collect::<Vec<AccessGrant>>(),
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
    base_funds: &[Coin],
    marker_share_sale: &MarkerShareSaleAsk,
) -> Result<AskCreationData, ContractError> {
    if !base_funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "marker share sale ask requests should not include funds greater than the amount needed for ask fees",
        )
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
    let messages = match creation_type {
        AskCreationType::New => {
            get_marker_permission_revoke_messages(&marker, &env.contract.address)?
        }
        AskCreationType::Update { existing_ask_order } => {
            check_ask_type(
                &existing_ask_order.id,
                &existing_ask_order.ask_type,
                &RequestType::MarkerShareSale,
            )?;
            let existing_collateral = existing_ask_order.collateral.get_marker_share_sale()?;
            if existing_collateral.marker_denom != marker_share_sale.marker_denom {
                return ContractError::invalid_update(
                    format!(
                        "marker share sale with id [{}] cannot change marker denom with an update. current denom [{}], proposed new denom [{}]",
                        existing_ask_order.id,
                        existing_collateral.marker_denom,
                        marker_share_sale.marker_denom,
                    )
                ).to_err();
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
            &marker
                .permissions
                .into_iter()
                .filter(|perm| perm.address != env.contract.address)
                .collect::<Vec<AccessGrant>>(),
            marker_share_sale.share_sale_type.to_owned(),
        ),
        messages,
    }
    .to_ok()
}

fn create_scope_trade_ask_collateral(
    creation_type: AskCreationType,
    deps: &DepsMut<ProvenanceQuery>,
    env: &Env,
    base_funds: &[Coin],
    scope_trade: &ScopeTradeAsk,
) -> Result<AskCreationData, ContractError> {
    if !base_funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "scope trade ask requests should not include funds greater than the amount needed for ask fees",
        )
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
                return ContractError::invalid_update(
                    format!(
                        "scope trade with id [{}] cannot change scope address with an update. current address [{}], proposed new address [{}]",
                        existing_ask_order.id,
                        existing_collateral.scope_address,
                        scope_trade.scope_address,
                    )
                ).to_err();
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
    if existing_ask_type != expected_ask_type {
        ContractError::invalid_update(format!(
            "ask with id [{}] cannot change ask type from [{}] to [{}]",
            ask_id.into(),
            existing_ask_type.get_name(),
            expected_ask_type.get_name(),
        ))
        .to_err()
    } else {
        ().to_ok()
    }
}
