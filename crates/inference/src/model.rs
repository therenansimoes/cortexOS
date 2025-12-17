use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::Result;

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Path to model file (GGUF format for llama.cpp)
    pub model_path: PathBuf,
    /// Context window size
    pub context_size: usize,
    /// Number of layers to offload to GPU (0 = CPU only)
    pub gpu_layers: u32,
    /// Number of threads for CPU inference
    pub threads: u32,
    /// Batch size
    pub batch_size: usize,
    /// Random seed
    pub seed: Option<u64>,
}

impl ModelConfig {
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            context_size: 4096,
            gpu_layers: 0,
            threads: 4,
            batch_size: 512,
            seed: None,
        }
    }

    pub fn with_context_size(mut self, size: usize) -> Self {
        self.context_size = size;
        self
    }

    pub fn with_gpu_layers(mut self, layers: u32) -> Self {
        self.gpu_layers = layers;
        self
    }

    pub fn with_threads(mut self, threads: u32) -> Self {
        self.threads = threads;
        self
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            context_size: 4096,
            gpu_layers: 0,
            threads: 4,
            batch_size: 512,
            seed: None,
        }
    }
}

/// What a model can do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Can generate text completions
    pub completion: bool,
    /// Can do chat (multi-turn)
    pub chat: bool,
    /// Can generate embeddings
    pub embeddings: bool,
    /// Can do code generation
    pub code: bool,
    /// Supported languages (for code models)
    pub languages: Vec<String>,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            completion: true,
            chat: false,
            embeddings: false,
            code: false,
            languages: Vec::new(),
        }
    }
}

/// Generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParams {
    /// Maximum tokens to generate
    pub max_tokens: usize,
    /// Temperature (0.0 = deterministic, 1.0 = creative)
    pub temperature: f32,
    /// Top-p sampling
    pub top_p: f32,
    /// Top-k sampling
    pub top_k: u32,
    /// Repetition penalty
    pub repeat_penalty: f32,
    /// Stop sequences
    pub stop: Vec<String>,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            max_tokens: 256,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            stop: Vec::new(),
        }
    }
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: ChatRole::System,
            content: content.to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: ChatRole::User,
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.to_string(),
        }
    }
}

/// Abstract model interface
#[async_trait]
pub trait Model: Send + Sync {
    /// Model name/identifier
    fn name(&self) -> &str;

    /// Get capabilities
    fn capabilities(&self) -> &ModelCapabilities;

    /// Load the model
    async fn load(&mut self) -> Result<()>;

    /// Unload the model
    async fn unload(&mut self) -> Result<()>;

    /// Check if loaded
    fn is_loaded(&self) -> bool;

    /// Text completion
    async fn complete(&self, prompt: &str, params: &GenerationParams) -> Result<String>;

    /// Chat completion
    async fn chat(&self, messages: &[ChatMessage], params: &GenerationParams) -> Result<String>;

    /// Generate embeddings
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Tokenize text
    fn tokenize(&self, text: &str) -> Result<Vec<u32>>;

    /// Count tokens
    fn count_tokens(&self, text: &str) -> Result<usize> {
        Ok(self.tokenize(text)?.len())
    }
}

/// Mock model for testing (no actual LLM)
pub struct MockModel {
    name: String,
    capabilities: ModelCapabilities,
    loaded: bool,
}

impl MockModel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            capabilities: ModelCapabilities::default(),
            loaded: false,
        }
    }
}

#[async_trait]
impl Model for MockModel {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &ModelCapabilities {
        &self.capabilities
    }

    async fn load(&mut self) -> Result<()> {
        self.loaded = true;
        Ok(())
    }

    async fn unload(&mut self) -> Result<()> {
        self.loaded = false;
        Ok(())
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    async fn complete(&self, prompt: &str, _params: &GenerationParams) -> Result<String> {
        // Mock: just echo back a response
        Ok(format!("[MockModel response to: {}...]", &prompt[..prompt.len().min(50)]))
    }

    async fn chat(&self, messages: &[ChatMessage], params: &GenerationParams) -> Result<String> {
        if let Some(last) = messages.last() {
            self.complete(&last.content, params).await
        } else {
            Ok("[No messages provided]".to_string())
        }
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Mock: return a simple hash-based embedding
        let hash = blake3::hash(text.as_bytes());
        let bytes = hash.as_bytes();
        Ok(bytes.iter().map(|b| (*b as f32) / 255.0).collect())
    }

    fn tokenize(&self, text: &str) -> Result<Vec<u32>> {
        // Mock: simple whitespace tokenization
        Ok(text.split_whitespace().enumerate().map(|(i, _)| i as u32).collect())
    }
}

// Conditional llama.cpp implementation
#[cfg(feature = "llama")]
pub mod llama {
    use super::*;
    // TODO: Implement actual llama.cpp bindings
    // use llama_cpp_2::...
}
