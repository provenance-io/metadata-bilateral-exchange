use crate::types::core::error::ContractError;
use crate::util::extensions::ResultExtensions;
use cosmwasm_std::{coin, Addr, Coin, CosmosMsg};
use provwasm_std::{
    grant_marker_access, revoke_marker_access, AccessGrant, Marker, MarkerAccess, MsgFeesMsgParams,
    Party, PartyType, ProvenanceMsg, ProvenanceMsgParams, Scope,
};

pub fn format_coin_display(coins: &[Coin]) -> String {
    coins
        .iter()
        .map(|coin| format!("{}{}", coin.amount.u128(), coin.denom))
        .collect::<Vec<String>>()
        .join(", ")
}

pub fn marker_has_permissions(
    marker: &Marker,
    address: &Addr,
    expected_permissions: &[MarkerAccess],
) -> bool {
    marker.permissions.iter().any(|permission| {
        &permission.address == address
            && expected_permissions
                .iter()
                .all(|expected_permission| permission.permissions.contains(expected_permission))
    })
}

pub fn marker_has_admin(marker: &Marker, admin_address: &Addr) -> bool {
    marker_has_permissions(marker, admin_address, &[MarkerAccess::Admin])
}

pub fn get_single_marker_coin_holding(marker: &Marker) -> Result<Coin, ContractError> {
    let marker_denom_holdings = marker
        .coins
        .iter()
        .cloned()
        .filter(|coin| coin.denom == marker.denom)
        .collect::<Vec<Coin>>();
    if marker_denom_holdings.len() != 1 {
        return ContractError::InvalidMarker {
            message: format!(
                "expected marker [{}] to have a single coin entry for denom [{}], but it did not. Holdings: [{}]",
                marker.address.as_str(),
                marker.denom,
                format_coin_display(&marker.coins),
            )
        }.to_err();
    }
    marker_denom_holdings.first().unwrap().to_owned().to_ok()
}

pub fn calculate_marker_quote(marker_share_count: u128, quote_per_share: &[Coin]) -> Vec<Coin> {
    quote_per_share
        .iter()
        .map(|c| coin(c.amount.u128() * marker_share_count, &c.denom))
        .to_owned()
        .collect::<Vec<Coin>>()
}

pub fn release_marker_from_contract<S: Into<String>>(
    marker_denom: S,
    contract_address: &Addr,
    permissions_to_grant: &[AccessGrant],
) -> Result<Vec<CosmosMsg<ProvenanceMsg>>, ContractError> {
    let marker_denom = marker_denom.into();
    let mut messages = vec![];
    // Restore all permissions that the marker had before it was transferred to the
    // contract.
    for permission in permissions_to_grant {
        messages.push(grant_marker_access(
            &marker_denom,
            permission.address.to_owned(),
            permission.permissions.to_owned(),
        )?);
    }
    // Remove the contract's ownership of the marker now that it is no longer available for
    // sale / trade.  This message HAS TO COME LAST because the contract will lose its permission
    // to restore the originally-revoked permissions otherwise.
    messages.push(revoke_marker_access(
        &marker_denom,
        contract_address.to_owned(),
    )?);
    messages.to_ok()
}

/// Verifies that the scope is properly owned.  At minimum, checks that the scope has only a singular owner.
/// If expected_owner is provided, the single owner with party type Owner must match this address.
/// If expected_value_owner is provided, the value_owner_address value must match this.
pub fn check_scope_owners(
    scope: &Scope,
    expected_owner: Option<&Addr>,
    expected_value_owner: Option<&Addr>,
) -> Result<(), ContractError> {
    let owners = scope
        .owners
        .iter()
        .filter(|owner| owner.role == PartyType::Owner)
        .collect::<Vec<&Party>>();
    // if more than one owner is specified, removing all of them can potentially cause data loss
    if owners.len() != 1 {
        return ContractError::InvalidScopeOwner {
            scope_address: scope.scope_id.to_owned(),
            explanation: format!(
                "the scope should only include a single owner, but found: [{}]",
                owners
                    .iter()
                    .map(|owner| owner.address.as_str())
                    .collect::<Vec<&str>>()
                    .join(", "),
            ),
        }
        .to_err();
    }
    if let Some(expected) = expected_owner {
        let owner = owners.first().unwrap();
        if &owner.address != expected {
            return ContractError::InvalidScopeOwner {
                scope_address: scope.scope_id.to_owned(),
                explanation: format!(
                    "the scope owner was expected to be [{}], not [{}]",
                    expected,
                    owner.address.as_str(),
                ),
            }
            .to_err();
        }
    }
    if let Some(expected) = expected_value_owner {
        if &scope.value_owner_address != expected {
            return ContractError::InvalidScopeOwner {
                scope_address: scope.scope_id.to_owned(),
                explanation: format!(
                    "the scope's value owner was expected to be [{}], not [{}]",
                    expected,
                    scope.value_owner_address.as_str(),
                ),
            }
            .to_err();
        }
    }
    ().to_ok()
}

