use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "agent:{}", self.0)
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentionId(pub Uuid);

impl IntentionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for IntentionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for IntentionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "intention:{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability(pub String);

impl Capability {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    capabilities: HashSet<String>,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.insert(cap.into());
        self
    }

    pub fn add(&mut self, cap: impl Into<String>) {
        self.capabilities.insert(cap.into());
    }

    pub fn has(&self, cap: &str) -> bool {
        self.capabilities.contains(cap)
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.capabilities.iter()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub kind: String,
    pub payload: Vec<u8>,
    pub source: Option<AgentId>,
    pub timestamp: Timestamp,
}

impl Event {
    pub fn new(kind: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            id: EventId::new(),
            kind: kind.into(),
            payload,
            source: None,
            timestamp: Timestamp::now(),
        }
    }

    pub fn with_source(mut self, source: AgentId) -> Self {
        self.source = Some(source);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPattern {
    pub kind_prefix: Option<String>,
    pub source: Option<AgentId>,
}

impl EventPattern {
    pub fn all() -> Self {
        Self {
            kind_prefix: None,
            source: None,
        }
    }

    pub fn kind(prefix: impl Into<String>) -> Self {
        Self {
            kind_prefix: Some(prefix.into()),
            source: None,
        }
    }

    pub fn from_source(source: AgentId) -> Self {
        Self {
            kind_prefix: None,
            source: Some(source),
        }
    }

    pub fn matches(&self, event: &Event) -> bool {
        let kind_matches = self
            .kind_prefix
            .as_ref()
            .map(|p| event.kind.starts_with(p))
            .unwrap_or(true);

        let source_matches = self
            .source
            .as_ref()
            .map(|s| event.source.as_ref() == Some(s))
            .unwrap_or(true);

        kind_matches && source_matches
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    pub node_type: Option<String>,
    pub limit: Option<usize>,
}

impl GraphQuery {
    pub fn all() -> Self {
        Self {
            node_type: None,
            limit: None,
        }
    }

    pub fn of_type(node_type: impl Into<String>) -> Self {
        Self {
            node_type: Some(node_type.into()),
            limit: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNode {
    pub id: NodeId,
    pub content: ThoughtContent,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtContent {
    pub node_type: String,
    pub data: Vec<u8>,
}

impl ThoughtContent {
    pub fn new(node_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            node_type: node_type.into(),
            data,
        }
    }
}
