use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{GridError, Result};
use crate::peer::NodeId;

/// Hash of an event chunk (32 bytes)
pub type ChunkHash = [u8; 32];

/// Request for syncing event chunks between peers
#[derive(Debug, Clone)]
pub struct SyncRequest {
    pub peer_id: NodeId,
    pub known_hashes: Vec<ChunkHash>,
}

/// Response with missing chunk hashes
#[derive(Debug, Clone)]
pub struct SyncResponse {
    pub missing_hashes: Vec<ChunkHash>,
}

/// Progress tracking for chunk sync operations
#[derive(Debug, Clone)]
pub struct SyncProgress {
    pub total_chunks: usize,
    pub synced_chunks: usize,
    pub failed_chunks: usize,
    pub bytes_transferred: u64,
    pub started_at: Instant,
}

impl SyncProgress {
    pub fn new(total_chunks: usize) -> Self {
        Self {
            total_chunks,
            synced_chunks: 0,
            failed_chunks: 0,
            bytes_transferred: 0,
            started_at: Instant::now(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.synced_chunks + self.failed_chunks >= self.total_chunks
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total_chunks == 0 {
            return 100.0;
        }
        (self.synced_chunks as f64 / self.total_chunks as f64) * 100.0
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

/// Bandwidth throttle configuration
#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    /// Maximum bytes per second (0 = unlimited)
    pub max_bytes_per_sec: u64,
    /// Window duration for rate limiting
    pub window_duration: Duration,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            max_bytes_per_sec: 1_000_000, // 1 MB/s default
            window_duration: Duration::from_secs(1),
        }
    }
}

/// Tracks bandwidth usage for throttling
struct BandwidthTracker {
    config: ThrottleConfig,
    bytes_sent: u64,
    window_start: Instant,
}

impl BandwidthTracker {
    fn new(config: ThrottleConfig) -> Self {
        Self {
            config,
            bytes_sent: 0,
            window_start: Instant::now(),
        }
    }

    fn reset_window(&mut self) {
        self.bytes_sent = 0;
        self.window_start = Instant::now();
    }

