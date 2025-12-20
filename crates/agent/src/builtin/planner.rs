use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::context::AgentContext;
use crate::error::AgentError;
use crate::intention::{Intention, IntentionStatus};
use crate::traits::Agent;
use crate::types::{AgentId, CapabilitySet, Event, IntentionId, ThoughtContent};

/// Planning strategy for goal decomposition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PlanningStrategy {
    /// Sequential: execute subgoals one after another
    Sequential,
    /// Parallel: execute all subgoals concurrently
    Parallel,
    /// Adaptive: let the planner decide based on dependencies
    #[default]
    Adaptive,
}

/// A planned task with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTask {
    pub intention_id: IntentionId,
    pub goal: String,
    pub subgoals: Vec<String>,
    pub strategy: PlanningStrategy,
    pub dependencies: Vec<IntentionId>,
    pub priority: u32,
}

impl PlannedTask {
    pub fn new(intention_id: IntentionId, goal: String) -> Self {
        Self {
            intention_id,
            goal,
            subgoals: Vec::new(),
            strategy: PlanningStrategy::default(),
            dependencies: Vec::new(),
            priority: 0,
        }
    }

    pub fn with_subgoals(mut self, subgoals: Vec<String>) -> Self {
        self.subgoals = subgoals;
        self
    }

    pub fn with_strategy(mut self, strategy: PlanningStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// Planning statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlanningStats {
    pub plans_created: u64,
    pub subgoals_generated: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub coordination_events: u64,
}

/// Planner Agent for AI-assisted task planning and orchestration
pub struct PlannerAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
    active_plans: Arc<RwLock<HashMap<IntentionId, PlannedTask>>>,
    stats: Arc<RwLock<PlanningStats>>,
    use_llm: bool,
}

impl PlannerAgent {
    pub fn new() -> Self {
        Self {
            id: AgentId::new(),
            name: "planner".to_string(),
            capabilities: CapabilitySet::new()
                .with_capability("planning")
                .with_capability("task-decomposition")
                .with_capability("agent-coordination")
                .with_capability("goal-management"),
            active_plans: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PlanningStats::default())),
            use_llm: false,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_llm(mut self, enabled: bool) -> Self {
        self.use_llm = enabled;
        self
    }

    pub async fn stats(&self) -> PlanningStats {
        self.stats.read().await.clone()
    }

    /// Decompose a goal into subgoals
    async fn decompose_goal(&self, goal: &str) -> Vec<String> {
        // Simple rule-based decomposition for now
        // In the future, this can use LLM for more intelligent decomposition
        
        let goal_lower = goal.to_lowercase();
        
        // Pattern matching for common task types
        if goal_lower.contains("build") && goal_lower.contains("web server") {
            vec![
                "Design API endpoints".to_string(),
                "Implement HTTP server".to_string(),
                "Add request routing".to_string(),
                "Add error handling".to_string(),
                "Write tests".to_string(),
            ]
        } else if goal_lower.contains("implement") && goal_lower.contains("http") {
            vec![
                "Setup HTTP library".to_string(),
                "Create server instance".to_string(),
                "Add request handlers".to_string(),
                "Implement response handling".to_string(),
            ]
        } else if goal_lower.contains("compile") || goal_lower.contains("build code") {
            vec![
                "Setup build environment".to_string(),
                "Run compiler".to_string(),
                "Check for errors".to_string(),
                "Generate artifacts".to_string(),
            ]
        } else if goal_lower.contains("test") {
            vec![
                "Setup test environment".to_string(),
                "Run test suite".to_string(),
                "Collect test results".to_string(),
                "Generate test report".to_string(),
            ]
        } else {
            // Default: break into phases
            vec![
                format!("Analyze requirements: {}", goal),
                format!("Plan implementation: {}", goal),
                format!("Execute: {}", goal),
                format!("Verify: {}", goal),
            ]
        }
    }

