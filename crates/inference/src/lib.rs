pub mod error;
pub mod model;
pub mod skill;

pub use error::{InferenceError, Result};
pub use model::{ChatMessage, ChatRole, GenerationParams, Model, ModelCapabilities, ModelConfig};
pub use skill::{ChatSkill, CompletionSkill, EmbeddingSkill, InferenceSkill};

// Re-export llama module when feature is enabled
#[cfg(feature = "llama")]
pub use model::llama::LlamaModel;
