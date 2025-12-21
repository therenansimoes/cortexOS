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

use cortex_grid::{NodeId, PeerStore, GridOrchestrator};
use cortex_skill::{NetworkSkillRegistry, SkillId};
use cortex_reputation::TrustGraph;
use cortex_core::runtime::EventBus;

mod api;
mod dashboard;

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
    let peer_store = Arc::new(PeerStore::new(Duration::from_secs(120)));
    let skill_registry = Arc::new(RwLock::new(NetworkSkillRegistry::new(node_id)));
    let trust_graph = Arc::new(RwLock::new(TrustGraph::new(node_id)));
    let event_bus = Arc::new(EventBus::default());

    // Create orchestrator
    let orchestrator = GridOrchestrator::new(
        node_id,
        peer_store.as_ref().clone(),
        Arc::clone(&event_bus),
    );
    orchestrator.start().await?;
    let orchestrator = Arc::new(RwLock::new(orchestrator));

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
        .route("/api/status", get(get_status))
        .route("/api/peers", get(get_peers))
        .route("/api/skills", get(get_skills))
        .route("/api/tasks", get(get_tasks))
        .route("/api/tasks/delegate", post(delegate_task))
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

