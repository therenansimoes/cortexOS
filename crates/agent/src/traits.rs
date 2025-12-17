use async_trait::async_trait;

use crate::context::AgentContext;
use crate::error::AgentError;
use crate::types::{AgentId, CapabilitySet, Event};

#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> &AgentId;

    fn name(&self) -> &str;

    fn capabilities(&self) -> &CapabilitySet;

    async fn init(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError>;

    async fn on_event(&mut self, event: &Event, ctx: &mut AgentContext) -> Result<(), AgentError>;

    async fn tick(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError>;

    async fn shutdown(&mut self, ctx: &mut AgentContext) -> Result<(), AgentError>;
}

pub trait Emitter: Send + Sync {
    fn emit(&self, event: &Event);
}
