//! Application State
//! 
//! Manages peer state, chat, queues, and configuration.
//! The app IS the peer - it runs discovery and accepts tasks.

use cortex_core::DeviceCapabilities;
use cortex_grid::{NodeId, PeerStore, PeerInfo, LanDiscovery, Discovery, Capabilities};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::RwLock;
use tracing::{info, error, debug};

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub from_node: String,
    pub from_name: String,
    pub content: String,
    pub timestamp: u64,
    pub is_system: bool,
    pub is_mine: bool,
}

/// Task in queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTask {
    pub id: String,
    pub task_type: String,
    pub status: String,
    pub from_node: String,
    pub size_bytes: u64,
    pub progress: u8,
    pub created_at: u64,
}

/// App configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub display_name: String,
    pub p2p_port: u16,
    pub tensor_port: u16,
    pub task_port: u16,
    pub max_queue_size: usize,
    pub auto_start: bool,
    pub contribute_on_battery: bool,
    pub max_cpu_percent: u8,
    pub max_ram_mb: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            display_name: "Anonymous".to_string(),
            p2p_port: 7654,
            tensor_port: 9000,
            task_port: 8654,
            max_queue_size: 10,
            auto_start: true,
            contribute_on_battery: false,
            max_cpu_percent: 80,
            max_ram_mb: 4096,
        }
    }
}

/// Server status
#[derive(Debug, Clone, PartialEq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

/// Main application state
pub struct AppState {
    pub node_id: NodeId,
    pub capabilities: DeviceCapabilities,
    pub peer_store: Arc<RwLock<PeerStore>>,
    pub is_contributing: bool,
    pub config: AppConfig,
    pub server_status: ServerStatus,
    
    // Chat
    pub chat_messages: VecDeque<ChatMessage>,
    pub chat_rx: Option<tokio::sync::mpsc::Receiver<ChatMessage>>,
    
    // Queues
    pub inbound_queue: VecDeque<QueueTask>,
    pub outbound_queue: VecDeque<QueueTask>,
    
    // Stats
    pub tasks_processed: u64,
    pub tasks_received: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub uptime_start: u64,
    
    // Local addresses
    pub local_ip: String,
}

impl AppState {
    pub fn new() -> Self {
        let node_id = NodeId::random();
        let capabilities = DeviceCapabilities::detect();
        let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());
        
        let mut chat_messages = VecDeque::new();
        chat_messages.push_back(ChatMessage {
            id: "welcome".to_string(),
            from_node: "system".to_string(),
            from_name: "System".to_string(),
            content: format!("Welcome to CortexOS! 🧠\nYour Node ID: {}...\nLocal IP: {}", 
                &node_id.to_string()[..12], local_ip),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
            is_system: true,
            is_mine: false,
        });
        
        info!("🆔 Node ID: {}", node_id);
        info!("💻 Device: {}", capabilities.summary());
        info!("🌐 Local IP: {}", local_ip);
        
