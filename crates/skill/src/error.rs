use thiserror::Error;

#[derive(Error, Debug)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("No capable node found for skill: {0}")]
    NoCapableNode(String),

    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Task timeout")]
    Timeout,

    #[error("Task cancelled")]
    Cancelled,

    #[error("Invalid task input: {0}")]
    InvalidInput(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Reputation error: {0}")]
    ReputationError(#[from] cortex_reputation::ReputationError),
}

pub type Result<T> = std::result::Result<T, SkillError>;
