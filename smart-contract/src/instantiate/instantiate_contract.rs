use crate::storage::contract_info::{get_contract_info, set_contract_info, ContractInfoV2};
use crate::types::core::error::ContractError;
use crate::types::core::msg::InstantiateMsg;
use crate::util::extensions::ResultExtensions;
use crate::validation::instantiation_validation::validate_instantiate_msg;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use provwasm_std::{bind_name, NameBinding, ProvenanceMsg, ProvenanceQuery};

pub fn instantiate_contract(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    validate_instantiate_msg(&msg)?;

    // set contract info
    let contract_info = ContractInfoV2::new(
        info.sender,
        msg.bind_name,
        msg.contract_name,
        msg.create_ask_nhash_fee,
        msg.create_bid_nhash_fee,
    );
    set_contract_info(deps.storage, &contract_info)?;

    // create name binding provenance message
    let bind_name_msg = bind_name(
        contract_info.bind_name,
        env.contract.address,
        NameBinding::Restricted,
    )?;

    // build response
    Response::new()
        .add_message(bind_name_msg)
        .add_attribute(
            "contract_info",
            format!("{:?}", get_contract_info(deps.storage)?),
        )
        .add_attribute("action", "init")
        .to_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::instantiate;
    use crate::storage::contract_info::{CONTRACT_TYPE, CONTRACT_VERSION};
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, Addr, CosmosMsg, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{NameMsgParams, ProvenanceMsgParams, ProvenanceRoute};

    #[test]
    fn instantiate_with_valid_data() {
        // create valid init data
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("contract_admin", &[]);
        let init_msg = InstantiateMsg {
            bind_name: "contract_bind_name".to_string(),
            contract_name: "contract_name".to_string(),
            create_ask_nhash_fee: Some(Uint128::new(100)),
            create_bid_nhash_fee: Some(Uint128::new(200)),
        };

        // initialize
        let init_response = instantiate(deps.as_mut(), mock_env(), info, init_msg.clone());

        // verify initialize response
        match init_response {
            Ok(init_response) => {
                assert_eq!(init_response.messages.len(), 1);
                assert_eq!(
                    init_response.messages[0].msg,
                    CosmosMsg::Custom(ProvenanceMsg {
                        route: ProvenanceRoute::Name,
                        params: ProvenanceMsgParams::Name(NameMsgParams::BindName {
                            name: init_msg.bind_name,
                            address: Addr::unchecked(MOCK_CONTRACT_ADDR),
                            restrict: true
                        }),
                        version: "2.0.0".to_string(),
                    })
                );
                let expected_contract_info = ContractInfoV2 {
                    admin: Addr::unchecked("contract_admin"),
                    bind_name: "contract_bind_name".to_string(),
                    contract_name: "contract_name".to_string(),
                    contract_type: CONTRACT_TYPE.into(),
                    contract_version: CONTRACT_VERSION.into(),
                    create_ask_nhash_fee: Uint128::new(100),
                    create_bid_nhash_fee: Uint128::new(200),
                };

                assert_eq!(init_response.attributes.len(), 2);
                assert_eq!(
                    init_response.attributes[0],
                    attr("contract_info", format!("{:?}", expected_contract_info))
                );
                assert_eq!(init_response.attributes[1], attr("action", "init"));
            }
            error => panic!("failed to initialize: {:?}", error),
        }
    }

    #[test]
    fn instantiate_with_invalid_data() {
        // create invalid init data
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("contract_owner", &[]);
        let init_msg = InstantiateMsg {
            bind_name: "".to_string(),
            contract_name: "contract_name".to_string(),
            create_ask_nhash_fee: Some(Uint128::new(10)),
            create_bid_nhash_fee: Some(Uint128::new(20)),
        };

        // initialize
        let init_response = instantiate(deps.as_mut(), mock_env(), info.to_owned(), init_msg);

        // verify initialize response
        match init_response {
            Ok(_) => panic!("expected error, but init_response ok"),
            Err(error) => match error {
                ContractError::ValidationError { messages } => {
                    assert_eq!(1, messages.len(), "one message should be emitted");
                    assert_eq!(
                        "bind_name value was empty",
                        messages.first().unwrap(),
                        "incorrect validation message was emitted",
                    );
                }
                error => panic!("unexpected error: {:?}", error),
            },
        }
    }
}
