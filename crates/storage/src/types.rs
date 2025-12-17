use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self(duration.as_millis() as u64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub kind: String,
    pub source: String,
    pub timestamp: Timestamp,
    pub payload: serde_json::Value,
    pub privacy: PrivacyLevel,
}

impl Event {
    pub fn new(kind: impl Into<String>, source: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: EventId::new(),
            kind: kind.into(),
            source: source.into(),
            timestamp: Timestamp::now(),
            payload,
            privacy: PrivacyLevel::Private,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

impl Tag {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn emotion(value: impl Into<String>) -> Self {
        Self::new("emotion", value)
    }

    pub fn priority(value: impl Into<String>) -> Self {
        Self::new("priority", value)
    }

    pub fn novelty(value: impl Into<String>) -> Self {
        Self::new("novelty", value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PrivacyLevel {
    #[default]
    Private,
    Shareable,
    Public,
}

impl PrivacyLevel {
    pub fn can_share(&self) -> bool {
        matches!(self, PrivacyLevel::Shareable | PrivacyLevel::Public)
    }

    pub fn is_public(&self) -> bool {
        matches!(self, PrivacyLevel::Public)
    }
}
