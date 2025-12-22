//! Swarm Processing - Split tasks across multiple nodes and combine results
//! 
//! This enables true distributed AI where multiple nodes work on parts of a task,
//! like a global decentralized GPU.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{info, warn};

use cortex_grid::PeerStore;

/// Swarm task request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmTaskRequest {
    pub task_id: String,
    pub skill: String,
    pub payload: String,
    pub from_node: String,
}

/// Individual node response in swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResponse {
    pub node_id: String,
    pub result: String,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// Combined swarm response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmResponse {
    pub task_id: String,
    pub success: bool,
    pub combined_result: String,
    pub node_responses: Vec<NodeResponse>,
    pub total_nodes: usize,
    pub successful_nodes: usize,
    pub total_time_ms: u64,
}

/// Split a complex prompt into subtasks for parallel processing
pub fn split_task(payload: &str) -> Vec<String> {
    let lower = payload.to_lowercase();
    
    // Research/analysis tasks - split into aspects
    if lower.contains("research") || lower.contains("analyze") || lower.contains("explain") {
        return vec![
            format!("Provide a brief definition of: {}", payload),
            format!("List key facts about: {}", payload),
            format!("Explain the importance of: {}", payload),
        ];
    }
    
    // Comparison tasks
    if lower.contains("compare") || lower.contains("difference") || lower.contains("versus") || lower.contains(" vs ") {
        return vec![
            format!("Explain the first concept in: {}", payload),
            format!("Explain the second concept in: {}", payload),
            format!("List similarities and differences for: {}", payload),
        ];
    }
    
    // List/enumeration tasks
    if lower.contains("list") || lower.contains("examples") || lower.contains("types of") {
        return vec![
            format!("Give first set of examples for: {}", payload),
            format!("Give more examples for: {}", payload),
            format!("Summarize all: {}", payload),
        ];
    }
    
    // Brainstorming tasks
    if lower.contains("brainstorm") || lower.contains("ideas") || lower.contains("suggest") {
        return vec![
            format!("Creative ideas for: {}", payload),
            format!("Practical ideas for: {}", payload),
            format!("Innovative ideas for: {}", payload),
        ];
    }
    
    // Default: don't split simple tasks
    vec![payload.to_string()]
}

/// Combine responses from multiple nodes
pub fn combine_responses(responses: &[NodeResponse], original_task: &str) -> String {
    if responses.is_empty() {
        return "No responses from swarm".to_string();
    }
    
    if responses.len() == 1 {
        return responses[0].result.clone();
    }
    
    // Combine multiple responses
    let mut combined = String::new();
    combined.push_str("🌐 **Swarm Combined Response**\n\n");
    
    for (i, resp) in responses.iter().enumerate() {
        if resp.success {
            combined.push_str(&format!("**Part {} ({}ms):**\n{}\n\n", 
                i + 1, 
                resp.execution_time_ms,
                resp.result.trim()
            ));
        }
    }
    
    combined.push_str(&format!("\n---\n*Processed by {} nodes in parallel*", responses.len()));
    
    combined
}

/// Execute a task across multiple swarm nodes
/// 
/// Sends the same task to ALL available nodes in parallel and returns
/// the first successful result. This is like racing multiple nodes to
/// get the fastest response.
pub async fn execute_swarm_task(
    peer_store: Arc<PeerStore>,
    task_id: &str,
    skill: &str,
    payload: &str,
    from_node: &str,
) -> SwarmResponse {
    let start = std::time::Instant::now();
    
    info!("🌐 Swarm task {} starting parallel execution", task_id);
    
    // Get available compute peers
    let peers = peer_store
        .find_by_capability(|caps| caps.can_compute)
        .await;
    
    if peers.is_empty() {
        return SwarmResponse {
            task_id: task_id.to_string(),
            success: false,
            combined_result: "No compute peers available in swarm".to_string(),
            node_responses: vec![],
            total_nodes: 0,
            successful_nodes: 0,
            total_time_ms: start.elapsed().as_millis() as u64,
        };
    }
    
    // Send to ALL peers in parallel (race them)
    let mut handles = vec![];
    
    for (i, peer) in peers.iter().enumerate() {
        if let Some(addr) = peer.addresses.first() {
            let addr_str = addr.to_string();
            if let Some(task_addr) = get_task_address(&addr_str) {
                let task_id = format!("{}-{}", task_id, i);
                let payload = payload.to_string();
                let skill = skill.to_string();
                let from_node = from_node.to_string();
                
                let handle = tokio::spawn(async move {
                    send_subtask(&task_addr, &task_id, &skill, &payload, &from_node).await
                });
                handles.push(handle);
            }
        }
    }
    
    // Collect results - take first successful one
    let mut node_responses = vec![];
    let mut successful_nodes = 0;
    let mut first_success: Option<String> = None;
    
    for handle in handles {
        match handle.await {
            Ok(Ok(response)) => {
                if response.success {
                    successful_nodes += 1;
                    if first_success.is_none() {
                        first_success = Some(response.result.clone());
                    }
                }
                node_responses.push(response);
            }
            Ok(Err(e)) => {
                // Don't log as warning - node might just not have this skill
                node_responses.push(NodeResponse {
                    node_id: "unknown".to_string(),
                    result: format!("Error: {}", e),
                    execution_time_ms: 0,
                    success: false,
                });
            }
            Err(e) => {
                warn!("Task join error: {}", e);
            }
        }
    }
    
    // Return first successful result, or combine errors
    let combined_result = if let Some(result) = first_success {
        result
    } else {
        "No node was able to process this task".to_string()
    };
    
    SwarmResponse {
        task_id: task_id.to_string(),
        success: successful_nodes > 0,
        combined_result,
        node_responses,
        total_nodes: peers.len(),
        successful_nodes,
        total_time_ms: start.elapsed().as_millis() as u64,
    }
}

