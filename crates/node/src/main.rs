use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tokio::sync::RwLock;
use tracing::{info, Level};

use cortex_grid::{NodeId, RelayNode, LanDiscovery, Discovery, PeerStore, PeerInfo};
use cortex_reputation::{TrustGraph, SkillId};
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

    let config = NodeConfig::new(cli.name, cli.port, cli.data_dir, cli.skills);

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
    let trust_graph = Arc::new(RwLock::new(TrustGraph::new(node_id)));
    let skill_registry = Arc::new(RwLock::new(NetworkSkillRegistry::new(node_id)));

    // Register local skills
    if !config.skills.is_empty() {
        let mut registry = skill_registry.write().await;
        for skill in &config.skills {
            registry.register_my_skill(SkillId::new(skill));
            info!("📚 Registered skill: {}", skill);
        }
        info!("");
    }

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
    tokio::spawn(async move {
        while let Some(event) = discovery_rx.recv().await {
            info!("✨ Discovered peer: {} at {:?}", event.peer_id, event.addresses);
            
            // Create peer info and insert
            let mut peer = PeerInfo::new(event.peer_id, [0u8; 32]);
            peer.addresses = event.addresses;
            peer_store_clone.insert(peer).await;
        }
    });

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
