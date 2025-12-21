use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::peer::NodeId;

/// Priority levels for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl From<u8> for TaskPriority {
    /// Convert u8 priority to TaskPriority with proper bucketing.
    /// Maps: 0-63 -> Low, 64-127 -> Normal, 128-191 -> High, 192-255 -> Critical
    fn from(value: u8) -> Self {
        match value {
            0..=63 => TaskPriority::Low,
            64..=127 => TaskPriority::Normal,
            128..=191 => TaskPriority::High,
            192..=255 => TaskPriority::Critical,
        }
    }
}

/// A queued task with metadata
#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub task_id: [u8; 32],
    pub payload: Vec<u8>,
    pub priority: TaskPriority,
    pub target_node: Option<NodeId>,
    pub retries: u32,
    pub created_at: std::time::Instant,
}

/// Task queue manager with priority support
/// 
/// **Backpressure Policy**: drop_new
/// When a priority queue reaches max_queue_size, new tasks for that priority
/// are dropped (rejected) and enqueu returns false. This prevents memory exhaustion
/// while maintaining fairness across priority levels.
pub struct TaskQueue {
    /// Priority queues (Critical -> High -> Normal -> Low)
    queues: Arc<RwLock<HashMap<TaskPriority, VecDeque<QueuedTask>>>>,
    /// In-flight tasks (task_id -> task)
    in_flight: Arc<RwLock<HashMap<[u8; 32], QueuedTask>>>,
    /// Maximum queue size per priority
    max_queue_size: usize,
}

impl TaskQueue {
    pub fn new(max_queue_size: usize) -> Self {
        let mut queues = HashMap::new();
        queues.insert(TaskPriority::Low, VecDeque::new());
        queues.insert(TaskPriority::Normal, VecDeque::new());
        queues.insert(TaskPriority::High, VecDeque::new());
        queues.insert(TaskPriority::Critical, VecDeque::new());

        Self {
            queues: Arc::new(RwLock::new(queues)),
            in_flight: Arc::new(RwLock::new(HashMap::new())),
            max_queue_size,
        }
    }

    /// Enqueue a task
    pub async fn enqueue(&self, task: QueuedTask) -> bool {
        let mut queues = self.queues.write().await;
        
        if let Some(queue) = queues.get_mut(&task.priority) {
            if queue.len() >= self.max_queue_size {
                warn!(
                    "Task queue full for priority {:?}, dropping task {}",
                    task.priority,
                    hex_id(&task.task_id)
                );
                return false;
            }

            debug!(
                "Enqueued task {} with priority {:?}",
                hex_id(&task.task_id),
                task.priority
            );
            queue.push_back(task);
            true
        } else {
            false
        }
    }

    /// Dequeue the highest priority task
    pub async fn dequeue(&self) -> Option<QueuedTask> {
        let mut queues = self.queues.write().await;

        // Try in priority order
        for priority in [
            TaskPriority::Critical,
            TaskPriority::High,
            TaskPriority::Normal,
            TaskPriority::Low,
        ] {
            if let Some(queue) = queues.get_mut(&priority) {
                if let Some(task) = queue.pop_front() {
                    debug!(
                        "Dequeued task {} with priority {:?}",
                        hex_id(&task.task_id),
                        priority
                    );
                    
                    // Move to in-flight
                    self.in_flight.write().await.insert(task.task_id, task.clone());
                    return Some(task);
                }
            }
        }

        None
    }

    /// Mark a task as completed and remove from in-flight
    pub async fn complete(&self, task_id: &[u8; 32]) -> Option<QueuedTask> {
        let result = self.in_flight.write().await.remove(task_id);
        if result.is_some() {
            info!("Task {} completed", hex_id(task_id));
        }
        result
    }

    /// Mark a task as failed and optionally re-queue
    pub async fn fail(&self, task_id: &[u8; 32], requeue: bool) -> Option<QueuedTask> {
        if let Some(mut task) = self.in_flight.write().await.remove(task_id) {
            info!("Task {} failed", hex_id(task_id));
            
            if requeue {
                task.retries += 1;
                if self.enqueue(task.clone()).await {
                    info!("Re-queued task {} (attempt {})", hex_id(task_id), task.retries + 1);
                }
            }
            
            Some(task)
        } else {
            None
        }
    }

