use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::IntentionError;
use crate::types::{AgentId, CapabilitySet, IntentionId, Timestamp};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intention {
    pub id: IntentionId,
    pub goal: String,
    pub status: IntentionStatus,
    pub created_at: Timestamp,
    pub subgoals: Vec<IntentionId>,
    pub assigned_agent: Option<AgentId>,
}

impl Intention {
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            id: IntentionId::new(),
            goal: goal.into(),
            status: IntentionStatus::Pending,
            created_at: Timestamp::now(),
            subgoals: Vec::new(),
            assigned_agent: None,
        }
    }

    pub fn with_subgoal(mut self, subgoal: IntentionId) -> Self {
        self.subgoals.push(subgoal);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentionStatus {
    Pending,
    InProgress,
    Completed,
    Failed { reason: String },
    Blocked { waiting_for: String },
}

impl IntentionStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            IntentionStatus::Completed | IntentionStatus::Failed { .. }
        )
    }
}

pub struct IntentionManager {
    intentions: Arc<RwLock<HashMap<IntentionId, Intention>>>,
    agent_capabilities: Arc<RwLock<HashMap<AgentId, CapabilitySet>>>,
}

impl IntentionManager {
    pub fn new() -> Self {
        Self {
            intentions: Arc::new(RwLock::new(HashMap::new())),
            agent_capabilities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_intention(&self, intention: Intention) -> IntentionId {
        let id = intention.id;
        self.intentions.write().await.insert(id, intention);
        id
    }

    pub async fn create_intention(&self, goal: impl Into<String>) -> IntentionId {
        let intention = Intention::new(goal);
        self.register_intention(intention).await
    }

    pub async fn get_intention(&self, id: &IntentionId) -> Option<Intention> {
        self.intentions.read().await.get(id).cloned()
    }

    pub async fn update_status(
        &self,
        id: &IntentionId,
        status: IntentionStatus,
    ) -> Result<(), IntentionError> {
        let mut intentions = self.intentions.write().await;
        let intention = intentions
            .get_mut(id)
            .ok_or_else(|| IntentionError::NotFound(*id))?;

        if intention.status.is_terminal() {
            return Err(IntentionError::AlreadyCompleted(*id));
        }

        intention.status = status;
        Ok(())
    }

    pub async fn complete(&self, id: &IntentionId) -> Result<(), IntentionError> {
        self.update_status(id, IntentionStatus::Completed).await
    }

    pub async fn fail(
        &self,
        id: &IntentionId,
        reason: impl Into<String>,
    ) -> Result<(), IntentionError> {
        self.update_status(
            id,
            IntentionStatus::Failed {
                reason: reason.into(),
            },
        )
        .await
    }

    pub async fn block(
        &self,
        id: &IntentionId,
        waiting_for: impl Into<String>,
    ) -> Result<(), IntentionError> {
        self.update_status(
            id,
            IntentionStatus::Blocked {
                waiting_for: waiting_for.into(),
            },
        )
        .await
    }

    pub async fn add_subgoal(
        &self,
        parent_id: &IntentionId,
        subgoal: IntentionId,
    ) -> Result<(), IntentionError> {
        let mut intentions = self.intentions.write().await;
        let parent = intentions
            .get_mut(parent_id)
            .ok_or_else(|| IntentionError::NotFound(*parent_id))?;

        parent.subgoals.push(subgoal);
        Ok(())
    }

    pub async fn register_agent_capabilities(
        &self,
        agent_id: AgentId,
        capabilities: CapabilitySet,
    ) {
        self.agent_capabilities
            .write()
            .await
            .insert(agent_id, capabilities);
    }

    pub async fn unregister_agent(&self, agent_id: &AgentId) {
        self.agent_capabilities.write().await.remove(agent_id);
    }

    pub async fn find_matching_agent(&self, goal: &str) -> Option<AgentId> {
        let capabilities = self.agent_capabilities.read().await;

        for (agent_id, caps) in capabilities.iter() {
            for cap in caps.iter() {
                if goal.to_lowercase().contains(&cap.to_lowercase()) {
                    return Some(*agent_id);
                }
            }
        }

        None
    }

    pub async fn assign_agent(
        &self,
        intention_id: &IntentionId,
        agent_id: AgentId,
    ) -> Result<(), IntentionError> {
        let mut intentions = self.intentions.write().await;
        let intention = intentions
            .get_mut(intention_id)
            .ok_or_else(|| IntentionError::NotFound(*intention_id))?;

        intention.assigned_agent = Some(agent_id);
        intention.status = IntentionStatus::InProgress;
        Ok(())
    }

    pub async fn list_pending(&self) -> Vec<Intention> {
        self.intentions
            .read()
            .await
            .values()
            .filter(|i| i.status == IntentionStatus::Pending)
            .cloned()
            .collect()
    }

    pub async fn match_and_assign(
        &self,
        intention_id: &IntentionId,
    ) -> Result<AgentId, IntentionError> {
        let intention = self
            .get_intention(intention_id)
            .await
            .ok_or_else(|| IntentionError::NotFound(*intention_id))?;

        let agent_id = self
            .find_matching_agent(&intention.goal)
            .await
            .ok_or_else(|| IntentionError::NoMatchingAgent(intention.goal.clone()))?;

        self.assign_agent(intention_id, agent_id).await?;
        Ok(agent_id)
    }
}

impl Default for IntentionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for IntentionManager {
    fn clone(&self) -> Self {
        Self {
            intentions: Arc::clone(&self.intentions),
            agent_capabilities: Arc::clone(&self.agent_capabilities),
        }
    }
}
