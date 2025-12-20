use std::sync::Arc;
use std::time::Duration;

use cortex_core::{
    async_trait,
    capability::{Capability, CapabilitySet},
    event::{Event, Payload},
    runtime::{Agent, EventBus, Runtime},
    Result,
};
use cortex_grid::NodeId;
use serde::{Deserialize, Serialize};
use tracing::{info, Level};

/// Sample Rust code to compile
const SAMPLE_CODE: &str = r#"
fn main() {
    println!("Hello from distributed compilation!");
}
"#;

/// Compilation task
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompilationTask {
    task_id: String,
    source_code: String,
    language: String,
    target: String,
}

/// Compilation result
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompilationResult {
    task_id: String,
    success: bool,
    output: String,
    node_id: String,
}

/// Planner node - orchestrates the compilation process
struct PlannerNode {
    name: String,
    node_id: NodeId,
    caps: CapabilitySet,
    event_bus: Arc<EventBus>,
}

impl PlannerNode {
    fn new(name: &str, node_id: NodeId, event_bus: Arc<EventBus>) -> Self {
        let caps = CapabilitySet::new()
            .with_capability(Capability::grid_full())
            .with_capability(Capability::EventBus {
                publish: vec!["task.*".to_string(), "plan.*".to_string()],
                subscribe: vec!["*".to_string()],
            });
        
        Self {
            name: name.to_string(),
            node_id,
            caps,
            event_bus,
        }
    }
}

#[async_trait]
impl Agent for PlannerNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn start(&self) -> Result<()> {
        info!(
            node = %self.name,
            node_id = %self.node_id,
            "🎯 Planner node started - ready to coordinate tasks"
        );
        
        // Simulate task delegation after startup
        let event_bus = Arc::clone(&self.event_bus);
        
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            let task = CompilationTask {
                task_id: "task-001".to_string(),
                source_code: SAMPLE_CODE.to_string(),
                language: "rust".to_string(),
                target: "wasm32-wasi".to_string(),
            };
            
            info!(
                task_id = %task.task_id,
                "📋 Creating compilation plan for distributed build"
            );
            
            let payload = serde_json::to_vec(&task).unwrap_or_default();
            let event = Event::new("planner", "task.compile", Payload::inline(payload));
            
            let _ = event_bus.publish(event);
        });
        
        Ok(())
    }

    async fn handle(&self, event: Event) -> Result<()> {
        if event.kind() == "compilation.result" {
            if let Payload::Inline(bytes) = &event.payload {
                if let Ok(result) = serde_json::from_slice::<CompilationResult>(bytes) {
                    info!(
                        task_id = %result.task_id,
                        success = result.success,
                        node = %result.node_id,
                        "✅ Compilation completed by node"
                    );
                    info!("   Output: {}", result.output.lines().next().unwrap_or(""));
                }
            }
        }
        Ok(())
    }
}

/// Compiler node - performs the actual compilation
struct CompilerNode {
    name: String,
    node_id: NodeId,
    caps: CapabilitySet,
    event_bus: Arc<EventBus>,
}

impl CompilerNode {
    fn new(name: &str, node_id: NodeId, event_bus: Arc<EventBus>) -> Self {
        let caps = CapabilitySet::new()
            .with_capability(Capability::grid_worker())
            .with_capability(Capability::EventBus {
                publish: vec!["compilation.*".to_string()],
                subscribe: vec!["task.compile".to_string()],
            });
        
        Self {
            name: name.to_string(),
            node_id,
            caps,
            event_bus,
        }
    }
    
    fn compile(&self, task: &CompilationTask) -> CompilationResult {
        info!(
            node = %self.name,
            task_id = %task.task_id,
            target = %task.target,
            "⚙️  Compiling code..."
        );
        
        // Simulate compilation
        std::thread::sleep(Duration::from_millis(300));
        
        CompilationResult {
            task_id: task.task_id.clone(),
            success: true,
            output: format!(
                "Compiled {} code to {}\nGenerated: {}.wasm (simulated)",
                task.language, task.target, task.task_id
            ),
            node_id: self.node_id.to_string(),
        }
    }
}

#[async_trait]
impl Agent for CompilerNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn start(&self) -> Result<()> {
        info!(
            node = %self.name,
            node_id = %self.node_id,
            "🔨 Compiler node started - ready to build"
        );
        Ok(())
    }

    async fn handle(&self, event: Event) -> Result<()> {
        if event.kind() == "task.compile" {
            if let Payload::Inline(bytes) = &event.payload {
                if let Ok(task) = serde_json::from_slice::<CompilationTask>(bytes) {
                    let result = self.compile(&task);
                    
                    let payload = serde_json::to_vec(&result).unwrap_or_default();
                    let result_event = Event::new(
                        &self.name,
                        "compilation.result",
                        Payload::inline(payload),
                    );
                    
                    let _ = self.event_bus.publish(result_event);
                }
            }
        }
        Ok(())
    }
}

