use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn, error};

use cortex_core::event::{Event, Payload};
use cortex_core::runtime::EventBus;
use cortex_grid::{
    GridError, GridOrchestrator, Message, MetricsTracker, NodeId, QueuedTask,
    TaskPriority, TaskQueue, TaskStatus as GridTaskStatus,
};
use cortex_reputation::{SkillId, TrustGraph};

use crate::definition::SkillInput;
use crate::error::{SkillError, Result};
use crate::executor::SkillExecutor;
use crate::registry::{LocalSkillRegistry, NetworkSkillRegistry};
use crate::router::SkillRouter;
use crate::task::{SkillTask, TaskId, TaskResult, TaskStatus};

const MAX_RETRIES: u32 = 3;
const TASK_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// Coordinates task delegation across the Grid
pub struct DelegationCoordinator {
    my_id: NodeId,
    /// Grid orchestrator for network communication
    orchestrator: Arc<GridOrchestrator>,
    /// Skill executor for local execution
    executor: Arc<SkillExecutor>,
    /// Skill router for finding best node
    router: Arc<SkillRouter>,
    /// Task queue
    task_queue: Arc<TaskQueue>,
    /// Metrics tracker
    metrics: Arc<MetricsTracker>,
    /// Event bus for publishing results
    event_bus: Arc<EventBus>,
    /// Active tasks (TaskId -> SkillTask)
    active_tasks: Arc<RwLock<HashMap<TaskId, SkillTask>>>,
    /// Task results waiting for collection
    results: Arc<RwLock<HashMap<TaskId, TaskResult>>>,
    /// Shutdown signal
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

impl DelegationCoordinator {
    pub fn new(
        my_id: NodeId,
        orchestrator: Arc<GridOrchestrator>,
        executor: Arc<SkillExecutor>,
        router: Arc<SkillRouter>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Self {
            my_id,
            orchestrator,
            executor,
            router,
            task_queue: Arc::new(TaskQueue::new(1000)),
            metrics: Arc::new(MetricsTracker::new()),
            event_bus,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
        }
    }

    /// Submit a task for delegation
    pub async fn submit_task(&self, task: SkillTask) -> Result<TaskId> {
        info!("Submitting task {} for skill {}", task.id, task.skill);

        // Route the task to find the best node
        let route_decision = self.router.route(&task).await?;
        let target_node = route_decision.node;

        debug!(
            "Routed task {} to node {} (score: {:.2})",
            task.id, target_node, route_decision.route_score
        );

        // Record metrics
        self.metrics.record_submitted(target_node).await;

        // Store task
        self.active_tasks.write().await.insert(task.id, task.clone());

        // Encode task payload
        let payload = bincode::serialize(&task)
            .map_err(|e| SkillError::SerializationError(e.to_string()))?;

        // Generate Grid task ID from SkillTask ID
        let grid_task_id = task_id_to_bytes(&task.id);

        // Create queued task
        let queued_task = QueuedTask {
            task_id: grid_task_id,
            payload,
            priority: TaskPriority::from(task.priority),
            target_node: Some(target_node),
            retries: 0,
            created_at: Instant::now(),
        };

        // Enqueue the task
        if !self.task_queue.enqueue(queued_task).await {
            return Err(SkillError::QueueFull);
        }

        // If target is self, execute locally
        if target_node == self.my_id {
            info!("Task {} assigned to self, executing locally", task.id);
            let result = self.executor.execute_task(task).await;
            self.handle_result(result).await?;
        } else {
            // Delegate to remote node via orchestrator
            self.orchestrator
                .delegate_task(grid_task_id, payload)
                .await
                .map_err(|e| SkillError::DelegationFailed(e.to_string()))?;
        }

        Ok(task.id)
    }

    /// Handle a task result (local or remote)
    async fn handle_result(&self, result: TaskResult) -> Result<()> {
        let task_id = result.task_id;
        let success = result.success;

        // Remove from active tasks
        if let Some(task) = self.active_tasks.write().await.remove(&task_id) {
            // Record metrics
            if success {
                let duration = std::time::Duration::from_millis(result.duration_ms);
                self.metrics.record_completed(result.executor, duration).await;
            } else {
                self.metrics.record_failed(result.executor).await;
            }

            // Complete in queue
            let grid_task_id = task_id_to_bytes(&task_id);
            self.task_queue.complete(&grid_task_id).await;

            // Store result
            self.results.write().await.insert(task_id, result.clone());

            // Publish event
            let event_kind = if success {
                "skill.task.completed"
            } else {
                "skill.task.failed"
            };

            let event_payload = serde_json::to_vec(&result)
                .unwrap_or_default();

            let event = Event::new("delegation.coordinator", event_kind, Payload::inline(event_payload));
            let _ = self.event_bus.publish(event);

            info!("Task {} {}", task_id, if success { "completed" } else { "failed" });
        }

        Ok(())
    }

    /// Get result for a completed task
    pub async fn get_result(&self, task_id: &TaskId) -> Option<TaskResult> {
        self.results.read().await.get(task_id).cloned()
    }

    /// Get metrics snapshot
    pub async fn metrics(&self) -> cortex_grid::TaskMetrics {
        self.metrics.snapshot().await
    }