        Self {
            node_id,
            capabilities,
            peer_store: Arc::new(RwLock::new(PeerStore::new(Duration::from_secs(300)))),
            is_contributing: true,
            config: AppConfig::default(),
            server_status: ServerStatus::Stopped,
            chat_messages,
            chat_rx: None,
            inbound_queue: VecDeque::new(),
            outbound_queue: VecDeque::new(),
            tasks_processed: 0,
            tasks_received: 0,
            bytes_received: 0,
            bytes_sent: 0,
            uptime_start: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            local_ip,
        }
    }
    
    /// Start all peer services (discovery, task server, tensor server)
    pub async fn start(&mut self) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.server_status = ServerStatus::Starting;
        
        info!("🚀 Starting CortexOS peer services...");
        
        let node_id = self.node_id.clone();
        let p2p_port = self.config.p2p_port;
        let task_port = self.config.task_port;
        let peer_store = Arc::clone(&self.peer_store);
        
        // Generate keypair for discovery
        let pubkey = rand::random::<[u8; 32]>();
        
        // 1. Start LAN Discovery (UDP multicast)
        info!("📡 Starting LAN discovery on port {}...", p2p_port);
        let (mut discovery, mut discovery_rx) = LanDiscovery::new(node_id.clone(), pubkey, p2p_port);
        
        // Handle discovered peers
        let peer_store_clone = Arc::clone(&peer_store);
        tokio::spawn(async move {
            while let Some(event) = discovery_rx.recv().await {
                info!("🔗 Discovered peer: {} at {:?}", event.peer_id.short_id(), event.addresses);
                
                let mut peer = PeerInfo::new(event.peer_id.clone(), [0u8; 32]); // Placeholder pubkey
                peer.addresses = event.addresses;
                peer.capabilities.can_compute = true;
                peer.capabilities.can_relay = true;
                
                let mut store = peer_store_clone.write().await;
                store.insert(peer).await;
            }
        });
        
        // Start discovery broadcast
        tokio::spawn(async move {
            if let Err(e) = discovery.start().await {
                error!("Discovery error: {}", e);
            }
        });
        
        // 2. Start Task Server (TCP) with chat channel
        info!("🎯 Starting task server on port {}...", task_port);
        let task_listener = TcpListener::bind(format!("0.0.0.0:{}", task_port)).await?;
        
        // Create channel for receiving chat messages
        let (chat_tx, chat_rx) = tokio::sync::mpsc::channel::<ChatMessage>(100);
        self.chat_rx = Some(chat_rx);
        
        tokio::spawn(async move {
            loop {
                match task_listener.accept().await {
                    Ok((socket, addr)) => {
                        let chat_tx = chat_tx.clone();
                        tokio::spawn(handle_task_connection(socket, addr, chat_tx));
                    }
                    Err(e) => {
                        error!("Task server error: {}", e);
                    }
                }
            }
        });
        
        self.server_status = ServerStatus::Running;
        
        // Add system message
        self.chat_messages.push_back(ChatMessage {
            id: "started".to_string(),
            from_node: "system".to_string(),
            from_name: "System".to_string(),
            content: format!("✅ Peer services started!\n📡 Discovery: port {}\n🎯 Tasks: port {}", 
                p2p_port, task_port),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
            is_system: true,
            is_mine: false,
        });
        
        info!("✅ All peer services running!");
        Ok(())
    }
    
    pub async fn get_status(&self) -> serde_json::Value {
        let peers = self.peer_store.read().await;
        let active_peers = peers.list_active().await;
        let uptime = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - self.uptime_start;
        
        let gpu_str = self.capabilities.gpu.as_ref()
            .map(|g| format!("{} ({} MB)", g.model, g.vram_mb));
        
        let server_status = match &self.server_status {
            ServerStatus::Stopped => "stopped",
            ServerStatus::Starting => "starting",
            ServerStatus::Running => "running",
            ServerStatus::Error(_) => "error",
        };
        
        serde_json::json!({
            "node_id": self.node_id.to_string(),
            "short_id": self.node_id.short_id(),
            "is_contributing": self.is_contributing,
            "server_status": server_status,
            "local_ip": self.local_ip,
            "ports": {
                "p2p": self.config.p2p_port,
                "task": self.config.task_port,
                "tensor": self.config.tensor_port,
            },
            "device": {
                "cpu_model": self.capabilities.cpu.model,
                "cpu_cores": self.capabilities.cpu.cores,
                "ram_total_mb": self.capabilities.memory.total_mb,
                "ram_available_mb": self.capabilities.memory.available_mb,
                "gpu": gpu_str,
                "capacity_score": self.capabilities.capacity_score,
                "max_layers": self.capabilities.max_layers,
            },
            "stats": {
                "peers_count": active_peers.len(),
                "tasks_processed": self.tasks_processed,
                "tasks_received": self.tasks_received,
                "bytes_received": self.bytes_received,
                "bytes_sent": self.bytes_sent,
                "inbound_queue": self.inbound_queue.len(),
                "outbound_queue": self.outbound_queue.len(),
                "uptime_seconds": uptime,
            }
        })
    }
    
    pub async fn set_contributing(&mut self, enabled: bool) {
        self.is_contributing = enabled;
        info!("Contributing: {}", enabled);
    }
    
    pub async fn get_peers(&self) -> Vec<serde_json::Value> {
        let peers = self.peer_store.read().await;
        let active = peers.list_active().await;
        active.iter().map(|p| {
            serde_json::json!({
                "node_id": p.node_id.to_string(),
                "short_id": p.node_id.short_id(),
                "address": p.addresses.first().map(|a| a.to_string()).unwrap_or_default(),
                "can_compute": p.capabilities.can_compute,
                "can_relay": p.capabilities.can_relay,
                "latency_ms": p.latency_ms,
            })
        }).collect()
    }
    
    pub async fn get_queue(&self) -> serde_json::Value {
        serde_json::json!({
            "inbound": self.inbound_queue.iter().collect::<Vec<_>>(),
            "outbound": self.outbound_queue.iter().collect::<Vec<_>>(),
        })
    }
    
    pub async fn get_chat_messages(&self) -> Vec<serde_json::Value> {
        self.chat_messages.iter().map(|m| {
            serde_json::json!({
                "id": m.id,
                "from_node": m.from_node,
                "from_name": m.from_name,
                "content": m.content,
                "timestamp": m.timestamp,
                "is_system": m.is_system,
                "is_mine": m.is_mine,
            })
        }).collect()
    }
    
    pub async fn send_chat_message(&mut self, content: &str) {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        let hash = blake3::hash(format!("{}{}{}", self.node_id, timestamp, content).as_bytes());
        
        let message = ChatMessage {
            id: hash.to_hex().to_string()[..16].to_string(),
            from_node: self.node_id.to_string(),
            from_name: self.config.display_name.clone(),
            content: content.to_string(),
            timestamp,
            is_system: false,
            is_mine: true,
        };
        
        // Add to local messages
        self.chat_messages.push_back(message.clone());
        
        // Keep only last 100 messages
        while self.chat_messages.len() > 100 {
            self.chat_messages.pop_front();
        }
        
        // Broadcast to all peers
        let peers = self.peer_store.read().await;
        let active_peers = peers.list_active().await;
        
        for peer in active_peers {
            if let Some(addr) = peer.addresses.first() {
                // Extract IP and use chat port (task_port)
                let ip = addr.ip();
                let chat_addr = format!("{}:{}", ip, self.config.task_port);
                
                let chat_msg = serde_json::json!({
                    "type": "chat",
                    "id": message.id,
                    "from_node": message.from_node,
                    "from_name": message.from_name,
                    "content": message.content,
                    "timestamp": message.timestamp,
                });
                
                // Send in background (don't block UI)
                let chat_addr_clone = chat_addr.clone();
                let msg_bytes = serde_json::to_vec(&chat_msg).unwrap_or_default();
                
                tokio::spawn(async move {
                    if let Ok(mut stream) = TcpStream::connect(&chat_addr_clone).await {
                        let _ = stream.write_all(&(msg_bytes.len() as u32).to_le_bytes()).await;
                        let _ = stream.write_all(&msg_bytes).await;
                    }
                });
            }
        }
    }
    
    /// Receive a chat message from another peer
    pub fn receive_chat_message(&mut self, msg: ChatMessage) {
        // Don't add if it's our own message or duplicate
        if msg.from_node == self.node_id.to_string() {
            return;
        }
        if self.chat_messages.iter().any(|m| m.id == msg.id) {
            return;
        }
        
        info!("💬 Chat from {}: {}", msg.from_name, msg.content);
        self.chat_messages.push_back(msg);
        
        // Keep only last 100 messages
        while self.chat_messages.len() > 100 {
            self.chat_messages.pop_front();
        }
    }
    
    /// Poll for incoming chat messages (called from UI refresh)
    pub fn poll_chat_messages(&mut self) {
        // Collect messages first to avoid borrow issues
        let mut incoming = Vec::new();
        if let Some(ref mut rx) = self.chat_rx {
            while let Ok(msg) = rx.try_recv() {
                incoming.push(msg);
            }
        }
        
        // Then process them
        for msg in incoming {
            self.receive_chat_message(msg);
        }
    }
    
    pub fn set_display_name(&mut self, name: &str) {
        self.config.display_name = name.to_string();
    }
    
    pub fn get_config(&self) -> serde_json::Value {
        serde_json::to_value(&self.config).unwrap_or_default()
    }
    
    pub fn set_config(&mut self, config: serde_json::Value) {
        if let Ok(cfg) = serde_json::from_value::<AppConfig>(config) {
            self.config = cfg;
        }
    }
}

