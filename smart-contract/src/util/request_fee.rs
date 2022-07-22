use crate::storage::contract_info::{get_contract_info, ContractInfoV2};
use crate::types::core::error::ContractError;
use crate::util::constants::NHASH;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{coin, Addr, CosmosMsg, Deps};
use provwasm_std::{assess_custom_fee, ProvenanceMsg, ProvenanceQuery};

pub fn generate_request_fee_msg<S: Into<String>, F: Fn(&ContractInfoV2) -> u128>(
    fee_type: S,
    deps: &Deps<ProvenanceQuery>,
    contract_addr: Addr,
    fee_extractor: F,
) -> Result<Option<CosmosMsg<ProvenanceMsg>>, ContractError> {
    let fee_type = fee_type.into();
    let contract_info = get_contract_info(deps.storage)?;
    let nhash_fee_amount = fee_extractor(&contract_info);
    // Only dispatch a fee message if the fee amount is greater than zero. Charging a fee of zero
    // means nothing
    if nhash_fee_amount > 0 {
        Some(assess_custom_fee(
            // Provenance Blockchain fees are required to be sent as either usd or nhash.  This
            // contract only supports nhash for now
            coin(nhash_fee_amount, NHASH),
            // Specify a somewhat descriptive message to ensure that signers using the Provenance
            // Blockchain wallet can understand the reason for the fee
            Some(format!("{} {} fee", &fee_type, NHASH)),
            // The contract's address must be used as the "from" value.  This does not mean that
            // the contract sends the fee, but it is required for the contract to sign and dispatch
            // the message that will charge the request sender the fee
            contract_addr,
            // Always send fees charged to the admin address.  This ensures that the admin is
            // always funded in order to make future requests
            Some(contract_info.admin),
        )?)
    } else {
        None
    }
    .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::test::mock_instantiate::{
        default_instantiate, test_instantiate, TestInstantiate, DEFAULT_ADMIN_ADDRESS,
    };
    use crate::util::constants::NHASH;
    use crate::util::request_fee::generate_request_fee_msg;
    use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
    use cosmwasm_std::{coin, Addr, CosmosMsg, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{MsgFeesMsgParams, ProvenanceMsg, ProvenanceMsgParams};

    #[test]
    fn test_generate_request_fee_no_fee_necessary() {
        let mut deps = mock_dependencies(&[]);
        // Default instantiate creates the contract without any fees
        default_instantiate(deps.as_mut());
        let fee_msg_option = generate_request_fee_msg(
            "no fee",
            &deps.as_ref(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            |c| c.create_bid_nhash_fee.u128(),
        )
        .expect("no fees requested should produce a valid response");
        assert!(
            fee_msg_option.is_none(),
            "no fee message should be generated when the fee amount is equal to zero",
        );
    }

    #[test]
    fn test_generate_request_fee_with_fee() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(
            deps.as_mut(),
            TestInstantiate {
                create_bid_nhash_fee: Some(Uint128::new(100)),
                ..TestInstantiate::default()
            },
        );
        let fee_msg_option = generate_request_fee_msg(
            "some fee",
            &deps.as_ref(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            |c| c.create_bid_nhash_fee.u128(),
        )
        .expect("proper fee specification should result in a fee msg");
        match fee_msg_option {
            Some(CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee {
                        amount,
                        from,
                        name,
                        recipient,
                    }),
                ..
            })) => {
                assert_eq!(
                    coin(100, NHASH),
                    amount,
                    "the correct amount of nhash should be sent",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    from.as_str(),
                    "the from address should be specified as the contract",
                );
                assert_eq!(
                    "some fee nhash fee",
                    name.expect("the message's name should be set"),
                    "the name should be set to specify that it is an nhash fee",
                );
                assert_eq!(
                    DEFAULT_ADMIN_ADDRESS,
                    recipient.expect("a recipient should be set").as_str(),
                    "the recipient should be the admin",
                );
            }
            other => panic!("unexpected response option: {:?}", other),
        };
    }
}