    /// Create a plan for an intention
    async fn create_plan(
        &self,
        intention_id: IntentionId,
        goal: &str,
        ctx: &AgentContext,
    ) -> Result<PlannedTask, AgentError> {
        debug!(
            agent_id = %self.id,
            intention_id = %intention_id,
            goal = %goal,
            "Creating plan for goal"
        );

        // Decompose goal into subgoals
        let subgoals = if self.use_llm {
            // TODO: Use LLM for intelligent decomposition when available
            self.decompose_goal(goal).await
        } else {
            self.decompose_goal(goal).await
        };

        let mut stats = self.stats.write().await;
        stats.plans_created += 1;
        stats.subgoals_generated += subgoals.len() as u64;
        drop(stats);

        let plan = PlannedTask::new(intention_id, goal.to_string())
            .with_subgoals(subgoals.clone())
            .with_strategy(PlanningStrategy::Sequential);

        // Store plan in thought graph
        let plan_data = serde_json::to_vec(&plan).unwrap_or_default();
        ctx.add_thought(ThoughtContent::new("plan", plan_data))
            .await?;

        // Create subgoal intentions
        for subgoal in &subgoals {
            let subgoal_id = ctx.set_intention(subgoal).await?;
            ctx.intentions()
                .add_subgoal(&intention_id, subgoal_id)
                .await
                .map_err(|e| AgentError::IntentionError(e.to_string()))?;
        }

        self.active_plans
            .write()
            .await
            .insert(intention_id, plan.clone());

        info!(
            agent_id = %self.id,
            intention_id = %intention_id,
            subgoals = subgoals.len(),
            "Plan created successfully"
        );

        Ok(plan)
    }

    /// Coordinate agents to execute a plan
    async fn coordinate_plan(
        &self,
        plan: &PlannedTask,
        ctx: &AgentContext,
    ) -> Result<(), AgentError> {
        debug!(
            agent_id = %self.id,
            intention_id = %plan.intention_id,
            "Coordinating plan execution"
        );

        let intentions = ctx.intentions();

        // Try to match and assign agents to subgoals
        let intention = intentions
            .get_intention(&plan.intention_id)
            .await
            .ok_or_else(|| {
                AgentError::IntentionError(format!(
                    "Intention not found: {}",
                    plan.intention_id
                ))
            })?;

        for subgoal_id in &intention.subgoals {
            match intentions.match_and_assign(subgoal_id).await {
                Ok(agent_id) => {
                    debug!(
                        planner_id = %self.id,
                        subgoal_id = %subgoal_id,
                        assigned_agent = %agent_id,
                        "Subgoal assigned to agent"
                    );
                    self.stats.write().await.coordination_events += 1;
                }
                Err(e) => {
                    warn!(
                        planner_id = %self.id,
                        subgoal_id = %subgoal_id,
                        error = %e,
                        "Failed to assign subgoal to agent"
                    );
                }
            }
        }

        Ok(())
    }

    /// Monitor plan progress and update status
    async fn monitor_plans(&self, ctx: &AgentContext) -> Result<(), AgentError> {
        let plans: Vec<PlannedTask> = self
            .active_plans
            .read()
            .await
            .values()
            .cloned()
            .collect();

        for plan in plans {
            let intention = ctx
                .intentions()
                .get_intention(&plan.intention_id)
                .await;

            if let Some(intention) = intention {
                // Check if all subgoals are completed
                let all_completed = self.check_subgoals_completed(&intention, ctx).await;

                if all_completed {
                    ctx.intentions()
                        .complete(&plan.intention_id)
                        .await
                        .map_err(|e| AgentError::IntentionError(e.to_string()))?;

                    self.active_plans.write().await.remove(&plan.intention_id);
                    self.stats.write().await.tasks_completed += 1;

                    info!(
                        agent_id = %self.id,
                        intention_id = %plan.intention_id,
                        "Plan completed successfully"
                    );

                    // Emit completion event
                    let payload = serde_json::to_vec(&serde_json::json!({
                        "planner_id": self.id.to_string(),
                        "intention_id": plan.intention_id.to_string(),
                        "goal": plan.goal,
                        "subgoals": plan.subgoals.len(),
                    }))
                    .unwrap_or_default();

                    ctx.emit_event("planner.plan_completed", payload).await?;
                }
            }
        }

        Ok(())
    }

    /// Check if all subgoals of an intention are completed
    async fn check_subgoals_completed(
        &self,
        intention: &Intention,
        ctx: &AgentContext,
    ) -> bool {
        for subgoal_id in &intention.subgoals {
            if let Some(subgoal) = ctx.intentions().get_intention(subgoal_id).await {
                if !matches!(subgoal.status, IntentionStatus::Completed) {
                    return false;
                }
            } else {
                return false;
            }
        }
        !intention.subgoals.is_empty()
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

        // Register capabilities with intention manager
        ctx.intentions()
            .register_agent_capabilities(*self.id(), self.capabilities.clone())
            .await;

        ctx.emit_event(
            "agent.started",
            serde_json::to_vec(&serde_json::json!({
                "agent_id": self.id.to_string(),
                "agent_name": self.name,
                "capabilities": vec!["planning", "task-decomposition", "agent-coordination", "goal-management"],
            }))
            .unwrap_or_default(),
        )
        .await?;

        Ok(())
    }

