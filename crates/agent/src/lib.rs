pub mod builtin;
pub mod context;
pub mod error;
pub mod intention;
pub mod lifecycle;
pub mod traits;
pub mod types;

pub use context::{AgentContext, EventBusHandle, GraphStoreHandle, Subscription};
pub use error::{AgentError, IntentionError};
pub use intention::{Intention, IntentionManager, IntentionStatus};
pub use lifecycle::{AgentHandle, AgentManager, AgentManagerConfig, AgentState};
pub use traits::{Agent, Emitter};
pub use types::{
    AgentId, CapabilitySet, Event, EventId, EventPattern, GraphQuery, IntentionId, NodeId,
    ThoughtContent, ThoughtNode, Timestamp,
};

pub mod prelude {
    pub use crate::builtin::{HeartbeatAgent, LoggerAgent, PlannerAgent, RelayAgent};
    pub use crate::context::AgentContext;
    pub use crate::error::AgentError;
    pub use crate::intention::{Intention, IntentionManager, IntentionStatus};
    pub use crate::lifecycle::{AgentHandle, AgentManager, AgentState};
    pub use crate::traits::Agent;
    pub use crate::types::{AgentId, CapabilitySet, Event, EventPattern};
    pub use async_trait::async_trait;
}
