use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

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
    pub target_node: Option<String>,
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
    let payload = request.payload.as_bytes().to_vec();
    let task_id_hash = blake3::hash(&payload);
    let task_id: [u8; 32] = *task_id_hash.as_bytes();

    if let Some(orchestrator) = &state.orchestrator {
        let orch = orchestrator.write().await;
        match orch.delegate_task(task_id, payload).await {
            Ok(target_node) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "task_id": hex::encode(&task_id[..8]),
                    "target_node": target_node.to_string(),
                })))
            }
            Err(e) => {
                Ok(Json(serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                })))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "error": "Orchestrator not available",
        })))
    }
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

