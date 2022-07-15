use crate::types::core::error::ContractError;
use crate::types::request::bid_types::bid::{
    Bid, CoinTradeBid, MarkerShareSaleBid, MarkerTradeBid, ScopeTradeBid,
};
use crate::types::request::bid_types::bid_collateral::BidCollateral;
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::get_single_marker_coin_holding;
use crate::util::request_fee::generate_request_fee;
use cosmwasm_std::{Coin, CosmosMsg, DepsMut, MessageInfo};
use provwasm_std::{ProvenanceMsg, ProvenanceQuerier, ProvenanceQuery};

pub enum BidCreationType {
    New,
    Update { existing_bid_order: Box<BidOrder> },
}

pub struct BidOrderCreationResponse {
    pub bid_order: BidOrder,
    pub fee_send_msg: Option<CosmosMsg<ProvenanceMsg>>,
}

pub fn create_bid_order(
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    bid: Bid,
    descriptor: Option<RequestDescriptor>,
    creation_type: BidCreationType,
) -> Result<BidOrderCreationResponse, ContractError> {
    let (fee_send_msg, funds_after_fee) = match &creation_type {
        BidCreationType::New => {
            // TODO: Refactor this after provwasm fees are available. That will deeply simplify the fee charge process
            let resp = generate_request_fee("bid fee", &deps.as_ref(), info, |c| &c.bid_fee)?;
            (resp.fee_send_msg, resp.funds_after_fee)
        }
        // Updates do not charge creation fees
        BidCreationType::Update { .. } => (None, info.funds.to_owned()),
    };
    let collateral = match &bid {
        Bid::CoinTrade(coin_trade) => {
            create_coin_trade_collateral(creation_type, &funds_after_fee, coin_trade)
        }
        Bid::MarkerTrade(marker_trade) => {
            create_marker_trade_collateral(creation_type, deps, &funds_after_fee, marker_trade)
        }
        Bid::MarkerShareSale(marker_share_sale) => create_marker_share_sale_collateral(
            creation_type,
            deps,
            &funds_after_fee,
            marker_share_sale,
        ),
        Bid::ScopeTrade(scope_trade) => {
            create_scope_trade_collateral(creation_type, &funds_after_fee, scope_trade)
        }
    }?;
    let bid_order = BidOrder::new(bid.get_id(), info.sender.clone(), collateral, descriptor)?;
    BidOrderCreationResponse {
        bid_order,
        fee_send_msg,
    }
    .to_ok()
}

fn create_coin_trade_collateral(
    creation_type: BidCreationType,
    quote_funds: &[Coin],
    coin_trade: &CoinTradeBid,
) -> Result<BidCollateral, ContractError> {
    if coin_trade.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if coin_trade.base.is_empty() {
        return ContractError::missing_field("base").to_err();
    }
    if quote_funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "coin trade bid requests should include enough funds for bid fee + quote",
        )
        .to_err();
    }
    match creation_type {
        BidCreationType::New => {}
        BidCreationType::Update { existing_bid_order } => {
            check_bid_type(
                &existing_bid_order.id,
                &existing_bid_order.bid_type,
                &RequestType::CoinTrade,
            )?;
        }
    }
    BidCollateral::coin_trade(&coin_trade.base, quote_funds).to_ok()
}

fn create_marker_trade_collateral(
    creation_type: BidCreationType,
    deps: &DepsMut<ProvenanceQuery>,
    quote_funds: &[Coin],
    marker_trade: &MarkerTradeBid,
) -> Result<BidCollateral, ContractError> {
    if marker_trade.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if marker_trade.marker_denom.is_empty() {
        return ContractError::missing_field("marker_denom").to_err();
    }
    if quote_funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "funds must be provided during a marker trade bid to pay bid fees + establish a quote",
        )
        .to_err();
    }
    match creation_type {
        BidCreationType::New => {}
        BidCreationType::Update { existing_bid_order } => {
            check_bid_type(
                &existing_bid_order.id,
                &existing_bid_order.bid_type,
                &RequestType::MarkerTrade,
            )?;
        }
    }
    // This grants us access to the marker address, as well as ensuring that the marker is real
    let marker =
        ProvenanceQuerier::new(&deps.querier).get_marker_by_denom(&marker_trade.marker_denom)?;
    BidCollateral::marker_trade(
        marker.address,
        &marker_trade.marker_denom,
        quote_funds,
        marker_trade.withdraw_shares_after_match,
    )
    .to_ok()
}