    /// Get pending task count across all priorities
    pub async fn pending_count(&self) -> usize {
        let queues = self.queues.read().await;
        queues.values().map(|q| q.len()).sum()
    }

    /// Get in-flight task count
    pub async fn in_flight_count(&self) -> usize {
        self.in_flight.read().await.len()
    }

    /// Get queue statistics
    pub async fn stats(&self) -> TaskQueueStats {
        let queues = self.queues.read().await;
        let in_flight = self.in_flight.read().await.len();

        TaskQueueStats {
            low_priority: queues.get(&TaskPriority::Low).map_or(0, |q| q.len()),
            normal_priority: queues.get(&TaskPriority::Normal).map_or(0, |q| q.len()),
            high_priority: queues.get(&TaskPriority::High).map_or(0, |q| q.len()),
            critical_priority: queues.get(&TaskPriority::Critical).map_or(0, |q| q.len()),
            in_flight,
        }
    }

    /// Remove timed-out tasks from in-flight
    pub async fn cleanup_timeouts(&self, timeout_secs: u64) -> Vec<[u8; 32]> {
        let mut in_flight = self.in_flight.write().await;
        let now = std::time::Instant::now();
        let mut timed_out = Vec::new();

        in_flight.retain(|task_id, task| {
            if now.duration_since(task.created_at).as_secs() > timeout_secs {
                warn!("Task {} timed out", hex_id(task_id));
                timed_out.push(*task_id);
                false
            } else {
                true
            }
        });

        timed_out
    }
}

#[derive(Debug, Clone)]
pub struct TaskQueueStats {
    pub low_priority: usize,
    pub normal_priority: usize,
    pub high_priority: usize,
    pub critical_priority: usize,
    pub in_flight: usize,
}

impl TaskQueueStats {
    pub fn total_queued(&self) -> usize {
        self.low_priority + self.normal_priority + self.high_priority + self.critical_priority
    }
}

fn hex_id(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_queue_priority() {
        let queue = TaskQueue::new(100);

        // Enqueue tasks with different priorities
        let low_task = QueuedTask {
            task_id: [1u8; 32],
            payload: vec![],
            priority: TaskPriority::Low,
            target_node: None,
            retries: 0,
            created_at: std::time::Instant::now(),
        };

        let high_task = QueuedTask {
            task_id: [2u8; 32],
            payload: vec![],
            priority: TaskPriority::High,
            target_node: None,
            retries: 0,
            created_at: std::time::Instant::now(),
        };

        queue.enqueue(low_task).await;
        queue.enqueue(high_task.clone()).await;

        // High priority should be dequeued first
        let dequeued = queue.dequeue().await.unwrap();
        assert_eq!(dequeued.task_id, high_task.task_id);
    }

    #[tokio::test]
    async fn test_task_completion() {
        let queue = TaskQueue::new(100);

        let task = QueuedTask {
            task_id: [1u8; 32],
            payload: vec![],
            priority: TaskPriority::Normal,
            target_node: None,
            retries: 0,
            created_at: std::time::Instant::now(),
        };

        queue.enqueue(task.clone()).await;
        queue.dequeue().await;

        assert_eq!(queue.in_flight_count().await, 1);

        queue.complete(&task.task_id).await;

        assert_eq!(queue.in_flight_count().await, 0);
    }

    #[tokio::test]
    async fn test_task_retry() {
        let queue = TaskQueue::new(100);

        let task = QueuedTask {
            task_id: [1u8; 32],
            payload: vec![],
            priority: TaskPriority::Normal,
            target_node: None,
            retries: 0,
            created_at: std::time::Instant::now(),
        };

        queue.enqueue(task.clone()).await;
        queue.dequeue().await;

        // Fail and requeue
        queue.fail(&task.task_id, true).await;

        assert_eq!(queue.pending_count().await, 1);
        assert_eq!(queue.in_flight_count().await, 0);
    }
}
