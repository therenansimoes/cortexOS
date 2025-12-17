use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use cortex_skill::definition::{
    Skill, SkillCapability, SkillInput, SkillOutput, SkillMetadata,
};
use cortex_skill::Result as SkillResult;

use crate::model::{Model, GenerationParams, ChatMessage};

/// Base inference skill that wraps a model
pub struct InferenceSkill {
    metadata: SkillMetadata,
    model: Arc<RwLock<dyn Model>>,
}

impl InferenceSkill {
    pub fn new(skill_id: &str, model: Arc<RwLock<dyn Model>>) -> Self {
        Self {
            metadata: SkillMetadata::new(
                skill_id,
                skill_id,
                "LLM inference skill",
            ),
            model,
        }
    }
}

/// Text completion skill
pub struct CompletionSkill {
    metadata: SkillMetadata,
    model: Arc<RwLock<dyn Model>>,
    default_params: GenerationParams,
}

impl CompletionSkill {
    pub fn new(model: Arc<RwLock<dyn Model>>) -> Self {
        Self {
            metadata: SkillMetadata::new(
                "llm.completion",
                "Text Completion",
                "Generate text completions using LLM",
            ).with_tags(vec!["llm", "text", "generation"]),
            model,
            default_params: GenerationParams::default(),
        }
    }

    pub fn with_params(mut self, params: GenerationParams) -> Self {
        self.default_params = params;
        self
    }
}

#[async_trait]
impl Skill for CompletionSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> SkillCapability {
        SkillCapability {
            hardware: vec![],
            models: vec!["llm".to_string()],
            min_memory_mb: 1024,
            needs_network: false,
            needs_storage: true,
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillResult<SkillOutput> {
        let prompt = input.get_text().ok_or_else(|| {
            cortex_skill::SkillError::InvalidInput("No text prompt provided".to_string())
        })?;

        let params = input
            .get_param::<GenerationParams>("params")
            .unwrap_or_else(|| self.default_params.clone());

        let model = self.model.read().await;
        let response = model.complete(&prompt, &params).await
            .map_err(|e| cortex_skill::SkillError::ExecutionFailed(e.to_string()))?;

        Ok(SkillOutput::new()
            .with_text(&response)
            .with_result("tokens", serde_json::json!(response.split_whitespace().count())))
    }
}

/// Chat skill (multi-turn conversation)
pub struct ChatSkill {
    metadata: SkillMetadata,
    model: Arc<RwLock<dyn Model>>,
    system_prompt: Option<String>,
    default_params: GenerationParams,
}

impl ChatSkill {
    pub fn new(model: Arc<RwLock<dyn Model>>) -> Self {
        Self {
            metadata: SkillMetadata::new(
                "llm.chat",
                "Chat",
                "Multi-turn chat using LLM",
            ).with_tags(vec!["llm", "chat", "conversation"]),
            model,
            system_prompt: None,
            default_params: GenerationParams::default(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }
}

#[async_trait]
impl Skill for ChatSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> SkillCapability {
        SkillCapability {
            hardware: vec![],
            models: vec!["llm".to_string()],
            min_memory_mb: 2048,
            needs_network: false,
            needs_storage: true,
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillResult<SkillOutput> {
        // Parse messages from input
        let messages: Vec<ChatMessage> = if let Some(msgs) = input.get_param::<Vec<ChatMessage>>("messages") {
            msgs
        } else {
            let text = input.get_text().ok_or_else(|| {
                cortex_skill::SkillError::InvalidInput("No messages or text provided".to_string())
            })?;
            // Single message as user input
            let mut msgs = Vec::new();
            if let Some(ref sys) = self.system_prompt {
                msgs.push(ChatMessage::system(sys));
            }
            msgs.push(ChatMessage::user(&text));
            msgs
        };

        let params = input
            .get_param::<GenerationParams>("params")
            .unwrap_or_else(|| self.default_params.clone());

        let model = self.model.read().await;
        let response = model.chat(&messages, &params).await
            .map_err(|e| cortex_skill::SkillError::ExecutionFailed(e.to_string()))?;

        Ok(SkillOutput::new().with_text(&response))
    }
}

/// Embedding skill
pub struct EmbeddingSkill {
    metadata: SkillMetadata,
    model: Arc<RwLock<dyn Model>>,
}

impl EmbeddingSkill {
    pub fn new(model: Arc<RwLock<dyn Model>>) -> Self {
        Self {
            metadata: SkillMetadata::new(
                "llm.embedding",
                "Text Embedding",
                "Generate vector embeddings for text",
            ).with_tags(vec!["llm", "embedding", "vector"]),
            model,
        }
    }
}

#[async_trait]
impl Skill for EmbeddingSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> SkillCapability {
        SkillCapability {
            hardware: vec![],
            models: vec!["embedding".to_string()],
            min_memory_mb: 512,
            needs_network: false,
            needs_storage: true,
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillResult<SkillOutput> {
        let text = input.get_text().ok_or_else(|| {
            cortex_skill::SkillError::InvalidInput("No text provided".to_string())
        })?;

        let model = self.model.read().await;
        let embedding = model.embed(&text).await
            .map_err(|e| cortex_skill::SkillError::ExecutionFailed(e.to_string()))?;

        Ok(SkillOutput::new()
            .with_result("embedding", serde_json::json!(embedding))
            .with_result("dimensions", serde_json::json!(embedding.len())))
    }
}

/// Code generation skill
pub struct CodeSkill {
    metadata: SkillMetadata,
    model: Arc<RwLock<dyn Model>>,
    language: String,
}

impl CodeSkill {
    pub fn new(model: Arc<RwLock<dyn Model>>, language: &str) -> Self {
        let skill_id = format!("llm.code.{}", language);
        Self {
            metadata: SkillMetadata::new(
                &skill_id,
                &format!("{} Code Generation", language),
                &format!("Generate {} code using LLM", language),
            ).with_tags(vec!["llm", "code", language]),
            model,
            language: language.to_string(),
        }
    }
}

#[async_trait]
impl Skill for CodeSkill {
    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> SkillCapability {
        SkillCapability {
            hardware: vec![],
            models: vec!["code-llm".to_string()],
            min_memory_mb: 4096,
            needs_network: false,
            needs_storage: true,
        }
    }

    async fn execute(&self, input: SkillInput) -> SkillResult<SkillOutput> {
        let prompt = input.get_text().ok_or_else(|| {
            cortex_skill::SkillError::InvalidInput("No prompt provided".to_string())
        })?;

        // Build code generation prompt
        let code_prompt = format!(
            "Write {} code for the following task:\n\n{}\n\nCode:\n```{}\n",
            self.language, prompt, self.language
        );

        let params = GenerationParams {
            temperature: 0.2, // Lower temperature for code
            max_tokens: 1024,
            stop: vec![format!("```")],
            ..Default::default()
        };

        let model = self.model.read().await;
        let response = model.complete(&code_prompt, &params).await
            .map_err(|e| cortex_skill::SkillError::ExecutionFailed(e.to_string()))?;

        // Extract code from response
        let code = response.trim_end_matches("```").trim();

        Ok(SkillOutput::new()
            .with_text(code)
            .with_result("language", serde_json::json!(self.language)))
    }
}
