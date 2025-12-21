use thiserror::Error;

/// Errors that can occur in the CortexOS core runtime and event system.
///
/// These errors cover event processing, agent management, capability checks,
/// and general runtime operations.
#[derive(Error, Debug)]
pub enum CoreError {
    /// Event queue has reached capacity and cannot accept more events
    #[error("Event queue is full")]
    QueueFull,

    /// Attempted to pop from an empty event queue
    #[error("Event queue is empty")]
    QueueEmpty,

    /// Referenced agent does not exist in the runtime
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Attempted to register an agent with a name that's already in use
    #[error("Agent already registered: {0}")]
    AgentAlreadyRegistered(String),

    /// Agent attempted an operation without the required capability
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),

    /// Event validation failed or event data is malformed
    #[error("Invalid event: {0}")]
    InvalidEvent(String),

    /// Failed to serialize or deserialize data
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// Communication channel was closed unexpectedly
    #[error("Channel closed")]
    ChannelClosed,

    /// Runtime is shutting down and cannot process new requests
    #[error("Runtime shutdown")]
    RuntimeShutdown,

    /// Event pattern matching failed
    #[error("Pattern match error: {0}")]
    PatternError(String),

    /// Hash slice has invalid length for conversion
    #[error("Invalid hash slice length: expected 16 bytes, got {0}")]
    InvalidHashLength(usize),
}

/// Convenience Result type for core operations
pub type Result<T> = std::result::Result<T, CoreError>;