    async fn throttle(&mut self, bytes: u64) -> Result<()> {
        if self.config.max_bytes_per_sec == 0 {
            // Unlimited
            return Ok(());
        }

        // Check if we need to reset the window
        if self.window_start.elapsed() >= self.config.window_duration {
            self.reset_window();
        }

        self.bytes_sent += bytes;

        if self.bytes_sent > self.config.max_bytes_per_sec {
            let wait_time = self.config.window_duration - self.window_start.elapsed();
            debug!(
                "Throttling: waiting {:?} (sent {} bytes)",
                wait_time, self.bytes_sent
            );
            tokio::time::sleep(wait_time).await;
            self.reset_window();
        }

        Ok(())
    }
}

/// Manages event chunk synchronization between peers
pub struct EventChunkSyncManager {
    local_node_id: NodeId,
    throttle_config: ThrottleConfig,
    bandwidth_tracker: Arc<RwLock<BandwidthTracker>>,
    active_syncs: Arc<RwLock<HashMap<NodeId, SyncProgress>>>,
    chunk_cache: Arc<RwLock<HashMap<ChunkHash, Vec<u8>>>>,
}

impl EventChunkSyncManager {
    pub fn new(local_node_id: NodeId, throttle_config: ThrottleConfig) -> Self {
        let bandwidth_tracker = BandwidthTracker::new(throttle_config.clone());
        Self {
            local_node_id,
            throttle_config,
            bandwidth_tracker: Arc::new(RwLock::new(bandwidth_tracker)),
            active_syncs: Arc::new(RwLock::new(HashMap::new())),
            chunk_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Request chunks from a peer based on known hashes
    pub async fn request_sync(
        &self,
        peer_id: NodeId,
        known_hashes: Vec<ChunkHash>,
    ) -> Result<Vec<ChunkHash>> {
        info!(
            "Requesting sync from peer {:?} with {} known hashes",
            peer_id,
            known_hashes.len()
        );

        // This would send a message to the peer
        // For now, we return an empty list (will be implemented with actual networking)
        Ok(Vec::new())
    }

    /// Handle incoming chunk get request
    pub async fn handle_chunk_get(&self, hash: ChunkHash) -> Result<Option<Vec<u8>>> {
        debug!("Handling chunk get request for hash: {:02x?}...", &hash[..4]);

        // Check cache first
        let chunk = self.chunk_cache.read().get(&hash).cloned();
        
        if chunk.is_some() {
            debug!("Chunk found in cache");
        } else {
            debug!("Chunk not found in cache");
        }

        Ok(chunk)
    }

    /// Handle incoming chunk put (receive chunk from peer)
    pub async fn handle_chunk_put(&self, hash: ChunkHash, data: Vec<u8>) -> Result<()> {
        debug!(
            "Handling chunk put for hash: {:02x?}... ({} bytes)",
            &hash[..4],
            data.len()
        );

        // Verify hash matches data
        if !self.verify_chunk_hash(&hash, &data) {
            error!("Chunk hash mismatch!");
            return Err(GridError::ProtocolError("Chunk hash mismatch".into()));
        }

        // Apply bandwidth throttling
        {
            let mut tracker = self.bandwidth_tracker.write();
            tracker.throttle(data.len() as u64).await?;
        }

        // Store in cache
        self.chunk_cache.write().insert(hash, data);

        info!("Chunk stored successfully");
        Ok(())
    }

    /// Verify that the hash matches the chunk data
    fn verify_chunk_hash(&self, hash: &ChunkHash, data: &[u8]) -> bool {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        let computed_hash: [u8; 32] = hasher.finalize().into();
        &computed_hash == hash
    }

    /// Get chunk from local cache
    pub fn get_chunk(&self, hash: &ChunkHash) -> Option<Vec<u8>> {
        self.chunk_cache.read().get(hash).cloned()
    }

    /// Store chunk in local cache
    pub fn store_chunk(&self, hash: ChunkHash, data: Vec<u8>) -> Result<()> {
        if !self.verify_chunk_hash(&hash, &data) {
            return Err(GridError::ProtocolError("Invalid chunk hash".into()));
        }
        self.chunk_cache.write().insert(hash, data);
        Ok(())
    }

    /// Start a sync operation with a peer
    pub fn start_sync(&self, peer_id: NodeId, total_chunks: usize) {
        let progress = SyncProgress::new(total_chunks);
        self.active_syncs.write().insert(peer_id, progress);
        info!(
            "Started sync with peer {:?} ({} chunks)",
            peer_id, total_chunks
        );
    }

    /// Update sync progress
    pub fn update_sync_progress(
        &self,
        peer_id: &NodeId,
        synced: usize,
        failed: usize,
        bytes: u64,
    ) {
        if let Some(progress) = self.active_syncs.write().get_mut(peer_id) {
            progress.synced_chunks += synced;
            progress.failed_chunks += failed;
            progress.bytes_transferred += bytes;

            debug!(
                "Sync progress with {:?}: {:.1}% ({}/{} chunks)",
                peer_id,
                progress.progress_percent(),
                progress.synced_chunks,
                progress.total_chunks
            );
        }
    }

    /// Get sync progress for a peer
    pub fn get_sync_progress(&self, peer_id: &NodeId) -> Option<SyncProgress> {
        self.active_syncs.read().get(peer_id).cloned()
    }

    /// Complete sync with a peer
    pub fn complete_sync(&self, peer_id: &NodeId) -> Option<SyncProgress> {
        let progress = self.active_syncs.write().remove(peer_id);
        if let Some(ref p) = progress {
            info!(
                "Completed sync with {:?}: {} chunks in {:?}",
                peer_id,
                p.synced_chunks,
                p.elapsed()
            );
        }
        progress
    }

    /// Get all active syncs
    pub fn active_sync_count(&self) -> usize {
        self.active_syncs.read().len()
    }

    /// Clear cache (for testing or memory management)
    pub fn clear_cache(&self) {
        self.chunk_cache.write().clear();
    }

    /// Get cache size in bytes
    pub fn cache_size_bytes(&self) -> usize {
        self.chunk_cache
            .read()
            .values()
            .map(|v: &Vec<u8>| v.len())
            .sum()
    }
}

/// Delta sync protocol - sync only missing chunks
pub struct DeltaSyncProtocol {
    sync_manager: Arc<EventChunkSyncManager>,
}

impl DeltaSyncProtocol {
    pub fn new(sync_manager: Arc<EventChunkSyncManager>) -> Self {
        Self { sync_manager }
    }

    /// Compute delta between local and remote state
    pub async fn compute_delta(
        &self,
        local_hashes: &[ChunkHash],
        remote_hashes: &[ChunkHash],
    ) -> Vec<ChunkHash> {
        let local_set: HashSet<_> = local_hashes.iter().collect();
        let remote_set: HashSet<_> = remote_hashes.iter().collect();

        // Find chunks in remote but not in local
        remote_set
            .difference(&local_set)
            .map(|&&hash| hash)
            .collect()
    }

    /// Request missing chunks from peer
    pub async fn request_missing_chunks(
        &self,
        peer_id: NodeId,
        missing_hashes: Vec<ChunkHash>,
    ) -> Result<Vec<(ChunkHash, Vec<u8>)>> {
        info!(
            "Requesting {} missing chunks from peer {:?}",
            missing_hashes.len(),
            peer_id
        );

        self.sync_manager
            .start_sync(peer_id, missing_hashes.len());

        let mut chunks = Vec::new();
        let mut synced = 0;
        let mut failed = 0;
        let mut total_bytes = 0u64;

        for hash in missing_hashes {
            // This would actually send a request to the peer
            // For now, we check local cache
            match self.sync_manager.handle_chunk_get(hash).await {
                Ok(Some(data)) => {
                    let bytes = data.len() as u64;
                    total_bytes += bytes;
                    chunks.push((hash, data));
                    synced += 1;
                }
                Ok(None) => {
                    warn!("Chunk {:02x?}... not found", &hash[..4]);
                    failed += 1;
                }
                Err(e) => {
                    error!("Error fetching chunk: {}", e);
                    failed += 1;
                }
            }
        }

        self.sync_manager
            .update_sync_progress(&peer_id, synced, failed, total_bytes);
        self.sync_manager.complete_sync(&peer_id);

        info!("Delta sync complete: {} chunks synced", chunks.len());
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let node_id = NodeId::random();
        let config = ThrottleConfig::default();
        let manager = EventChunkSyncManager::new(node_id, config);
        assert_eq!(manager.active_sync_count(), 0);
        assert_eq!(manager.cache_size_bytes(), 0);
    }

    #[tokio::test]
    async fn test_chunk_storage_and_retrieval() {
        let node_id = NodeId::random();
        let config = ThrottleConfig::default();
        let manager = EventChunkSyncManager::new(node_id, config);

        let data = b"test chunk data".to_vec();
        let hash = {
            use blake3::Hasher;
            let mut hasher = Hasher::new();
            hasher.update(&data);
            hasher.finalize().into()
        };

        // Store chunk
        manager.store_chunk(hash, data.clone()).unwrap();

        // Retrieve chunk
        let retrieved = manager.get_chunk(&hash);
        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn test_chunk_verification() {
        let node_id = NodeId::random();
        let config = ThrottleConfig::default();
        let manager = EventChunkSyncManager::new(node_id, config);

        let data = b"test data".to_vec();
        let wrong_hash = [0u8; 32];

        // Should fail with wrong hash
        let result = manager.store_chunk(wrong_hash, data);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sync_progress() {
        let node_id = NodeId::random();
        let peer_id = NodeId::random();
        let config = ThrottleConfig::default();
        let manager = EventChunkSyncManager::new(node_id, config);

        // Start sync
        manager.start_sync(peer_id, 10);
        assert_eq!(manager.active_sync_count(), 1);

        // Update progress
        manager.update_sync_progress(&peer_id, 5, 0, 1024);
        let progress = manager.get_sync_progress(&peer_id).unwrap();
        assert_eq!(progress.synced_chunks, 5);
        assert_eq!(progress.bytes_transferred, 1024);
        assert!(!progress.is_complete());

        // Complete sync
        manager.update_sync_progress(&peer_id, 5, 0, 1024);
        let final_progress = manager.complete_sync(&peer_id).unwrap();
        assert_eq!(final_progress.synced_chunks, 10);
        assert!(final_progress.is_complete());
        assert_eq!(manager.active_sync_count(), 0);
    }

    #[tokio::test]
    async fn test_delta_sync() {
        let node_id = NodeId::random();
        let config = ThrottleConfig::default();
        let manager = Arc::new(EventChunkSyncManager::new(node_id, config));
        let delta = DeltaSyncProtocol::new(manager);

        let local_hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let remote_hashes = vec![[2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]];

        let missing = delta.compute_delta(&local_hashes, &remote_hashes).await;
        assert_eq!(missing.len(), 2); // [4] and [5] are missing
        assert!(missing.contains(&[4u8; 32]));
        assert!(missing.contains(&[5u8; 32]));
    }

    #[tokio::test]
    async fn test_bandwidth_throttling() {
        let config = ThrottleConfig {
            max_bytes_per_sec: 1000,
            window_duration: Duration::from_millis(100),
        };
        let mut tracker = BandwidthTracker::new(config);

        let start = Instant::now();
        
        // Send data within limit
        tracker.throttle(500).await.unwrap();
        let elapsed1 = start.elapsed();
        assert!(elapsed1 < Duration::from_millis(50)); // Should not throttle

        // Send more data, should trigger throttle
        tracker.throttle(600).await.unwrap();
        let elapsed2 = start.elapsed();
        assert!(elapsed2 >= Duration::from_millis(100)); // Should wait
    }
}
