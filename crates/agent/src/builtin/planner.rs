use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::context::AgentContext;
use crate::error::AgentError;
use crate::traits::Agent;
use crate::types::{AgentId, CapabilitySet, Event};

/// Task plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub goal: String,
    pub steps: Vec<TaskStep>,
    pub dependencies: HashMap<String, Vec<String>>,
}

/// Individual task step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub step_id: String,
    pub action: String,
    pub agent_type: String,
    pub inputs: HashMap<String, String>,
    pub outputs: Vec<String>,
}

/// Planning request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningRequest {
    pub request_id: String,
    pub goal: String,
    pub context: HashMap<String, String>,
}

/// Planner Agent - decomposes goals into executable tasks
/// 
/// This agent is responsible for:
/// - Breaking down complex goals into subtasks
/// - Creating task plans with dependencies
/// - Coordinating multiple agents
/// - Monitoring task execution
pub struct PlannerAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
    active_plans: HashMap<String, TaskPlan>,
}

impl PlannerAgent {
    pub fn new() -> Self {
        Self {
            id: AgentId::new(),
            name: "planner".to_string(),
            capabilities: CapabilitySet::new()
                .with_capability("planner.decompose")
                .with_capability("planner.coordinate"),
            active_plans: HashMap::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Create a plan for distributed compilation
    fn create_compilation_plan(&self, request: &PlanningRequest) -> TaskPlan {
        // For distributed compilation, we need:
        // 1. Planner: decompose the goal
        // 2. Compiler: compile the code
        // 3. Executor: run tests/validation
        
        let mut dependencies = HashMap::new();
        dependencies.insert("compile".to_string(), vec!["plan".to_string()]);
        dependencies.insert("execute".to_string(), vec!["compile".to_string()]);
        
        TaskPlan {
            goal: request.goal.clone(),
            steps: vec![
                TaskStep {
                    step_id: "plan".to_string(),
                    action: "decompose_goal".to_string(),
                    agent_type: "planner".to_string(),
                    inputs: request.context.clone(),
                    outputs: vec!["compilation_spec".to_string()],
                },
                TaskStep {
                    step_id: "compile".to_string(),
                    action: "compile_code".to_string(),
                    agent_type: "compiler".to_string(),
                    inputs: HashMap::from([
                        ("source".to_string(), "main.rs".to_string()),
                        ("target".to_string(), "wasm32-wasi".to_string()),
                    ]),
                    outputs: vec!["binary.wasm".to_string()],
                },
                TaskStep {
                    step_id: "execute".to_string(),
                    action: "run_tests".to_string(),
                    agent_type: "executor".to_string(),
                    inputs: HashMap::from([
                        ("binary".to_string(), "binary.wasm".to_string()),
                    ]),
                    outputs: vec!["test_results".to_string()],
                },
            ],
            dependencies,
        }
    }

    /// Handle a planning request
    async fn handle_planning_request(&mut self, request: PlanningRequest, ctx: &mut AgentContext) -> TaskPlan {
        let plan = self.create_compilation_plan(&request);
        self.active_plans.insert(request.request_id.clone(), plan.clone());
        
        // Emit plan created event
        let _ = ctx.emit_event(
            "plan.created",
            serde_json::to_vec(&plan).unwrap_or_default(),
        ).await;
        
        plan
    }
}

impl Default for PlannerAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for PlannerAgent {
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
        info!(agent_id = %self.id, "PlannerAgent initialized");
        let _ = ctx.emit_event("agent.started", serde_json::to_vec(&serde_json::json!({
            "agent_id": self.id.to_string(),
            "agent_name": self.name,
        })).unwrap_or_default()).await;
        Ok(())
    }

    async fn on_event(&mut self, event: &Event, ctx: &mut AgentContext) -> Result<(), AgentError> {
        // Handle planning requests
        if event.kind.starts_with("task.plan") {
            if let Ok(request) = serde_json::from_slice::<PlanningRequest>(&event.payload) {
                let plan = self.handle_planning_request(request, ctx).await;
                
                info!(
                    agent_id = %self.id,
                    goal = %plan.goal,
                    steps = plan.steps.len(),
                    "Plan created"
                );
            }
        }
        
        Ok(())
    }

    async fn tick(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        info!(agent_id = %self.id, "PlannerAgent shutting down");
        Ok(())
    }
}
