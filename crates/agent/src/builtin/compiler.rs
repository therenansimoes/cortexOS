use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::context::AgentContext;
use crate::error::AgentError;
use crate::traits::Agent;
use crate::types::{AgentId, CapabilitySet, Event};

/// Compilation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationRequest {
    pub task_id: String,
    pub source_code: String,
    pub language: String,
    pub target: String,
}

/// Compilation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<String>,
    pub errors: Vec<String>,
}

/// Compiler Agent - handles code compilation tasks
/// 
/// This agent is responsible for:
/// - Receiving compilation requests
/// - Compiling source code
/// - Returning compilation results
/// - Managing compilation artifacts
pub struct CompilerAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
}

impl CompilerAgent {
    pub fn new() -> Self {
        Self {
            id: AgentId::new(),
            name: "compiler".to_string(),
            capabilities: CapabilitySet::new()
                .with_capability("compiler.rust")
                .with_capability("compiler.wasm"),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Process a compilation request
    async fn handle_compilation_request(&self, request: CompilationRequest, ctx: &mut AgentContext) -> CompilationResult {
        // In a real implementation, this would:
        // 1. Write source code to temporary directory
        // 2. Invoke rustc or cargo
        // 3. Collect output and artifacts
        // 4. Return results
        
        // For now, simulate compilation
        let success = !request.source_code.is_empty();
        let output = if success {
            format!(
                "Successfully compiled {} code for target {}\nArtifacts: binary",
                request.language,
                request.target
            )
        } else {
            "Compilation failed: empty source code".to_string()
        };

        let result = CompilationResult {
            task_id: request.task_id.clone(),
            success,
            output: output.clone(),
            artifacts: if success {
                vec![format!("{}.wasm", request.task_id)]
            } else {
                vec![]
            },
            errors: if !success {
                vec!["Empty source code".to_string()]
            } else {
                vec![]
            },
        };

        // Emit compilation result event
        let _ = ctx.emit_event(
            "compilation.completed",
            serde_json::to_vec(&result).unwrap_or_default(),
        ).await;

        result
    }
}

impl Default for CompilerAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for CompilerAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn init(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError> {
        info!(agent_id = %self.id, "CompilerAgent initialized");
        let _ = ctx.emit_event("agent.started", serde_json::to_vec(&serde_json::json!({
            "agent_id": self.id.to_string(),
            "agent_name": self.name,
        })).unwrap_or_default()).await;
        Ok(())
    }

    async fn on_event(&mut self, event: &Event, ctx: &mut AgentContext) -> Result<(), AgentError> {
        // Handle compilation requests
        if event.kind.starts_with("task.compile") {
            if let Ok(request) = serde_json::from_slice::<CompilationRequest>(&event.payload) {
                let result = self.handle_compilation_request(request, ctx).await;
                
                info!(
                    agent_id = %self.id,
                    task_id = %result.task_id,
                    success = result.success,
                    "Compilation completed"
                );
            }
        }
        
        Ok(())
    }

    async fn tick(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        info!(agent_id = %self.id, "CompilerAgent shutting down");
        Ok(())
    }
}