fn create_marker_share_sale_collateral(
    creation_type: BidCreationType,
    deps: &DepsMut<ProvenanceQuery>,
    quote_funds: &[Coin],
    marker_share_sale: &MarkerShareSaleBid,
) -> Result<BidCollateral, ContractError> {
    if marker_share_sale.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if marker_share_sale.marker_denom.is_empty() {
        return ContractError::missing_field("marker_denom").to_err();
    }
    if marker_share_sale.share_count.is_zero() {
        return ContractError::validation_error(&[
            "share count must be at least one for a marker share sale",
        ])
        .to_err();
    }
    if quote_funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "funds must be provided during a marker share trade bid to pay bid fees + establish a quote",
        )
            .to_err();
    }
    let marker = ProvenanceQuerier::new(&deps.querier)
        .get_marker_by_denom(&marker_share_sale.marker_denom)?;
    let marker_shares_available = get_single_marker_coin_holding(&marker)?.amount.u128();
    if marker_share_sale.share_count.u128() > marker_shares_available {
        return ContractError::validation_error(&[format!(
            "share count [{}] must be less than or equal to remaining [{}] shares available [{}]",
            marker_share_sale.share_count.u128(),
            marker_share_sale.marker_denom,
            marker_shares_available,
        )])
        .to_err();
    }
    match creation_type {
        BidCreationType::New => {}
        BidCreationType::Update { existing_bid_order } => {
            check_bid_type(
                &existing_bid_order.id,
                &existing_bid_order.bid_type,
                &RequestType::MarkerShareSale,
            )?;
        }
    }
    BidCollateral::marker_share_sale(
        marker.address,
        &marker_share_sale.marker_denom,
        marker_share_sale.share_count.u128(),
        quote_funds,
    )
    .to_ok()
}

fn create_scope_trade_collateral(
    creation_type: BidCreationType,
    quote_funds: &[Coin],
    scope_trade: &ScopeTradeBid,
) -> Result<BidCollateral, ContractError> {
    if scope_trade.id.is_empty() {
        return ContractError::missing_field("id").to_err();
    }
    if scope_trade.scope_address.is_empty() {
        return ContractError::missing_field("scope_address").to_err();
    }
    if quote_funds.is_empty() {
        return ContractError::invalid_funds_provided(
            "funds must be provided during a scope trade bid to pay bid fees + establish a quote",
        )
        .to_err();
    }
    match creation_type {
        BidCreationType::New => {}
        BidCreationType::Update { existing_bid_order } => {
            check_bid_type(
                &existing_bid_order.id,
                &existing_bid_order.bid_type,
                &RequestType::ScopeTrade,
            )?;
        }
    }
    BidCollateral::scope_trade(&scope_trade.scope_address, quote_funds).to_ok()
}

fn check_bid_type<S: Into<String>>(
    bid_id: S,
    existing_bid_type: &RequestType,
    expected_bid_type: &RequestType,
) -> Result<(), ContractError> {
    if existing_bid_type != expected_bid_type {
        ContractError::invalid_update(format!(
            "bid with id [{}] cannot change bid type from [{}] to [{}]",
            bid_id.into(),
            existing_bid_type.get_name(),
            expected_bid_type.get_name(),
        ))
        .to_err()
    } else {
        ().to_ok()
    }
}
