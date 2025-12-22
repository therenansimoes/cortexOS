//! Real-time logging system for debugging communications
//! 
//! Tracks all node communications and exposes them via API

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Serialize;
use chrono::{DateTime, Utc};

/// Maximum number of logs to keep in memory
const MAX_LOGS: usize = 500;

/// Type of log entry
#[derive(Debug, Clone, Serialize)]
pub enum LogType {
    Discovery,
    TaskSent,
    TaskReceived,
    TaskCompleted,
    TaskFailed,
    PipelineBuilt,
    PipelineStage,
    NetworkError,
    Info,
    Warning,
    Debug,
}

/// A single log entry
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub log_type: LogType,
    pub source: String,
    pub target: Option<String>,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub duration_ms: Option<u64>,
}

/// Global log store
pub struct LogStore {
    logs: RwLock<VecDeque<LogEntry>>,
}

impl LogStore {
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(VecDeque::with_capacity(MAX_LOGS)),
        }
    }

    pub async fn add(&self, entry: LogEntry) {
        let mut logs = self.logs.write().await;
        if logs.len() >= MAX_LOGS {
            logs.pop_front();
        }
        logs.push_back(entry);
    }

    pub async fn get_recent(&self, count: usize) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    pub async fn get_by_type(&self, log_type: LogType, count: usize) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.iter()
            .rev()
            .filter(|l| std::mem::discriminant(&l.log_type) == std::mem::discriminant(&log_type))
            .take(count)
            .cloned()
            .collect()
    }

    pub async fn clear(&self) {
        let mut logs = self.logs.write().await;
        logs.clear();
    }

    /// Log a discovery event
    pub async fn log_discovery(&self, source: &str, peer_id: &str, addresses: &[String]) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::Discovery,
            source: source.to_string(),
            target: Some(peer_id.to_string()),
            message: format!("Discovered peer {} at {:?}", &peer_id[..8], addresses),
            details: Some(serde_json::json!({
                "peer_id": peer_id,
                "addresses": addresses,
            })),
            duration_ms: None,
        }).await;
    }

    /// Log a task being sent
    pub async fn log_task_sent(&self, from: &str, to: &str, task_id: &str, skill: &str, payload_len: usize) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::TaskSent,
            source: from.to_string(),
            target: Some(to.to_string()),
            message: format!("Task {} sent to {} (skill: {}, {} bytes)", 
                &task_id[..8], &to[..8.min(to.len())], skill, payload_len),
            details: Some(serde_json::json!({
                "task_id": task_id,
                "skill": skill,
                "payload_bytes": payload_len,
            })),
            duration_ms: None,
        }).await;
    }

    /// Log a task being received
    pub async fn log_task_received(&self, node: &str, task_id: &str, from: &str) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::TaskReceived,
            source: node.to_string(),
            target: Some(from.to_string()),
            message: format!("Task {} received from {}", &task_id[..8], &from[..8.min(from.len())]),
            details: Some(serde_json::json!({
                "task_id": task_id,
                "from_node": from,
            })),
            duration_ms: None,
        }).await;
    }

    /// Log task completion
    pub async fn log_task_completed(&self, node: &str, task_id: &str, duration_ms: u64, result_len: usize) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::TaskCompleted,
            source: node.to_string(),
            target: None,
            message: format!("Task {} completed in {}ms ({} bytes result)", 
                &task_id[..8], duration_ms, result_len),
            details: Some(serde_json::json!({
                "task_id": task_id,
                "result_bytes": result_len,
            })),
            duration_ms: Some(duration_ms),
        }).await;
    }

    /// Log task failure
    pub async fn log_task_failed(&self, node: &str, task_id: &str, error: &str) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::TaskFailed,
            source: node.to_string(),
            target: None,
            message: format!("Task {} FAILED: {}", &task_id[..8], error),
            details: Some(serde_json::json!({
                "task_id": task_id,
                "error": error,
            })),
            duration_ms: None,
        }).await;
    }

    /// Log pipeline build
    pub async fn log_pipeline_built(&self, node_count: usize, nodes: &[String]) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::PipelineBuilt,
            source: "coordinator".to_string(),
            target: None,
            message: format!("Pipeline built with {} nodes", node_count),
            details: Some(serde_json::json!({
                "node_count": node_count,
                "nodes": nodes,
            })),
            duration_ms: None,
        }).await;
    }

    /// Log pipeline stage execution
    pub async fn log_pipeline_stage(&self, stage: u32, total: u32, node: &str, role: &str, duration_ms: u64) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::PipelineStage,
            source: node.to_string(),
            target: None,
            message: format!("Stage {}/{} ({}) completed in {}ms", stage, total, role, duration_ms),
            details: Some(serde_json::json!({
                "stage": stage,
                "total_stages": total,
                "role": role,
            })),
            duration_ms: Some(duration_ms),
        }).await;
    }

    /// Log network error
    pub async fn log_network_error(&self, source: &str, target: &str, error: &str) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::NetworkError,
            source: source.to_string(),
            target: Some(target.to_string()),
            message: format!("Network error to {}: {}", &target[..8.min(target.len())], error),
            details: Some(serde_json::json!({
                "error": error,
            })),
            duration_ms: None,
        }).await;
    }

    /// Log info message
    pub async fn log_info(&self, source: &str, message: &str) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::Info,
            source: source.to_string(),
            target: None,
            message: message.to_string(),
            details: None,
            duration_ms: None,
        }).await;
    }

    /// Log debug message with details
    pub async fn log_debug(&self, source: &str, message: &str, details: serde_json::Value) {
        self.add(LogEntry {
            timestamp: Utc::now(),
            log_type: LogType::Debug,
            source: source.to_string(),
            target: None,
            message: message.to_string(),
            details: Some(details),
            duration_ms: None,
        }).await;
    }
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Global log store instance
lazy_static::lazy_static! {
    pub static ref LOGS: Arc<LogStore> = Arc::new(LogStore::new());
}

/// Helper macro for logging
#[macro_export]
macro_rules! cortex_log {
    (discovery $source:expr, $peer_id:expr, $addresses:expr) => {
        $crate::logs::LOGS.log_discovery($source, $peer_id, $addresses).await
    };
    (task_sent $from:expr, $to:expr, $task_id:expr, $skill:expr, $len:expr) => {
        $crate::logs::LOGS.log_task_sent($from, $to, $task_id, $skill, $len).await
    };
    (task_received $node:expr, $task_id:expr, $from:expr) => {
        $crate::logs::LOGS.log_task_received($node, $task_id, $from).await
    };
    (task_completed $node:expr, $task_id:expr, $duration:expr, $len:expr) => {
        $crate::logs::LOGS.log_task_completed($node, $task_id, $duration, $len).await
    };
    (task_failed $node:expr, $task_id:expr, $error:expr) => {
        $crate::logs::LOGS.log_task_failed($node, $task_id, $error).await
    };
    (pipeline_built $count:expr, $nodes:expr) => {
        $crate::logs::LOGS.log_pipeline_built($count, $nodes).await
    };
    (pipeline_stage $stage:expr, $total:expr, $node:expr, $role:expr, $duration:expr) => {
        $crate::logs::LOGS.log_pipeline_stage($stage, $total, $node, $role, $duration).await
    };
    (network_error $source:expr, $target:expr, $error:expr) => {
        $crate::logs::LOGS.log_network_error($source, $target, $error).await
    };
    (info $source:expr, $message:expr) => {
        $crate::logs::LOGS.log_info($source, $message).await
    };
}

