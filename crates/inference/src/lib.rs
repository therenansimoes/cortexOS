pub mod model;
pub mod skill;
pub mod error;

pub use model::{Model, ModelConfig, ModelCapabilities, GenerationParams, ChatMessage, ChatRole};
pub use skill::{InferenceSkill, CompletionSkill, ChatSkill, EmbeddingSkill};
pub use error::{InferenceError, Result};

// Re-export llama module when feature is enabled
#[cfg(feature = "llama")]
pub use model::llama::LlamaModel;
