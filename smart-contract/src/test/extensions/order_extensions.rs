use crate::types::core::error::ContractError;
use crate::types::request::ask_types::ask_collateral::AskCollateral;
use crate::types::request::ask_types::ask_order::AskOrder;
use crate::types::request::bid_types::bid_collateral::BidCollateral;
use crate::types::request::bid_types::bid_order::BidOrder;
use crate::types::request::request_descriptor::RequestDescriptor;
use crate::types::request::request_type::RequestType;
use crate::util::extensions::ResultExtensions;
use crate::validation::ask_order_validation::validate_ask_order;
use crate::validation::bid_order_validation::validate_bid_order;
use cosmwasm_std::Addr;

impl AskOrder {
    pub fn new<S: Into<String>>(
        id: S,
        owner: Addr,
        collateral: AskCollateral,
        descriptor: Option<RequestDescriptor>,
    ) -> Result<Self, ContractError> {
        let ask_order = Self::new_unchecked(id, owner, collateral, descriptor);
        validate_ask_order(&ask_order)?;
        ask_order.to_ok()
    }

    pub fn new_unchecked<S: Into<String>>(
        id: S,
        owner: Addr,
        collateral: AskCollateral,
        descriptor: Option<RequestDescriptor>,
    ) -> Self {
        Self {
            id: id.into(),
            ask_type: RequestType::from_ask_collateral(&collateral),
            owner,
            collateral,
            descriptor,
        }
    }
}

impl BidOrder {
    pub fn new<S: Into<String>>(
        id: S,
        owner: Addr,
        collateral: BidCollateral,
        descriptor: Option<RequestDescriptor>,
    ) -> Result<Self, ContractError> {
        let bid_order = Self::new_unchecked(id, owner, collateral, descriptor);
        validate_bid_order(&bid_order)?;
        bid_order.to_ok()
    }

    pub fn new_unchecked<S: Into<String>>(
        id: S,
        owner: Addr,
        collateral: BidCollateral,
        descriptor: Option<RequestDescriptor>,
    ) -> Self {
        Self {
            id: id.into(),
            bid_type: RequestType::from_bid_collateral(&collateral),
            owner,
            collateral,
            descriptor,
        }
    }
}
