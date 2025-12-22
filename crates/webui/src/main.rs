use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use cortex_grid::{NodeId, PeerStore, PeerInfo, Capabilities, GridOrchestrator, LanDiscovery, KademliaDiscovery, Discovery};
use cortex_skill::NetworkSkillRegistry;
use cortex_reputation::TrustGraph;
use cortex_core::runtime::EventBus;

mod api;
mod dashboard;
mod distributed;
mod logs;
mod swarm;

pub use logs::LOGS;

use api::*;

#[derive(Clone)]
struct AppState {
    node_id: NodeId,
    peer_store: Arc<PeerStore>,
    skill_registry: Arc<RwLock<NetworkSkillRegistry>>,
    trust_graph: Arc<RwLock<TrustGraph>>,
    event_bus: Arc<EventBus>,
    orchestrator: Option<Arc<RwLock<GridOrchestrator>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Initialize node components
    let node_id = NodeId::random();
    let pubkey = [0u8; 32]; // Placeholder pubkey for discovery
    let peer_store = Arc::new(PeerStore::new(Duration::from_secs(120)));
    let skill_registry = Arc::new(RwLock::new(NetworkSkillRegistry::new(node_id)));
    let trust_graph = Arc::new(RwLock::new(TrustGraph::new(node_id)));
    let event_bus = Arc::new(EventBus::default());

    // Create orchestrator
    let mut orchestrator = GridOrchestrator::new(
        node_id,
        peer_store.as_ref().clone(),
        Arc::clone(&event_bus),
    );
    orchestrator.start().await?;
    let orchestrator = Arc::new(RwLock::new(orchestrator));

    // Start LAN discovery to find other nodes
    let (mut lan_discovery, mut lan_rx) = LanDiscovery::new(node_id, pubkey, 8080);
    lan_discovery.start().await?;
    tracing::info!("🔍 Started LAN discovery for peer detection");

    // Start Kademlia discovery
    let (mut kad_discovery, mut kad_rx) = KademliaDiscovery::new(node_id, pubkey, 8080)?;
    kad_discovery.start().await?;
    tracing::info!("🌐 Started Kademlia discovery");

    // Spawn a task to handle discovered peers
    let peer_store_clone = Arc::clone(&peer_store);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(event) = lan_rx.recv() => {
                    let mut peer = PeerInfo::new(event.peer_id, [0u8; 32]);
                    // Set capabilities - discovered nodes are assumed to be compute-capable
                    peer.capabilities = Capabilities {
                        can_relay: true,
                        can_store: true,
                        can_compute: true,
                        max_storage_mb: 1024,
                    };
                    peer.addresses = event.addresses;
                    peer_store_clone.insert(peer).await;
                    tracing::info!("📡 LAN discovered compute peer: {:?}", event.peer_id);
                }
                Some(event) = kad_rx.recv() => {
                    let mut peer = PeerInfo::new(event.peer_id, [0u8; 32]);
                    // Set capabilities - discovered nodes are assumed to be compute-capable
                    peer.capabilities = Capabilities {
                        can_relay: true,
                        can_store: true,
                        can_compute: true,
                        max_storage_mb: 1024,
                    };
                    peer.addresses = event.addresses;
                    peer_store_clone.insert(peer).await;
                    tracing::info!("🌐 Kademlia discovered compute peer: {:?}", event.peer_id);
                }
            }
        }
    });

    let app_state = AppState {
        node_id,
        peer_store,
        skill_registry,
        trust_graph,
        event_bus,
        orchestrator: Some(orchestrator),
    };

    // Build router
    let app = Router::new()
        .route("/", get(index))
        .route("/admin", get(admin))
        .route("/docs", get(docs))
        .route("/manifest.json", get(manifest))
        .route("/api/status", get(get_status))
        .route("/api/peers", get(get_peers))
        .route("/api/peers/detailed", get(get_peers_detailed))
        .route("/api/system", get(get_system_info))
        .route("/api/logs", get(get_logs))
        .route("/api/logs/clear", post(clear_logs))
        .route("/api/skills", get(get_skills))
        .route("/api/tasks", get(get_tasks))
        .route("/api/tasks/delegate", post(delegate_task))
        .route("/api/tasks/swarm", post(swarm_task))
        .route("/api/tasks/distributed", post(distributed_task))
        .route("/api/tasks/pipeline", post(pipeline_task))
        .route("/api/tasks/tensor", post(distributed_tensor_inference))
        .route("/api/pipeline/status", get(pipeline_status))
        .route("/api/stats", get(get_stats))
        // Static files are embedded in the binary
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("🌐 CortexOS Web UI running on http://localhost:8080");
    
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn admin() -> Html<&'static str> {
    Html(include_str!("../static/admin.html"))
}

async fn docs() -> Html<&'static str> {
    Html(include_str!("../static/docs.html"))
}

async fn manifest() -> (axum::http::HeaderMap, &'static str) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/manifest+json".parse().unwrap(),
    );
    (headers, include_str!("../static/manifest.json"))
}

