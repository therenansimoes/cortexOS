use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        Self(bytes)
    }

    pub fn from_pubkey(pubkey: &[u8; 32]) -> Self {
        let hash = blake3::hash(pubkey);
        Self(*hash.as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn short_id(&self) -> String {
        hex::encode(&self.0[..4])
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..8]))
    }
}

impl AsRef<[u8]> for NodeId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub can_relay: bool,
    pub can_store: bool,
    pub can_compute: bool,
    pub max_storage_mb: u32,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            can_relay: true,
            can_store: false,
            can_compute: false,
            max_storage_mb: 0,
        }
    }
}

impl Capabilities {
    pub fn encode(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        bincode::deserialize(data).ok()
    }
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub pubkey: [u8; 32],
    pub addresses: Vec<SocketAddr>,
    pub capabilities: Capabilities,
    pub last_seen: Instant,
    pub latency_ms: Option<u32>,
    pub reputation: i32,
}

impl PeerInfo {
    pub fn new(node_id: NodeId, pubkey: [u8; 32]) -> Self {
        Self {
            node_id,
            pubkey,
            addresses: Vec::new(),
            capabilities: Capabilities::default(),
            last_seen: Instant::now(),
            latency_ms: None,
            reputation: 0,
        }
    }

    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed() > timeout
    }

    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }
}

pub struct PeerStore {
    peers: Arc<RwLock<HashMap<NodeId, PeerInfo>>>,
    stale_timeout: Duration,
}

impl PeerStore {
    pub fn new(stale_timeout: Duration) -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            stale_timeout,
        }
    }

    pub async fn insert(&self, peer: PeerInfo) {
        let mut peers = self.peers.write().await;
        peers.insert(peer.node_id, peer);
    }

    pub async fn get(&self, node_id: &NodeId) -> Option<PeerInfo> {
        let peers = self.peers.read().await;
        peers.get(node_id).cloned()
    }

    pub async fn remove(&self, node_id: &NodeId) -> Option<PeerInfo> {
        let mut peers = self.peers.write().await;
        peers.remove(node_id)
    }

    pub async fn touch(&self, node_id: &NodeId) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.touch();
        }
    }

    pub async fn update_latency(&self, node_id: &NodeId, latency_ms: u32) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.latency_ms = Some(latency_ms);
        }
    }

    pub async fn list_active(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers
            .values()
            .filter(|p| !p.is_stale(self.stale_timeout))
            .cloned()
            .collect()
    }

    pub async fn prune_stale(&self) -> usize {
        let mut peers = self.peers.write().await;
        let before = peers.len();
        peers.retain(|_, p| !p.is_stale(self.stale_timeout));
        before - peers.len()
    }

    pub async fn count(&self) -> usize {
        self.peers.read().await.len()
    }

    pub async fn find_by_capability<F>(&self, predicate: F) -> Vec<PeerInfo>
    where
        F: Fn(&Capabilities) -> bool,
    {
        let peers = self.peers.read().await;
        peers
            .values()
            .filter(|p| predicate(&p.capabilities) && !p.is_stale(self.stale_timeout))
            .cloned()
            .collect()
    }
}

impl Clone for PeerStore {
    fn clone(&self) -> Self {
        Self {
            peers: Arc::clone(&self.peers),
            stale_timeout: self.stale_timeout,
        }
    }
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
