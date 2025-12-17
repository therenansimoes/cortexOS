use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::id::NodeId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventKind(String);

impl EventKind {
    pub fn new(kind: impl Into<String>) -> Self {
        Self(kind.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub kind: EventKind,
    pub origin: NodeId,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

impl Event {
    pub fn new(kind: EventKind, origin: NodeId, payload: Vec<u8>) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            origin,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            payload,
        }
    }
}
