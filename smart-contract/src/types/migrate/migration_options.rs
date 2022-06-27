use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrationOptions {
    pub new_admin_address: Option<String>,
}
impl MigrationOptions {
    pub fn has_changes(&self) -> bool {
        self.new_admin_address.is_some()
    }
}

#[cfg(test)]
mod tests {
    use crate::types::migrate::migration_options::MigrationOptions;

    #[test]
    fn test_has_changes() {
        let no_changes = MigrationOptions {
            new_admin_address: None,
        };
        assert!(
            !no_changes.has_changes(),
            "no populated values should equate to no changes",
        );
        let changes = MigrationOptions {
            new_admin_address: Some("best-admin".to_string()),
        };
        assert!(
            changes.has_changes(),
            "a populated new admin address should trigger has_changes",
        );
    }
}
