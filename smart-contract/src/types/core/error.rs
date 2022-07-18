use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("Cannot create [{id_type}] with id [{id}]. One with that id already exists")]
    ExistingId { id_type: String, id: String },

    #[error("{message}")]
    GenericError { message: String },

    #[error("Invalid funds provided: {message}")]
    InvalidFundsProvided { message: String },

    #[error("Invalid marker: {message}")]
    InvalidMarker { message: String },

    #[error("Invalid migration: {message}")]
    InvalidMigration { message: String },

    #[error("Scope at address [{scope_address}] has invalid owner: {explanation}")]
    InvalidScopeOwner {
        scope_address: String,
        explanation: String,
    },

    #[error("Invalid type encountered: {explanation}")]
    InvalidType { explanation: String },

    #[error("Invalid update: {explanation}")]
    InvalidUpdate { explanation: String },

    #[error("Missing field: {field:?}")]
    MissingField { field: String },

    #[error("{0}")]
    SemVerError(#[from] semver::Error),

    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Contact storage error occurred: {message}")]
    StorageError { message: String },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Validation failed with messages: {messages:?}")]
    ValidationError { messages: Vec<String> },
}
