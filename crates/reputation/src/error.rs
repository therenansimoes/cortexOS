use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReputationError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Invalid rating: {0}")]
    InvalidRating(String),

    #[error("Self-rating not allowed")]
    SelfRatingNotAllowed,

    #[error("Rating already exists")]
    DuplicateRating,

    #[error("Trust computation failed: {0}")]
    TrustComputationFailed(String),

    #[error("Gossip error: {0}")]
    GossipError(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, ReputationError>;
