use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

use cortex_core::event::{Event, Payload};
use cortex_core::runtime::EventBus;
use cortex_grid::{
    GridOrchestrator, MetricsTracker, NodeId, QueuedTask,
    TaskPriority, TaskQueue,
};
use crate::error::{SkillError, Result};
use crate::executor::SkillExecutor;
use crate::router::SkillRouter;
use crate::task::{SkillTask, TaskId, TaskResult};

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
    shutdown_tx: broadcast::Sender<()>,
}

impl DelegationCoordinator {
    pub fn new(
        my_id: NodeId,
        orchestrator: Arc<GridOrchestrator>,
        executor: Arc<SkillExecutor>,
        router: Arc<SkillRouter>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

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
        }
    }

    /// Submit a task for delegation
    pub async fn submit_task(&self, task: SkillTask) -> Result<TaskId> {
        info!("Submitting task {} for skill {}", task.id, task.skill);

        // Save task ID before any moves
        let task_id = task.id;

        // Route the task to find the best node
        let route_decision = self.router.route(&task).await?;
        let target_node = route_decision.node;

        debug!(
            "Routed task {} to node {} (score: {:.2})",
            task_id, target_node, route_decision.route_score
        );

        // Record metrics
        self.metrics.record_submitted(target_node).await;

        // Store task
        self.active_tasks.write().await.insert(task_id, task.clone());

        // Encode task payload
        let payload = bincode::serialize(&task)
            .map_err(|e| SkillError::SerializationError(e.to_string()))?;

        // Generate Grid task ID from SkillTask ID
        let grid_task_id = task_id_to_bytes(&task_id);

        // Create queued task
        let queued_task = QueuedTask {
            task_id: grid_task_id,
            payload: payload.clone(),
            priority: TaskPriority::from(task.priority),
            target_node: Some(target_node),
            retries: 0,
            created_at: Instant::now(),
        };

        // If target is self, execute locally without enqueueing
        if target_node == self.my_id {
            info!("Task {} assigned to self, executing locally", task_id);
            let result = self.executor.execute_task(task).await;
            self.handle_result(result).await?;
        } else {
            // Enqueue the task for remote processing
            if !self.task_queue.enqueue(queued_task).await {
                return Err(SkillError::QueueFull);
            }
            
            // Delegate to remote node via orchestrator
            self.orchestrator
                .delegate_task(grid_task_id, payload)
                .await
                .map_err(|e| SkillError::DelegationFailed(e.to_string()))?;
        }

        Ok(task_id)
    }

    /// Handle a task result (local or remote)
    async fn handle_result(&self, result: TaskResult) -> Result<()> {
        handle_task_result(
            result,
            &self.active_tasks,
            &self.task_queue,
            &self.results,
            &self.metrics,
            &self.event_bus,
        ).await;
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
        // Spawn timeout checker
        let task_queue = Arc::clone(&self.task_queue);
        let metrics = Arc::clone(&self.metrics);
        let active_tasks = Arc::clone(&self.active_tasks);
        let mut shutdown_rx_timeout = self.shutdown_tx.subscribe();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
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
                    _ = shutdown_rx_timeout.recv() => {
                        info!("Timeout checker shutting down");
                        break;
                    }
                }
            }
        });

        // Spawn task processor
        let task_queue_proc = Arc::clone(&self.task_queue);
        let active_tasks_proc = Arc::clone(&self.active_tasks);
        let executor_proc = Arc::clone(&self.executor);
        let my_id_proc = self.my_id;
        let results_proc = Arc::clone(&self.results);
        let event_bus_proc = Arc::clone(&self.event_bus);
        let metrics_proc = Arc::clone(&self.metrics);
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
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
                                        
                                        // Handle result using shared helper
                                        handle_task_result(
                                            result,
                                            &active_tasks_proc,
                                            &task_queue_proc,
                                            &results_proc,
                                            &metrics_proc,
                                            &event_bus_proc,
                                        ).await;
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
        let _ = self.shutdown_tx.send(());
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

/// Helper function to handle task results (shared between submit_task and task processor)
async fn handle_task_result(
    result: TaskResult,
    active_tasks: &Arc<RwLock<HashMap<TaskId, SkillTask>>>,
    task_queue: &Arc<TaskQueue>,
    results: &Arc<RwLock<HashMap<TaskId, TaskResult>>>,
    metrics: &Arc<MetricsTracker>,
    event_bus: &Arc<EventBus>,
) {
    let task_id = result.task_id;
    let success = result.success;

    // Remove from active tasks
    if active_tasks.write().await.remove(&task_id).is_some() {
        // Record metrics
        if success {
            let duration = std::time::Duration::from_millis(result.duration_ms);
            metrics.record_completed(result.executor, duration).await;
        } else {
            metrics.record_failed(result.executor).await;
        }

        // Complete in queue
        let grid_task_id = task_id_to_bytes(&task_id);
        task_queue.complete(&grid_task_id).await;

        // Store result
        results.write().await.insert(task_id, result.clone());

        // Publish event
        let event_kind = if success {
            "skill.task.completed"
        } else {
            "skill.task.failed"
        };

        let event_payload = serde_json::to_vec(&result).unwrap_or_default();
        let event = Event::new("delegation.coordinator", event_kind, Payload::inline(event_payload));
        let _ = event_bus.publish(event);

        info!("Task {} {}", task_id, if success { "completed" } else { "failed" });
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
    use cortex_reputation::TrustGraph;
    use crate::registry::{LocalSkillRegistry, NetworkSkillRegistry};
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
