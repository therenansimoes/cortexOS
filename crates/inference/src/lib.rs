pub mod model;
pub mod skill;
pub mod error;

pub use model::{Model, ModelConfig, ModelCapabilities};
pub use skill::{InferenceSkill, CompletionSkill, ChatSkill, EmbeddingSkill};
pub use error::{InferenceError, Result};
