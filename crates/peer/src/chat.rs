//! P2P Chat System
//! 
//! Send messages directly to other peers in the network.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub from_node: String,
    pub from_name: String,
    pub to_node: Option<String>, // None = broadcast to all
    pub content: String,
    pub timestamp: u64,
    pub is_system: bool,
}

impl ChatMessage {
    pub fn new(from_node: &str, from_name: &str, content: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let hash = blake3::hash(format!("{}{}{}", from_node, timestamp, content).as_bytes());
        
        Self {
            id: hash.to_hex().to_string()[..16].to_string(),
            from_node: from_node.to_string(),
            from_name: from_name.to_string(),
            to_node: None,
            content: content.to_string(),
            timestamp,
            is_system: false,
        }
    }
    
    pub fn system(content: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let hash = blake3::hash(format!("system{}{}", timestamp, content).as_bytes());
        
        Self {
            id: hash.to_hex().to_string()[..16].to_string(),
            from_node: "system".to_string(),
            from_name: "System".to_string(),
            to_node: None,
            content: content.to_string(),
            timestamp,
            is_system: true,
        }
    }
}

/// Chat history and state
pub struct ChatStore {
    messages: VecDeque<ChatMessage>,
    max_messages: usize,
    my_name: String,
}

impl ChatStore {
    pub fn new(my_name: &str) -> Self {
        let mut store = Self {
            messages: VecDeque::new(),
            max_messages: 100,
            my_name: my_name.to_string(),
        };
        
        // Welcome message
        store.add_message(ChatMessage::system("Welcome to CortexOS P2P Chat! 🧠"));
        
        store
    }
    
    pub fn add_message(&mut self, msg: ChatMessage) {
        self.messages.push_back(msg);
        if self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }
    
    pub fn get_messages(&self, count: usize) -> Vec<ChatMessage> {
        self.messages.iter().rev().take(count).cloned().collect::<Vec<_>>().into_iter().rev().collect()
    }
    
    pub fn set_name(&mut self, name: &str) {
        self.my_name = name.to_string();
    }
    
    pub fn get_name(&self) -> &str {
        &self.my_name
    }
}

/// P2P Chat message packet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPacket {
    pub packet_type: String, // "chat", "join", "leave"
    pub message: ChatMessage,
}

/// Send a chat message to a peer
pub async fn send_chat_to_peer(
    peer_addr: &str,
    message: &ChatMessage,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let packet = ChatPacket {
        packet_type: "chat".to_string(),
        message: message.clone(),
    };
    
    let data = bincode::serialize(&packet)?;
    
    // Connect to peer's chat port (tensor port + 1)
    let addr = if peer_addr.contains(':') {
        let parts: Vec<&str> = peer_addr.split(':').collect();
        let port: u16 = parts.last().unwrap_or(&"9000").parse().unwrap_or(9000);
        format!("{}:{}", parts[0], port + 1)
    } else {
        format!("{}:9001", peer_addr)
    };
    
    let mut stream = TcpStream::connect(&addr).await?;
    
    let len = data.len() as u64;
    stream.write_all(&len.to_le_bytes()).await?;
    stream.write_all(&data).await?;
    stream.flush().await?;
    
    debug!("📤 Sent chat to {}", addr);
    
    Ok(())
}

/// Broadcast a chat message to all peers
pub async fn broadcast_chat(
    peers: &[(String, String)], // (node_id, address)
    message: &ChatMessage,
) -> Vec<String> {
    let mut success = Vec::new();
    
    for (node_id, addr) in peers {
        match send_chat_to_peer(addr, message).await {
            Ok(_) => {
                success.push(node_id.clone());
            }
            Err(e) => {
                debug!("Failed to send to {}: {}", node_id, e);
            }
        }
    }
    
    success
}

