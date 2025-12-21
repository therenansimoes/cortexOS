use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::StoreError;
use crate::graph::{ThoughtContent, ThoughtEdge, ThoughtNode};
use crate::types::{NodeId, Tag, Timestamp};

#[derive(Debug, Clone, Default)]
pub struct GraphQuery {
    pub kind: Option<String>,
    pub tags: Vec<Tag>,
    pub time_range: Option<(Timestamp, Timestamp)>,
    pub text_search: Option<String>,
}

impl GraphQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn with_time_range(mut self, from: Timestamp, to: Timestamp) -> Self {
        self.time_range = Some((from, to));
        self
    }

    pub fn with_text_search(mut self, text: impl Into<String>) -> Self {
        self.text_search = Some(text.into());
        self
    }
}

#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn add_node(&self, node: ThoughtNode) -> Result<NodeId, StoreError>;
    async fn add_edge(&self, edge: ThoughtEdge) -> Result<(), StoreError>;
    async fn get_node(&self, id: &NodeId) -> Result<Option<ThoughtNode>, StoreError>;
    async fn get_edges(&self, node_id: &NodeId) -> Result<Vec<ThoughtEdge>, StoreError>;
    async fn query(&self, query: GraphQuery) -> Result<Vec<ThoughtNode>, StoreError>;
}

#[derive(Default)]
pub struct MemoryGraphStore {
    nodes: RwLock<HashMap<NodeId, ThoughtNode>>,
    edges: RwLock<Vec<ThoughtEdge>>,
}

impl MemoryGraphStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn matches_query(node: &ThoughtNode, query: &GraphQuery) -> bool {
        if let Some(ref kind) = query.kind {
            let node_kind = match &node.content {
                ThoughtContent::Perception { .. } => "perception",
                ThoughtContent::Intention { .. } => "intention",
                ThoughtContent::Action { .. } => "action",
                ThoughtContent::Memory { .. } => "memory",
                ThoughtContent::Concept { .. } => "concept",
            };
            if node_kind != kind {
                return false;
            }
        }

        if !query.tags.is_empty() {
            let has_all_tags = query.tags.iter().all(|qt| {
                node.tags
                    .iter()
                    .any(|nt| nt.key == qt.key && nt.value == qt.value)
            });
            if !has_all_tags {
                return false;
            }
        }

        if let Some((from, to)) = query.time_range {
            if node.created_at < from || node.created_at > to {
                return false;
            }
        }

        if let Some(ref text) = query.text_search {
            let text_lower = text.to_lowercase();
            let content_matches = match &node.content {
                ThoughtContent::Perception { summary, .. } => {
                    summary.to_lowercase().contains(&text_lower)
                }
                ThoughtContent::Intention { goal, .. } => goal.to_lowercase().contains(&text_lower),
                ThoughtContent::Action { description, .. } => {
                    description.to_lowercase().contains(&text_lower)
                }
                ThoughtContent::Memory { text: t } => t.to_lowercase().contains(&text_lower),
                ThoughtContent::Concept {
                    name, definition, ..
                } => {
                    name.to_lowercase().contains(&text_lower)
                        || definition.to_lowercase().contains(&text_lower)
                }
            };
            if !content_matches {
                return false;
            }
        }

        true
    }
}

#[async_trait]
impl GraphStore for MemoryGraphStore {
    async fn add_node(&self, node: ThoughtNode) -> Result<NodeId, StoreError> {
        let id = node.id;
        self.nodes.write().insert(id, node);
        Ok(id)
    }

    async fn add_edge(&self, edge: ThoughtEdge) -> Result<(), StoreError> {
        self.edges.write().push(edge);
        Ok(())
    }

    async fn get_node(&self, id: &NodeId) -> Result<Option<ThoughtNode>, StoreError> {
        Ok(self.nodes.read().get(id).cloned())
    }

    async fn get_edges(&self, node_id: &NodeId) -> Result<Vec<ThoughtEdge>, StoreError> {
        let edges = self.edges.read();
        let result: Vec<ThoughtEdge> = edges
            .iter()
            .filter(|e| &e.from == node_id || &e.to == node_id)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn query(&self, query: GraphQuery) -> Result<Vec<ThoughtNode>, StoreError> {
        let nodes = self.nodes.read();
        let result: Vec<ThoughtNode> = nodes
            .values()
            .filter(|n| Self::matches_query(n, &query))
            .cloned()
            .collect();
        Ok(result)
    }
}

#[cfg(feature = "rocksdb")]
pub mod rocks {
    use super::*;
    use rocksdb::{ColumnFamilyDescriptor, Options, DB};
    use std::path::Path;

    const CF_NODES: &str = "nodes";
    const CF_EDGES: &str = "edges";
    const CF_BY_KIND: &str = "nodes_by_kind";
    const CF_BY_TIME: &str = "nodes_by_time";

