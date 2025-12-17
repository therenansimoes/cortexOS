use thiserror::Error;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Storage backend error: {0}")]
    Backend(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Integrity error: {0}")]
    Integrity(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Lock poisoned")]
    LockPoisoned,

    #[error("Privacy violation: {0}")]
    PrivacyViolation(String),
}
