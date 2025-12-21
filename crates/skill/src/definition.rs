use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use cortex_reputation::SkillId;

/// Metadata about a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Unique skill identifier
    pub id: SkillId,
    /// Human-readable name
    pub name: String,
    /// Description of what this skill does
    pub description: String,
    /// Version
    pub version: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Input schema (JSON Schema or description)
    pub input_schema: Option<String>,
    /// Output schema
    pub output_schema: Option<String>,
    /// Estimated cost (compute, time, etc.)
    pub cost_estimate: Option<CostEstimate>,
}

impl SkillMetadata {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: SkillId::new(id),
            name: name.to_string(),
            description: description.to_string(),
            version: "1.0.0".to_string(),
            tags: Vec::new(),
            input_schema: None,
            output_schema: None,
            cost_estimate: None,
        }
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(String::from).collect();
        self
    }
}

/// Estimated cost for executing a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Time in milliseconds
    pub time_ms: u64,
    /// Compute units (abstract)
    pub compute_units: u32,
    /// Memory in MB
    pub memory_mb: u32,
}

/// Capability requirements for a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCapability {
    /// Required hardware (gpu, tpu, etc.)
    pub hardware: Vec<String>,
    /// Required models (llama-7b, whisper, etc.)
    pub models: Vec<String>,
    /// Minimum memory
    pub min_memory_mb: u32,
    /// Requires network access
    pub needs_network: bool,
    /// Requires storage
    pub needs_storage: bool,
}

impl Default for SkillCapability {
    fn default() -> Self {
        Self {
            hardware: Vec::new(),
            models: Vec::new(),
            min_memory_mb: 0,
            needs_network: false,
            needs_storage: false,
        }
    }
}

/// A skill that can be executed
#[async_trait]
pub trait Skill: Send + Sync {
    /// Get skill metadata
    fn metadata(&self) -> &SkillMetadata;

    /// Get capability requirements
    fn capabilities(&self) -> SkillCapability {
        SkillCapability::default()
    }

    /// Check if this node can execute the skill
    fn can_execute(&self) -> bool {
        true
    }

    /// Execute the skill with given input
    async fn execute(&self, input: SkillInput) -> crate::Result<SkillOutput>;

    /// Estimate execution cost
    fn estimate_cost(&self, _input: &SkillInput) -> Option<CostEstimate> {
        self.metadata().cost_estimate.clone()
    }
}

/// Input for skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    /// Raw data
    pub data: Vec<u8>,
    /// Structured parameters
    pub params: HashMap<String, serde_json::Value>,
    /// Context from previous steps
    pub context: Option<String>,
}

impl SkillInput {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            params: HashMap::new(),
            context: None,
        }
    }

    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    pub fn with_param(mut self, key: &str, value: serde_json::Value) -> Self {
        self.params.insert(key.to_string(), value);
        self
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.data = text.as_bytes().to_vec();
        self
    }

    pub fn get_text(&self) -> Option<String> {
        String::from_utf8(self.data.clone()).ok()
    }

    pub fn get_param<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.params
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

impl Default for SkillInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Output from skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    /// Raw output data
    pub data: Vec<u8>,
    /// Structured result
    pub result: HashMap<String, serde_json::Value>,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

impl SkillOutput {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            result: HashMap::new(),
            metadata: ExecutionMetadata::default(),
        }
    }

    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.data = text.as_bytes().to_vec();
        self
    }

    pub fn with_result(mut self, key: &str, value: serde_json::Value) -> Self {
        self.result.insert(key.to_string(), value);
        self
    }

    pub fn get_text(&self) -> Option<String> {
        String::from_utf8(self.data.clone()).ok()
    }
}

impl Default for SkillOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about an execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionMetadata {
    /// Time taken in milliseconds
    pub duration_ms: u64,
    /// Tokens used (if LLM)
    pub tokens_used: Option<u32>,
    /// Model used
    pub model: Option<String>,
}
