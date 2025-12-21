use thiserror::Error;

/// Errors in the skill framework and network.
///
/// Skills are capabilities that nodes can offer to the Grid. These errors
/// cover skill registration, discovery, execution, and network delegation.
#[derive(Error, Debug)]
pub enum SkillError {
    /// Requested skill is not registered
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    /// No node in the Grid can perform this skill
    #[error("No capable node found for skill: {0}")]
    NoCapableNode(String),

    /// Skill execution failed on the remote node
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Skill execution exceeded time limit
    #[error("Task timeout")]
    Timeout,

    /// Skill execution was cancelled
    #[error("Task cancelled")]
    Cancelled,

    /// Input parameters are invalid for this skill
    #[error("Invalid task input: {0}")]
    InvalidInput(String),

    /// Failed to serialize or deserialize skill data
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Network communication error during skill delegation
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Reputation system error during skill routing
    #[error("Reputation error: {0}")]
    ReputationError(#[from] cortex_reputation::ReputationError),
}

/// Convenience Result type for skill operations
pub type Result<T> = std::result::Result<T, SkillError>;
