use cosmwasm_std::{Addr, Coin, StdResult, Storage};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::core::error::ContractError;

const NAMESPACE_CONTRACT_INFO: &str = "contract_info";
pub const CONTRACT_TYPE: &str = env!("CARGO_CRATE_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const CONTRACT_INFO: Item<ContractInfo> = Item::new(NAMESPACE_CONTRACT_INFO);

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

impl ContractInfo {
    pub fn new(
        admin: Addr,
        bind_name: String,
        contract_name: String,
        ask_fee: Option<Vec<Coin>>,
        bid_fee: Option<Vec<Coin>>,
    ) -> ContractInfo {
        ContractInfo {
            admin,
            bind_name,
            contract_name,
            contract_type: CONTRACT_TYPE.into(),
            contract_version: CONTRACT_VERSION.into(),
            ask_fee,
            bid_fee,
        }
    }
}

pub fn set_contract_info(
    store: &mut dyn Storage,
    contract_info: &ContractInfo,
) -> Result<(), ContractError> {
    let result = CONTRACT_INFO.save(store, contract_info);
    result.map_err(ContractError::Std)
}

pub fn get_contract_info(store: &dyn Storage) -> StdResult<ContractInfo> {
    CONTRACT_INFO.load(store)
}

pub fn may_get_contract_info(store: &dyn Storage) -> Option<ContractInfo> {
    CONTRACT_INFO.may_load(store).unwrap_or(None)
}

#[cfg(test)]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::storage::contract_info::{
        get_contract_info, may_get_contract_info, set_contract_info, ContractInfo, CONTRACT_TYPE,
        CONTRACT_VERSION,
    };
    use crate::test::mock_instantiate::default_instantiate;
    use cosmwasm_std::{coins, Addr};

    #[test]
    pub fn set_contract_info_with_valid_data() {
        let mut deps = mock_dependencies(&[]);
        let result = set_contract_info(
            &mut deps.storage,
            &ContractInfo::new(
                Addr::unchecked("contract_admin"),
                "contract_bind_name".into(),
                "contract_name".into(),
                Some(coins(100, "nhash")),
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
                assert_eq!(contract_info.bind_name, "contract_bind_name");
                assert_eq!(contract_info.contract_name, "contract_name");
                assert_eq!(contract_info.contract_type, CONTRACT_TYPE);
                assert_eq!(contract_info.contract_version, CONTRACT_VERSION);
                assert_eq!(contract_info.ask_fee, Some(coins(100, "nhash")));
                assert_eq!(contract_info.bid_fee, None);
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
        default_instantiate(deps.as_mut().storage);
        assert!(
            may_get_contract_info(deps.as_ref().storage).is_some(),
            "contract info should be available after instantiation",
        );
    }
}
