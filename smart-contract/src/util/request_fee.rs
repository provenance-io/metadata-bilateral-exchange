use crate::storage::contract_info::{get_contract_info, ContractInfo};
use crate::types::core::error::ContractError;
use crate::util::coin_utilities::funds_minus_fees;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{BankMsg, Coin, CosmosMsg, Deps, MessageInfo};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

#[derive(Debug)]
pub struct RequestFeeDetail {
    pub fee_send_msg: Option<CosmosMsg<ProvenanceMsg>>,
    pub funds_after_fee: Vec<Coin>,
}

pub fn generate_request_fee<S: Into<String>, F: Fn(&ContractInfo) -> &Option<Vec<Coin>>>(
    fee_type: S,
    deps: &Deps<ProvenanceQuery>,
    message_info: &MessageInfo,
    fee_extractor: F,
) -> Result<RequestFeeDetail, ContractError> {
    let contract_info = get_contract_info(deps.storage)?;
    let fee_amount = fee_extractor(&contract_info);
    match fee_amount {
        Some(fee) => RequestFeeDetail {
            fee_send_msg: Some(CosmosMsg::Bank(BankMsg::Send {
                to_address: contract_info.admin.to_string(),
                amount: fee.clone(),
            })),
            funds_after_fee: funds_minus_fees(
                format!("{} calculation", fee_type.into()),
                &message_info.funds,
                fee,
            )?,
        },
        None => RequestFeeDetail {
            fee_send_msg: None,
            funds_after_fee: message_info.funds.clone(),
        },
    }
    .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::test::mock_instantiate::{
        default_instantiate, test_instantiate, TestInstantiate, DEFAULT_ADMIN_ADDRESS,
    };
    use crate::types::core::error::ContractError;
    use crate::util::request_fee::generate_request_fee;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{coin, coins, BankMsg, CosmosMsg};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_generate_request_fee_invalid_coin_difference() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(
            deps.as_mut().storage,
            TestInstantiate {
                ask_fee: Some(coins(100, "nhash")),
                ..TestInstantiate::default()
            },
        );
        let err = generate_request_fee(
            "some fee",
            &deps.as_ref(),
            &mock_info("some asker", &coins(99, "nhash")),
            |c| &c.ask_fee,
        )
        .expect_err("an error should occur when the coin difference is impossible to calculate");
        match err {
            ContractError::GenericError { message } => {
                assert_eq!(
                    "some fee calculation: expected at least [100nhash] to be provided in funds. funds: [99nhash], fees: [100nhash]",
                    message,
                    "unexpected message produced",
                );
            }
            e => panic!("unexpected error produced: {:?}", e),
        };
    }

    #[test]
    fn test_generate_request_fee_no_fee_necessary() {
        let mut deps = mock_dependencies(&[]);
        // Default instantiate creates the contract without any fees
        default_instantiate(deps.as_mut().storage);
        let fee_detail = generate_request_fee(
            "no fee",
            &deps.as_ref(),
            &mock_info("some bidder", &coins(1000, "biddercoin")),
            |c| &c.bid_fee,
        )
        .expect("no fees requested should produce a valid response");
        assert_eq!(
            None, fee_detail.fee_send_msg,
            "no fee message should be generated because no fees were charged",
        );
        assert_eq!(
            coins(1000, "biddercoin"),
            fee_detail.funds_after_fee,
            "all the bidder's coin should remain",
        );
    }

    #[test]
    fn test_generate_request_fee_with_fee() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(
            deps.as_mut().storage,
            TestInstantiate {
                bid_fee: Some(coins(100, "feecoin")),
                ..TestInstantiate::default()
            },
        );
        let fee_detail = generate_request_fee(
            "some fee",
            &deps.as_ref(),
            &mock_info("bidder", &[coin(100, "feecoin"), coin(100, "quote_1")]),
            |c| &c.bid_fee,
        )
        .expect("proper fee specification should result in a fee detail");
        match &fee_detail.fee_send_msg {
            Some(CosmosMsg::Bank(BankMsg::Send { to_address, amount })) => {
                assert_eq!(
                    DEFAULT_ADMIN_ADDRESS, to_address,
                    "the fee should always be sent to the admin",
                );
                assert_eq!(
                    &coins(100, "feecoin"),
                    amount,
                    "the correct fee amount should be sent to the admin",
                );
            }
            Some(msg) => panic!("unexpected message generated: {:?}", msg),
            None => panic!("expected a fee message to be generated from valid input"),
        };
        assert_eq!(
            coins(100, "quote_1"),
            fee_detail.funds_after_fee,
            "the quote funds sent by the bidder should remain after extracting the fee",
        );
    }
}
