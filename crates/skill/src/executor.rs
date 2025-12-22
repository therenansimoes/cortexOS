use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info, error};

use cortex_grid::NodeId;
use cortex_reputation::{Rating, SkillId, TrustGraph};

use crate::definition::{SkillInput, SkillOutput};
use crate::registry::LocalSkillRegistry;
use crate::task::{SkillTask, TaskResult};
use crate::error::{SkillError, Result};

/// Context for skill execution
pub struct ExecutionContext {
    /// Who requested this
    pub requester: NodeId,
    /// Current node
    pub executor: NodeId,
    /// Trust graph for looking up other nodes
    pub trust_graph: Arc<RwLock<TrustGraph>>,
}

/// Result of execution with metadata
#[derive(Debug)]
pub struct ExecutionResult {
    pub output: SkillOutput,
    pub duration_ms: u64,
    pub success: bool,
}

/// Executes skills locally
pub struct SkillExecutor {
    my_id: NodeId,
    local_skills: Arc<RwLock<LocalSkillRegistry>>,
    _trust_graph: Arc<RwLock<TrustGraph>>,
}

impl SkillExecutor {
    pub fn new(
        my_id: NodeId,
        local_skills: Arc<RwLock<LocalSkillRegistry>>,
        trust_graph: Arc<RwLock<TrustGraph>>,
    ) -> Self {
        Self {
            my_id,
            local_skills,
            _trust_graph: trust_graph,
        }
    }

    /// Execute a skill locally
    pub async fn execute(&self, skill_id: &SkillId, input: SkillInput) -> Result<ExecutionResult> {
        let start = Instant::now();

        let skills = self.local_skills.read().await;
        let skill = skills
            .get(skill_id)
            .ok_or_else(|| SkillError::SkillNotFound(skill_id.to_string()))?;

        if !skill.can_execute() {
            return Err(SkillError::ExecutionFailed(format!(
                "Skill {} cannot execute on this node",
                skill_id
            )));
        }

        debug!("Executing skill: {}", skill_id);

        match skill.execute(input).await {
            Ok(output) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                info!(
                    "Skill {} executed successfully in {}ms",
                    skill_id, duration_ms
                );
                Ok(ExecutionResult {
                    output,
                    duration_ms,
                    success: true,
                })
            }
            Err(e) => {
                error!("Skill {} execution failed: {}", skill_id, e);
                Err(e)
            }
        }
    }

    /// Execute a task and report result
    pub async fn execute_task(&self, mut task: SkillTask) -> TaskResult {
        let _start = Instant::now();

        task.start(self.my_id);

        match self.execute(&task.skill, task.input.clone()).await {
            Ok(result) => {
                task.complete();
                TaskResult::success(task.id, result.output, self.my_id, result.duration_ms)
            }
            Err(e) => {
                task.fail(&e.to_string());
                TaskResult::failure(task.id, &e.to_string(), self.my_id)
            }
        }
    }

    /// Execute and rate the result (for when receiving from network)
    pub async fn execute_and_rate(
        &self,
        task: SkillTask,
        rate_self: bool,
    ) -> (TaskResult, Option<Rating>) {
        let result = self.execute_task(task.clone()).await;

        // Auto-rate based on success/failure
        let rating = if rate_self {
            None // Don't rate self
        } else {
            Some(if result.success {
                Rating::positive()
            } else {
                Rating::negative()
            })
        };

        (result, rating)
    }

    /// Get list of skills this executor can handle
    pub async fn available_skills(&self) -> Vec<SkillId> {
        self.local_skills.read().await.list_skills()
    }
}

/// Remote executor - sends tasks to other nodes
pub struct RemoteExecutor {
    _my_id: NodeId,
    trust_graph: Arc<RwLock<TrustGraph>>,
}

impl RemoteExecutor {
    pub fn new(my_id: NodeId, trust_graph: Arc<RwLock<TrustGraph>>) -> Self {
        Self { _my_id: my_id, trust_graph }
    }

    /// Rate a remote execution result
    pub async fn rate_execution(
        &self,
        executor: NodeId,
        skill: SkillId,
        success: bool,
    ) -> Result<()> {
        let rating = if success {
            Rating::positive()
        } else {
            Rating::negative()
        };

        let graph = self.trust_graph.write().await;
        graph.rate(executor, skill.clone(), rating)?;

        info!(
            "Rated {} for skill {}: {}",
            executor,
            skill.as_str(),
            if success { "+1" } else { "-1" }
        );

        Ok(())
    }
}
