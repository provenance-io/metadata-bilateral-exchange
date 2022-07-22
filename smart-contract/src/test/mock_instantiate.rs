use crate::instantiate::instantiate_contract::instantiate_contract;
use crate::types::core::msg::InstantiateMsg;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{DepsMut, Uint128};
use provwasm_std::ProvenanceQuery;

pub const DEFAULT_ADMIN_ADDRESS: &str = "contract_admin";
pub const DEFAULT_CONTRACT_BIND_NAME: &str = "contract_bind_name";
pub const DEFAULT_CONTRACT_NAME: &str = "contract_name";

pub struct TestInstantiate {
    pub admin_address: String,
    pub contract_bind_name: String,
    pub contract_name: String,
    pub create_ask_nhash_fee: Option<Uint128>,
    pub create_bid_nhash_fee: Option<Uint128>,
}
impl Default for TestInstantiate {
    fn default() -> Self {
        Self {
            admin_address: DEFAULT_ADMIN_ADDRESS.to_string(),
            contract_bind_name: DEFAULT_CONTRACT_BIND_NAME.to_string(),
            contract_name: DEFAULT_CONTRACT_NAME.to_string(),
            create_ask_nhash_fee: None,
            create_bid_nhash_fee: None,
        }
    }
}
impl TestInstantiate {
    pub fn to_instantiate_msg(self) -> InstantiateMsg {
        InstantiateMsg {
            bind_name: self.contract_bind_name,
            contract_name: self.contract_name,
            create_ask_nhash_fee: self.create_ask_nhash_fee,
            create_bid_nhash_fee: self.create_bid_nhash_fee,
        }
    }
}

pub fn default_instantiate(deps: DepsMut<ProvenanceQuery>) {
    test_instantiate(deps, TestInstantiate::default())
}

pub fn test_instantiate(deps: DepsMut<ProvenanceQuery>, instantiate: TestInstantiate) {
    instantiate_contract(
        deps,
        mock_env(),
        mock_info(&instantiate.admin_address, &[]),
        instantiate.to_instantiate_msg(),
    )
    .expect("expected instantiation to succeed");
}
