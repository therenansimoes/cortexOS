use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::AppState;
use cortex_grid::{NodeId, PeerStore};
use cortex_skill::NetworkSkillRegistry;
use cortex_reputation::TrustGraph;

#[derive(Serialize)]
pub struct StatusResponse {
    pub node_id: String,
    pub name: String,
    pub status: String,
    pub peers_count: usize,
    pub skills_count: usize,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct PeerResponse {
    pub node_id: String,
    pub addresses: Vec<String>,
    pub capabilities: CapabilitiesResponse,
    pub latency_ms: Option<u32>,
    pub last_seen: String,
}

#[derive(Serialize)]
pub struct CapabilitiesResponse {
    pub can_relay: bool,
    pub can_store: bool,
    pub can_compute: bool,
    pub max_storage_mb: u32,
}

#[derive(Serialize)]
pub struct SkillResponse {
    pub skill_id: String,
    pub node_id: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub status: String,
    pub target_node: Option<String>,
    pub created_at: String,
    pub payload_size: usize,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub total_peers: usize,
    pub compute_peers: usize,
    pub relay_peers: usize,
    pub total_skills: usize,
    pub pending_tasks: usize,
}

#[derive(Deserialize)]
pub struct DelegateTaskRequest {
    pub payload: String,
    pub skill: Option<String>,
    pub target_node: Option<String>,
}

/// Task request sent over network
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskNetworkRequest {
    task_id: String,
    skill: String,
    payload: String,
    from_node: String,
}

/// Task response from remote node
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskNetworkResponse {
    task_id: String,
    success: bool,
    result: Option<String>,
    error: Option<String>,
    executor_node: String,
    execution_time_ms: u64,
}

pub async fn get_status(State(state): State<AppState>) -> Result<Json<StatusResponse>, StatusCode> {
    let peers_count = state.peer_store.count().await;
    let skills_count = {
        let registry = state.skill_registry.read().await;
        registry.skill_distribution().len()
    };

    Ok(Json(StatusResponse {
        node_id: state.node_id.to_string(),
        name: "CortexOS Node".to_string(),
        status: "running".to_string(),
        peers_count,
        skills_count,
        uptime_seconds: 0, // TODO: Track uptime
    }))
}

pub async fn get_peers(State(state): State<AppState>) -> Result<Json<Vec<PeerResponse>>, StatusCode> {
    let peers = state.peer_store.list_active().await;
    
    let response: Vec<PeerResponse> = peers
        .iter()
        .map(|peer| PeerResponse {
            node_id: peer.node_id.to_string(),
            addresses: peer.addresses.iter().map(|a| a.to_string()).collect(),
            capabilities: CapabilitiesResponse {
                can_relay: peer.capabilities.can_relay,
                can_store: peer.capabilities.can_store,
                can_compute: peer.capabilities.can_compute,
                max_storage_mb: peer.capabilities.max_storage_mb,
            },
            latency_ms: peer.latency_ms,
            last_seen: format!("{:?}", peer.last_seen),
        })
        .collect();

    Ok(Json(response))
}

pub async fn get_skills(State(state): State<AppState>) -> Result<Json<Vec<SkillResponse>>, StatusCode> {
    let registry = state.skill_registry.read().await;
    let distribution = registry.skill_distribution();
    
    // TODO: Get actual skill details from registry
    let response: Vec<SkillResponse> = distribution
        .iter()
        .map(|(skill_id, _count)| SkillResponse {
            skill_id: skill_id.to_string(),
            node_id: "unknown".to_string(), // TODO: Get from registry
            description: None,
        })
        .collect();

    Ok(Json(response))
}

pub async fn get_tasks(State(_state): State<AppState>) -> Result<Json<Vec<TaskResponse>>, StatusCode> {
    // TODO: Get tasks from orchestrator
    Ok(Json(vec![]))
}

pub async fn delegate_task(
    State(state): State<AppState>,
    Json(request): Json<DelegateTaskRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let payload = request.payload.clone();
    let task_id_hash = blake3::hash(payload.as_bytes());
    let task_id = hex::encode(&task_id_hash.as_bytes()[..8]);
    let skill = request.skill.clone().unwrap_or_else(|| "general".to_string());

    // Find compute peers
    let peers = state.peer_store
        .find_by_capability(|caps| caps.can_compute)
        .await;

    if peers.is_empty() {
        return Ok(Json(serde_json::json!({
            "success": false,
            "error": "No compute peers available",
        })));
    }

    // Try each peer until we find one with the skill
    for target_peer in &peers {
        // Get peer address - they should have at least one address
        // The task server runs on port + 1000 from the discovery port
        let task_addr = if let Some(addr) = target_peer.addresses.first() {
            let addr_str = addr.to_string();
            if let Some(ip) = extract_ip_from_multiaddr(&addr_str) {
                let discovery_port = extract_port_from_multiaddr(&addr_str).unwrap_or(7654);
                let task_port = discovery_port + 1000;
                format!("{}:{}", ip, task_port)
            } else {
                continue; // Skip this peer if we can't parse address
            }
        } else {
            continue;
        };

        info!("📤 Trying task {} on {} at {}", task_id, target_peer.node_id, task_addr);

        // Send task via TCP
        match send_task_tcp(&task_addr, &task_id, &skill, &payload, &state.node_id.to_string()).await {
            Ok(response) => {
                if response.success {
                    info!("✅ Task {} completed by {} in {}ms", 
                        task_id, response.executor_node, response.execution_time_ms);
                    return Ok(Json(serde_json::json!({
                        "success": true,
                        "task_id": task_id,
                        "target_node": target_peer.node_id.to_string(),
                        "result": response.result,
                        "execution_time_ms": response.execution_time_ms,
                    })));
                } else {
                    // Skill not available on this node, try next
                    info!("⏭️ Node {} doesn't have skill '{}', trying next...", target_peer.node_id, skill);
                    continue;
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to connect to {}: {}", task_addr, e);
                continue;
            }
        }
    }

    // No peer had the skill
    Ok(Json(serde_json::json!({
        "success": false,
        "error": format!("No peer found with skill '{}'", skill),
    })))
}

/// Extract IP address from address string (handles both "IP:port" and multiaddr formats)
fn extract_ip_from_multiaddr(addr: &str) -> Option<String> {
    // First try simple IP:port format (e.g., "192.168.1.250:7655")
    if let Some(ip) = addr.split(':').next() {
        if ip.contains('.') || ip.contains(':') {
            // Looks like an IP address
            return Some(ip.to_string());
        }
    }
    
    // Try multiaddr format "/ip4/192.168.1.1/tcp/7654"
    let parts: Vec<&str> = addr.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "ip4" || *part == "ip6" {
            if i + 1 < parts.len() {
                return Some(parts[i + 1].to_string());
            }
        }
    }
    None
}

/// Extract port from address string (handles both "IP:port" and multiaddr formats)
fn extract_port_from_multiaddr(addr: &str) -> Option<u16> {
    // First try simple IP:port format
    if let Some(port_str) = addr.rsplit(':').next() {
        if let Ok(port) = port_str.parse::<u16>() {
            return Some(port);
        }
    }
    
    // Try multiaddr format
    let parts: Vec<&str> = addr.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "tcp" || *part == "udp" {
            if i + 1 < parts.len() {
                return parts[i + 1].parse().ok();
            }
        }
    }
    None
}

