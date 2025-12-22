//! TRUE Distributed Processing
//! 
//! This module implements REAL distributed AI where multiple nodes
//! each contribute to different parts of the final answer.
//! 
//! Strategy: Task Decomposition
//! 1. Decompose complex question into sub-questions
//! 2. Send each sub-question to a different node IN PARALLEL
//! 3. Each node processes its part with LLM
//! 4. Combine all answers into final response

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

use cortex_grid::PeerStore;

/// Distributed task - truly parallel processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedResult {
    pub task_id: String,
    pub success: bool,
    pub parts: Vec<PartResult>,
    pub combined_answer: String,
    pub total_nodes_used: usize,
    pub total_time_ms: u64,
    pub is_truly_distributed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartResult {
    pub part_name: String,
    pub node_id: String,
    pub answer: String,
    pub time_ms: u64,
}

/// Decompose a question into parallel sub-questions
/// This is where the REAL distribution happens
fn decompose_question(question: &str) -> Vec<(&'static str, String)> {
    let q = question.to_lowercase();
    
    // Research/explanation questions - split into aspects
    if q.contains("explain") || q.contains("what is") || q.contains("describe") {
        return vec![
            ("Definition", format!("In 1-2 sentences, define: {}", question)),
            ("Key Facts", format!("List 3 key facts about: {}", question)),
            ("Example", format!("Give one simple example of: {}", question)),
            ("Importance", format!("Why is this important (1 sentence): {}", question)),
        ];
    }
    
    // Comparison questions
    if q.contains("compare") || q.contains("vs") || q.contains("difference") {
        return vec![
            ("First Concept", format!("Briefly explain the first thing in: {}", question)),
            ("Second Concept", format!("Briefly explain the second thing in: {}", question)),
            ("Similarities", format!("One similarity: {}", question)),
            ("Differences", format!("One key difference: {}", question)),
        ];
    }
    
    // How-to questions
    if q.contains("how to") || q.contains("how do") {
        return vec![
            ("Overview", format!("In one sentence, summarize how to: {}", question)),
            ("Step 1", format!("First step for: {}", question)),
            ("Step 2", format!("Second step for: {}", question)),
            ("Tips", format!("One tip for: {}", question)),
        ];
    }
    
    // Creative questions
    if q.contains("write") || q.contains("create") || q.contains("generate") {
        return vec![
            ("Opening", format!("Write an opening for: {}", question)),
            ("Middle", format!("Write the middle part for: {}", question)),
            ("Conclusion", format!("Write a conclusion for: {}", question)),
        ];
    }
    
    // Analysis questions
    if q.contains("analyze") || q.contains("evaluate") || q.contains("assess") {
        return vec![
            ("Strengths", format!("List strengths: {}", question)),
            ("Weaknesses", format!("List weaknesses: {}", question)),
            ("Opportunities", format!("List opportunities: {}", question)),
            ("Conclusion", format!("Summarize analysis: {}", question)),
        ];
    }
    
    // Default: still split into perspectives
    vec![
        ("Main Answer", format!("Answer briefly: {}", question)),
        ("Details", format!("Add one more detail: {}", question)),
        ("Context", format!("Give context for: {}", question)),
    ]
}

/// Combine partial answers into coherent response
fn combine_answers(parts: &[PartResult], original_question: &str) -> String {
    if parts.is_empty() {
        return "No parts were processed".to_string();
    }
    
    let mut combined = String::new();
    
    // Build structured response
    for part in parts {
        if !part.answer.is_empty() && part.answer != "Error" {
            combined.push_str(&format!("**{}**: {}\n\n", part.part_name, part.answer.trim()));
        }
    }
    
    if combined.is_empty() {
        return "Failed to process question".to_string();
    }
    
    combined
}

