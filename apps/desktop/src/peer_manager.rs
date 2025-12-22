//! Peer Manager
//! 
//! Manages multiple peer instances from the desktop app.
//! Can start, stop, and configure peers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, error, warn, debug};

/// Configuration for a peer instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    pub name: String,
    pub p2p_port: u16,
    pub task_port: u16,
    pub tensor_port: u16,
    pub enabled: bool,
    pub auto_start: bool,
    pub max_cpu_percent: u8,
    pub max_ram_mb: u64,
    pub skills: Vec<String>,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            name: "Peer 1".to_string(),
            p2p_port: 7654,
            task_port: 8654,
            tensor_port: 9000,
            enabled: true,
            auto_start: true,
            max_cpu_percent: 80,
            max_ram_mb: 4096,
            skills: vec!["general".to_string(), "math".to_string()],
        }
    }
}

impl PeerConfig {
    pub fn with_port_offset(mut self, offset: u16) -> Self {
        self.p2p_port += offset;
        self.task_port += offset;
        self.tensor_port += offset;
        self.name = format!("Peer {}", offset / 10 + 1);
        self
    }
}

/// Status of a peer instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PeerStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

/// A managed peer instance
#[derive(Debug)]
pub struct ManagedPeer {
    pub id: String,
    pub config: PeerConfig,
    pub status: PeerStatus,
    pub pid: Option<u32>,
    pub started_at: Option<u64>,
    pub tasks_processed: u64,
    pub bytes_processed: u64,
    pub logs: Vec<String>,
    pub last_error: Option<String>,
}

impl ManagedPeer {
    pub fn new(id: String, config: PeerConfig) -> Self {
        Self {
            id,
            config,
            status: PeerStatus::Stopped,
            pid: None,
            started_at: None,
            tasks_processed: 0,
            bytes_processed: 0,
            logs: Vec::new(),
            last_error: None,
        }
    }
    
    pub fn add_log(&mut self, msg: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push(format!("[{}] {}", timestamp, msg));
        // Keep only last 50 logs
        while self.logs.len() > 50 {
            self.logs.remove(0);
        }
    }
    
    pub fn to_json(&self) -> serde_json::Value {
        let uptime = self.started_at.map(|start| {
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - start
        }).unwrap_or(0);
        
        serde_json::json!({
            "id": self.id,
            "name": self.config.name,
            "status": format!("{:?}", self.status),
            "p2p_port": self.config.p2p_port,
            "task_port": self.config.task_port,
            "tensor_port": self.config.tensor_port,
            "enabled": self.config.enabled,
            "pid": self.pid,
            "uptime_seconds": uptime,
            "tasks_processed": self.tasks_processed,
            "bytes_processed": self.bytes_processed,
            "skills": self.config.skills,
            "logs": self.logs,
            "last_error": self.last_error,
        })
    }
}

/// Manages all peer instances
pub struct PeerManager {
    peers: HashMap<String, ManagedPeer>,
    processes: HashMap<String, Child>,
    cortexd_path: String,
    pub global_logs: Vec<String>,
}

impl PeerManager {
    pub fn new() -> Self {
        // Find cortexd binary - check multiple locations
        let possible_paths = vec![
            "./target/release/cortexd".to_string(),
            "./target/debug/cortexd".to_string(),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .map(|p| p.join("cortexd").to_string_lossy().to_string())
                .unwrap_or_default(),
        ];
        
        let cortexd_path = possible_paths.iter()
            .find(|p| !p.is_empty() && std::path::Path::new(p).exists())
            .cloned()
            .unwrap_or_else(|| "./target/debug/cortexd".to_string());
        
        let exists = std::path::Path::new(&cortexd_path).exists();
        info!("🔍 Looking for cortexd at: {} (exists: {})", cortexd_path, exists);
        
        if !exists {
            error!("❌ cortexd binary not found! Build it with: cargo build -p cortex-node");
        }
        
        let init_log = format!("Peer Manager initialized. cortexd: {} (exists: {})", cortexd_path, exists);
        Self {
            peers: HashMap::new(),
            processes: HashMap::new(),
            cortexd_path,
            global_logs: vec![init_log],
        }
    }
    
    pub fn add_global_log(&mut self, msg: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.global_logs.push(format!("[{}] {}", timestamp, msg));
        while self.global_logs.len() > 100 {
            self.global_logs.remove(0);
        }
    }
    
    /// Add a new peer configuration
    pub fn add_peer(&mut self, config: PeerConfig) -> String {
        let id = format!("peer-{}", blake3::hash(format!("{:?}{}", config, rand::random::<u32>()).as_bytes()).to_hex()[..8].to_string());
        let peer = ManagedPeer::new(id.clone(), config);
        self.peers.insert(id.clone(), peer);
        info!("Added peer: {}", id);
        id
    }
    
