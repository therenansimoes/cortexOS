use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("Model load failed: {0}")]
    ModelLoadFailed(String),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Context length exceeded: {0} > {1}")]
    ContextLengthExceeded(usize, usize),

    #[error("Model file not found: {0}")]
    ModelFileNotFound(String),

    #[error("Unsupported model format: {0}")]
    UnsupportedFormat(String),

    #[error("Tokenization error: {0}")]
    TokenizationError(String),

    #[error("Skill error: {0}")]
    SkillError(#[from] cortex_skill::SkillError),
}

pub type Result<T> = std::result::Result<T, InferenceError>;
