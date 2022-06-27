use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use crate::util::provenance_utilities::{
    get_single_marker_coin_holding, marker_has_admin, marker_has_permissions,
};
use cosmwasm_std::Addr;
use provwasm_std::{Marker, MarkerAccess, MarkerStatus};

pub fn validate_marker_for_ask(
    marker: &Marker,
    original_owner_address: &Addr,
    contract_address: &Addr,
    expected_contract_permissions: &[MarkerAccess],
) -> Result<(), ContractError> {
    if !marker_has_admin(marker, original_owner_address) {
        return ContractError::invalid_marker(format!(
            "expected sender [{}] to have admin privileges on marker [{}]",
            original_owner_address.as_str(),
            marker.denom,
        ))
        .to_err();
    }
    if !marker_has_permissions(marker, contract_address, expected_contract_permissions) {
        return ContractError::invalid_marker(format!(
            "expected this contract [{}] to have privileges {:?} on marker [{}]",
            contract_address.as_str(),
            expected_contract_permissions,
            marker.denom,
        ))
        .to_err();
    }
    if marker.status != MarkerStatus::Active {
        return ContractError::invalid_marker(format!(
            "expected marker [{}] to be active, but was in status [{:?}]",
            marker.denom, marker.status,
        ))
        .to_err();
    }
    let marker_coin = get_single_marker_coin_holding(marker)?;
    if marker_coin.amount.u128() == 0 {
        return ContractError::invalid_marker(format!(
            "expected marker [{}] to hold at least one of its supply of denom, but it had [{}]",
            marker.denom,
            marker_coin.amount.u128(),
        ))
        .to_err();
    }
    ().to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::test::mock_marker::{MockMarker, DEFAULT_MARKER_DENOM};
    use crate::types::core::error::ContractError;
    use crate::validation::marker_exchange_validation::validate_marker_for_ask;
    use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
    use cosmwasm_std::Addr;
    use provwasm_std::{AccessGrant, MarkerAccess};

    #[test]
    fn test_successful_validation() {
        // Owned marker includes many permissions for the "owner" and correct Admin/Withdraw
        // permissions for the contract address
        let marker = MockMarker::new_owned_marker("asker");
        validate_marker_for_ask(
            &marker,
            &Addr::unchecked("asker"),
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
            &[MarkerAccess::Admin, MarkerAccess::Withdraw],
        )
        .expect("expected validation to pass for a valid marker ask scenario");
    }

    #[test]
    fn test_owner_missing_permissions() {
        // new_marker only includes contract permissions, which excludes the owner
        let marker = MockMarker::new_marker();
        let err = validate_marker_for_ask(
            &marker,
            &Addr::unchecked("asker"),
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
            &[MarkerAccess::Admin, MarkerAccess::Withdraw],
        )
        .expect_err("an error should occur when the owner is missing from the marker permissions");
        assert_invalid_marker_error(
            err,
            format!(
                "expected sender [asker] to have admin privileges on marker [{}]",
                DEFAULT_MARKER_DENOM
            ),
        );
    }

    #[test]
    fn test_contract_missing_permissions() {
        let marker = MockMarker {
            permissions: vec![AccessGrant {
                address: Addr::unchecked("asker"),
                permissions: vec![MarkerAccess::Admin],
            }],
            ..MockMarker::default()
        }
        .to_marker();
        let err = validate_marker_for_ask(
            &marker,
            &Addr::unchecked("asker"),
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
            &[MarkerAccess::Admin],
        )
        .expect_err(
            "expected an error to occur when the contract is not even listed on the marker",
        );
        assert_invalid_marker_error(
            err,
            format!(
                "expected this contract [{}] to have privileges {:?} on marker [{}]",
                MOCK_CONTRACT_ADDR,
                vec![MarkerAccess::Admin],
                DEFAULT_MARKER_DENOM,
            ),
        );
    }

    #[test]
    fn test_invalid_marker() {}

    fn assert_invalid_marker_error<S: Into<String>>(error: ContractError, expected_message: S) {
        match error {
            ContractError::InvalidMarker { message } => {
                assert_eq!(
                    expected_message.into(),
                    message,
                    "expected the correct invalid marker message"
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
    }
}
