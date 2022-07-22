use cosmwasm_std::{Addr, Coin, Storage, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::core::error::ContractError;

// TODO: Delete contract info v1 constants and structs after a successful migration
const NAMESPACE_CONTRACT_INFO: &str = "contract_info";
const NAMESPACE_CONTRACT_INFO_V2: &str = "contract_info_v2";
pub const CONTRACT_TYPE: &str = env!("CARGO_CRATE_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new(NAMESPACE_CONTRACT_INFO);
const CONTRACT_INFO_V2: Item<ContractInfoV2> = Item::new(NAMESPACE_CONTRACT_INFO_V2);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfo {
    pub admin: Addr,
    pub bind_name: String,
    pub contract_name: String,
    pub contract_type: String,
    pub contract_version: String,
    pub ask_fee: Option<Vec<Coin>>,
    pub bid_fee: Option<Vec<Coin>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfoV2 {
    pub admin: Addr,
    pub bind_name: String,
    pub contract_name: String,
    pub contract_type: String,
    pub contract_version: String,
    pub create_ask_nhash_fee: Uint128,
    pub create_bid_nhash_fee: Uint128,
}
impl ContractInfoV2 {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        admin: Addr,
        bind_name: S1,
        contract_name: S2,
        create_ask_nhash_fee: Option<Uint128>,
        create_bid_nhash_fee: Option<Uint128>,
    ) -> Self {
        Self {
            admin,
            bind_name: bind_name.into(),
            contract_name: contract_name.into(),
            contract_type: CONTRACT_TYPE.to_string(),
            contract_version: CONTRACT_VERSION.to_string(),
            create_ask_nhash_fee: create_ask_nhash_fee.unwrap_or_else(|| Uint128::zero()),
            create_bid_nhash_fee: create_bid_nhash_fee.unwrap_or_else(|| Uint128::zero()),
        }
    }
}

pub fn set_contract_info(
    storage: &mut dyn Storage,
    contract_info: &ContractInfoV2,
) -> Result<(), ContractError> {
    CONTRACT_INFO_V2
        .save(storage, contract_info)
        .map_err(|e| ContractError::StorageError {
            message: format!("{:?}", e),
        })
}

pub fn get_contract_info(storage: &dyn Storage) -> Result<ContractInfoV2, ContractError> {
    CONTRACT_INFO_V2
        .load(storage)
        .map_err(|e| ContractError::StorageError {
            message: format!("{:?}", e),
        })
}

pub fn may_get_contract_info(store: &dyn Storage) -> Option<ContractInfoV2> {
    CONTRACT_INFO_V2.may_load(store).unwrap_or(None)
}

#[cfg(test)]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::storage::contract_info::{
        get_contract_info, may_get_contract_info, set_contract_info, ContractInfoV2, CONTRACT_TYPE,
        CONTRACT_VERSION,
    };
    use crate::test::mock_instantiate::{
        default_instantiate, DEFAULT_ADMIN_ADDRESS, DEFAULT_CONTRACT_BIND_NAME,
        DEFAULT_CONTRACT_NAME,
    };
    use cosmwasm_std::{Addr, Uint128};

    #[test]
    pub fn set_contract_info_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        let result = set_contract_info(
            &mut deps.storage,
            &ContractInfoV2::new(
                Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
                DEFAULT_CONTRACT_BIND_NAME.to_string(),
                DEFAULT_CONTRACT_NAME.to_string(),
                Some(Uint128::new(100)),
                None,
            ),
        );
        match result {
            Ok(()) => {}
            result => panic!("unexpected error: {:?}", result),
        }

        let contract_info = get_contract_info(&deps.storage);
        match contract_info {
            Ok(contract_info) => {
                assert_eq!(contract_info.admin, Addr::unchecked("contract_admin"));
                assert_eq!(contract_info.bind_name, DEFAULT_CONTRACT_BIND_NAME);
                assert_eq!(contract_info.contract_name, DEFAULT_CONTRACT_NAME);
                assert_eq!(contract_info.contract_type, CONTRACT_TYPE);
                assert_eq!(contract_info.contract_version, CONTRACT_VERSION);
                assert_eq!(contract_info.create_ask_nhash_fee.u128(), 100);
                assert_eq!(contract_info.create_bid_nhash_fee.u128(), 0);
            }
            result => panic!("unexpected error: {:?}", result),
        }
    }

    #[test]
    fn test_may_get_contract_info() {
        let mut deps = mock_dependencies(&[]);
        assert!(
            may_get_contract_info(deps.as_ref().storage).is_none(),
            "contract info should not load when it has not yet been stored",
        );
        default_instantiate(deps.as_mut());
        assert!(
            may_get_contract_info(deps.as_ref().storage).is_some(),
            "contract info should be available after instantiation",
        );
    }
}