    /// Start a peer
    pub fn start_peer(&mut self, id: &str) -> Result<(), String> {
        // Check if cortexd exists
        if !std::path::Path::new(&self.cortexd_path).exists() {
            let err = format!("cortexd not found at: {}. Build with: cargo build -p cortex-node", self.cortexd_path);
            self.add_global_log(format!("❌ {}", err));
            return Err(err);
        }
        
        // Get peer config info first
        let (p2p_port, skills, name) = {
            let peer = self.peers.get(id).ok_or("Peer not found")?;
            if peer.status == PeerStatus::Running {
                return Err("Peer already running".to_string());
            }
            (peer.config.p2p_port, peer.config.skills.clone(), peer.config.name.clone())
        };
        
        // Build command
        let cmd_str = format!(
            "{} --port {} --compute --skills {}",
            self.cortexd_path,
            p2p_port,
            skills.join(",")
        );
        self.add_global_log(format!("Starting {}: {}", name, cmd_str));
        
        // Update peer status
        if let Some(peer) = self.peers.get_mut(id) {
            peer.status = PeerStatus::Starting;
            peer.last_error = None;
            peer.add_log(format!("Starting peer with cortexd: {}", self.cortexd_path));
            peer.add_log(format!("Command: {}", cmd_str));
        }
        
        let mut cmd = Command::new(&self.cortexd_path);
        cmd.arg("--port").arg(p2p_port.to_string())
           .arg("--compute");
        
        if !skills.is_empty() {
            cmd.arg("--skills").arg(skills.join(","));
        }
        
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                
                // Update peer
                if let Some(peer) = self.peers.get_mut(id) {
                    peer.pid = Some(pid);
                    peer.status = PeerStatus::Running;
                    peer.started_at = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
                    peer.add_log(format!("✅ Started with PID: {}", pid));
                }
                
                self.processes.insert(id.to_string(), child);
                self.add_global_log(format!("✅ {} started (PID: {})", name, pid));
                
                info!("Started peer {} (PID: {})", id, pid);
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Failed to start: {}", e);
                
                if let Some(peer) = self.peers.get_mut(id) {
                    peer.status = PeerStatus::Error(err_msg.clone());
                    peer.last_error = Some(err_msg.clone());
                    peer.add_log(format!("❌ {}", err_msg));
                }
                
                self.add_global_log(format!("❌ {} failed: {}", name, e));
                error!("Failed to start peer {}: {}", id, e);
                Err(err_msg)
            }
        }
    }
    
    /// Stop a peer
    pub fn stop_peer(&mut self, id: &str) -> Result<(), String> {
        let peer = self.peers.get_mut(id).ok_or("Peer not found")?;
        
        if let Some(mut child) = self.processes.remove(id) {
            match child.kill() {
                Ok(_) => {
                    info!("Stopped peer {} (PID: {:?})", id, peer.pid);
                }
                Err(e) => {
                    warn!("Failed to kill peer process: {}", e);
                }
            }
        }
        
        peer.status = PeerStatus::Stopped;
        peer.pid = None;
        peer.started_at = None;
        
        Ok(())
    }
    
    /// Remove a peer (must be stopped first)
    pub fn remove_peer(&mut self, id: &str) -> Result<(), String> {
        let peer = self.peers.get(id).ok_or("Peer not found")?;
        
        if peer.status == PeerStatus::Running {
            return Err("Stop the peer first".to_string());
        }
        
        self.peers.remove(id);
        info!("Removed peer: {}", id);
        Ok(())
    }
    
    /// Update peer config (must be stopped first)
    pub fn update_peer_config(&mut self, id: &str, config: PeerConfig) -> Result<(), String> {
        let peer = self.peers.get_mut(id).ok_or("Peer not found")?;
        
        if peer.status == PeerStatus::Running {
            return Err("Stop the peer first".to_string());
        }
        
        peer.config = config;
        Ok(())
    }
    
    /// Get all peers
    pub fn get_peers(&self) -> Vec<serde_json::Value> {
        self.peers.values().map(|p| p.to_json()).collect()
    }
    
    /// Get a specific peer
    pub fn get_peer(&self, id: &str) -> Option<serde_json::Value> {
        self.peers.get(id).map(|p| p.to_json())
    }
    
    /// Check and update peer statuses
    pub fn refresh_statuses(&mut self) {
        let mut dead_peers = Vec::new();
        
        for (id, process) in &mut self.processes {
            match process.try_wait() {
                Ok(Some(status)) => {
                    // Process has exited
                    if let Some(peer) = self.peers.get_mut(id) {
                        if status.success() {
                            peer.status = PeerStatus::Stopped;
                        } else {
                            peer.status = PeerStatus::Error(format!("Exit code: {:?}", status.code()));
                        }
                        peer.pid = None;
                    }
                    dead_peers.push(id.clone());
                }
                Ok(None) => {
                    // Still running
                }
                Err(e) => {
                    error!("Error checking process status: {}", e);
                }
            }
        }
        
        for id in dead_peers {
            self.processes.remove(&id);
        }
    }
    
    /// Start all enabled peers
    pub fn start_all(&mut self) {
        let ids: Vec<String> = self.peers.iter()
            .filter(|(_, p)| p.config.enabled && p.status == PeerStatus::Stopped)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in ids {
            if let Err(e) = self.start_peer(&id) {
                error!("Failed to start {}: {}", id, e);
            }
        }
    }
    
    /// Stop all running peers
    pub fn stop_all(&mut self) {
        let ids: Vec<String> = self.peers.iter()
            .filter(|(_, p)| p.status == PeerStatus::Running)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in ids {
            if let Err(e) = self.stop_peer(&id) {
                error!("Failed to stop {}: {}", id, e);
            }
        }
    }
    
    /// Get summary stats
    pub fn get_summary(&self) -> serde_json::Value {
        let total = self.peers.len();
        let running = self.peers.values().filter(|p| p.status == PeerStatus::Running).count();
        let stopped = self.peers.values().filter(|p| p.status == PeerStatus::Stopped).count();
        let errors = self.peers.values().filter(|p| matches!(p.status, PeerStatus::Error(_))).count();
        
        serde_json::json!({
            "total_peers": total,
            "running": running,
            "stopped": stopped,
            "errors": errors,
        })
    }
}

impl Drop for PeerManager {
    fn drop(&mut self) {
        // Stop all peers when manager is dropped
        self.stop_all();
    }
}

