use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tokio::sync::RwLock;
use tracing::{info, Level};

use cortex_core::runtime::{EventBus, Runtime};
use cortex_grid::{
    Capabilities, Discovery, GridOrchestrator, KademliaDiscovery, LanDiscovery, NodeId, PeerInfo,
    PeerStore, RelayNode,
};
use cortex_reputation::{SkillId, TrustGraph};
use cortex_skill::NetworkSkillRegistry;

mod config;
mod network;

use config::NodeConfig;

#[derive(Parser)]
#[command(name = "cortexd")]
#[command(about = "CortexOS Node Daemon - decentralized AI network", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Node name (defaults to hostname)
    #[arg(short, long)]
    name: Option<String>,

    /// Port to listen on
    #[arg(short, long, default_value = "7654")]
    port: u16,

    /// Data directory
    #[arg(short, long)]
    data_dir: Option<String>,

    /// Skills this node provides (comma-separated)
    #[arg(short, long)]
    skills: Option<String>,

    /// Enable Kademlia wide-area discovery
    #[arg(long, default_value = "true")]
    kademlia: bool,

    /// Enable grid orchestrator
    #[arg(long, default_value = "true")]
    orchestrator: bool,

    /// Enable compute capability
    #[arg(long, default_value = "true")]
    compute: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the node daemon
    Start,
    /// Show node status
    Status,
    /// List discovered peers
    Peers,
    /// List known skills in network
    Skills,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    let mut config = NodeConfig::new(cli.name, cli.port, cli.data_dir, cli.skills);
    config.enable_kademlia = cli.kademlia;
    config.enable_orchestrator = cli.orchestrator;
    config.can_compute = cli.compute;

    match cli.command {
        Some(Commands::Start) | None => {
            run_daemon(config).await?;
        }
        Some(Commands::Status) => {
            println!("Node status: (not implemented yet)");
        }
        Some(Commands::Peers) => {
            println!("Peers: (not implemented yet)");
        }
        Some(Commands::Skills) => {
            println!("Skills: (not implemented yet)");
        }
    }

    Ok(())
}

async fn run_daemon(config: NodeConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!("🧠 CortexOS Node Daemon");
    info!("   Version: 0.1.0");
    info!("");

    // Generate or load node ID
    let node_id = NodeId::random();

    // Generate a random pubkey for discovery
    let mut pubkey = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut pubkey);

    info!("📍 Node ID: {}", node_id);
    info!("   Name: {}", config.name);
    info!("   Port: {}", config.port);
    info!("");

    // Initialize components
    let peer_store = Arc::new(PeerStore::new(Duration::from_secs(120)));
    let _trust_graph = Arc::new(RwLock::new(TrustGraph::new(node_id)));
    let skill_registry = Arc::new(RwLock::new(NetworkSkillRegistry::new(node_id)));

    // Initialize event bus and runtime for orchestrator
    let event_bus = Arc::new(EventBus::default());
    let _runtime = Arc::new(Runtime::new());

    // Register local skills
    if !config.skills.is_empty() {
        let mut registry = skill_registry.write().await;
        for skill in &config.skills {
            registry.register_my_skill(SkillId::new(skill));
            info!("📚 Registered skill: {}", skill);
        }
        info!("");
    }

    // Set peer capabilities based on config
    let local_capabilities = Capabilities {
        can_relay: true,
        can_store: false,
        can_compute: config.can_compute,
        max_storage_mb: 0,
    };
    info!(
        "💪 Node capabilities: compute={}, relay=true",
        config.can_compute
    );

    // Start relay node (AirTag-style mesh)
    let (relay_node, mut relay_rx) = RelayNode::new(node_id);
    relay_node.start().await?;
    info!("📡 Relay mesh started");

    // Start LAN discovery
    info!("🔍 Starting LAN discovery...");
    let (mut discovery, mut discovery_rx) = LanDiscovery::new(node_id, pubkey, config.port);
    discovery.start().await?;

    // Spawn discovery handler
    let peer_store_clone = Arc::clone(&peer_store);
    let local_caps = local_capabilities.clone();
    tokio::spawn(async move {
        while let Some(event) = discovery_rx.recv().await {
            info!(
                "✨ Discovered peer: {} at {:?}",
                event.peer_id, event.addresses
            );

            // Create peer info and insert
            let mut peer = PeerInfo::new(event.peer_id, [0u8; 32]);
            peer.addresses = event.addresses;
            peer.capabilities = local_caps.clone();
            peer_store_clone.insert(peer).await;
        }
    });

    // Start Kademlia discovery if enabled
    if config.enable_kademlia {
        info!("🌐 Starting Kademlia wide-area discovery...");
        match KademliaDiscovery::new(node_id, pubkey, config.port) {
            Ok((mut kad_discovery, mut kad_rx)) => {
                kad_discovery.start().await?;

                let peer_store_kad = Arc::clone(&peer_store);
                let local_caps_kad = local_capabilities.clone();
                tokio::spawn(async move {
                    while let Some(event) = kad_rx.recv().await {
                        info!(
                            "🌍 Kademlia discovered peer: {} at {:?}",
                            event.peer_id, event.addresses
                        );

                        let mut peer = PeerInfo::new(event.peer_id, [0u8; 32]);
                        peer.addresses = event.addresses;
                        peer.capabilities = local_caps_kad.clone();
                        peer_store_kad.insert(peer).await;
                    }
                });
            }
            Err(e) => {
                info!("⚠️  Kademlia discovery initialization failed: {}", e);
            }
        }
    }

    // Start Grid Orchestrator if enabled
    if config.enable_orchestrator {
        info!("🎯 Starting Grid Orchestrator...");
        let mut orchestrator = GridOrchestrator::new(
            node_id,
            Arc::clone(&peer_store).as_ref().clone(),
            Arc::clone(&event_bus),
        );

        orchestrator.start().await?;
        info!("✅ Grid Orchestrator started");
    }

    // Spawn relay message handler
    tokio::spawn(async move {
        while let Some(_msg) = relay_rx.recv().await {
            info!("📨 Relay message received");
        }
    });

    info!("");
    info!("═══════════════════════════════════════════════════════════");
    info!("  🚀 Node is running!");
    info!("");
    info!("  To test distribution, open another terminal and run:");
    info!("    cargo run -p cortexd -- --port 7655 --skills coding");
    info!("");
    info!("  Nodes will auto-discover each other on your local network!");
    info!("═══════════════════════════════════════════════════════════");
    info!("");

    // Print periodic status
    let peer_store_status = Arc::clone(&peer_store);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            let peer_count = peer_store_status.count().await;
            info!("📊 Status: {} peers connected", peer_count);
        }
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("\n🛑 Shutting down...");

    relay_node.stop().await;
    discovery.stop().await?;
    info!("✅ Node stopped");

    Ok(())
}
