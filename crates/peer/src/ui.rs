//! Peer Web UI
//! 
//! Dashboard with stats, contribute toggle, and P2P chat.

use axum::{
    extract::State,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::chat::{ChatStore, ChatMessage, broadcast_chat};
use crate::PeerState;

/// UI state shared with handlers
pub struct UiState {
    pub peer_state: Arc<PeerState>,
    pub chat_store: Arc<RwLock<ChatStore>>,
}

#[derive(Serialize)]
struct StatusResponse {
    node_id: String,
    device: DeviceInfo,
    network: NetworkInfo,
    stats: StatsInfo,
    is_contributing: bool,
}

#[derive(Serialize)]
struct DeviceInfo {
    cpu_model: String,
    cpu_cores: u32,
    ram_total_mb: u64,
    ram_available_mb: u64,
    gpu: Option<String>,
    capacity_score: u32,
    max_layers: u32,
}

#[derive(Serialize)]
struct NetworkInfo {
    peers_count: usize,
    peers: Vec<PeerSummary>,
}

#[derive(Serialize)]
struct PeerSummary {
    node_id: String,
    address: String,
    can_compute: bool,
}

#[derive(Serialize)]
struct StatsInfo {
    tasks_received: u64,
    tasks_processed: u64,
    bytes_received: u64,
    queue_size: usize,
}

#[derive(Deserialize)]
pub struct ToggleRequest {
    contribute: bool,
}

/// Create the UI router
pub fn create_router(state: Arc<UiState>) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/api/status", get(get_status))
        .route("/api/toggle", post(toggle_contribute))
        .route("/api/chat", get(get_chat))
        .route("/api/chat/send", post(send_chat))
        .route("/api/chat/name", post(set_name))
        .with_state(state)
}

/// Main dashboard HTML
async fn dashboard() -> Html<&'static str> {
    Html(include_str!("ui.html"))
}

/// Get current status
async fn get_status(State(state): State<Arc<UiState>>) -> Json<StatusResponse> {
    let caps = &state.peer_state.capabilities;
    let stats = state.peer_state.stats.read().await;
    let is_active = *state.peer_state.is_active.read().await;
    let peers = state.peer_state.peer_store.list_active().await;
    let queue_stats = state.peer_state.task_queue.stats().await;
    
    Json(StatusResponse {
        node_id: state.peer_state.node_id.to_string(),
        device: DeviceInfo {
            cpu_model: caps.cpu.model.clone(),
            cpu_cores: caps.cpu.cores,
            ram_total_mb: caps.memory.total_mb,
            ram_available_mb: caps.memory.available_mb,
            gpu: caps.gpu.as_ref().map(|g| format!("{} ({} MB)", g.model, g.vram_mb)),
            capacity_score: caps.capacity_score,
            max_layers: caps.max_layers,
        },
        network: NetworkInfo {
            peers_count: peers.len(),
            peers: peers.iter().map(|p| PeerSummary {
                node_id: p.node_id.to_string(),
                address: p.addresses.first().map(|a| a.to_string()).unwrap_or_default(),
                can_compute: p.capabilities.can_compute,
            }).collect(),
        },
        stats: StatsInfo {
            tasks_received: stats.tasks_received,
            tasks_processed: stats.tasks_processed,
            bytes_received: stats.bytes_received,
            queue_size: queue_stats.current_queue_size,
        },
        is_contributing: is_active,
    })
}

/// Toggle contribution on/off
async fn toggle_contribute(
    State(state): State<Arc<UiState>>,
    Json(req): Json<ToggleRequest>,
) -> Json<serde_json::Value> {
    *state.peer_state.is_active.write().await = req.contribute;
    
    info!("🔄 Contribution toggled: {}", if req.contribute { "ON" } else { "OFF" });
    
    Json(serde_json::json!({
        "success": true,
        "contributing": req.contribute
    }))
}

#[derive(Deserialize)]
pub struct SendChatRequest {
    message: String,
}

#[derive(Deserialize)]
pub struct SetNameRequest {
    name: String,
}

/// Get chat messages
async fn get_chat(State(state): State<Arc<UiState>>) -> Json<Vec<ChatMessage>> {
    let chat = state.chat_store.read().await;
    Json(chat.get_messages(50))
}

/// Send a chat message
async fn send_chat(
    State(state): State<Arc<UiState>>,
    Json(req): Json<SendChatRequest>,
) -> Json<serde_json::Value> {
    let node_id = state.peer_state.node_id.to_string();
    let name = state.chat_store.read().await.get_name().to_string();
    
    let message = ChatMessage::new(&node_id, &name, &req.message);
    
    // Add to our own store
    state.chat_store.write().await.add_message(message.clone());
    
    // Broadcast to all peers
    let peers = state.peer_state.peer_store.list_active().await;
    let peer_addrs: Vec<(String, String)> = peers.iter()
        .filter_map(|p| {
            p.addresses.first().map(|a| (p.node_id.to_string(), a.to_string()))
        })
        .collect();
    
    let sent_to = broadcast_chat(&peer_addrs, &message).await;
    
    info!("💬 Chat sent to {} peers", sent_to.len());
    
    Json(serde_json::json!({
        "success": true,
        "sent_to": sent_to.len()
    }))
}

/// Set display name
async fn set_name(
    State(state): State<Arc<UiState>>,
    Json(req): Json<SetNameRequest>,
) -> Json<serde_json::Value> {
    state.chat_store.write().await.set_name(&req.name);
    
    info!("👤 Name set to: {}", req.name);
    
    Json(serde_json::json!({
        "success": true,
        "name": req.name
    }))
}

/// Start the UI server
pub async fn start_ui_server(state: Arc<UiState>, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router(state);
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🌐 Peer UI available at http://localhost:{}", port);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

