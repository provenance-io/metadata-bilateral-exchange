use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::Addr;
use provwasm_std::{Party, PartyType, Scope};

pub const DEFAULT_SCOPE_ID: &str = "scope";
pub const DEFAULT_SCOPE_SPEC_ID: &str = "scope-spec";
pub const DEFAULT_SCOPE_OWNER_ADDR: &str = MOCK_CONTRACT_ADDR;

pub struct MockScope {
    pub scope_id: String,
    pub specification_id: String,
    pub owners: Vec<Party>,
    pub data_access: Vec<Addr>,
    pub value_owner_address: Addr,
}
impl Default for MockScope {
    fn default() -> Self {
        Self {
            scope_id: DEFAULT_SCOPE_ID.to_string(),
            specification_id: DEFAULT_SCOPE_SPEC_ID.to_string(),
            owners: vec![Party {
                address: Addr::unchecked(DEFAULT_SCOPE_OWNER_ADDR),
                role: PartyType::Owner,
            }],
            data_access: vec![],
            value_owner_address: Addr::unchecked(DEFAULT_SCOPE_OWNER_ADDR),
        }
    }
}
impl MockScope {
    pub fn new_with_owner<S: Into<String>>(owner_address: S) -> Scope {
        let owner_address = owner_address.into();
        Self {
            owners: vec![Party {
                address: Addr::unchecked(&owner_address),
                role: PartyType::Owner,
            }],
            value_owner_address: Addr::unchecked(owner_address),
            ..Self::default()
        }
        .to_scope()
    }

    pub fn to_scope(self) -> Scope {
        Scope {
            scope_id: self.scope_id,
            specification_id: self.specification_id,
            owners: self.owners,
            data_access: self.data_access,
            value_owner_address: self.value_owner_address,
        }
    }
}
