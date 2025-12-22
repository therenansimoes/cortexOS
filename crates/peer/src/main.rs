//! CortexOS Peer
//! 
//! A distributed AI peer that contributes compute power to the swarm.
//! Download, run, contribute - that's it!
//! 
//! Works on: macOS, Linux, Windows, iOS, Android

mod chat;
mod ui;

use clap::Parser;
use cortex_core::{
    DeviceCapabilities, TaskQueue, TensorChunk, ProcessedChunk,
};
use cortex_grid::{Discovery, LanDiscovery, PeerInfo, PeerStore, NodeId};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tracing::{debug, error, info, Level};

#[derive(Parser)]
#[command(name = "cortex-peer")]
#[command(about = "CortexOS distributed AI peer - contribute your compute!")]
struct Args {
    /// Port for peer-to-peer communication
    #[arg(short, long, default_value = "7654")]
    port: u16,
    
    /// Port for tensor streaming
    #[arg(long, default_value = "9000")]
    tensor_port: u16,
    
    /// Port for web UI
    #[arg(long, default_value = "3000")]
    ui_port: u16,
    
    /// Enable compute contribution (process AI tasks)
    #[arg(long, default_value = "true")]
    contribute: bool,
    
    /// Maximum queue size (tasks to buffer)
    #[arg(long, default_value = "10")]
    max_queue: usize,
}

/// Peer state
pub struct PeerState {
    pub node_id: NodeId,
    pub capabilities: DeviceCapabilities,
    pub task_queue: TaskQueue,
    pub peer_store: Arc<PeerStore>,
    pub is_active: Arc<RwLock<bool>>,
    pub stats: Arc<RwLock<PeerStats>>,
}

#[derive(Default)]
pub struct PeerStats {
    pub tasks_received: u64,
    pub tasks_processed: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub uptime_seconds: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();
    
    let args = Args::parse();
    
    // Generate node ID
    let node_id = NodeId::random();
    
    // Detect REAL device capabilities
    let capabilities = DeviceCapabilities::detect();
    
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║               🧠 CortexOS Distributed AI Peer                ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Contribute your compute to the decentralized AI swarm!      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    info!("📱 Device detected:");
    info!("   CPU: {} ({} cores)", capabilities.cpu.model, capabilities.cpu.cores);
    info!("   RAM: {} MB total, {} MB available", 
          capabilities.memory.total_mb, capabilities.memory.available_mb);
    if let Some(ref gpu) = capabilities.gpu {
        info!("   GPU: {} ({} MB VRAM)", gpu.model, gpu.vram_mb);
    }
    info!("   Capacity Score: {}/100", capabilities.capacity_score);
    info!("   Max Layers: {}", capabilities.max_layers);
    info!("");
    info!("🆔 Node ID: {}", node_id);
    info!("🔌 P2P Port: {}", args.port);
    info!("📡 Tensor Port: {}", args.tensor_port);
    info!("");
    
    // Create peer state
    let peer_store = Arc::new(PeerStore::new(Duration::from_secs(300)));
    
    let state = Arc::new(PeerState {
        node_id: node_id.clone(),
        capabilities: capabilities.clone(),
        task_queue: TaskQueue::new(args.max_queue),
        peer_store: Arc::clone(&peer_store),
        is_active: Arc::new(RwLock::new(true)),
        stats: Arc::new(RwLock::new(PeerStats::default())),
    });
    
    // Start discovery
    let pubkey = [0u8; 32]; // Placeholder pubkey
    let (mut discovery, mut discovery_rx) = LanDiscovery::new(node_id.clone(), pubkey, args.port);
    info!("🔍 Starting peer discovery...");
    
    // Handle discovery events
    let peer_store_clone = Arc::clone(&peer_store);
    let caps = capabilities.clone();
    
    tokio::spawn(async move {
        while let Some(event) = discovery_rx.recv().await {
            // event is DiscoveryEvent { peer_id, addresses }
            let mut peer = PeerInfo::new(event.peer_id.clone(), pubkey);
            // Add addresses (SocketAddr type)
            peer.addresses.extend(event.addresses);
            peer.capabilities.can_compute = true;
            peer.capabilities.max_storage_mb = caps.memory.available_mb as u32;
            
            peer_store_clone.insert(peer).await;
            info!("🔗 Discovered peer: {}", event.peer_id);
        }
    });
    
    // Start discovery broadcast
    tokio::spawn(async move {
        if let Err(e) = discovery.start().await {
            error!("Discovery error: {}", e);
        }
    });
    
    // Start tensor server
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        if let Err(e) = run_tensor_server(state_clone, args.tensor_port).await {
            error!("Tensor server error: {}", e);
        }
    });
    
    // Start task processor
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        run_task_processor(state_clone).await;
    });
    
    // Start Web UI
    let chat_store = Arc::new(RwLock::new(chat::ChatStore::new("Anonymous")));
    let ui_state = Arc::new(ui::UiState {
        peer_state: Arc::clone(&state),
        chat_store: Arc::clone(&chat_store),
    });
    let ui_port = args.ui_port;
    tokio::spawn(async move {
        if let Err(e) = ui::start_ui_server(ui_state, ui_port).await {
            error!("UI server error: {}", e);
        }
    });
    
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  🌐 Web UI: http://localhost:{}                            ║", args.ui_port);
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    // Print status periodically
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        
        let peers = state.peer_store.list_active().await;
        let queue_stats = state.task_queue.stats().await;
        let stats = state.stats.read().await;
        
        info!("📊 Status: {} peers | Queue: {} | Processed: {} | Received: {} bytes",
              peers.len(),
              queue_stats.current_queue_size,
              stats.tasks_processed,
              stats.bytes_received);
    }
}

