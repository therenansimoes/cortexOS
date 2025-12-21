use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::peer::NodeId;

/// Metrics for task delegation
#[derive(Debug, Clone)]
pub struct TaskMetrics {
    /// Total tasks submitted
    pub total_submitted: u64,
    /// Total tasks completed successfully
    pub total_completed: u64,
    /// Total tasks failed
    pub total_failed: u64,
    /// Total tasks timed out
    pub total_timed_out: u64,
    /// Average execution time (ms)
    pub avg_execution_time_ms: u64,
    /// Min execution time (ms)
    pub min_execution_time_ms: u64,
    /// Max execution time (ms)
    pub max_execution_time_ms: u64,
    /// Per-node metrics
    pub per_node: HashMap<NodeId, NodeMetrics>,
}

impl Default for TaskMetrics {
    fn default() -> Self {
        Self {
            total_submitted: 0,
            total_completed: 0,
            total_failed: 0,
            total_timed_out: 0,
            avg_execution_time_ms: 0,
            min_execution_time_ms: u64::MAX,
            max_execution_time_ms: 0,
            per_node: HashMap::new(),
        }
    }
}

impl TaskMetrics {
    /// Success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_submitted == 0 {
            return 0.0;
        }
        self.total_completed as f64 / self.total_submitted as f64
    }

    /// Failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        if self.total_submitted == 0 {
            return 0.0;
        }
        self.total_failed as f64 / self.total_submitted as f64
    }

    /// Get metrics for a specific node
    pub fn node_metrics(&self, node: &NodeId) -> Option<&NodeMetrics> {
        self.per_node.get(node)
    }
}

/// Per-node task execution metrics
#[derive(Debug, Clone)]
pub struct NodeMetrics {
    /// Tasks assigned to this node
    pub tasks_assigned: u64,
    /// Tasks completed by this node
    pub tasks_completed: u64,
    /// Tasks failed on this node
    pub tasks_failed: u64,
    /// Average execution time on this node (ms)
    pub avg_execution_time_ms: u64,
    /// Last task completion time
    pub last_completed_at: Option<Instant>,
}

impl Default for NodeMetrics {
    fn default() -> Self {
        Self {
            tasks_assigned: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            avg_execution_time_ms: 0,
            last_completed_at: None,
        }
    }
}

impl NodeMetrics {
    /// Success rate for this node (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.tasks_assigned == 0 {
            return 0.0;
        }
        self.tasks_completed as f64 / self.tasks_assigned as f64
    }
}

/// Tracker for task execution metrics
pub struct MetricsTracker {
    metrics: Arc<RwLock<TaskMetrics>>,
    /// Execution times for calculating running average
    execution_times: Arc<RwLock<Vec<u64>>>,
}

impl MetricsTracker {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(TaskMetrics::default())),
            execution_times: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a task submission
    pub async fn record_submitted(&self, node: NodeId) {
        let mut metrics = self.metrics.write().await;
        metrics.total_submitted += 1;

        let node_metrics = metrics.per_node.entry(node).or_default();
        node_metrics.tasks_assigned += 1;
    }

    /// Record a task completion
    pub async fn record_completed(&self, node: NodeId, duration: Duration) {
        let duration_ms = duration.as_millis() as u64;
        let mut metrics = self.metrics.write().await;
        
        metrics.total_completed += 1;
        
        // Update execution time stats
        if duration_ms < metrics.min_execution_time_ms {
            metrics.min_execution_time_ms = duration_ms;
        }
        if duration_ms > metrics.max_execution_time_ms {
            metrics.max_execution_time_ms = duration_ms;
        }

        // Update node metrics
        let node_metrics = metrics.per_node.entry(node).or_default();
        node_metrics.tasks_completed += 1;
        node_metrics.last_completed_at = Some(Instant::now());

        // Update average execution time for node
        let total_time = node_metrics.avg_execution_time_ms * (node_metrics.tasks_completed - 1);
        node_metrics.avg_execution_time_ms = (total_time + duration_ms) / node_metrics.tasks_completed;

        // Update global average
        drop(metrics);
        let mut times = self.execution_times.write().await;
        times.push(duration_ms);
        let avg = times.iter().sum::<u64>() / times.len() as u64;
        self.metrics.write().await.avg_execution_time_ms = avg;
    }

    /// Record a task failure
    pub async fn record_failed(&self, node: NodeId) {
        let mut metrics = self.metrics.write().await;
        metrics.total_failed += 1;

        let node_metrics = metrics.per_node.entry(node).or_default();
        node_metrics.tasks_failed += 1;
    }

    /// Record a task timeout
    pub async fn record_timed_out(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_timed_out += 1;
    }

    /// Get current metrics snapshot
    pub async fn snapshot(&self) -> TaskMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        *self.metrics.write().await = TaskMetrics::default();
        self.execution_times.write().await.clear();
    }
}

impl Default for MetricsTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_tracking() {
        let tracker = MetricsTracker::new();
        let node = NodeId::random();

        tracker.record_submitted(node).await;
        tracker.record_completed(node, Duration::from_millis(100)).await;

        let metrics = tracker.snapshot().await;
        assert_eq!(metrics.total_submitted, 1);
        assert_eq!(metrics.total_completed, 1);
        assert_eq!(metrics.success_rate(), 1.0);
    }

    #[tokio::test]
    async fn test_node_metrics() {
        let tracker = MetricsTracker::new();
        let node = NodeId::random();

        tracker.record_submitted(node).await;
        tracker.record_submitted(node).await;
        tracker.record_completed(node, Duration::from_millis(100)).await;
        tracker.record_failed(node).await;

        let metrics = tracker.snapshot().await;
        let node_metrics = metrics.node_metrics(&node).unwrap();
        
        assert_eq!(node_metrics.tasks_assigned, 2);
        assert_eq!(node_metrics.tasks_completed, 1);
        assert_eq!(node_metrics.tasks_failed, 1);
        assert_eq!(node_metrics.success_rate(), 0.5);
    }

    #[tokio::test]
    async fn test_execution_time_stats() {
        let tracker = MetricsTracker::new();
        let node = NodeId::random();

        tracker.record_submitted(node).await;
        tracker.record_completed(node, Duration::from_millis(100)).await;
        
        tracker.record_submitted(node).await;
        tracker.record_completed(node, Duration::from_millis(200)).await;

        let metrics = tracker.snapshot().await;
        assert_eq!(metrics.avg_execution_time_ms, 150);
        assert_eq!(metrics.min_execution_time_ms, 100);
        assert_eq!(metrics.max_execution_time_ms, 200);
    }
}