/// Send a task to a remote node via TCP
async fn send_task_tcp(
    target_addr: &str,
    task_id: &str,
    skill: &str,
    payload: &str,
    from_node: &str,
) -> Result<TaskNetworkResponse, Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = TcpStream::connect(target_addr).await?;
    
    let request = TaskNetworkRequest {
        task_id: task_id.to_string(),
        skill: skill.to_string(),
        payload: payload.to_string(),
        from_node: from_node.to_string(),
    };

    let request_bytes = serde_json::to_vec(&request)?;
    let len_bytes = (request_bytes.len() as u32).to_be_bytes();
    
    stream.write_all(&len_bytes).await?;
    stream.write_all(&request_bytes).await?;
    stream.flush().await?;

    // Read response
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    
    let mut response_buf = vec![0u8; len];
    stream.read_exact(&mut response_buf).await?;
    
    let response: TaskNetworkResponse = serde_json::from_slice(&response_buf)?;
    
    Ok(response)
}

pub async fn get_stats(State(state): State<AppState>) -> Result<Json<StatsResponse>, StatusCode> {
    let peers = state.peer_store.list_active().await;
    let compute_peers = state.peer_store
        .find_by_capability(|caps| caps.can_compute)
        .await
        .len();
    let relay_peers = state.peer_store
        .find_by_capability(|caps| caps.can_relay)
        .await
        .len();
    
    let skills_count = {
        let registry = state.skill_registry.read().await;
        registry.skill_distribution().len()
    };

    Ok(Json(StatsResponse {
        total_peers: peers.len(),
        compute_peers,
        relay_peers,
        total_skills: skills_count,
        pending_tasks: 0, // TODO: Get from orchestrator
    }))
}

