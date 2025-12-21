use thiserror::Error;

/// Errors that can occur in event storage and graph persistence operations.
///
/// These errors cover serialization, storage backend operations, data integrity,
/// and privacy enforcement for the persistent thought graph and event log.
#[derive(Error, Debug)]
pub enum StoreError {
    /// Failed to serialize data for storage
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Failed to deserialize stored data
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Storage backend (e.g., RocksDB) operation failed
    #[error("Storage backend error: {0}")]
    Backend(String),

    /// Requested data was not found in storage
    #[error("Not found: {0}")]
    NotFound(String),

    /// Data integrity check failed (e.g., hash mismatch)
    #[error("Integrity error: {0}")]
    Integrity(String),

    /// File system I/O operation failed
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Lock was poisoned due to panic while held
    #[error("Lock poisoned")]
    LockPoisoned,

    /// Attempted operation violates privacy rules
    #[error("Privacy violation: {0}")]
    PrivacyViolation(String),
}

/// Convenience Result type for storage operations
pub type Result<T> = std::result::Result<T, StoreError>;
