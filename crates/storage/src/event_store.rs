use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::StoreError;
use crate::types::{Event, EventId, Timestamp};

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, event: &Event) -> Result<EventId, StoreError>;
    async fn get(&self, id: &EventId) -> Result<Option<Event>, StoreError>;
    async fn range(&self, from: Timestamp, to: Timestamp) -> Result<Vec<Event>, StoreError>;
    async fn by_kind(&self, kind: &str) -> Result<Vec<Event>, StoreError>;
    async fn by_source(&self, source: &str) -> Result<Vec<Event>, StoreError>;
}

#[derive(Default)]
pub struct MemoryEventStore {
    events: RwLock<HashMap<EventId, Event>>,
    timeline: RwLock<Vec<EventId>>,
}

impl MemoryEventStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl EventStore for MemoryEventStore {
    async fn append(&self, event: &Event) -> Result<EventId, StoreError> {
        let id = event.id;
        self.events.write().insert(id, event.clone());
        self.timeline.write().push(id);
        Ok(id)
    }

    async fn get(&self, id: &EventId) -> Result<Option<Event>, StoreError> {
        Ok(self.events.read().get(id).cloned())
    }

    async fn range(&self, from: Timestamp, to: Timestamp) -> Result<Vec<Event>, StoreError> {
        let events = self.events.read();
        let result: Vec<Event> = events
            .values()
            .filter(|e| e.timestamp >= from && e.timestamp <= to)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn by_kind(&self, kind: &str) -> Result<Vec<Event>, StoreError> {
        let events = self.events.read();
        let result: Vec<Event> = events
            .values()
            .filter(|e| e.kind == kind)
            .cloned()
            .collect();
        Ok(result)
    }

    async fn by_source(&self, source: &str) -> Result<Vec<Event>, StoreError> {
        let events = self.events.read();
        let result: Vec<Event> = events
            .values()
            .filter(|e| e.source == source)
            .cloned()
            .collect();
        Ok(result)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod rocks {
    use super::*;
    use rocksdb::{ColumnFamilyDescriptor, Options, DB};
    use std::path::Path;

    const CF_EVENTS: &str = "events";
    const CF_BY_TIME: &str = "by_time";
    const CF_BY_KIND: &str = "by_kind";
    const CF_BY_SOURCE: &str = "by_source";

    pub struct RocksEventStore {
        db: Arc<DB>,
    }

    impl RocksEventStore {
        pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StoreError> {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cfs = vec![
                ColumnFamilyDescriptor::new(CF_EVENTS, Options::default()),
                ColumnFamilyDescriptor::new(CF_BY_TIME, Options::default()),
                ColumnFamilyDescriptor::new(CF_BY_KIND, Options::default()),
                ColumnFamilyDescriptor::new(CF_BY_SOURCE, Options::default()),
            ];

            let db = DB::open_cf_descriptors(&opts, path, cfs)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            Ok(Self { db: Arc::new(db) })
        }

        fn serialize_event(event: &Event) -> Result<Vec<u8>, StoreError> {
            bincode::serialize(event).map_err(|e| StoreError::Serialization(e.to_string()))
        }

        fn deserialize_event(bytes: &[u8]) -> Result<Event, StoreError> {
            bincode::deserialize(bytes).map_err(|e| StoreError::Deserialization(e.to_string()))
        }
    }

    #[async_trait]
    impl EventStore for RocksEventStore {
        async fn append(&self, event: &Event) -> Result<EventId, StoreError> {
            let id = event.id;
            let id_bytes = id.0.as_bytes();
            let event_bytes = Self::serialize_event(event)?;

            let cf_events = self
                .db
                .cf_handle(CF_EVENTS)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;
            let cf_time = self
                .db
                .cf_handle(CF_BY_TIME)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;
            let cf_kind = self
                .db
                .cf_handle(CF_BY_KIND)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;
            let cf_source = self
                .db
                .cf_handle(CF_BY_SOURCE)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            self.db
                .put_cf(&cf_events, id_bytes, &event_bytes)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            let time_key = format!("{:016x}:{}", event.timestamp.0, id.0);
            self.db
                .put_cf(&cf_time, time_key.as_bytes(), id_bytes)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            let kind_key = format!("{}:{}", event.kind, id.0);
            self.db
                .put_cf(&cf_kind, kind_key.as_bytes(), id_bytes)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            let source_key = format!("{}:{}", event.source, id.0);
            self.db
                .put_cf(&cf_source, source_key.as_bytes(), id_bytes)
                .map_err(|e| StoreError::Backend(e.to_string()))?;

            Ok(id)
        }

        async fn get(&self, id: &EventId) -> Result<Option<Event>, StoreError> {
            let cf = self
                .db
                .cf_handle(CF_EVENTS)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            match self.db.get_cf(&cf, id.0.as_bytes()) {
                Ok(Some(bytes)) => Ok(Some(Self::deserialize_event(&bytes)?)),
                Ok(None) => Ok(None),
                Err(e) => Err(StoreError::Backend(e.to_string())),
            }
        }

        async fn range(&self, from: Timestamp, to: Timestamp) -> Result<Vec<Event>, StoreError> {
            let cf_time = self
                .db
                .cf_handle(CF_BY_TIME)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;
            let cf_events = self
                .db
                .cf_handle(CF_EVENTS)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            let start_key = format!("{:016x}:", from.0);
            let end_key = format!("{:016x}:{}", to.0, "\u{00ff}");

            let mut results = Vec::new();
            let iter = self.db.iterator_cf(
                &cf_time,
                rocksdb::IteratorMode::From(start_key.as_bytes(), rocksdb::Direction::Forward),
            );

            for item in iter {
                let (key, value) = item.map_err(|e| StoreError::Backend(e.to_string()))?;
                if key.as_ref() > end_key.as_bytes() {
                    break;
                }

                if let Some(bytes) = self
                    .db
                    .get_cf(&cf_events, &value)
                    .map_err(|e| StoreError::Backend(e.to_string()))?
                {
                    results.push(Self::deserialize_event(&bytes)?);
                }
            }

            Ok(results)
        }

        async fn by_kind(&self, kind: &str) -> Result<Vec<Event>, StoreError> {
            let cf_kind = self
                .db
                .cf_handle(CF_BY_KIND)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;
            let cf_events = self
                .db
                .cf_handle(CF_EVENTS)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            let prefix = format!("{}:", kind);
            let mut results = Vec::new();

            let iter = self.db.prefix_iterator_cf(&cf_kind, prefix.as_bytes());

            for item in iter {
                let (key, value) = item.map_err(|e| StoreError::Backend(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);
                if !key_str.starts_with(&prefix) {
                    break;
                }

                if let Some(bytes) = self
                    .db
                    .get_cf(&cf_events, &value)
                    .map_err(|e| StoreError::Backend(e.to_string()))?
                {
                    results.push(Self::deserialize_event(&bytes)?);
                }
            }

            Ok(results)
        }

        async fn by_source(&self, source: &str) -> Result<Vec<Event>, StoreError> {
            let cf_source = self
                .db
                .cf_handle(CF_BY_SOURCE)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;
            let cf_events = self
                .db
                .cf_handle(CF_EVENTS)
                .ok_or_else(|| StoreError::Backend("CF not found".into()))?;

            let prefix = format!("{}:", source);
            let mut results = Vec::new();

            let iter = self.db.prefix_iterator_cf(&cf_source, prefix.as_bytes());

            for item in iter {
                let (key, value) = item.map_err(|e| StoreError::Backend(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);
                if !key_str.starts_with(&prefix) {
                    break;
                }

                if let Some(bytes) = self
                    .db
                    .get_cf(&cf_events, &value)
                    .map_err(|e| StoreError::Backend(e.to_string()))?
                {
                    results.push(Self::deserialize_event(&bytes)?);
                }
            }

            Ok(results)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use rocks::RocksEventStore;
