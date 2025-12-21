use thiserror::Error;

use crate::types::{AgentId, IntentionId};

/// Errors that can occur in agent lifecycle and execution.
///
/// These errors cover agent initialization, event handling, intention management,
/// and interaction with the event bus and thought graph.
#[derive(Debug, Error)]
pub enum AgentError {
    /// Agent failed to initialize properly
    #[error("Agent initialization failed: {0}")]
    InitFailed(String),

    /// Agent's event handler returned an error
    #[error("Event handling failed: {0}")]
    EventHandlingFailed(String),

    /// Referenced agent does not exist
    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),

    /// Agent with this ID already exists
    #[error("Agent already exists: {0}")]
    AgentAlreadyExists(AgentId),

    /// Agent panicked during execution
    #[error("Agent panicked: {0}")]
    AgentPanicked(String),

    /// Agent failed to shut down gracefully
    #[error("Shutdown failed: {0}")]
    ShutdownFailed(String),

    /// Failed to spawn agent task
    #[error("Spawn failed: {0}")]
    SpawnFailed(String),

    /// Thought graph operation failed
    #[error("Graph error: {0}")]
    GraphError(String),

    /// Event bus interaction failed
    #[error("Event bus error: {0}")]
    EventBusError(String),

    /// Intention operation failed
    #[error("Intention error: {0}")]
    IntentionError(String),

    /// Event subscription failed
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    /// Internal agent framework error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience Result type for agent operations
pub type Result<T> = std::result::Result<T, AgentError>;

/// Errors specific to intention management.
///
/// Intentions represent goals or tasks that agents are trying to accomplish.
#[derive(Debug, Error)]
pub enum IntentionError {
    /// Referenced intention does not exist
    #[error("Intention not found: {0}")]
    NotFound(IntentionId),

    /// Invalid state transition attempted
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    /// No agent capable of handling this intention
    #[error("No matching agent for intention: {0}")]
    NoMatchingAgent(String),

    /// Intention has already been completed
    #[error("Intention already completed: {0}")]
    AlreadyCompleted(IntentionId),
}

/// Convenience Result type for intention operations
pub type IntentionResult<T> = std::result::Result<T, IntentionError>;
