use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Maximum allowed size for inline payloads (1MB)
const MAX_INLINE_PAYLOAD_SIZE: usize = 1024 * 1024;

/// Maximum allowed length for source and kind fields (256 chars)
const MAX_FIELD_LENGTH: usize = 256;

/// Global event metrics
static EVENTS_CREATED: AtomicU64 = AtomicU64::new(0);
static EVENTS_VALIDATED: AtomicU64 = AtomicU64::new(0);
static VALIDATION_FAILURES: AtomicU64 = AtomicU64::new(0);

/// Event validation error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    SourceTooLong { length: usize, max: usize },
    KindTooLong { length: usize, max: usize },
    KindInvalid { reason: String },
    PayloadTooLarge { size: usize, max: usize },
    TimestampInFuture { timestamp: u64, now: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_content(content: &[u8]) -> Self {
        let hash = blake3::hash(content);
        let bytes: [u8; 16] = hash.as_bytes()[..16].try_into().unwrap();
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

    /// Get the size of the payload in bytes
    pub fn size(&self) -> u64 {
        match self {
            Payload::Inline(data) => data.len() as u64,
            Payload::Reference { size, .. } => *size,
        }
    }

    /// Validate payload size constraints
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Payload::Inline(data) => {
                if data.len() > MAX_INLINE_PAYLOAD_SIZE {
                    return Err(ValidationError::PayloadTooLarge {
                        size: data.len(),
                        max: MAX_INLINE_PAYLOAD_SIZE,
                    });
                }
            }
            Payload::Reference { .. } => {
                // References are always valid as they just point to data
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
}

impl Trace {
    /// Create a new root trace context
    pub fn new_root() -> Self {
        Self {
            trace_id: Some(Uuid::new_v4().to_string()),
            span_id: Some(Uuid::new_v4().to_string()),
            parent_span_id: None,
        }
    }

    /// Create a child trace context from parent
    pub fn new_child(parent: &Trace) -> Self {
        Self {
            trace_id: parent.trace_id.clone(),
            span_id: Some(Uuid::new_v4().to_string()),
            parent_span_id: parent.span_id.clone(),
        }
    }

    /// Check if this trace has a valid trace_id
    pub fn is_valid(&self) -> bool {
        self.trace_id.is_some()
    }
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            trace_id: None,
            span_id: None,
            parent_span_id: None,
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
        EVENTS_CREATED.fetch_add(1, Ordering::Relaxed);
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

    /// Create a new event with automatic trace context
    pub fn new_with_trace(source: &str, kind: &str, payload: Payload) -> Self {
        let mut event = Self::new(source, kind, payload);
        event.trace = Trace::new_root();
        event
    }

    /// Create a child event that propagates trace context
    pub fn new_child(&self, source: &str, kind: &str, payload: Payload) -> Self {
        EVENTS_CREATED.fetch_add(1, Ordering::Relaxed);
        Self {
            id: EventId::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            source: source.to_string(),
            kind: kind.to_string(),
            payload,
            trace: Trace::new_child(&self.trace),
        }
    }

    pub fn with_trace(mut self, trace_id: &str, span_id: &str) -> Self {
        self.trace = Trace {
            trace_id: Some(trace_id.to_string()),
            span_id: Some(span_id.to_string()),
            parent_span_id: None,
        };
        self
    }

    /// Validate event fields and constraints
    pub fn validate(&self) -> Result<(), ValidationError> {
        EVENTS_VALIDATED.fetch_add(1, Ordering::Relaxed);

        // Validate source length
        if self.source.len() > MAX_FIELD_LENGTH {
            VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(ValidationError::SourceTooLong {
                length: self.source.len(),
                max: MAX_FIELD_LENGTH,
            });
        }

        // Validate kind length
        if self.kind.len() > MAX_FIELD_LENGTH {
            VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(ValidationError::KindTooLong {
                length: self.kind.len(),
                max: MAX_FIELD_LENGTH,
            });
        }

        // Validate kind format (should be lowercase with dots, e.g., "sensor.mic.v1")
        if !self.kind.chars().all(|c| c.is_ascii_lowercase() || c == '.' || c.is_ascii_digit()) {
            VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(ValidationError::KindInvalid {
                reason: "kind must contain only lowercase letters, dots, and digits".to_string(),
            });
        }

        // Validate timestamp is not in the future (allow 1 second clock skew)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if self.timestamp > now + 1000 {
            VALIDATION_FAILURES.fetch_add(1, Ordering::Relaxed);
            return Err(ValidationError::TimestampInFuture {
                timestamp: self.timestamp,
                now,
            });
        }

        // Validate payload
        self.payload.validate()?;

        Ok(())
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Event metrics for monitoring
pub struct EventMetrics {
    pub events_created: u64,
    pub events_validated: u64,
    pub validation_failures: u64,
}

impl EventMetrics {
    /// Get current event metrics
    pub fn snapshot() -> Self {
        Self {
            events_created: EVENTS_CREATED.load(Ordering::Relaxed),
            events_validated: EVENTS_VALIDATED.load(Ordering::Relaxed),
            validation_failures: VALIDATION_FAILURES.load(Ordering::Relaxed),
        }
    }

    /// Reset all metrics (useful for testing)
    #[cfg(test)]
    pub fn reset() {
        EVENTS_CREATED.store(0, Ordering::Relaxed);
        EVENTS_VALIDATED.store(0, Ordering::Relaxed);
        VALIDATION_FAILURES.store(0, Ordering::Relaxed);
    }
}

pub type Timestamp = u64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_validation_valid() {
        EventMetrics::reset();
        let event = Event::new("test-source", "sensor.mic.v1", Payload::inline(vec![1, 2, 3]));
        assert!(event.validate().is_ok());
        let metrics = EventMetrics::snapshot();
        assert_eq!(metrics.events_created, 1);
        assert_eq!(metrics.events_validated, 1);
        assert_eq!(metrics.validation_failures, 0);
    }

    #[test]
    fn test_event_validation_source_too_long() {
        EventMetrics::reset();
        let long_source = "a".repeat(300);
        let event = Event::new(&long_source, "test.v1", Payload::inline(vec![]));
        let result = event.validate();
        assert!(matches!(result, Err(ValidationError::SourceTooLong { .. })));
        let metrics = EventMetrics::snapshot();
        assert_eq!(metrics.validation_failures, 1);
    }

    #[test]
    fn test_event_validation_kind_too_long() {
        EventMetrics::reset();
        let long_kind = "a".repeat(300);
        let event = Event::new("source", &long_kind, Payload::inline(vec![]));
        let result = event.validate();
        assert!(matches!(result, Err(ValidationError::KindTooLong { .. })));
    }

    #[test]
    fn test_event_validation_invalid_kind_format() {
        EventMetrics::reset();
        let event = Event::new("source", "Invalid-Kind", Payload::inline(vec![]));
        let result = event.validate();
        assert!(matches!(result, Err(ValidationError::KindInvalid { .. })));
    }

    #[test]
    fn test_payload_validation_too_large() {
        EventMetrics::reset();
        let large_data = vec![0u8; MAX_INLINE_PAYLOAD_SIZE + 1];
        let payload = Payload::inline(large_data);
        let result = payload.validate();
        assert!(matches!(result, Err(ValidationError::PayloadTooLarge { .. })));
    }

    #[test]
    fn test_trace_context_propagation() {
        let parent = Event::new_with_trace("parent", "test.v1", Payload::inline(vec![]));
        assert!(parent.trace.is_valid());
        assert!(parent.trace.trace_id.is_some());
        assert!(parent.trace.span_id.is_some());
        assert!(parent.trace.parent_span_id.is_none());

        let child = parent.new_child("child", "test.v2", Payload::inline(vec![]));
        assert!(child.trace.is_valid());
        assert_eq!(child.trace.trace_id, parent.trace.trace_id);
        assert_ne!(child.trace.span_id, parent.trace.span_id);
        assert_eq!(child.trace.parent_span_id, parent.trace.span_id);
    }

    #[test]
    fn test_trace_new_child() {
        let parent_trace = Trace::new_root();
        let child_trace = Trace::new_child(&parent_trace);
        
        assert_eq!(child_trace.trace_id, parent_trace.trace_id);
        assert_ne!(child_trace.span_id, parent_trace.span_id);
        assert_eq!(child_trace.parent_span_id, parent_trace.span_id);
    }

    #[test]
    fn test_payload_size() {
        let inline = Payload::inline(vec![1, 2, 3, 4, 5]);
        assert_eq!(inline.size(), 5);

        let reference = Payload::reference([0u8; 32], 12345);
        assert_eq!(reference.size(), 12345);
    }

    #[test]
    fn test_event_metrics() {
        EventMetrics::reset();
        
        let event1 = Event::new("test", "test.v1", Payload::inline(vec![]));
        let event2 = Event::new("test", "test.v1", Payload::inline(vec![]));
        
        event1.validate().unwrap();
        event2.validate().unwrap();
        
        let metrics = EventMetrics::snapshot();
        assert_eq!(metrics.events_created, 2);
        assert_eq!(metrics.events_validated, 2);
        assert_eq!(metrics.validation_failures, 0);
    }
}
