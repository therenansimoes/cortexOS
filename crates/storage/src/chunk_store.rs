use std::collections::HashMap;
use std::sync::Arc;
use blake3::Hasher;
use parking_lot::RwLock;

use crate::error::StoreError;
use crate::types::Event;

/// Hash type compatible with grid's ChunkHash
pub type ChunkHash = [u8; 32];

/// Store for managing event chunks for synchronization
pub struct EventChunkStore {
    /// Chunks indexed by hash
    chunks: Arc<RwLock<HashMap<ChunkHash, Vec<Event>>>>,
    /// Chunk size (number of events per chunk)
    chunk_size: usize,
}

impl EventChunkStore {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunks: Arc::new(RwLock::new(HashMap::new())),
            chunk_size,
        }
    }

    /// Create chunks from a list of events
    pub fn create_chunks(&self, events: &[Event]) -> Result<Vec<(ChunkHash, Vec<Event>)>, StoreError> {
        let mut result = Vec::new();

        for chunk_events in events.chunks(self.chunk_size) {
            let events_vec = chunk_events.to_vec();
            let hash = self.compute_chunk_hash(&events_vec)?;
            
            // Store in memory
            self.chunks.write().insert(hash, events_vec.clone());
            
            result.push((hash, events_vec));
        }

        Ok(result)
    }

    /// Compute hash for a chunk of events
    fn compute_chunk_hash(&self, events: &[Event]) -> Result<ChunkHash, StoreError> {
        // Use canonical JSON serialization for deterministic hashing
        // serde_json preserves insertion order, but we need canonical form
        let bytes = serde_json::to_vec(events)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
        
        // Parse and re-serialize to ensure canonical form
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
        
        // Serialize with sorted keys for determinism
        let canonical_bytes = serde_json::to_vec(&value)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
        
        let mut hasher = Hasher::new();
        hasher.update(&canonical_bytes);
        Ok(hasher.finalize().into())
    }

    /// Get chunk by hash
    pub fn get_chunk(&self, hash: &ChunkHash) -> Option<Vec<Event>> {
        self.chunks.read().get(hash).cloned()
    }

    /// Store a chunk
    pub fn store_chunk(&self, hash: ChunkHash, events: Vec<Event>) -> Result<(), StoreError> {
        // Verify hash
        let computed_hash = self.compute_chunk_hash(&events)?;
        if computed_hash != hash {
            return Err(StoreError::Integrity("Chunk hash mismatch".into()));
        }

        self.chunks.write().insert(hash, events);
        Ok(())
    }

    /// Get all chunk hashes
    pub fn get_all_hashes(&self) -> Vec<ChunkHash> {
        self.chunks.read().keys().copied().collect()
    }

    /// Serialize chunk to bytes for network transfer
    pub fn serialize_chunk(&self, hash: &ChunkHash) -> Result<Vec<u8>, StoreError> {
        let chunk = self.chunks.read().get(hash).cloned()
            .ok_or_else(|| StoreError::NotFound(format!("Chunk {:02x?}... not found", &hash[..4])))?;
        
        // Use JSON serialization since Event contains serde_json::Value
        serde_json::to_vec(&chunk)
            .map_err(|e| StoreError::Serialization(e.to_string()))
    }

    /// Deserialize chunk from bytes
    pub fn deserialize_chunk(&self, data: &[u8]) -> Result<Vec<Event>, StoreError> {
        // Use JSON deserialization since Event contains serde_json::Value
        serde_json::from_slice(data)
            .map_err(|e| StoreError::Deserialization(e.to_string()))
    }

    /// Import chunk from bytes and verify
    pub fn import_chunk(&self, hash: ChunkHash, data: &[u8]) -> Result<(), StoreError> {
        let events = self.deserialize_chunk(data)?;
        self.store_chunk(hash, events)
    }

    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunks.read().len()
    }

    /// Clear all chunks (for testing)
    pub fn clear(&self) {
        self.chunks.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Event, Timestamp};

    fn create_test_event(id: usize) -> Event {
        Event {
            id: crate::types::EventId::new(),
            kind: format!("test.event.{}", id),
            source: "test".to_string(),
            timestamp: Timestamp::now(),
            payload: serde_json::json!({"id": id}),
            privacy: crate::types::PrivacyLevel::Public,
        }
    }

    #[test]
    fn test_chunk_store_creation() {
        let store = EventChunkStore::new(10);
        assert_eq!(store.chunk_count(), 0);
    }

    #[test]
    fn test_create_chunks() {
        let store = EventChunkStore::new(3);
        let events: Vec<Event> = (0..7).map(create_test_event).collect();

        let chunks = store.create_chunks(&events).unwrap();
        
        // Should create 3 chunks: [0,1,2], [3,4,5], [6]
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].1.len(), 3);
        assert_eq!(chunks[1].1.len(), 3);
        assert_eq!(chunks[2].1.len(), 1);
    }

    #[test]
    fn test_chunk_storage_and_retrieval() {
        let store = EventChunkStore::new(5);
        let events: Vec<Event> = (0..5).map(create_test_event).collect();

        let chunks = store.create_chunks(&events).unwrap();
        let (hash, _) = &chunks[0];

        // Retrieve chunk
        let retrieved = store.get_chunk(hash);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 5);
    }

    #[test]
    fn test_chunk_hash_verification() {
        let store = EventChunkStore::new(5);
        let events: Vec<Event> = (0..5).map(create_test_event).collect();
        
        let wrong_hash = [0u8; 32];
        let result = store.store_chunk(wrong_hash, events);
        
        // Should fail due to hash mismatch
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_serialization() {
        let store = EventChunkStore::new(3);
        let events: Vec<Event> = (0..3).map(create_test_event).collect();

        let chunks = store.create_chunks(&events).unwrap();
        let (hash, _) = &chunks[0];

        // Serialize
        let bytes = store.serialize_chunk(hash).unwrap();
        assert!(!bytes.is_empty());

        // Deserialize
        let deserialized = store.deserialize_chunk(&bytes).unwrap();
        assert_eq!(deserialized.len(), 3);
    }

    #[test]
    fn test_import_chunk() {
        let store = EventChunkStore::new(3);
        let events: Vec<Event> = (0..3).map(create_test_event).collect();

        let chunks = store.create_chunks(&events).unwrap();
        let (hash, _) = &chunks[0];
        let bytes = store.serialize_chunk(hash).unwrap();

        // Clear and reimport
        store.clear();
        assert_eq!(store.chunk_count(), 0);

        store.import_chunk(*hash, &bytes).unwrap();
        assert_eq!(store.chunk_count(), 1);

        let retrieved = store.get_chunk(hash);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_get_all_hashes() {
        let store = EventChunkStore::new(2);
        let events: Vec<Event> = (0..5).map(create_test_event).collect();

        store.create_chunks(&events).unwrap();
        
        let hashes = store.get_all_hashes();
        assert_eq!(hashes.len(), 3); // 3 chunks: [0,1], [2,3], [4]
    }
}
