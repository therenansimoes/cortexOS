use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::error::{CoreError, Result};

/// Maximum payload size for inline data (1MB)
const MAX_INLINE_PAYLOAD_SIZE: usize = 1024 * 1024;

/// Maximum length for event kind strings
const MAX_KIND_LENGTH: usize = 256;

/// Maximum length for event source strings
const MAX_SOURCE_LENGTH: usize = 256;

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

    /// Create a validated event with bounds checking
    pub fn new_validated(source: &str, kind: &str, payload: Payload) -> Result<Self> {
        // Validate source length
        if source.is_empty() {
            return Err(CoreError::InvalidEvent("Event source cannot be empty".to_string()));
        }
        if source.len() > MAX_SOURCE_LENGTH {
            return Err(CoreError::InvalidEvent(format!(
                "Event source too long: {} > {}",
                source.len(),
                MAX_SOURCE_LENGTH
            )));
        }

        // Validate kind length
        if kind.is_empty() {
            return Err(CoreError::InvalidEvent("Event kind cannot be empty".to_string()));
        }
        if kind.len() > MAX_KIND_LENGTH {
            return Err(CoreError::InvalidEvent(format!(
                "Event kind too long: {} > {}",
                kind.len(),
                MAX_KIND_LENGTH
            )));
        }

        // Validate kind format (should be versioned like "sensor.mic.v1")
        // Must have at least 2 parts separated by dots for basic namespacing
        let parts: Vec<&str> = kind.split('.').collect();
        if parts.len() < 2 {
            return Err(CoreError::InvalidEvent(
                "Event kind must have at least 2 dot-separated parts (e.g., 'sensor.mic' or 'sensor.mic.v1')".to_string(),
            ));
        }

        // Validate payload size for inline payloads
        if let Payload::Inline(ref data) = payload {
            if data.len() > MAX_INLINE_PAYLOAD_SIZE {
                return Err(CoreError::InvalidEvent(format!(
                    "Inline payload too large: {} > {} bytes",
                    data.len(),
                    MAX_INLINE_PAYLOAD_SIZE
                )));
            }
        }

        Ok(Self {
            id: EventId::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            source: sanitize_string(source),
            kind: sanitize_string(kind),
            payload,
            trace: Trace::default(),
        })
    }

    pub fn with_trace(mut self, trace_id: &str, span_id: &str) -> Self {
        self.trace = Trace {
            trace_id: Some(trace_id.to_string()),
            span_id: Some(span_id.to_string()),
        };
        self
    }

    /// Validate event structure
    pub fn validate(&self) -> Result<()> {
        if self.source.is_empty() {
            return Err(CoreError::InvalidEvent("Event source cannot be empty".to_string()));
        }
        if self.kind.is_empty() {
            return Err(CoreError::InvalidEvent("Event kind cannot be empty".to_string()));
        }
        if let Payload::Inline(ref data) = self.payload {
            if data.len() > MAX_INLINE_PAYLOAD_SIZE {
                return Err(CoreError::InvalidEvent("Inline payload too large".to_string()));
            }
        }
        Ok(())
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Sanitize a string by removing control characters.
///
/// This function filters out control characters from the input string to prevent
/// injection attacks or display issues. Newlines (`\n`) and tabs (`\t`) are preserved
/// as they are commonly used in legitimate text data.
///
/// # Arguments
///
/// * `s` - The input string to sanitize
///
/// # Returns
///
/// A new string with control characters removed, except for newline and tab.
///
/// # Examples
///
/// ```ignore
/// let input = "hello\x00\x01world";
/// let sanitized = sanitize_string(input);
/// assert_eq!(sanitized, "helloworld");
/// ```
fn sanitize_string(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
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

    #[test]
    fn test_event_validation_success() {
        let event = Event::new_validated("test-source", "sensor.mic.v1", Payload::inline(vec![1, 2, 3]));
        assert!(event.is_ok());
    }

    #[test]
    fn test_event_validation_empty_source() {
        let result = Event::new_validated("", "sensor.mic.v1", Payload::inline(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_event_validation_empty_kind() {
        let result = Event::new_validated("source", "", Payload::inline(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_event_validation_invalid_kind_format() {
        let result = Event::new_validated("source", "invalid", Payload::inline(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_event_validation_long_source() {
        let long_source = "a".repeat(300);
        let result = Event::new_validated(&long_source, "test.v1", Payload::inline(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_event_validation_long_kind() {
        let long_kind = "a".repeat(300);
        let result = Event::new_validated("source", &long_kind, Payload::inline(vec![]));
        assert!(result.is_err());
    }

    #[test]
    fn test_event_validation_large_payload() {
        let large_payload = vec![0u8; 2 * 1024 * 1024]; // 2MB
        let result = Event::new_validated("source", "test.v1", Payload::inline(large_payload));
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_control_characters() {
        let source_with_control = "test\x00\x01source";
        let event = Event::new_validated(source_with_control, "test.v1", Payload::inline(vec![])).unwrap();
        assert!(!event.source().contains('\x00'));
        assert!(!event.source().contains('\x01'));
    }

    #[test]
    fn test_event_validate_method() {
        let event = Event::new("source", "kind", Payload::inline(vec![1, 2, 3]));
        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_max_inline_payload_boundary() {
        let max_size_payload = vec![0u8; MAX_INLINE_PAYLOAD_SIZE];
        let result = Event::new_validated("source", "test.v1", Payload::inline(max_size_payload));
        assert!(result.is_ok());
    }
}
