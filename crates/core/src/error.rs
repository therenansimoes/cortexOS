use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Event queue is full")]
    QueueFull,

    #[error("Event queue is empty")]
    QueueEmpty,

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent already registered: {0}")]
    AgentAlreadyRegistered(String),

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),

    #[error("Invalid event: {0}")]
    InvalidEvent(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Runtime shutdown")]
    RuntimeShutdown,

    #[error("Pattern match error: {0}")]
    PatternError(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