/// Handle incoming task/chat connection
async fn handle_task_connection(
    mut socket: TcpStream, 
    addr: SocketAddr,
    chat_tx: tokio::sync::mpsc::Sender<ChatMessage>,
) {
    // Read request length
    let mut len_buf = [0u8; 4];
    if socket.read_exact(&mut len_buf).await.is_err() {
        return;
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    
    // Read request
    let mut buffer = vec![0u8; len];
    if socket.read_exact(&mut buffer).await.is_err() {
        return;
    }
    
    // Parse request
    if let Ok(request) = serde_json::from_slice::<serde_json::Value>(&buffer) {
        let msg_type = request.get("type").and_then(|v| v.as_str()).unwrap_or("task");
        
        match msg_type {
            "chat" => {
                // Handle chat message
                let chat_msg = ChatMessage {
                    id: request.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    from_node: request.get("from_node").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    from_name: request.get("from_name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                    content: request.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    timestamp: request.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0),
                    is_system: false,
                    is_mine: false,
                };
                
                // Send to main thread via channel
                let _ = chat_tx.send(chat_msg).await;
            }
            _ => {
                // Handle task
                let skill = request.get("skill").and_then(|v| v.as_str()).unwrap_or("general");
                let payload = request.get("payload").and_then(|v| v.as_str()).unwrap_or("");
                
                info!("📋 Task from {}: skill={}, payload_len={}", addr, skill, payload.len());
                
                // Execute skill (simple implementation)
                let result = match skill {
                    "math" => format!("Math result for: {}", payload),
                    "echo" | "general" => format!("Echo: {}", payload),
                    _ => format!("Unknown skill: {}", skill),
                };
                
                // Send response
                let response = serde_json::json!({
                    "success": true,
                    "result": result,
                });
                
                let response_bytes = serde_json::to_vec(&response).unwrap_or_default();
                let _ = socket.write_all(&(response_bytes.len() as u32).to_le_bytes()).await;
                let _ = socket.write_all(&response_bytes).await;
            }
        }
    }
}

/// Get local IP address
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    
    // Connect to a public IP to determine local IP
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}
