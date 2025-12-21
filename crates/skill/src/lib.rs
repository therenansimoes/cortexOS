pub mod definition;
pub mod executor;
pub mod router;
pub mod registry;
pub mod task;
pub mod error;
pub mod delegation;

pub use definition::{Skill, SkillCapability, SkillMetadata, SkillInput, SkillOutput};
pub use executor::{SkillExecutor, ExecutionResult, ExecutionContext};
pub use router::{SkillRouter, RouteDecision};
pub use registry::{LocalSkillRegistry, NetworkSkillRegistry};
pub use task::{SkillTask, TaskStatus, TaskResult};
pub use error::{SkillError, Result};
pub use delegation::DelegationCoordinator;
