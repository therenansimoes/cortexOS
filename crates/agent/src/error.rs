use thiserror::Error;

use crate::types::{AgentId, IntentionId};

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent initialization failed: {0}")]
    InitFailed(String),

    #[error("Event handling failed: {0}")]
    EventHandlingFailed(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("Agent already exists: {0}")]
    AgentAlreadyExists(AgentId),

    #[error("Agent panicked: {0}")]
    AgentPanicked(String),

    #[error("Shutdown failed: {0}")]
    ShutdownFailed(String),

    #[error("Spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Graph error: {0}")]
    GraphError(String),

    #[error("Event bus error: {0}")]
    EventBusError(String),

    #[error("Intention error: {0}")]
    IntentionError(String),

    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AgentError>;

#[derive(Debug, Error)]
pub enum IntentionError {
    #[error("Intention not found: {0}")]
    NotFound(IntentionId),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("No matching agent for intention: {0}")]
    NoMatchingAgent(String),

    #[error("Intention already completed: {0}")]
    AlreadyCompleted(IntentionId),
}

pub type IntentionResult<T> = std::result::Result<T, IntentionError>;