/// TCP server for receiving tensor chunks
async fn run_tensor_server(
    state: Arc<PeerState>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🎧 Tensor server listening on port {}", port);
    
    loop {
        let (stream, addr) = listener.accept().await?;
        debug!("📥 Tensor connection from {}", addr);
        
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = handle_tensor_connection(state, stream).await {
                error!("Connection error: {}", e);
            }
        });
    }
}

/// Handle incoming tensor stream
async fn handle_tensor_connection(
    state: Arc<PeerState>,
    mut stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read message length
    let mut len_buf = [0u8; 8];
    stream.read_exact(&mut len_buf).await?;
    let len = u64::from_le_bytes(len_buf) as usize;
    
    // Read message
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;
    
    // Update stats
    {
        let mut stats = state.stats.write().await;
        stats.bytes_received += len as u64;
        stats.tasks_received += 1;
    }
    
    // Deserialize chunk
    let chunk: TensorChunk = bincode::deserialize(&data)?;
    info!("📦 Received chunk {}/{} for task {} (layers {}-{})",
          chunk.chunk_idx, chunk.total_chunks, 
          &chunk.task_id[..8.min(chunk.task_id.len())],
          chunk.start_layer, chunk.end_layer);
    
    // Enqueue for processing
    state.task_queue.enqueue(chunk).await?;
    
    // Send acknowledgment
    let ack = b"ACK";
    stream.write_all(ack).await?;
    
    Ok(())
}

/// Process tasks from the queue
async fn run_task_processor(state: Arc<PeerState>) {
    info!("⚙️ Task processor started");
    
    loop {
        // Wait for task
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        if !*state.is_active.read().await {
            continue;
        }
        
        // Get next task
        if let Some(chunk) = state.task_queue.dequeue().await {
            let start = std::time::Instant::now();
            
            info!("🔧 Processing chunk {}/{} (layers {}-{})",
                  chunk.chunk_idx, chunk.total_chunks,
                  chunk.start_layer, chunk.end_layer);
            
            // Process the chunk through our layers
            let result_data = process_chunk(&state.capabilities, &chunk).await;
            
            let processing_time = start.elapsed().as_millis() as u64;
            
            // Create result
            let processed = ProcessedChunk {
                task_id: chunk.task_id.clone(),
                chunk_idx: chunk.chunk_idx,
                total_chunks: chunk.total_chunks,
                result_data,
                result_shape: chunk.shape.clone(),
                processing_time_ms: processing_time,
                processor_node: state.node_id.to_string(),
            };
            
            // Mark complete
            state.task_queue.complete(processed.clone()).await;
            
            // Update stats
            {
                let mut stats = state.stats.write().await;
                stats.tasks_processed += 1;
            }
            
            info!("✅ Chunk processed in {}ms", processing_time);
            
            // Send result back to source
            if let Err(e) = send_result_back(&chunk.source_node, &processed).await {
                error!("Failed to send result: {}", e);
            }
        }
    }
}

/// Process a tensor chunk through assigned layers
async fn process_chunk(
    capabilities: &DeviceCapabilities,
    chunk: &TensorChunk,
) -> Vec<u8> {
    // In production, this would:
    // 1. Load the assigned layers of the model
    // 2. Run the tensor through those layers
    // 3. Return the processed tensor
    
    // For now, simulate processing based on device capability
    let layers = chunk.end_layer - chunk.start_layer + 1;
    let process_time_ms = (layers as u64 * 50) / (capabilities.capacity_score as u64 + 1);
    
    tokio::time::sleep(tokio::time::Duration::from_millis(process_time_ms)).await;
    
    // Return "processed" data (in real impl, actual tensor computation)
    let mut result = chunk.tensor_data.clone();
    
    // Simulate transformation
    for byte in result.iter_mut() {
        *byte = byte.wrapping_add(1);
    }
    
    result
}

/// Send processed result back to the requesting node
async fn send_result_back(
    source_addr: &str,
    result: &ProcessedChunk,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse address and connect
    let addr = if source_addr.contains(':') {
        source_addr.to_string()
    } else {
        format!("{}:9000", source_addr)
    };
    
    let mut stream = TcpStream::connect(&addr).await?;
    
    // Serialize and send
    let data = bincode::serialize(result)?;
    let len = data.len() as u64;
    
    stream.write_all(&len.to_le_bytes()).await?;
    stream.write_all(&data).await?;
    
    info!("📤 Sent result back to {}", addr);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_device_detection() {
        let caps = DeviceCapabilities::detect();
        
        println!("Device Type: {:?}", caps.device_type);
        println!("CPU: {}", caps.cpu.model);
        println!("Cores: {}", caps.cpu.cores);
        println!("RAM: {} MB", caps.memory.total_mb);
        println!("Score: {}", caps.capacity_score);
        println!("Max Layers: {}", caps.max_layers);
        
        assert!(caps.capacity_score > 0);
        assert!(caps.max_layers > 0);
    }
}

