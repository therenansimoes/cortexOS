use async_trait::async_trait;
use tracing::{debug, info};

use crate::context::AgentContext;
use crate::error::AgentError;
use crate::traits::Agent;
use crate::types::{AgentId, CapabilitySet, Event, EventPattern};

pub struct LoggerAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
    event_count: u64,
    log_level: LogLevel,
    filter: Option<EventPattern>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
}

impl LoggerAgent {
    pub fn new() -> Self {
        Self {
            id: AgentId::new(),
            name: "logger".to_string(),
            capabilities: CapabilitySet::new()
                .with_capability("logging")
                .with_capability("monitoring"),
            event_count: 0,
            log_level: LogLevel::Info,
            filter: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    pub fn with_filter(mut self, pattern: EventPattern) -> Self {
        self.filter = Some(pattern);
        self
    }

    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    fn should_log(&self, event: &Event) -> bool {
        self.filter
            .as_ref()
            .map(|f| f.matches(event))
            .unwrap_or(true)
    }

    fn log_event(&self, event: &Event) {
        let source = event
            .source
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        match self.log_level {
            LogLevel::Debug => {
                debug!(
                    event_id = %event.id.0,
                    kind = %event.kind,
                    source = %source,
                    payload_len = event.payload.len(),
                    "Event received"
                );
            }
            LogLevel::Info => {
                info!(
                    kind = %event.kind,
                    source = %source,
                    "Event: {}", event.kind
                );
            }
            LogLevel::Warn => {
                if event.kind.contains("error") || event.kind.contains("warn") {
                    tracing::warn!(
                        kind = %event.kind,
                        source = %source,
                        "Event: {}", event.kind
                    );
                }
            }
        }
    }
}

impl Default for LoggerAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for LoggerAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn init(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        info!(agent_id = %self.id, "LoggerAgent initialized");
        Ok(())
    }

    async fn on_event(&mut self, event: &Event, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        if self.should_log(event) {
            self.event_count += 1;
            self.log_event(event);
        }
        Ok(())
    }

    async fn tick(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        info!(
            agent_id = %self.id,
            total_events = self.event_count,
            "LoggerAgent shutting down"
        );
        Ok(())
    }
}