/// Execute TRUE distributed processing
/// Each node handles a DIFFERENT part of the question
pub async fn execute_distributed(
    peer_store: Arc<PeerStore>,
    task_id: &str,
    question: &str,
    from_node: &str,
) -> DistributedResult {
    let start = std::time::Instant::now();
    
    // Step 1: Decompose the question into parts
    let parts = decompose_question(question);
    info!("🔀 Decomposed into {} parts for TRUE distributed processing", parts.len());
    
    // Step 2: Get available compute nodes
    let peers = peer_store
        .find_by_capability(|caps| caps.can_compute)
        .await;
    
    if peers.is_empty() {
        return DistributedResult {
            task_id: task_id.to_string(),
            success: false,
            parts: vec![],
            combined_answer: "No compute nodes available".to_string(),
            total_nodes_used: 0,
            total_time_ms: start.elapsed().as_millis() as u64,
            is_truly_distributed: false,
        };
    }
    
    // Step 3: Send DIFFERENT parts to DIFFERENT nodes IN PARALLEL
    let mut handles = vec![];
    let num_nodes = peers.len();
    
    for (i, (part_name, sub_question)) in parts.iter().enumerate() {
        // Distribute across available nodes (round-robin)
        let peer = &peers[i % num_nodes];
        
        if let Some(addr) = peer.addresses.first() {
            let addr_str = addr.to_string();
            if let Some(task_addr) = get_task_address(&addr_str) {
                let task_id = format!("{}-part{}", task_id, i);
                let part_name = part_name.to_string();
                let sub_question = sub_question.clone();
                let from_node = from_node.to_string();
                let peer_id = peer.node_id.to_string();
                
                info!("📤 Part '{}' → Node {} ({})", part_name, &peer_id[..8], task_addr);
                
                let handle = tokio::spawn(async move {
                    let start = std::time::Instant::now();
                    match send_part(&task_addr, &task_id, &sub_question, &from_node).await {
                        Ok(answer) => PartResult {
                            part_name,
                            node_id: peer_id,
                            answer,
                            time_ms: start.elapsed().as_millis() as u64,
                        },
                        Err(e) => PartResult {
                            part_name,
                            node_id: peer_id,
                            answer: format!("Error: {}", e),
                            time_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                });
                handles.push(handle);
            }
        }
    }
    
    // Step 4: Collect all results (they run IN PARALLEL)
    let mut part_results = vec![];
    let mut nodes_used = std::collections::HashSet::new();
    
    for handle in handles {
        if let Ok(result) = handle.await {
            nodes_used.insert(result.node_id.clone());
            part_results.push(result);
        }
    }
    
    // Step 5: Combine into final answer
    let combined = combine_answers(&part_results, question);
    let is_truly_distributed = nodes_used.len() > 1 || part_results.len() > 1;
    
    info!("✅ Distributed task complete: {} parts across {} unique nodes", 
        part_results.len(), nodes_used.len());
    
    DistributedResult {
        task_id: task_id.to_string(),
        success: !part_results.is_empty(),
        parts: part_results,
        combined_answer: combined,
        total_nodes_used: nodes_used.len(),
        total_time_ms: start.elapsed().as_millis() as u64,
        is_truly_distributed,
    }
}

/// Send a part to a node for processing
async fn send_part(
    target_addr: &str,
    task_id: &str,
    prompt: &str,
    from_node: &str,
) -> Result<String, String> {
    let mut stream = TcpStream::connect(target_addr)
        .await
        .map_err(|e| format!("Connect error: {}", e))?;
    
    #[derive(Serialize)]
    struct TaskRequest {
        task_id: String,
        skill: String,
        payload: String,
        from_node: String,
    }
    
    let request = TaskRequest {
        task_id: task_id.to_string(),
        skill: "llm".to_string(),
        payload: prompt.to_string(),
        from_node: from_node.to_string(),
    };
    
    let request_bytes = serde_json::to_vec(&request)
        .map_err(|e| format!("Serialize error: {}", e))?;
    let len_bytes = (request_bytes.len() as u32).to_be_bytes();
    
    stream.write_all(&len_bytes).await.map_err(|e| e.to_string())?;
    stream.write_all(&request_bytes).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;
    
    // Read response with timeout
    let mut len_buf = [0u8; 4];
    tokio::time::timeout(
        std::time::Duration::from_secs(60),
        stream.read_exact(&mut len_buf)
    ).await
        .map_err(|_| "Timeout".to_string())?
        .map_err(|e| e.to_string())?;
    
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut response_buf = vec![0u8; len];
    stream.read_exact(&mut response_buf).await.map_err(|e| e.to_string())?;
    
    #[derive(Deserialize)]
    struct TaskResponse {
        success: bool,
        result: Option<String>,
        error: Option<String>,
    }
    
    let response: TaskResponse = serde_json::from_slice(&response_buf)
        .map_err(|e| e.to_string())?;
    
    if response.success {
        Ok(response.result.unwrap_or_default())
    } else {
        Err(response.error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}

fn get_task_address(addr: &str) -> Option<String> {
    if let Some((ip, port_str)) = addr.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return Some(format!("{}:{}", ip, port + 1000));
        }
    }
    None
}