/// Send a subtask to a specific node
async fn send_subtask(
    target_addr: &str,
    task_id: &str,
    skill: &str,
    payload: &str,
    from_node: &str,
) -> Result<NodeResponse, String> {
    let mut stream = TcpStream::connect(target_addr)
        .await
        .map_err(|e| format!("Connect error: {}", e))?;
    
    let request = SwarmTaskRequest {
        task_id: task_id.to_string(),
        skill: skill.to_string(),
        payload: payload.to_string(),
        from_node: from_node.to_string(),
    };
    
    let request_bytes = serde_json::to_vec(&request)
        .map_err(|e| format!("Serialize error: {}", e))?;
    let len_bytes = (request_bytes.len() as u32).to_be_bytes();
    
    stream.write_all(&len_bytes).await
        .map_err(|e| format!("Write error: {}", e))?;
    stream.write_all(&request_bytes).await
        .map_err(|e| format!("Write error: {}", e))?;
    stream.flush().await
        .map_err(|e| format!("Flush error: {}", e))?;
    
    // Read response with timeout
    let mut len_buf = [0u8; 4];
    tokio::time::timeout(
        std::time::Duration::from_secs(60),
        stream.read_exact(&mut len_buf)
    ).await
        .map_err(|_| "Timeout waiting for response".to_string())?
        .map_err(|e| format!("Read error: {}", e))?;
    
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut response_buf = vec![0u8; len];
    
    stream.read_exact(&mut response_buf).await
        .map_err(|e| format!("Read error: {}", e))?;
    
    #[derive(Deserialize)]
    struct TaskResponse {
        task_id: String,
        success: bool,
        result: Option<String>,
        error: Option<String>,
        executor_node: String,
        execution_time_ms: u64,
    }
    
    let response: TaskResponse = serde_json::from_slice(&response_buf)
        .map_err(|e| format!("Parse error: {}", e))?;
    
    Ok(NodeResponse {
        node_id: response.executor_node,
        result: response.result.unwrap_or_else(|| response.error.unwrap_or_default()),
        execution_time_ms: response.execution_time_ms,
        success: response.success,
    })
}

/// Extract task port address from peer address
fn get_task_address(addr: &str) -> Option<String> {
    // Handle different address formats
    if let Some((ip, port_str)) = addr.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            let task_port = port + 1000;
            return Some(format!("{}:{}", ip, task_port));
        }
    }
    
    // Try multiaddr format
    let parts: Vec<&str> = addr.split('/').collect();
    let mut ip = None;
    let mut port = None;
    
    for (i, part) in parts.iter().enumerate() {
        if *part == "ip4" || *part == "ip6" {
            if i + 1 < parts.len() {
                ip = Some(parts[i + 1]);
            }
        }
        if *part == "tcp" || *part == "udp" {
            if i + 1 < parts.len() {
                port = parts[i + 1].parse::<u16>().ok();
            }
        }
    }
    
    if let (Some(ip), Some(port)) = (ip, port) {
        return Some(format!("{}:{}", ip, port + 1000));
    }
    
    None
}

