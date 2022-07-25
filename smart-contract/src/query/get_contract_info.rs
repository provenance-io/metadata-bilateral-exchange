use crate::storage::contract_info::may_get_contract_info;
use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{to_binary, Binary, Deps};
use provwasm_std::ProvenanceQuery;

pub fn query_contract_info(deps: Deps<ProvenanceQuery>) -> Result<Binary, ContractError> {
    to_binary(&may_get_contract_info(deps.storage))?.to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::query;
    use crate::storage::contract_info::ContractInfoV2;
    use crate::test::mock_instantiate::default_instantiate;
    use crate::types::core::msg::QueryMsg;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::mock_env;
    use provwasm_mocks::mock_dependencies;

    #[test]
    pub fn query_with_valid_data() {
        // setup
        let mut deps = mock_dependencies(&[]);
        default_instantiate(deps.as_mut());

        // query for contract_inf
        let result_binary = query(deps.as_ref(), mock_env(), QueryMsg::GetContractInfo {}).expect(
            "a binary should be returned after the contract info is available from instantiation",
        );

        from_binary::<Option<ContractInfoV2>>(&result_binary)
            .expect("the optional contract info should deserialize from binary")
            .expect("the option should be populated with contract info");
    }
}
