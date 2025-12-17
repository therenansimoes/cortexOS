use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::error::StoreError;
use crate::graph::ThoughtNode;
use crate::privacy::{PrivacyFilter, PrivacyAware};
use crate::types::{Event, NodeId, PrivacyLevel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    pub fn compute<T: Serialize>(item: &T) -> Result<Self, StoreError> {
        let bytes =
            bincode::serialize(item).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let mut hasher = Hasher::new();
        hasher.update(&bytes);
        Ok(Self(hasher.finalize().into()))
    }

    pub fn as_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub version: u32,
    pub node_count: usize,
    pub event_count: usize,
    pub root_hash: ContentHash,
    pub chunks: Vec<ChunkInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub id: u32,
    pub hash: ContentHash,
    pub size: usize,
    pub node_ids: Vec<NodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffRequest {
    pub known_hashes: Vec<ContentHash>,
    pub privacy_filter: PrivacyLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResponse {
    pub missing_chunks: Vec<u32>,
    pub new_hashes: Vec<ContentHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportChunk {
    pub id: u32,
    pub hash: ContentHash,
    pub nodes: Vec<ThoughtNode>,
    pub events: Vec<Event>,
}

impl ExportChunk {
    pub fn new(id: u32, nodes: Vec<ThoughtNode>, events: Vec<Event>) -> Result<Self, StoreError> {
        let combined: (Vec<&ThoughtNode>, Vec<&Event>) =
            (nodes.iter().collect(), events.iter().collect());
        let hash = ContentHash::compute(&combined)?;
        Ok(Self {
            id,
            hash,
            nodes,
            events,
        })
    }

    pub fn verify(&self) -> Result<bool, StoreError> {
        let combined: (Vec<&ThoughtNode>, Vec<&Event>) =
            (self.nodes.iter().collect(), self.events.iter().collect());
        let computed = ContentHash::compute(&combined)?;
        Ok(computed.0 == self.hash.0)
    }
}

pub struct SyncManager {
    privacy_filter: PrivacyFilter,
    chunk_size: usize,
}

impl Default for SyncManager {
    fn default() -> Self {
        Self {
            privacy_filter: PrivacyFilter::shareable(),
            chunk_size: 100,
        }
    }
}

impl SyncManager {
    pub fn new(privacy_filter: PrivacyFilter, chunk_size: usize) -> Self {
        Self {
            privacy_filter,
            chunk_size,
        }
    }

    pub fn create_manifest(
        &self,
        nodes: &[ThoughtNode],
        events: &[Event],
    ) -> Result<SyncManifest, StoreError> {
        let filtered_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| self.privacy_filter.allows(&n.privacy_level()))
            .collect();
        let filtered_events: Vec<_> = events
            .iter()
            .filter(|e| self.privacy_filter.allows(&e.privacy_level()))
            .collect();

        let mut chunks = Vec::new();
        let mut chunk_id = 0u32;

        for chunk_nodes in filtered_nodes.chunks(self.chunk_size) {
            let node_ids: Vec<NodeId> = chunk_nodes.iter().map(|n| n.id).collect();
            let hash = ContentHash::compute(&chunk_nodes)?;
            chunks.push(ChunkInfo {
                id: chunk_id,
                hash,
                size: chunk_nodes.len(),
                node_ids,
            });
            chunk_id += 1;
        }

        let root_hash = ContentHash::compute(&(&filtered_nodes, &filtered_events))?;

        Ok(SyncManifest {
            version: 1,
            node_count: filtered_nodes.len(),
            event_count: filtered_events.len(),
            root_hash,
            chunks,
        })
    }

    pub fn compute_diff(&self, request: &DiffRequest, manifest: &SyncManifest) -> DiffResponse {
        let missing_chunks: Vec<u32> = manifest
            .chunks
            .iter()
            .filter(|c| !request.known_hashes.iter().any(|kh| kh.0 == c.hash.0))
            .map(|c| c.id)
            .collect();

        let new_hashes: Vec<ContentHash> = manifest
            .chunks
            .iter()
            .filter(|c| !request.known_hashes.iter().any(|kh| kh.0 == c.hash.0))
            .map(|c| c.hash.clone())
            .collect();

        DiffResponse {
            missing_chunks,
            new_hashes,
        }
    }

    pub fn export_chunk(
        &self,
        chunk_id: u32,
        nodes: &[ThoughtNode],
        events: &[Event],
    ) -> Result<ExportChunk, StoreError> {
        let filtered_nodes: Vec<ThoughtNode> = nodes
            .iter()
            .filter(|n| self.privacy_filter.allows(&n.privacy_level()))
            .cloned()
            .collect();

        let start = chunk_id as usize * self.chunk_size;
        let end = (start + self.chunk_size).min(filtered_nodes.len());

        if start >= filtered_nodes.len() {
            return Err(StoreError::NotFound(format!("Chunk {} not found", chunk_id)));
        }

        let chunk_nodes = filtered_nodes[start..end].to_vec();
        let chunk_events: Vec<Event> = events
            .iter()
            .filter(|e| self.privacy_filter.allows(&e.privacy_level()))
            .cloned()
            .collect();

        ExportChunk::new(chunk_id, chunk_nodes, chunk_events)
    }

    pub fn import_chunk(&self, chunk: &ExportChunk) -> Result<(Vec<ThoughtNode>, Vec<Event>), StoreError> {
        if !chunk.verify()? {
            return Err(StoreError::Integrity("Chunk hash mismatch".into()));
        }

        let nodes = chunk.nodes.clone();
        let events = chunk.events.clone();

        Ok((nodes, events))
    }
}