/// Switches the scope's current owner value to the given owner value.
pub fn replace_scope_owner(mut scope: Scope, new_owner: Addr) -> Scope {
    // Empty out all owners from the scope now that it's verified safe to do
    scope.owners.retain(|owner| owner.role != PartyType::Owner);
    // Append the target value as the new sole owner
    scope.owners.push(Party {
        address: new_owner.clone(),
        role: PartyType::Owner,
    });
    // Swap over the value owner, ensuring that the target owner not only is listed as an owner,
    // but has full access control over the scope
    scope.value_owner_address = new_owner;
    scope
}

pub fn get_custom_fee_amount_display(
    msg: &CosmosMsg<ProvenanceMsg>,
) -> Result<String, ContractError> {
    match msg {
        CosmosMsg::Custom(ProvenanceMsg {
            params: ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee { amount, .. }),
            ..
        }) => format!("{}{}", amount.amount.u128(), &amount.denom).to_ok(),
        msg => ContractError::GenericError {
            message: format!(
                "expected MsgFees AssessCustomFee provenance msg but got: {:?}",
                msg
            ),
        }
        .to_err(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::mock_instantiate::DEFAULT_ADMIN_ADDRESS;
    use crate::test::mock_marker::MockMarker;
    use crate::test::mock_scope::MockScope;
    use crate::util::constants::NHASH;
    use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
    use cosmwasm_std::{coins, BankMsg};
    use provwasm_std::{assess_custom_fee, MarkerMsgParams, ProvenanceMsgParams};

    #[test]
    fn test_format_coin_display() {
        assert_eq!(
            "",
            format_coin_display(&[]),
            "empty display should produce an empty string",
        );
        assert_eq!(
            "150nhash",
            format_coin_display(&coins(150, NHASH)),
            "single coin display should produce a simple result",
        );
        assert_eq!(
            "12acoin, 13bcoin, 14ccoin",
            format_coin_display(&[coin(12, "acoin"), coin(13, "bcoin"), coin(14, "ccoin")]),
            "multiple coin display should produce a space-including csv result",
        );
    }

    #[test]
    fn test_marker_has_permissions() {
        let target_address = Addr::unchecked("target_address");
        let marker = MockMarker {
            permissions: vec![AccessGrant {
                address: target_address.clone(),
                permissions: vec![
                    MarkerAccess::Admin,
                    MarkerAccess::Mint,
                    MarkerAccess::Delete,
                ],
            }],
            ..MockMarker::default()
        }
        .to_marker();
        assert!(
            marker_has_permissions(&marker, &target_address, &[]),
            "no permissions passed in with an existing address on the marker should produce a true response",
        );
        assert!(
            marker_has_permissions(&marker, &target_address, &[MarkerAccess::Admin]),
            "single target permission for correct address should produce a true response",
        );
        assert!(
            marker_has_permissions(&marker, &target_address, &[MarkerAccess::Admin, MarkerAccess::Mint, MarkerAccess::Delete]),
            "multiple target with all values present for correct address should produce a true response",
        );
        assert!(
            !marker_has_permissions(&marker, &Addr::unchecked("not the same address"), &[]),
            "no permissions passed in with an address not found in the marker should produce a false response",
        );
        assert!(
            !marker_has_permissions(&marker, &Addr::unchecked("not the same address"), &[MarkerAccess::Admin]),
            "single target permission for address not in marker permissions should produce a false response",
        );
        assert!(
            !marker_has_permissions(
                &marker,
                &Addr::unchecked("not the same address"),
                &[
                    MarkerAccess::Admin,
                    MarkerAccess::Mint,
                    MarkerAccess::Delete
                ]
            ),
            "multiple target with bad target address should produce a false response",
        );
    }

    #[test]
    fn test_marker_has_admin() {
        let admin1 = Addr::unchecked("admin1");
        let admin2 = Addr::unchecked("admin2");
        let normie = Addr::unchecked("normie2");
        let missing = Addr::unchecked("missing");
        let marker = MockMarker {
            permissions: vec![
                AccessGrant {
                    address: admin1.clone(),
                    permissions: vec![MarkerAccess::Admin],
                },
                AccessGrant {
                    address: admin2.clone(),
                    permissions: vec![
                        MarkerAccess::Admin,
                        MarkerAccess::Mint,
                        MarkerAccess::Burn,
                        MarkerAccess::Deposit,
                        MarkerAccess::Transfer,
                        MarkerAccess::Delete,
                    ],
                },
                AccessGrant {
                    address: normie.clone(),
                    permissions: vec![MarkerAccess::Withdraw, MarkerAccess::Deposit],
                },
            ],
            ..MockMarker::default()
        }
        .to_marker();
        assert!(
            marker_has_admin(&marker, &admin1),
            "the first admin with ONLY admin access type should produce a true response",
        );
        assert!(
            marker_has_admin(&marker, &admin2),
            "the second admin with many access types should produce a true response",
        );
        assert!(
            !marker_has_admin(&marker, &normie),
            "the account without admin access should produce a false response",
        );
        assert!(
            !marker_has_admin(&marker, &missing),
            "the account not present in the marker permissions should produce a false response",
        );
    }

    #[test]
    fn test_get_single_marker_coin_holding() {
        let no_denom_marker = MockMarker {
            address: Addr::unchecked("nodenomaddr"),
            denom: "nodenom".to_string(),
            coins: vec![],
            ..MockMarker::default()
        }
        .to_marker();
        match get_single_marker_coin_holding(&no_denom_marker)
            .expect_err("expected an error to occur when a marker had none of its own coin")
        {
            ContractError::InvalidMarker { message } => {
                assert_eq!(
                    message,
                    "expected marker [nodenomaddr] to have a single coin entry for denom [nodenom], but it did not. Holdings: []", 
                    "unexpected error message",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let invalid_coin_marker = MockMarker {
            address: Addr::unchecked("badcoinaddr"),
            denom: "badcoin".to_string(),
            coins: vec![coin(100, "othercoin"), coin(15, "moredifferentcoin")],
            ..MockMarker::default()
        }
        .to_marker();
        match get_single_marker_coin_holding(&invalid_coin_marker).expect_err(
            "expected an error to occur when a marker had other coins, but none of its own",
        ) {
            ContractError::InvalidMarker { message } => {
                assert_eq!(
                    message,
                    "expected marker [badcoinaddr] to have a single coin entry for denom [badcoin], but it did not. Holdings: [100othercoin, 15moredifferentcoin]",
                    "unexpected error message",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        }
        let duplicate_coin_marker = MockMarker {
            address: Addr::unchecked("weirdaddr"),
            denom: "weird".to_string(),
            coins: vec![coin(12, "weird"), coin(15, "weird")],
            ..MockMarker::default()
        }
        .to_marker();
        match get_single_marker_coin_holding(&duplicate_coin_marker).expect_err(
            "expected an error to occur when a marker had more than one entry for its own denom",
        ) {
            ContractError::InvalidMarker { message } => {
                assert_eq!(
                    message,
                    "expected marker [weirdaddr] to have a single coin entry for denom [weird], but it did not. Holdings: [12weird, 15weird]",
                    "unexpected error message",
                );
            }
            e => panic!("unexpected error encountered: {:?}", e),
        };
        let mut good_marker = MockMarker {
            address: Addr::unchecked("goodaddr"),
            denom: "good".to_string(),
            coins: vec![coin(150, "good")],
            ..MockMarker::default()
        }
        .to_marker();
        let marker_coin = get_single_marker_coin_holding(&good_marker).expect(
            "expected a marker containing a single entry of its denom to produce a coin response",
        );
        assert_eq!(
            150,
            marker_coin.amount.u128(),
            "expected the coin's amount to be unaltered",
        );
        assert_eq!(
            "good", marker_coin.denom,
            "expected the coin's denom to be unaltered",
        );
        good_marker.coins = vec![marker_coin.clone(), coin(10, "bitcoin"), coin(15, NHASH)];
        let extra_holdings_coin = get_single_marker_coin_holding(&good_marker).expect("expected a marker containing a single entry of its own denom and some other holdings to produce a coin response");
        assert_eq!(
            marker_coin, extra_holdings_coin,
            "the same coin should be produced in similar good scenarios",
        );
    }

    #[test]
    fn test_release_marker_from_contract_produces_correct_output() {
        let messages = release_marker_from_contract(
            "testdenom",
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
            &[
                AccessGrant {
                    address: Addr::unchecked("asker"),
                    permissions: vec![MarkerAccess::Admin, MarkerAccess::Burn],
                },
                AccessGrant {
                    address: Addr::unchecked("innocent_bystander"),
                    permissions: vec![MarkerAccess::Withdraw, MarkerAccess::Transfer],
                },
            ],
        )
        .expect("expected a result to be returned for good input");
        assert_eq!(
            3,
            messages.len(),
            "the correct number of messages should be produced",
        );
        messages.into_iter().for_each(|msg| match msg {
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::RevokeMarkerAccess { denom, address }), .. }) => {
                assert_eq!(
                    "testdenom",
                    denom,
                    "the revocation message should refer to the marker denom",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    address.as_str(),
                    "the target address for revocation should always be the contract's address",
                );
            },
            CosmosMsg::Custom(ProvenanceMsg { params: ProvenanceMsgParams::Marker(MarkerMsgParams::GrantMarkerAccess { denom, address, permissions }), .. }) => {
                assert_eq!(
                    "testdenom",
                    denom,
                    "each grant message should refer to the marker's denom",
                );
                match address.as_str() {
                    "asker" => {
                        assert_eq!(
                            vec![MarkerAccess::Admin, MarkerAccess::Burn],
                            permissions,
                            "expected the asker's permissions to be granted",
                        );
                    },
                    "innocent_bystander" => {
                        assert_eq!(
                            vec![MarkerAccess::Withdraw, MarkerAccess::Transfer],
                            permissions,
                            "expected the bystander's permissions to be granted",
                        );
                    },
                    addr => panic!("unexpected address encountered in access grants: {}", addr),
                };
            },
            msg => panic!("unexpected message produced: {:?}", msg),
        });
    }

    #[test]
    fn test_check_scope_owners_incorrect_owner_count() {
        let mut scope = MockScope {
            owners: vec![
                Party {
                    address: Addr::unchecked("owner1"),
                    role: PartyType::Owner,
                },
                Party {
                    address: Addr::unchecked("owner2"),
                    role: PartyType::Owner,
                },
            ],
            ..MockScope::default()
        }
        .to_scope();
        let err = check_scope_owners(&scope, None, None)
            .expect_err("expected an error to occur when the scope included multiple owners");
        match err {
            ContractError::InvalidScopeOwner {
                scope_address,
                explanation,
            } => {
                assert_eq!(
                    scope_address, scope.scope_id,
                    "expected the scope address to be properly included in the error",
                );
                assert_eq!(
                    "the scope should only include a single owner, but found: [owner1, owner2]",
                    explanation,
                    "unexpected error explanation"
                );
            }
            e => panic!("unexpected error type encountered: {:?}", e),
        };
        scope.owners = vec![];
        let err = check_scope_owners(&scope, None, None)
            .expect_err("expected an error to occur when the scope did not include any owners");
        match err {
            ContractError::InvalidScopeOwner {
                scope_address,
                explanation,
            } => {
                assert_eq!(
                    scope_address, scope.scope_id,
                    "expected the scope address to be properly included in the error",
                );
                assert_eq!(
                    "the scope should only include a single owner, but found: []", explanation,
                    "unexpected error explanation",
                );
            }
            e => panic!("unexpected error type encountered: {:?}", e),
        };
        scope.owners = vec![Party {
            address: Addr::unchecked("single_owner"),
            role: PartyType::Owner,
        }];
        check_scope_owners(&scope, None, None)
            .expect("expected the check to pass when a single owner is provided");
    }

    #[test]
    fn test_check_scope_owners_invalid_owner_found() {
        let scope = MockScope::new_with_owner("owner1");
        let err = check_scope_owners(&scope, Some(&Addr::unchecked("owner2")), None)
            .expect_err("expected an error to occur when an incorrect owner is found");
        match err {
            ContractError::InvalidScopeOwner {
                scope_address,
                explanation,
            } => {
                assert_eq!(
                    scope_address, scope.scope_id,
                    "expected the scope address to be properly included in the error",
                );
                assert_eq!(
                    "the scope owner was expected to be [owner2], not [owner1]", explanation,
                    "unexpected error explanation",
                );
            }
            e => panic!("unexpected error type encountered: {:?}", e),
        }
        check_scope_owners(&scope, Some(&Addr::unchecked("owner1")), None)
            .expect("expected the check to pass when the correct target owner is used");
    }

    #[test]
    fn test_check_scope_owners_invalid_value_owner_found() {
        let scope = MockScope::new_with_owner("goodowner");
        let err = check_scope_owners(&scope, None, Some(&Addr::unchecked("badowner")))
            .expect_err("expected an error to occur when an incorrect value owner is found");
        match err {
            ContractError::InvalidScopeOwner {
                scope_address,
                explanation,
            } => {
                assert_eq!(
                    scope_address, scope.scope_id,
                    "expected the scope address to be properly included in the error",
                );
                assert_eq!(
                    "the scope's value owner was expected to be [badowner], not [goodowner]",
                    explanation,
                    "unexpected error explanation",
                );
            }
            e => panic!("unexpected error type encountered: {:?}", e),
        }
    }

    #[test]
    fn test_replace_scope_owner() {
        let mut scope = MockScope {
            owners: vec![
                Party {
                    address: Addr::unchecked("some_owner"),
                    role: PartyType::Owner,
                },
                Party {
                    address: Addr::unchecked("other_entity"),
                    role: PartyType::Affiliate,
                },
            ],
            value_owner_address: Addr::unchecked("some_owner"),
            ..MockScope::default()
        }
        .to_scope();
        scope = replace_scope_owner(scope, Addr::unchecked("new_owner"));
        assert_eq!(
            2,
            scope.owners.len(),
            "scope should include two owner values",
        );
        let scope_owner_role_owners = scope
            .owners
            .iter()
            .filter(|owner| owner.role == PartyType::Owner)
            .cloned()
            .collect::<Vec<Party>>();
        assert_eq!(
            1,
            scope_owner_role_owners.len(),
            "only one owner should have a role of Owner",
        );
        assert_eq!(
            "new_owner",
            scope_owner_role_owners.first().unwrap().address.as_str(),
            "the owner address should be changed to the new owner value",
        );
        assert_eq!(
            "new_owner", scope.value_owner_address,
            "the value owner address should also be changed to the new, correct value",
        );
    }

    #[test]
    fn test_get_custom_fee_amount_display() {
        let invalid_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
            to_address: "some_person".to_string(),
            amount: coins(100, NHASH),
        });
        let err = get_custom_fee_amount_display(&invalid_msg)
            .expect_err("when a non custom fees msg is used, an error should be produced");
        assert!(
            matches!(err, ContractError::GenericError { .. }),
            "a generic error should be returned when the wrong msg type is used, but got: {:?}",
            err,
        );
        let valid_msg: CosmosMsg<ProvenanceMsg> = assess_custom_fee(
            coin(250, NHASH),
            Some("some_person"),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Some(Addr::unchecked(DEFAULT_ADMIN_ADDRESS)),
        )
        .expect("expected custom fee msg to be created with issue");
        let display_string = get_custom_fee_amount_display(&valid_msg)
            .expect("a display string should be produced from a fee msg");
        assert_eq!(
            "250nhash", display_string,
            "the display string should be properly formatted",
        );
    }
}
