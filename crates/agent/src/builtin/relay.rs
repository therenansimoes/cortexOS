use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::context::AgentContext;
use crate::error::AgentError;
use crate::traits::Agent;
use crate::types::{AgentId, CapabilitySet, Event, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beacon {
    pub origin: String,
    pub hop_count: u32,
    pub max_hops: u32,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

impl Beacon {
    pub fn new(origin: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            origin: origin.into(),
            hop_count: 0,
            max_hops: 10,
            timestamp: Timestamp::now().0,
            payload,
        }
    }

    pub fn with_max_hops(mut self, max_hops: u32) -> Self {
        self.max_hops = max_hops;
        self
    }

    pub fn can_relay(&self) -> bool {
        self.hop_count < self.max_hops
    }

    pub fn relayed(&self) -> Self {
        Self {
            origin: self.origin.clone(),
            hop_count: self.hop_count + 1,
            max_hops: self.max_hops,
            timestamp: self.timestamp,
            payload: self.payload.clone(),
        }
    }
}

pub struct RelayAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
    relayed_count: u64,
    dropped_count: u64,
}

impl RelayAgent {
    pub fn new() -> Self {
        Self {
            id: AgentId::new(),
            name: "relay".to_string(),
            capabilities: CapabilitySet::new()
                .with_capability("relay")
                .with_capability("mesh")
                .with_capability("beacon"),
            relayed_count: 0,
            dropped_count: 0,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn relayed_count(&self) -> u64 {
        self.relayed_count
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    async fn handle_beacon(&mut self, beacon: Beacon, ctx: &mut AgentContext) -> Result<(), AgentError> {
        if beacon.can_relay() {
            let relayed = beacon.relayed();
            let payload = serde_json::to_vec(&relayed).unwrap_or_default();

            ctx.emit_event("relay.beacon", payload).await?;
            self.relayed_count += 1;

            debug!(
                origin = %beacon.origin,
                hop_count = relayed.hop_count,
                "Beacon relayed"
            );
        } else {
            self.dropped_count += 1;
            debug!(
                origin = %beacon.origin,
                hop_count = beacon.hop_count,
                max_hops = beacon.max_hops,
                "Beacon dropped (max hops reached)"
            );
        }

        Ok(())
    }
}

impl Default for RelayAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for RelayAgent {
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
        info!(agent_id = %self.id, "RelayAgent initialized");
        Ok(())
    }

    async fn on_event(&mut self, event: &Event, ctx: &mut AgentContext) -> Result<(), AgentError> {
        if event.kind == "relay.signal" || event.kind == "beacon" {
            if let Ok(beacon) = serde_json::from_slice::<Beacon>(&event.payload) {
                self.handle_beacon(beacon, ctx).await?;
            }
        }
        Ok(())
    }

    async fn tick(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &mut AgentContext) -> Result<(), AgentError> {
        info!(
            agent_id = %self.id,
            relayed = self.relayed_count,
            dropped = self.dropped_count,
            "RelayAgent shutting down"
        );
        Ok(())
    }
}
