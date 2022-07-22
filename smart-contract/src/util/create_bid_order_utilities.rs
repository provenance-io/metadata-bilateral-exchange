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
use crate::util::request_fee::generate_request_fee_msg;
use crate::validation::bid_order_validation::validate_bid_order;
use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo};
use provwasm_std::{ProvenanceMsg, ProvenanceQuerier, ProvenanceQuery};

pub enum BidCreationType {
    New,
    Update,
}

pub struct BidOrderCreationResponse {
    pub bid_order: BidOrder,
    pub bid_fee_msg: Option<CosmosMsg<ProvenanceMsg>>,
}

pub fn create_bid_order(
    deps: &DepsMut<ProvenanceQuery>,
    env: &Env,
    info: &MessageInfo,
    bid: Bid,
    descriptor: Option<RequestDescriptor>,
    creation_type: BidCreationType,
) -> Result<BidOrderCreationResponse, ContractError> {
    let bid_fee_msg = match &creation_type {
        BidCreationType::New => generate_request_fee_msg(
            "bid creation",
            &deps.as_ref(),
            env.contract.address.clone(),
            |c| c.create_bid_nhash_fee.u128(),
        )?,
        // Updates do not charge creation fees
        BidCreationType::Update => None,
    };
    let collateral = match &bid {
        Bid::CoinTrade(coin_trade) => create_coin_trade_collateral(info, coin_trade),
        Bid::MarkerTrade(marker_trade) => create_marker_trade_collateral(deps, info, marker_trade),
        Bid::MarkerShareSale(marker_share_sale) => {
            create_marker_share_sale_collateral(deps, info, marker_share_sale)
        }
        Bid::ScopeTrade(scope_trade) => create_scope_trade_collateral(info, scope_trade),
    }?;
    let bid_order = BidOrder {
        id: bid.get_id().to_string(),
        bid_type: RequestType::from_bid_collateral(&collateral),
        owner: info.sender.clone(),
        collateral,
        descriptor,
    };
    validate_bid_order(&bid_order)?;
    BidOrderCreationResponse {
        bid_order,
        bid_fee_msg,
    }
    .to_ok()
}

fn create_coin_trade_collateral(
    info: &MessageInfo,
    coin_trade: &CoinTradeBid,
) -> Result<BidCollateral, ContractError> {
    if coin_trade.id.is_empty() {
        return ContractError::MissingField {
            field: "id".to_string(),
        }
        .to_err();
    }
    if coin_trade.base.is_empty() {
        return ContractError::MissingField {
            field: "base".to_string(),
        }
        .to_err();
    }
    if info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "coin trade bid requests should include enough funds for a quote".to_string(),
        }
        .to_err();
    }
    BidCollateral::coin_trade(&coin_trade.base, &info.funds).to_ok()
}

fn create_marker_trade_collateral(
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    marker_trade: &MarkerTradeBid,
) -> Result<BidCollateral, ContractError> {
    if marker_trade.id.is_empty() {
        return ContractError::MissingField {
            field: "id".to_string(),
        }
        .to_err();
    }
    if marker_trade.marker_denom.is_empty() {
        return ContractError::MissingField {
            field: "marker_denom".to_string(),
        }
        .to_err();
    }
    if info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "funds must be provided during a marker trade bid to establish a quote"
                .to_string(),
        }
        .to_err();
    }
    // This grants us access to the marker address, as well as ensuring that the marker is real
    let marker =
        ProvenanceQuerier::new(&deps.querier).get_marker_by_denom(&marker_trade.marker_denom)?;
    BidCollateral::marker_trade(
        marker.address,
        &marker_trade.marker_denom,
        &info.funds,
        marker_trade.withdraw_shares_after_match,
    )
    .to_ok()
}

fn create_marker_share_sale_collateral(
    deps: &DepsMut<ProvenanceQuery>,
    info: &MessageInfo,
    marker_share_sale: &MarkerShareSaleBid,
) -> Result<BidCollateral, ContractError> {
    if marker_share_sale.id.is_empty() {
        return ContractError::MissingField {
            field: "id".to_string(),
        }
        .to_err();
    }
    if marker_share_sale.marker_denom.is_empty() {
        return ContractError::MissingField {
            field: "marker_denom".to_string(),
        }
        .to_err();
    }
    if marker_share_sale.share_count.is_zero() {
        return ContractError::ValidationError {
            messages: vec!["share count must be at least one for a marker share sale".to_string()],
        }
        .to_err();
    }
    if info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "funds must be provided during a marker share trade bid to establish a quote"
                .to_string(),
        }
        .to_err();
    }
    let marker = ProvenanceQuerier::new(&deps.querier)
        .get_marker_by_denom(&marker_share_sale.marker_denom)?;
    let marker_shares_available = get_single_marker_coin_holding(&marker)?.amount.u128();
    if marker_share_sale.share_count.u128() > marker_shares_available {
        return ContractError::ValidationError {
            messages: vec![format!(
            "share count [{}] must be less than or equal to remaining [{}] shares available [{}]",
            marker_share_sale.share_count.u128(),
            marker_share_sale.marker_denom,
            marker_shares_available,
        )],
        }
        .to_err();
    }
    BidCollateral::marker_share_sale(
        marker.address,
        &marker_share_sale.marker_denom,
        marker_share_sale.share_count.u128(),
        &info.funds,
    )
    .to_ok()
}

fn create_scope_trade_collateral(
    info: &MessageInfo,
    scope_trade: &ScopeTradeBid,
) -> Result<BidCollateral, ContractError> {
    if scope_trade.id.is_empty() {
        return ContractError::MissingField {
            field: "id".to_string(),
        }
        .to_err();
    }
    if scope_trade.scope_address.is_empty() {
        return ContractError::MissingField {
            field: "scope_address".to_string(),
        }
        .to_err();
    }
    if info.funds.is_empty() {
        return ContractError::InvalidFundsProvided {
            message: "funds must be provided during a scope trade bid to establish a quote"
                .to_string(),
        }
        .to_err();
    }
    BidCollateral::scope_trade(&scope_trade.scope_address, &info.funds).to_ok()
}
