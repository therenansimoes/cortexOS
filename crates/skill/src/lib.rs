pub mod definition;
pub mod error;
pub mod executor;
pub mod registry;
pub mod router;
pub mod task;

pub use definition::{Skill, SkillCapability, SkillInput, SkillMetadata, SkillOutput};
pub use error::{Result, SkillError};
pub use executor::{ExecutionContext, ExecutionResult, SkillExecutor};
pub use registry::{LocalSkillRegistry, NetworkSkillRegistry};
pub use router::{RouteDecision, SkillRouter};
pub use task::{SkillTask, TaskResult, TaskStatus};
