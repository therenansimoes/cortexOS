use cortex_agent::prelude::*;
use cortex_agent::{
    builtin::PlannerAgent,
    context::{EventBusHandle, GraphStoreHandle},
    intention::IntentionManager,
};
use tokio::sync::mpsc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Simple task executor agent that responds to specific tasks
struct TaskExecutorAgent {
    id: AgentId,
    name: String,
    capabilities: CapabilitySet,
    tasks_executed: u32,
}

impl TaskExecutorAgent {
    fn new(name: &str, capabilities: &[&str]) -> Self {
        let mut cap_set = CapabilitySet::new();
        for cap in capabilities {
            cap_set.add(*cap);
        }

        Self {
            id: AgentId::new(),
            name: name.to_string(),
            capabilities: cap_set,
            tasks_executed: 0,
        }
    }
}

#[async_trait]
impl Agent for TaskExecutorAgent {
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
        info!(agent_id = %self.id, name = %self.name, "TaskExecutorAgent initialized");

        // Register capabilities
        ctx.intentions()
            .register_agent_capabilities(*self.id(), self.capabilities.clone())
            .await;

        Ok(())
    }

    async fn on_event(&mut self, event: &Event, ctx: &mut AgentContext) -> Result<(), AgentError> {
        // Respond to task assignment events
        if event.kind == "task.assigned" {
            if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&event.payload) {
                if let Some(task) = data.get("task").and_then(|t| t.as_str()) {
                    info!(
                        agent_id = %self.id,
                        name = %self.name,
                        task = %task,
                        "Executing task"
                    );

                    self.tasks_executed += 1;

                    // Simulate task execution
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Emit completion event
                    ctx.emit_event(
                        "task.completed",
                        serde_json::to_vec(&serde_json::json!({
                            "agent_id": self.id.to_string(),
                            "task": task,
                        }))
                        .unwrap_or_default(),
                    )
                    .await?;
                }
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
            name = %self.name,
            tasks_executed = self.tasks_executed,
            "TaskExecutorAgent shutting down"
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Planner Agent Demo");

    // Setup shared infrastructure
    let event_bus = EventBusHandle::new(1000);
    // Keep one subscription alive to prevent channel close
    let _keepalive = event_bus.subscribe();
    let graph = GraphStoreHandle::new();
    let intentions = IntentionManager::new();
    let (agent_spawn_tx, _agent_spawn_rx) = mpsc::channel(10);

    // Create planner agent
    let mut planner = PlannerAgent::new()
        .with_name("main-planner")
        .with_llm(false);

    // Create task executor agents
    let mut api_agent = TaskExecutorAgent::new("api-designer", &["design", "api", "endpoints"]);
    let mut http_agent = TaskExecutorAgent::new("http-implementer", &["implement", "http", "server"]);
    let mut routing_agent = TaskExecutorAgent::new("router", &["routing", "request"]);
    let mut error_agent = TaskExecutorAgent::new("error-handler", &["error", "handling"]);
    let mut test_agent = TaskExecutorAgent::new("tester", &["test", "write"]);

    // Create contexts for each agent
    let mut planner_ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx.clone(),
    );

    let mut api_ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx.clone(),
    );

    let mut http_ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx.clone(),
    );

    let mut routing_ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx.clone(),
    );

    let mut error_ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx.clone(),
    );

    let mut test_ctx = AgentContext::new(
        event_bus.clone(),
        graph.clone(),
        intentions.clone(),
        agent_spawn_tx.clone(),
    );

    // Initialize all agents
    info!("📋 Initializing agents...");
    planner.init(&mut planner_ctx).await?;
    api_agent.init(&mut api_ctx).await?;
    http_agent.init(&mut http_ctx).await?;
    routing_agent.init(&mut routing_ctx).await?;
    error_agent.init(&mut error_ctx).await?;
    test_agent.init(&mut test_ctx).await?;

    // Subscribe to planner events
    let mut plan_subscription = event_bus.subscribe();

    info!("🎯 Requesting plan for: 'build a web server'");

    // Send planning request
    event_bus.publish(cortex_agent::types::Event::new(
        "planner.plan_request",
        serde_json::to_vec(&serde_json::json!({
            "goal": "build a web server",
        }))?,
    ))?;

    // Process events for a short time
    let mut event_count = 0;
    let timeout = tokio::time::Duration::from_secs(2);
    let start = tokio::time::Instant::now();

    info!("📊 Processing planning events...");

    while start.elapsed() < timeout && event_count < 20 {
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                planner.tick(&mut planner_ctx).await?;
            }
            Ok(event) = plan_subscription.recv() => {
                event_count += 1;

                // Feed event to planner
                planner.on_event(&event, &mut planner_ctx).await?;

                if event.kind == "planner.plan_created" {
                    if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&event.payload) {
                        info!("✅ Plan created!");
                        if let Some(subgoals) = data.get("subgoals").and_then(|s| s.as_array()) {
                            info!("📝 Subgoals:");
                            for (i, subgoal) in subgoals.iter().enumerate() {
                                if let Some(goal_text) = subgoal.as_str() {
                                    info!("   {}. {}", i + 1, goal_text);
                                }
                            }
                        }
                    }
                }

                if event.kind == "planner.plan_completed" {
                    info!("🎉 Plan execution completed!");
                }

                // Feed events to executor agents
                api_agent.on_event(&event, &mut api_ctx).await?;
                http_agent.on_event(&event, &mut http_ctx).await?;
                routing_agent.on_event(&event, &mut routing_ctx).await?;
                error_agent.on_event(&event, &mut error_ctx).await?;
                test_agent.on_event(&event, &mut test_ctx).await?;
            }
        }
    }

    // Display final statistics
    info!("📈 Final Statistics:");
    let stats = planner.stats().await;
    info!("   Plans created: {}", stats.plans_created);
    info!("   Subgoals generated: {}", stats.subgoals_generated);
    info!("   Coordination events: {}", stats.coordination_events);

    // Shutdown agents
    info!("🛑 Shutting down agents...");
    planner.shutdown(&mut planner_ctx).await?;
    api_agent.shutdown(&mut api_ctx).await?;
    http_agent.shutdown(&mut http_ctx).await?;
    routing_agent.shutdown(&mut routing_ctx).await?;
    error_agent.shutdown(&mut error_ctx).await?;
    test_agent.shutdown(&mut test_ctx).await?;

    info!("✨ Demo completed successfully!");

    Ok(())
}
