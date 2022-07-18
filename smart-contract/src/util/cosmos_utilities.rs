use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{BankMsg, Coin, CosmosMsg};
use provwasm_std::ProvenanceMsg;

pub fn get_send_amount(msg: &CosmosMsg<ProvenanceMsg>) -> Result<Vec<Coin>, ContractError> {
    match msg {
        CosmosMsg::Bank(BankMsg::Send { amount, .. }) => amount.to_vec().to_ok(),
        msg => ContractError::GenericError {
            message: format!("expected CosmosMsg::Bank(BankMsg::Send) but got: {:?}", msg),
        }
        .to_err(),
    }
}

#[cfg(test)]
mod tests {
    use crate::test::mock_instantiate::DEFAULT_ADMIN_ADDRESS;
    use crate::types::core::error::ContractError;
    use crate::util::cosmos_utilities::get_send_amount;
    use cosmwasm_std::{coins, BankMsg, CosmosMsg};

    #[test]
    fn test_get_send_amount_valid_msg() {
        let amount = get_send_amount(&CosmosMsg::Bank(BankMsg::Send {
            to_address: DEFAULT_ADMIN_ADDRESS.to_string(),
            amount: coins(100, "nhash"),
        }))
        .expect("expected the function to properly recognize the message and extract the amount");
        assert_eq!(
            coins(100, "nhash"),
            amount,
            "the correct amount should be extracted from the message",
        );
    }

    #[test]
    fn test_get_send_amount_invalid_msg() {
        let err = get_send_amount(&CosmosMsg::Bank(BankMsg::Burn {
            amount: coins(100, "nhash"),
        }))
        .expect_err(
            "an error should occur when attempting to get the send amount from a non-send message",
        );
        match err {
            ContractError::GenericError { message } => {
                assert!(
                    message.contains("expected CosmosMsg::Bank(BankMsg::Send) but got"),
                    "invalid message contents. got message: {}",
                    message,
                );
            }
            e => panic!("unexpected error: {:?}", e),
        };
    }
}