/// Executor node - runs and tests compiled code
struct ExecutorNode {
    name: String,
    node_id: NodeId,
    caps: CapabilitySet,
    event_bus: Arc<EventBus>,
}

impl ExecutorNode {
    fn new(name: &str, node_id: NodeId, event_bus: Arc<EventBus>) -> Self {
        let caps = CapabilitySet::new()
            .with_capability(Capability::grid_worker())
            .with_capability(Capability::EventBus {
                publish: vec!["test.*".to_string()],
                subscribe: vec!["compilation.result".to_string()],
            });
        
        Self {
            name: name.to_string(),
            node_id,
            caps,
            event_bus,
        }
    }
}

#[async_trait]
impl Agent for ExecutorNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn start(&self) -> Result<()> {
        info!(
            node = %self.name,
            node_id = %self.node_id,
            "🚀 Executor node started - ready to run tests"
        );
        Ok(())
    }

    async fn handle(&self, event: Event) -> Result<()> {
        if event.kind() == "compilation.result" {
            if let Payload::Inline(bytes) = &event.payload {
                if let Ok(result) = serde_json::from_slice::<CompilationResult>(bytes) {
                    if result.success {
                        info!(
                            node = %self.name,
                            task_id = %result.task_id,
                            "🧪 Running tests on compiled artifact..."
                        );
                        
                        tokio::time::sleep(Duration::from_millis(200)).await;
                        
                        info!(
                            node = %self.name,
                            task_id = %result.task_id,
                            "✅ All tests passed!"
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🌐 CortexOS Distributed Compilation Demo");
    info!("   Demonstrating multi-node compilation across the Grid");
    info!("");

    // Create three separate "nodes" (simulated distributed nodes)
    let planner_id = NodeId::random();
    let compiler_id = NodeId::random();
    let executor_id = NodeId::random();

    info!("Creating distributed node network:");
    info!("  📋 Planner node:  {}", planner_id);
    info!("  🔨 Compiler node: {}", compiler_id);
    info!("  🚀 Executor node: {}", executor_id);
    info!("");

    // In a real distributed system, each node would be a separate process/machine
    // For demo purposes, we simulate this with a shared event bus
    let runtime = Arc::new(Runtime::new());
    let event_bus = runtime.event_bus();

    let planner = PlannerNode::new("planner-node", planner_id, Arc::clone(&event_bus));
    let compiler = CompilerNode::new("compiler-node", compiler_id, Arc::clone(&event_bus));
    let executor = ExecutorNode::new("executor-node", executor_id, Arc::clone(&event_bus));

    runtime.spawn_agent(planner).await.expect("Failed to spawn planner");
    runtime.spawn_agent(compiler).await.expect("Failed to spawn compiler");
    runtime.spawn_agent(executor).await.expect("Failed to spawn executor");

    // Subscribe planner to compilation results
    let mut planner_sub = runtime.subscribe("compilation.result");
    
    // Subscribe compiler to compilation tasks
    let mut compiler_sub = runtime.subscribe("task.compile");
    
    // Subscribe executor to compilation results
    let mut executor_sub = runtime.subscribe("compilation.result");

    info!("📡 Nodes connected and waiting for tasks...");
    info!("");

    // Route events to agents
    let runtime_clone = Arc::clone(&runtime);
    tokio::spawn(async move {
        while let Some(event) = planner_sub.recv().await {
            if let Some(agent) = runtime_clone.get_agent("planner-node") {
                let _ = agent.send(event).await;
            }
        }
    });

    let runtime_clone = Arc::clone(&runtime);
    tokio::spawn(async move {
        while let Some(event) = compiler_sub.recv().await {
            if let Some(agent) = runtime_clone.get_agent("compiler-node") {
                let _ = agent.send(event).await;
            }
        }
    });

    let runtime_clone = Arc::clone(&runtime);
    tokio::spawn(async move {
        while let Some(event) = executor_sub.recv().await {
            if let Some(agent) = runtime_clone.get_agent("executor-node") {
                let _ = agent.send(event).await;
            }
        }
    });

    // Run for a few seconds to let the workflow complete
    tokio::time::sleep(Duration::from_secs(3)).await;

    info!("");
    info!("✅ Distributed compilation workflow completed!");
    info!("");
    info!("Key concepts demonstrated:");
    info!("  • Planner node coordinates the compilation task");
    info!("  • Compiler node performs the actual compilation");
    info!("  • Executor node validates and tests the result");
    info!("  • Nodes communicate via Grid event protocol");
    info!("  • Each node has specialized capabilities");
    info!("");
    info!("In production, each node would be:");
    info!("  - Running on a different machine/device");
    info!("  - Connected via libp2p Grid protocol");
    info!("  - Coordinated by the GridOrchestrator");
    info!("  - Selected based on capability matching");

    let _ = runtime.shutdown().await;
}
