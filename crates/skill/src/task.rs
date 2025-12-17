use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use cortex_grid::NodeId;
use cortex_reputation::SkillId;

use crate::definition::{SkillInput, SkillOutput};

/// Unique task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a skill task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    /// Task is waiting to be assigned
    Pending,
    /// Task has been assigned to a node
    Assigned { node: NodeId },
    /// Task is currently executing
    Running { node: NodeId, progress: u8 },
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed { reason: String },
    /// Task was cancelled
    Cancelled,
    /// Task timed out
    TimedOut,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed | TaskStatus::Failed { .. } | TaskStatus::Cancelled | TaskStatus::TimedOut
        )
    }
}

/// A task requesting execution of a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTask {
    /// Unique task ID
    pub id: TaskId,
    /// Which skill to execute
    pub skill: SkillId,
    /// Input for the skill
    pub input: SkillInput,
    /// Who requested the task
    pub requester: NodeId,
    /// Who is executing (if assigned)
    pub executor: Option<NodeId>,
    /// Current status
    pub status: TaskStatus,
    /// When the task was created
    pub created_at: u64,
    /// When the task was last updated
    pub updated_at: u64,
    /// Timeout in seconds
    pub timeout_secs: u32,
    /// Priority (higher = more important)
    pub priority: u8,
    /// Minimum trust score required for executor
    pub min_trust: f32,
}

impl SkillTask {
    pub fn new(skill: SkillId, input: SkillInput, requester: NodeId) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: TaskId::new(),
            skill,
            input,
            requester,
            executor: None,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            timeout_secs: 300, // 5 minutes default
            priority: 5,
            min_trust: 0.3,
        }
    }

    pub fn with_timeout(mut self, secs: u32) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_min_trust(mut self, min_trust: f32) -> Self {
        self.min_trust = min_trust.clamp(0.0, 1.0);
        self
    }

    pub fn assign(&mut self, node: NodeId) {
        self.executor = Some(node);
        self.status = TaskStatus::Assigned { node };
        self.update_timestamp();
    }

    pub fn start(&mut self, node: NodeId) {
        self.executor = Some(node);
        self.status = TaskStatus::Running { node, progress: 0 };
        self.update_timestamp();
    }

    pub fn update_progress(&mut self, progress: u8) {
        if let TaskStatus::Running { node, .. } = &self.status {
            self.status = TaskStatus::Running {
                node: *node,
                progress: progress.min(100),
            };
            self.update_timestamp();
        }
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.update_timestamp();
    }

    pub fn fail(&mut self, reason: &str) {
        self.status = TaskStatus::Failed {
            reason: reason.to_string(),
        };
        self.update_timestamp();
    }

    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.update_timestamp();
    }

    fn update_timestamp(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.created_at + self.timeout_secs as u64
    }
}

/// Result of a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: TaskId,
    /// Whether it succeeded
    pub success: bool,
    /// Output (if successful)
    pub output: Option<SkillOutput>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution time in ms
    pub duration_ms: u64,
    /// Who executed it
    pub executor: NodeId,
}

impl TaskResult {
    pub fn success(task_id: TaskId, output: SkillOutput, executor: NodeId, duration_ms: u64) -> Self {
        Self {
            task_id,
            success: true,
            output: Some(output),
            error: None,
            duration_ms,
            executor,
        }
    }

    pub fn failure(task_id: TaskId, error: &str, executor: NodeId) -> Self {
        Self {
            task_id,
            success: false,
            output: None,
            error: Some(error.to_string()),
            duration_ms: 0,
            executor,
        }
    }
}
