use thiserror::Error;

/// Errors in local LLM inference and model management.
///
/// These errors cover model loading, inference execution, tokenization,
/// and resource constraints when running LLMs locally on-device.
#[derive(Error, Debug)]
pub enum InferenceError {
    /// Attempted to use a model that hasn't been loaded
    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),

    /// Model file loading failed
    #[error("Model load failed: {0}")]
    ModelLoadFailed(String),

    /// Inference operation failed during execution
    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    /// Input text or parameters are invalid
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// System ran out of memory during inference
    #[error("Out of memory")]
    OutOfMemory,

    /// Input exceeds model's maximum context length
    #[error("Context length exceeded: {0} > {1}")]
    ContextLengthExceeded(usize, usize),

    /// Model file not found at specified path
    #[error("Model file not found: {0}")]
    ModelFileNotFound(String),

    /// Model format is not supported
    #[error("Unsupported model format: {0}")]
    UnsupportedFormat(String),

    /// Tokenization failed
    #[error("Tokenization error: {0}")]
    TokenizationError(String),

    /// Skill framework error during inference-as-skill
    #[error("Skill error: {0}")]
    SkillError(#[from] cortex_skill::SkillError),
}

/// Convenience Result type for inference operations
pub type Result<T> = std::result::Result<T, InferenceError>;
