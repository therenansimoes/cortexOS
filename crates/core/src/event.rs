use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_content(content: &[u8]) -> Self {
        let hash = blake3::hash(content);
        // This is safe because blake3 always returns 32 bytes, and we're taking the first 16
        let bytes: [u8; 16] = hash.as_bytes()[..16]
            .try_into()
            .expect("blake3 hash slice is always 32 bytes");
        Self(Uuid::from_bytes(bytes))
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Payload {
    Inline(Vec<u8>),
    Reference { hash: [u8; 32], size: u64 },
}

impl Payload {
    pub fn inline(data: Vec<u8>) -> Self {
        Self::Inline(data)
    }

    pub fn reference(hash: [u8; 32], size: u64) -> Self {
        Self::Reference { hash, size }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Payload::Inline(data) => Some(data),
            Payload::Reference { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            trace_id: None,
            span_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub timestamp: u64,
    pub source: String,
    pub kind: String,
    pub payload: Payload,
    pub trace: Trace,
}

impl Event {
    pub fn new(source: &str, kind: &str, payload: Payload) -> Self {
        Self {
            id: EventId::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            source: source.to_string(),
            kind: kind.to_string(),
            payload,
            trace: Trace::default(),
        }
    }

    pub fn with_trace(mut self, trace_id: &str, span_id: &str) -> Self {
        self.trace = Trace {
            trace_id: Some(trace_id.to_string()),
            span_id: Some(span_id.to_string()),
        };
        self
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

pub type Timestamp = u64;