    async fn on_event(&mut self, event: &Event, ctx: &mut AgentContext) -> Result<(), AgentError> {
        // Listen for planning requests
        if event.kind == "planner.plan_request" {
            // Deserialize planning request
            if let Ok(request) = serde_json::from_slice::<serde_json::Value>(&event.payload) {
                if let Some(goal) = request.get("goal").and_then(|g| g.as_str()) {
                    debug!(
                        agent_id = %self.id,
                        goal = %goal,
                        "Received planning request"
                    );

                    // Create intention for this goal
                    let intention_id = ctx.set_intention(goal).await?;

                    // Create plan
                    let plan = self.create_plan(intention_id, goal, ctx).await?;

                    // Start coordination
                    self.coordinate_plan(&plan, ctx).await?;

                    // Emit plan created event
                    let payload = serde_json::to_vec(&serde_json::json!({
                        "planner_id": self.id.to_string(),
                        "intention_id": intention_id.to_string(),
                        "goal": goal,
                        "subgoals": plan.subgoals,
                        "strategy": format!("{:?}", plan.strategy),
                    }))
                    .unwrap_or_default();

                    ctx.emit_event("planner.plan_created", payload).await?;
                }
            }
        }

        Ok(())
    }

    async fn tick(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError> {
        // Monitor active plans
        self.monitor_plans(ctx).await?;

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError> {
        let stats = self.stats.read().await;

        info!(
            agent_id = %self.id,
            plans_created = stats.plans_created,
            tasks_completed = stats.tasks_completed,
            tasks_failed = stats.tasks_failed,
            "PlannerAgent shutting down"
        );

        ctx.emit_event(
            "agent.stopped",
            serde_json::to_vec(&serde_json::json!({
                "agent_id": self.id.to_string(),
                "agent_name": self.name,
                "stats": {
                    "plans_created": stats.plans_created,
                    "subgoals_generated": stats.subgoals_generated,
                    "tasks_completed": stats.tasks_completed,
                    "tasks_failed": stats.tasks_failed,
                    "coordination_events": stats.coordination_events,
                },
            }))
            .unwrap_or_default(),
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{EventBusHandle, GraphStoreHandle};
    use crate::intention::IntentionManager;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_planner_creation() {
        let planner = PlannerAgent::new();
        assert_eq!(planner.name(), "planner");
        assert!(planner.capabilities().has("planning"));
        assert!(planner.capabilities().has("task-decomposition"));
    }

    #[tokio::test]
    async fn test_goal_decomposition() {
        let planner = PlannerAgent::new();
        let subgoals = planner.decompose_goal("build a web server").await;
        assert!(!subgoals.is_empty());
        assert!(subgoals.len() >= 3);
    }

    #[tokio::test]
    async fn test_plan_creation() {
        let mut planner = PlannerAgent::new();
        
        let event_bus = EventBusHandle::new(100);
        // Subscribe to keep the bus alive
        let _sub = event_bus.subscribe();
        let graph = GraphStoreHandle::new();
        let intentions = IntentionManager::new();
        let (tx, _rx) = mpsc::channel(10);
        
        let mut ctx = AgentContext::new(event_bus, graph, intentions.clone(), tx);
        
        planner.init(&mut ctx).await.unwrap();
        
        let intention_id = ctx.set_intention("build HTTP server").await.unwrap();
        let plan = planner
            .create_plan(intention_id, "build HTTP server", &ctx)
            .await
            .unwrap();
        
        assert_eq!(plan.goal, "build HTTP server");
        assert!(!plan.subgoals.is_empty());
    }

    #[tokio::test]
    async fn test_planning_stats() {
        let mut planner = PlannerAgent::new();
        
        let event_bus = EventBusHandle::new(100);
        // Subscribe to keep the bus alive
        let _sub = event_bus.subscribe();
        let graph = GraphStoreHandle::new();
        let intentions = IntentionManager::new();
        let (tx, _rx) = mpsc::channel(10);
        
        let mut ctx = AgentContext::new(event_bus, graph, intentions.clone(), tx);
        planner.init(&mut ctx).await.unwrap();
        
        let intention_id = ctx.set_intention("test goal").await.unwrap();
        planner.create_plan(intention_id, "test goal", &ctx).await.unwrap();
        
        let stats = planner.stats().await;
        assert_eq!(stats.plans_created, 1);
        assert!(stats.subgoals_generated > 0);
    }
}
