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
    use llama_cpp_2::{
        context::params::LlamaContextParams,
        llama_backend::LlamaBackend,
        llama_batch::LlamaBatch,
        model::{params::LlamaModelParams, LlamaModel as LlamaCppModel, AddBos},
        token::data_array::LlamaTokenDataArray,
    };
    use std::num::NonZeroU32;
    use std::path::Path;

    /// Llama.cpp model implementation
    pub struct LlamaModel {
        name: String,
        config: ModelConfig,
        capabilities: ModelCapabilities,
        backend: Option<LlamaBackend>,
        model: Option<LlamaCppModel>,
    }

    impl LlamaModel {
        pub fn new(name: &str, config: ModelConfig) -> Self {
            Self {
                name: name.to_string(),
                config,
                capabilities: ModelCapabilities {
                    completion: true,
                    chat: true,
                    embeddings: true,
                    code: true,
                    languages: vec!["rust", "python", "javascript", "go"].iter().map(|s| s.to_string()).collect(),
                },
                backend: None,
                model: None,
            }
        }

        fn ensure_loaded(&self) -> Result<()> {
            if !self.is_loaded() {
                return Err(crate::error::InferenceError::ModelNotLoaded(
                    "Model not loaded. Call load() first.".to_string()
                ));
            }
            Ok(())
        }
    }

    #[async_trait]
    impl Model for LlamaModel {
        fn name(&self) -> &str {
            &self.name
        }

        fn capabilities(&self) -> &ModelCapabilities {
            &self.capabilities
        }

        async fn load(&mut self) -> Result<()> {
            if self.is_loaded() {
                return Ok(());
            }

            // Check if model file exists
            if !Path::new(&self.config.model_path).exists() {
                return Err(crate::error::InferenceError::ModelFileNotFound(
                    self.config.model_path.display().to_string()
                ));
            }

            // Initialize backend
            let backend = LlamaBackend::init()
                .map_err(|e| crate::error::InferenceError::ModelLoadFailed(format!("Backend init failed: {}", e)))?;

            // Load model
            let model_params = LlamaModelParams::default()
                .with_n_gpu_layers(self.config.gpu_layers);

            let model = LlamaCppModel::load_from_file(&backend, &self.config.model_path, &model_params)
                .map_err(|e| crate::error::InferenceError::ModelLoadFailed(format!("Model load failed: {}", e)))?;

            self.backend = Some(backend);
            self.model = Some(model);

            tracing::info!("Loaded model: {}", self.name);
            Ok(())
        }

        async fn unload(&mut self) -> Result<()> {
            self.model = None;
            self.backend = None;
            tracing::info!("Unloaded model: {}", self.name);
            Ok(())
        }

        fn is_loaded(&self) -> bool {
            self.model.is_some() && self.backend.is_some()
        }

        async fn complete(&self, prompt: &str, params: &GenerationParams) -> Result<String> {
            self.ensure_loaded()?;

            let model = self.model.as_ref().expect("Model should be loaded after ensure_loaded check");
            let backend = self.backend.as_ref().expect("Backend should be loaded after ensure_loaded check");
            
            // Create context for this inference
            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(self.config.context_size as u32))
                .with_n_threads(self.config.threads as i32)
                .with_n_threads_batch(self.config.threads as i32);

            let mut ctx = model.new_context(backend, ctx_params)
                .map_err(|e| crate::error::InferenceError::ModelLoadFailed(format!("Context creation failed: {}", e)))?;

            // Tokenize prompt
            let tokens = model.str_to_token(prompt, AddBos::Always)
                .map_err(|e| crate::error::InferenceError::TokenizationError(format!("Tokenization failed: {}", e)))?;

            let n_ctx = ctx.n_ctx() as usize;
            if tokens.len() > n_ctx {
                return Err(crate::error::InferenceError::ContextLengthExceeded(
                    tokens.len(),
                    n_ctx,
                ));
            }

            // Process prompt
            let mut batch = LlamaBatch::new(512, 1);
            for (i, &token) in tokens.iter().enumerate() {
                let is_last = i == tokens.len() - 1;
                batch.add(token, i as i32, &[0], is_last)
                    .map_err(|e| crate::error::InferenceError::InferenceFailed(format!("Failed to add token: {}", e)))?;
            }

            ctx.decode(&mut batch)
                .map_err(|e| crate::error::InferenceError::InferenceFailed(format!("Failed to decode: {}", e)))?;

            // Generate tokens
            let mut output_tokens = Vec::new();
            let mut n_cur = tokens.len();
            let n_len = params.max_tokens.min(n_ctx - tokens.len());

            for _ in 0..n_len {
                let candidates = ctx.candidates();
                let mut candidates_p = LlamaTokenDataArray::from_iter(candidates, false);
                
                // Sample token - use greedy sampling
                let token = candidates_p.sample_token_greedy();

                // Check for EOS - model's EOS token
                if token.0 == model.token_eos().0 {
                    break;
                }

                output_tokens.push(token);

                // Prepare next iteration
                n_cur += 1;
                if n_cur >= n_ctx {
                    break;
                }

                batch.clear();
                batch.add(token, n_cur as i32, &[0], true)
                    .map_err(|e| crate::error::InferenceError::InferenceFailed(format!("Failed to add token: {}", e)))?;

                ctx.decode(&mut batch)
                    .map_err(|e| crate::error::InferenceError::InferenceFailed(format!("Failed to decode: {}", e)))?;
            }

            // Detokenize
            let mut result = String::new();
            for &token in &output_tokens {
                let piece = model.token_to_str(token, llama_cpp_2::model::Special::Tokenize)
                    .map_err(|e| crate::error::InferenceError::InferenceFailed(format!("Detokenization failed: {}", e)))?;
                result.push_str(&piece);
            }

            Ok(result)
        }

        async fn chat(&self, messages: &[ChatMessage], params: &GenerationParams) -> Result<String> {
            self.ensure_loaded()?;

            // Format chat messages into a prompt
            let mut prompt = String::new();
            for msg in messages {
                match msg.role {
                    ChatRole::System => {
                        prompt.push_str(&format!("### System:\n{}\n\n", msg.content));
                    }
                    ChatRole::User => {
                        prompt.push_str(&format!("### User:\n{}\n\n", msg.content));
                    }
                    ChatRole::Assistant => {
                        prompt.push_str(&format!("### Assistant:\n{}\n\n", msg.content));
                    }
                }
            }
            prompt.push_str("### Assistant:\n");

            self.complete(&prompt, params).await
        }

        async fn embed(&self, text: &str) -> Result<Vec<f32>> {
            self.ensure_loaded()?;

            let model = self.model.as_ref().expect("Model should be loaded after ensure_loaded check");
            let backend = self.backend.as_ref().expect("Backend should be loaded after ensure_loaded check");

            // Create context for embedding
            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(self.config.context_size as u32))
                .with_n_threads(self.config.threads as i32)
                .with_embeddings(true);

            let mut ctx = model.new_context(backend, ctx_params)
                .map_err(|e| crate::error::InferenceError::ModelLoadFailed(format!("Context creation failed: {}", e)))?;

            // Tokenize
            let tokens = model.str_to_token(text, AddBos::Always)
                .map_err(|e| crate::error::InferenceError::TokenizationError(format!("Tokenization failed: {}", e)))?;

            // Create batch for embedding
            let mut batch = LlamaBatch::new(tokens.len(), 1);
            for (i, &token) in tokens.iter().enumerate() {
                batch.add(token, i as i32, &[0], false)
                    .map_err(|e| crate::error::InferenceError::InferenceFailed(format!("Failed to add token: {}", e)))?;
            }

            // Decode to get embeddings
            ctx.decode(&mut batch)
                .map_err(|e| crate::error::InferenceError::InferenceFailed(format!("Failed to decode: {}", e)))?;

            // Get embeddings from the sequence
            let embeddings_result = ctx.embeddings_seq_ith(0);
            
            let embeddings_slice = match embeddings_result {
                Ok(slice) => slice,
                Err(e) => return Err(crate::error::InferenceError::InferenceFailed(format!("Failed to get embeddings: {}", e))),
            };

            // Convert to Vec
            let embeddings: Vec<f32> = embeddings_slice.to_vec();

            Ok(embeddings)
        }

        fn tokenize(&self, text: &str) -> Result<Vec<u32>> {
            self.ensure_loaded()?;

            let model = self.model.as_ref().expect("Model should be loaded after ensure_loaded check");
            let tokens = model.str_to_token(text, AddBos::Always)
                .map_err(|e| crate::error::InferenceError::TokenizationError(format!("Tokenization failed: {}", e)))?;

            Ok(tokens.iter().map(|t| t.0 as u32).collect())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_llama_model_creation() {
            let config = ModelConfig::new("/tmp/test.gguf");
            let model = LlamaModel::new("test-model", config);
            
            assert_eq!(model.name(), "test-model");
            assert!(!model.is_loaded());
            assert!(model.capabilities().completion);
            assert!(model.capabilities().chat);
            assert!(model.capabilities().embeddings);
            assert!(model.capabilities().code);
        }

        #[tokio::test]
        async fn test_llama_model_unloaded() {
            let config = ModelConfig::new("/tmp/test.gguf");
            let model = LlamaModel::new("test-model", config);
            
            // Should error when trying to use unloaded model
            let result = model.complete("test", &GenerationParams::default()).await;
            assert!(result.is_err());
            
            let result = model.tokenize("test");
            assert!(result.is_err());
        }
    }
}
