//! Task Queue System
//! 
//! Manages incoming tensor processing tasks with priority handling.
//! Each peer has a queue and processes tasks based on their capacity.

use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tracing::debug;

/// A tensor chunk to be processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorChunk {
    /// Unique task ID
    pub task_id: String,
    /// Chunk index within the task
    pub chunk_idx: u32,
    /// Total chunks for this task
    pub total_chunks: u32,
    /// Layer range to process
    pub start_layer: u32,
    pub end_layer: u32,
    /// Serialized tensor data
    pub tensor_data: Vec<u8>,
    /// Shape of the tensor
    pub shape: Vec<usize>,
    /// Data type
    pub dtype: String,
    /// Node that sent this chunk
    pub source_node: String,
    /// Priority (higher = more urgent)
    pub priority: u32,
    /// Timestamp when created
    pub created_at: u64,
}

impl Eq for TensorChunk {}

impl PartialEq for TensorChunk {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id && self.chunk_idx == other.chunk_idx
    }
}

impl Ord for TensorChunk {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then older tasks first
        self.priority.cmp(&other.priority)
            .then_with(|| other.created_at.cmp(&self.created_at))
    }
}

impl PartialOrd for TensorChunk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Result of processing a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedChunk {
    pub task_id: String,
    pub chunk_idx: u32,
    pub total_chunks: u32,
    pub result_data: Vec<u8>,
    pub result_shape: Vec<usize>,
    pub processing_time_ms: u64,
    pub processor_node: String,
}

/// Task queue for a peer node
pub struct TaskQueue {
    /// Priority queue for incoming chunks
    queue: Arc<Mutex<BinaryHeap<TensorChunk>>>,
    /// Currently processing
    processing: Arc<RwLock<Option<TensorChunk>>>,
    /// Completed chunks waiting to be sent back
    completed: Arc<RwLock<VecDeque<ProcessedChunk>>>,
    /// Queue capacity (based on device)
    max_queue_size: usize,
    /// Stats
    stats: Arc<RwLock<QueueStats>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStats {
    pub total_received: u64,
    pub total_processed: u64,
    pub total_dropped: u64,
    pub average_processing_ms: u64,
    pub current_queue_size: usize,
}

impl TaskQueue {
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(BinaryHeap::new())),
            processing: Arc::new(RwLock::new(None)),
            completed: Arc::new(RwLock::new(VecDeque::new())),
            max_queue_size,
            stats: Arc::new(RwLock::new(QueueStats::default())),
        }
    }
    
    /// Enqueue a tensor chunk for processing
    pub async fn enqueue(&self, chunk: TensorChunk) -> Result<(), QueueError> {
        let mut queue = self.queue.lock().await;
        
        if queue.len() >= self.max_queue_size {
            let mut stats = self.stats.write().await;
            stats.total_dropped += 1;
            return Err(QueueError::QueueFull);
        }
        
        debug!("📥 Enqueued chunk {}/{} for task {}", 
               chunk.chunk_idx, chunk.total_chunks, &chunk.task_id[..8]);
        
        queue.push(chunk);
        
        let mut stats = self.stats.write().await;
        stats.total_received += 1;
        stats.current_queue_size = queue.len();
        
        Ok(())
    }
    
    /// Get next chunk to process
    pub async fn dequeue(&self) -> Option<TensorChunk> {
        let mut queue = self.queue.lock().await;
        let chunk = queue.pop();
        
        if let Some(ref c) = chunk {
            *self.processing.write().await = Some(c.clone());
            let mut stats = self.stats.write().await;
            stats.current_queue_size = queue.len();
        }
        
        chunk
    }
    
    /// Mark current chunk as completed
    pub async fn complete(&self, result: ProcessedChunk) {
        *self.processing.write().await = None;
        
        let mut completed = self.completed.write().await;
        completed.push_back(result);
        
        let mut stats = self.stats.write().await;
        stats.total_processed += 1;
    }
    
    /// Get completed chunks ready to send back
    pub async fn get_completed(&self) -> Vec<ProcessedChunk> {
        let mut completed = self.completed.write().await;
        completed.drain(..).collect()
    }
    
    /// Get queue stats
    pub async fn stats(&self) -> QueueStats {
        self.stats.read().await.clone()
    }
    
    /// Get queue length
    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }
    
    /// Is queue empty?
    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue is full")]
    QueueFull,
    #[error("Invalid chunk")]
    InvalidChunk,
}

/// Tracks partially assembled responses
pub struct ResponseAssembler {
    /// Chunks received for each task
    chunks: Arc<RwLock<HashMap<String, Vec<ProcessedChunk>>>>,
    /// Expected total chunks per task
    expected: Arc<RwLock<HashMap<String, u32>>>,
}

impl ResponseAssembler {
    pub fn new() -> Self {
        Self {
            chunks: Arc::new(RwLock::new(HashMap::new())),
            expected: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a new task with expected chunk count
    pub async fn register_task(&self, task_id: &str, total_chunks: u32) {
        self.expected.write().await.insert(task_id.to_string(), total_chunks);
        self.chunks.write().await.insert(task_id.to_string(), Vec::new());
    }
    
    /// Add a completed chunk
    pub async fn add_chunk(&self, chunk: ProcessedChunk) -> AssemblyStatus {
        let task_id = chunk.task_id.clone();
        
        let mut chunks = self.chunks.write().await;
        let task_chunks = chunks.entry(task_id.clone()).or_insert_with(Vec::new);
        task_chunks.push(chunk);
        
        let expected = self.expected.read().await;
        let total = *expected.get(&task_id).unwrap_or(&0);
        
        if task_chunks.len() as u32 >= total {
            AssemblyStatus::Complete
        } else {
            AssemblyStatus::Partial {
                received: task_chunks.len() as u32,
                total,
            }
        }
    }
    
    /// Get all chunks for a completed task, ordered by index
    pub async fn get_assembled(&self, task_id: &str) -> Option<Vec<ProcessedChunk>> {
        let mut chunks = self.chunks.write().await;
        let mut task_chunks = chunks.remove(task_id)?;
        
        // Sort by chunk index
        task_chunks.sort_by_key(|c| c.chunk_idx);
        
        // Clean up expected
        self.expected.write().await.remove(task_id);
        
        Some(task_chunks)
    }
}

#[derive(Debug, Clone)]
pub enum AssemblyStatus {
    Complete,
    Partial { received: u32, total: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_queue_priority() {
        let queue = TaskQueue::new(10);
        
        // Add low priority
        queue.enqueue(TensorChunk {
            task_id: "task1".to_string(),
            chunk_idx: 0,
            total_chunks: 1,
            start_layer: 0,
            end_layer: 5,
            tensor_data: vec![],
            shape: vec![1, 2, 3],
            dtype: "F32".to_string(),
            source_node: "node1".to_string(),
            priority: 1,
            created_at: 100,
        }).await.unwrap();
        
        // Add high priority
        queue.enqueue(TensorChunk {
            task_id: "task2".to_string(),
            chunk_idx: 0,
            total_chunks: 1,
            start_layer: 0,
            end_layer: 5,
            tensor_data: vec![],
            shape: vec![1, 2, 3],
            dtype: "F32".to_string(),
            source_node: "node1".to_string(),
            priority: 10,
            created_at: 200,
        }).await.unwrap();
        
        // Should get high priority first
        let first = queue.dequeue().await.unwrap();
        assert_eq!(first.task_id, "task2");
        assert_eq!(first.priority, 10);
        
        let second = queue.dequeue().await.unwrap();
        assert_eq!(second.task_id, "task1");
    }
}

