use thiserror::Error;

/// Errors in the decentralized reputation system.
///
/// The reputation system tracks peer reliability and skill proficiency
/// through ratings, trust scores, and gossip protocols.
#[derive(Error, Debug)]
pub enum ReputationError {
    /// Node ID not found in the reputation graph
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Skill identifier not recognized
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    /// Rating value is outside valid range or format
    #[error("Invalid rating: {0}")]
    InvalidRating(String),

    /// Nodes cannot rate themselves (prevents gaming)
    #[error("Self-rating not allowed")]
    SelfRatingNotAllowed,

    /// Rating already exists for this node-skill pair
    #[error("Rating already exists")]
    DuplicateRating,

    /// Trust score calculation failed
    #[error("Trust computation failed: {0}")]
    TrustComputationFailed(String),

    /// Gossip protocol error
    #[error("Gossip error: {0}")]
    GossipError(String),

    /// Failed to serialize or deserialize reputation data
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Convenience Result type for reputation operations
pub type Result<T> = std::result::Result<T, ReputationError>;