    /// Start the delegation coordinator
    pub async fn start(&mut self) -> Result<()> {
        let shutdown_rx = self.shutdown_rx.take()
            .ok_or_else(|| SkillError::ExecutionFailed("Coordinator already started".to_string()))?;

        // Spawn timeout checker
        let task_queue = Arc::clone(&self.task_queue);
        let metrics = Arc::clone(&self.metrics);
        let active_tasks = Arc::clone(&self.active_tasks);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;

                // Check for timed-out tasks
                let timed_out = task_queue.cleanup_timeouts(TASK_TIMEOUT_SECS).await;
                
                for grid_task_id in timed_out {
                    metrics.record_timed_out().await;
                    
                    // Try to find and remove from active tasks
                    let mut tasks = active_tasks.write().await;
                    tasks.retain(|_, task| {
                        let task_grid_id = task_id_to_bytes(&task.id);
                        task_grid_id != grid_task_id
                    });
                }
            }
        });

        // Spawn task processor
        let task_queue_proc = Arc::clone(&self.task_queue);
        let orchestrator_proc = Arc::clone(&self.orchestrator);
        let active_tasks_proc = Arc::clone(&self.active_tasks);
        let executor_proc = Arc::clone(&self.executor);
        let my_id_proc = self.my_id;
        let results_proc = Arc::clone(&self.results);
        let event_bus_proc = Arc::clone(&self.event_bus);
        let metrics_proc = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        // Try to process next task
                        if let Some(queued_task) = task_queue_proc.dequeue().await {
                            if let Some(target_node) = queued_task.target_node {
                                // Decode skill task
                                if let Ok(skill_task) = bincode::deserialize::<SkillTask>(&queued_task.payload) {
                                    if target_node == my_id_proc {
                                        // Execute locally
                                        let result = executor_proc.execute_task(skill_task).await;
                                        
                                        // Handle result
                                        let task_id = result.task_id;
                                        let success = result.success;
                                        
                                        active_tasks_proc.write().await.remove(&task_id);
                                        
                                        if success {
                                            let duration = std::time::Duration::from_millis(result.duration_ms);
                                            metrics_proc.record_completed(result.executor, duration).await;
                                        } else {
                                            metrics_proc.record_failed(result.executor).await;
                                        }
                                        
                                        results_proc.write().await.insert(task_id, result.clone());
                                        
                                        let event_kind = if success {
                                            "skill.task.completed"
                                        } else {
                                            "skill.task.failed"
                                        };
                                        
                                        let event_payload = serde_json::to_vec(&result).unwrap_or_default();
                                        let event = Event::new("delegation.coordinator", event_kind, Payload::inline(event_payload));
                                        let _ = event_bus_proc.publish(event);
                                    }
                                    // Remote execution is handled by orchestrator
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Delegation coordinator shutting down");
                        break;
                    }
                }
            }
        });

        info!("Delegation coordinator started");
        Ok(())
    }

    /// Stop the coordinator
    pub async fn stop(&self) -> Result<()> {
        self.shutdown_tx
            .send(())
            .await
            .map_err(|_| SkillError::ExecutionFailed("Failed to send shutdown signal".to_string()))?;
        Ok(())
    }

    /// Get queue statistics
    pub async fn queue_stats(&self) -> cortex_grid::TaskQueueStats {
        self.task_queue.stats().await
    }

    /// Get count of active tasks
    pub async fn active_count(&self) -> usize {
        self.active_tasks.read().await.len()
    }
}

/// Convert TaskId (UUID) to [u8; 32] for Grid
fn task_id_to_bytes(task_id: &TaskId) -> [u8; 32] {
    let uuid_bytes = task_id.0.as_bytes();
    let mut result = [0u8; 32];
    result[..16].copy_from_slice(uuid_bytes);
    // Hash the rest to fill 32 bytes
    let hash = blake3::hash(uuid_bytes);
    result[16..].copy_from_slice(&hash.as_bytes()[..16]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_grid::PeerStore;
    use std::time::Duration;

    #[tokio::test]
    async fn test_delegation_coordinator_creation() {
        let my_id = NodeId::random();
        let peer_store = PeerStore::new(Duration::from_secs(60));
        let event_bus = Arc::new(EventBus::default());
        let trust_graph = Arc::new(RwLock::new(TrustGraph::new(my_id)));
        
        let orchestrator = Arc::new(GridOrchestrator::new(
            my_id,
            peer_store.clone(),
            Arc::clone(&event_bus),
        ));
        
        let local_skills = Arc::new(RwLock::new(LocalSkillRegistry::new()));
        let executor = Arc::new(SkillExecutor::new(
            my_id,
            Arc::clone(&local_skills),
            Arc::clone(&trust_graph),
        ));
        
        let network_skills = Arc::new(RwLock::new(NetworkSkillRegistry::new(my_id)));
        let router = Arc::new(SkillRouter::new(
            my_id,
            Arc::clone(&trust_graph),
            Arc::clone(&network_skills),
        ));
        
        let coordinator = DelegationCoordinator::new(
            my_id,
            orchestrator,
            executor,
            router,
            event_bus,
        );
        
        assert_eq!(coordinator.active_count().await, 0);
    }
}
