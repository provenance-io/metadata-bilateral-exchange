use crate::types::request::ask_types::ask_collateral::{
    AskCollateral, CoinTradeAskCollateral, MarkerShareSaleAskCollateral, MarkerTradeAskCollateral,
    ScopeTradeAskCollateral,
};
use crate::types::request::bid_types::bid_collateral::{
    BidCollateral, CoinTradeBidCollateral, MarkerShareSaleBidCollateral, MarkerTradeBidCollateral,
    ScopeTradeBidCollateral,
};

impl AskCollateral {
    pub fn unwrap_coin_trade(&self) -> &CoinTradeAskCollateral {
        match self {
            AskCollateral::CoinTrade(collateral) => collateral,
            _ => panic!("expected coin trade ask collateral, but got {:?}", self),
        }
    }

    pub fn unwrap_marker_trade(&self) -> &MarkerTradeAskCollateral {
        match self {
            AskCollateral::MarkerTrade(collateral) => collateral,
            _ => panic!("expected marker trade ask collateral, but got {:?}", self),
        }
    }

    pub fn unwrap_marker_share_sale(&self) -> &MarkerShareSaleAskCollateral {
        match self {
            AskCollateral::MarkerShareSale(collateral) => collateral,
            _ => panic!(
                "expected marker share sale ask collateral, but got {:?}",
                self
            ),
        }
    }

    pub fn unwrap_scope_trade(&self) -> &ScopeTradeAskCollateral {
        match self {
            AskCollateral::ScopeTrade(collateral) => collateral,
            _ => panic!("expected scope trade ask collateral, but got {:?}", self),
        }
    }
}

impl BidCollateral {
    pub fn unwrap_coin_trade(&self) -> &CoinTradeBidCollateral {
        match self {
            BidCollateral::CoinTrade(collateral) => collateral,
            _ => panic!("expected coin trade bid collateral, but got {:?}", self),
        }
    }

    pub fn unwrap_marker_trade(&self) -> &MarkerTradeBidCollateral {
        match self {
            BidCollateral::MarkerTrade(collateral) => collateral,
            _ => panic!("expected marker trade bid collateral, but got {:?}", self),
        }
    }

    pub fn unwrap_marker_share_sale(&self) -> &MarkerShareSaleBidCollateral {
        match self {
            BidCollateral::MarkerShareSale(collateral) => collateral,
            _ => panic!(
                "expected marker share sale bid collateral, but got {:?}",
                self
            ),
        }
    }

    pub fn unwrap_scope_trade(&self) -> &ScopeTradeBidCollateral {
        match self {
            BidCollateral::ScopeTrade(collateral) => collateral,
            _ => panic!("expected scope trade bid collateral, but got {:?}", self),
        }
    }
}
