pub mod error;
pub mod types;
pub mod event_store;
pub mod graph;
pub mod graph_store;
pub mod privacy;
pub mod sync;

pub use error::StoreError;
pub use types::{Event, EventId, NodeId, PrivacyLevel, Tag, Timestamp};
pub use event_store::{EventStore, MemoryEventStore};
pub use graph::{IntentionStatus, Outcome, Relation, ThoughtContent, ThoughtEdge, ThoughtNode};
pub use graph_store::{GraphQuery, GraphStore, MemoryGraphStore};
pub use privacy::{PrivacyAware, PrivacyFilter};
pub use sync::{ContentHash, DiffRequest, DiffResponse, ExportChunk, SyncManager, SyncManifest};

#[cfg(feature = "rocksdb")]
pub use event_store::RocksEventStore;

#[cfg(feature = "rocksdb")]
pub use graph_store::RocksGraphStore;
