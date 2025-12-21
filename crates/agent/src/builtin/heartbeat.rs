use async_trait::async_trait;
use std::time::Duration;
use tracing::debug;

use crate::context::AgentContext;
use crate::error::AgentError;
use crate::traits::Agent;
use crate::types::{AgentId, CapabilitySet, Event, Timestamp};

pub struct HeartbeatAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
    interval: Duration,
    tick_count: u64,
    last_heartbeat: Option<Timestamp>,
}

impl HeartbeatAgent {
    pub fn new(interval: Duration) -> Self {
        Self {
            id: AgentId::new(),
            name: "heartbeat".to_string(),
            capabilities: CapabilitySet::new()
                .with_capability("heartbeat")
                .with_capability("health-check"),
            interval,
            tick_count: 0,
            last_heartbeat: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn last_heartbeat(&self) -> Option<Timestamp> {
        self.last_heartbeat
    }

    fn should_emit_heartbeat(&self) -> bool {
        self.tick_count % (self.interval.as_secs().max(1)) == 0
    }
}

impl Default for HeartbeatAgent {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

#[async_trait]
impl Agent for HeartbeatAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    async fn init(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError> {
        debug!(agent_id = %self.id, "HeartbeatAgent initialized");

        ctx.emit_event(
            "agent.started",
            serde_json::to_vec(&serde_json::json!({
                "agent_id": self.id.to_string(),
                "agent_name": self.name,
                "interval_secs": self.interval.as_secs(),
            }))
            .unwrap_or_default(),
        )
        .await?;

        Ok(())
    }

    async fn on_event(
        &mut self,
        _event: &Event,
        _ctx: &mut AgentContext,
    ) -> Result<(), AgentError> {
        Ok(())
    }

    async fn tick(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError> {
        self.tick_count += 1;

        if self.should_emit_heartbeat() {
            let now = Timestamp::now();
            self.last_heartbeat = Some(now);

            let payload = serde_json::to_vec(&serde_json::json!({
                "agent_id": self.id.to_string(),
                "tick_count": self.tick_count,
                "timestamp": now.0,
            }))
            .unwrap_or_default();

            ctx.emit_event("heartbeat", payload).await?;
            debug!(agent_id = %self.id, tick = self.tick_count, "Heartbeat emitted");
        }

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError> {
        debug!(agent_id = %self.id, total_ticks = self.tick_count, "HeartbeatAgent shutting down");

        ctx.emit_event(
            "agent.stopped",
            serde_json::to_vec(&serde_json::json!({
                "agent_id": self.id.to_string(),
                "agent_name": self.name,
                "total_ticks": self.tick_count,
            }))
            .unwrap_or_default(),
        )
        .await?;

        Ok(())
    }
}