    pub struct RocksGraphStore {
        db: Arc<DB>,
    }

    impl RocksGraphStore {
        pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cfs = vec![
                ColumnFamilyDescriptor::new(CF_NODES, Options::default()),
                ColumnFamilyDescriptor::new(CF_EDGES, Options::default()),
                ColumnFamilyDescriptor::new(CF_BY_KIND, Options::default()),
                ColumnFamilyDescriptor::new(CF_BY_TIME, Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cfs)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            Ok(Self { db: Arc::new(db) })
        }

        fn get_content_kind(content: &ThoughtContent) -> &'static str {
            match content {
                ThoughtContent::Perception { .. } => "perception",
                ThoughtContent::Intention { .. } => "intention",
                ThoughtContent::Action { .. } => "action",
                ThoughtContent::Memory { .. } => "memory",
                ThoughtContent::Concept { .. } => "concept",
            }
        }
    }

    #[async_trait]
    impl GraphStore for RocksGraphStore {
        async fn add_node(&self, node: ThoughtNode) -> Result<NodeId, StoreError> {
            let id = node.id;
            let id_bytes = id.0.as_bytes();
            let node_bytes =
                bincode::serialize(&node).map_err(|e| StoreError::Serialization(e.to_string()))?;

            let cf_nodes = self
                .db
                .cf_handle(CF_NODES)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;
            let cf_kind = self
                .db
                .cf_handle(CF_BY_KIND)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;
            let cf_time = self
                .db
                .cf_handle(CF_BY_TIME)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            self.db
                .put_cf(&cf_nodes, id_bytes, &node_bytes)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            let kind = Self::get_content_kind(&node.content);
            let kind_key = format!("{}:{}", kind, id.0);
            self.db
                .put_cf(&cf_kind, kind_key.as_bytes(), id_bytes)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            let time_key = format!("{:016x}:{}", node.created_at.0, id.0);
            self.db
                .put_cf(&cf_time, time_key.as_bytes(), id_bytes)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            Ok(id)
        }

        async fn add_edge(&self, edge: ThoughtEdge) -> Result<(), StoreError> {
            let cf = self
                .db
                .cf_handle(CF_EDGES)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            let key = format!("{}:{}", edge.from.0, edge.to.0);
            let edge_bytes =
                bincode::serialize(&edge).map_err(|e| StoreError::Serialization(e.to_string()))?;

            self.db
                .put_cf(&cf, key.as_bytes(), &edge_bytes)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            Ok(())
        }

        async fn get_node(&self, id: &NodeId) -> Result<Option<ThoughtNode>, StoreError> {
            let cf = self
                .db
                .cf_handle(CF_NODES)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            match self.db.get_cf(&cf, id.0.as_bytes()) {
                Ok(Some(bytes)) => {
                    let node = bincode::deserialize(&bytes)
                        .map_err(|e| StoreError::Deserialization(e.to_string()))?;
                    Ok(Some(node))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(StoreError::Backend(e.to_string())),
            }
        }

        async fn get_edges(&self, node_id: &NodeId) -> Result<Vec<ThoughtEdge>, StoreError> {
            let cf = self
                .db
                .cf_handle(CF_EDGES)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            let prefix = format!("{}:", node_id.0);
            let mut results = Vec::new();

            let iter = self.db.prefix_iterator_cf(&cf, prefix.as_bytes());
            for item in iter {
                let (key, value) = item.map_err(|e| StoreError::Backend(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);
                if !key_str.starts_with(&prefix) {
                    break;
                }

                let edge: ThoughtEdge = bincode::deserialize(&value)
                    .map_err(|e| StoreError::Deserialization(e.to_string()))?;
                results.push(edge);
            }

            let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);
            for item in iter {
                let (key, value) = item.map_err(|e| StoreError::Backend(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);
                if key_str.ends_with(&format!(":{}", node_id.0)) {
                    let edge: ThoughtEdge = bincode::deserialize(&value)
                        .map_err(|e| StoreError::Deserialization(e.to_string()))?;
                    results.push(edge);
                }
            }

            Ok(results)
        }

        async fn query(&self, query: GraphQuery) -> Result<Vec<ThoughtNode>, StoreError> {
            let cf_nodes = self
                .db
                .cf_handle(CF_NODES)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            let mut results = Vec::new();
            let iter = self.db.iterator_cf(&cf_nodes, rocksdb::IteratorMode::Start);

            for item in iter {
                let (_, value) = item.map_err(|e| StoreError::Backend(e.to_string()))?;
                let node: ThoughtNode = bincode::deserialize(&value)
                    .map_err(|e| StoreError::Deserialization(e.to_string()))?;

                if MemoryGraphStore::matches_query(&node, &query) {
                    results.push(node);
                }
            }

            Ok(results)
        }
    }
}

#[cfg(feature = "rocksdb")]
pub use rocks::RocksGraphStore;
