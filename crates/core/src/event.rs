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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_id_generation() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_event_id_from_content() {
        let content = b"test content";
        let id1 = EventId::from_content(content);
        let id2 = EventId::from_content(content);
        assert_eq!(id1, id2); // Same content should produce same ID
    }

    #[test]
    fn test_event_creation() {
        let event = Event::new("test-source", "test.event", Payload::inline(b"data".to_vec()));
        assert_eq!(event.source(), "test-source");
        assert_eq!(event.kind(), "test.event");
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_event_with_trace() {
        let event = Event::new("source", "kind", Payload::inline(vec![]))
            .with_trace("trace-123", "span-456");
        
        assert_eq!(event.trace.trace_id, Some("trace-123".to_string()));
        assert_eq!(event.trace.span_id, Some("span-456".to_string()));
    }

    #[test]
    fn test_payload_inline() {
        let data = b"test data".to_vec();
        let payload = Payload::inline(data.clone());
        assert_eq!(payload.as_bytes(), Some(data.as_slice()));
    }

    #[test]
    fn test_payload_reference() {
        let hash = [0u8; 32];
        let payload = Payload::reference(hash, 1024);
        assert!(payload.as_bytes().is_none());
        
        match payload {
            Payload::Reference { hash: h, size: s } => {
                assert_eq!(h, hash);
                assert_eq!(s, 1024);
            }
            _ => panic!("Expected Reference payload"),
        }
    }

    #[test]
    fn test_event_id_display() {
        let id = EventId::new();
        let display = format!("{}", id);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_trace_default() {
        let trace = Trace::default();
        assert!(trace.trace_id.is_none());
        assert!(trace.span_id.is_none());
    }
}
