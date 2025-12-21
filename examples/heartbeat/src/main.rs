use std::sync::Arc;
use std::time::Duration;

use cortex_core::{
    async_trait,
    capability::CapabilitySet,
    event::{Event, Payload},
    runtime::{Agent, EventBus, Runtime},
    Result,
};
use tracing::{info, Level};

struct HeartbeatAgent {
    name: String,
    caps: CapabilitySet,
    interval_ms: u64,
    event_bus: Arc<EventBus>,
}

impl HeartbeatAgent {
    fn new(name: &str, interval_ms: u64, event_bus: Arc<EventBus>) -> Self {
        Self {
            name: name.to_string(),
            caps: CapabilitySet::new(),
            interval_ms,
            event_bus,
        }
    }
}

#[async_trait]
impl Agent for HeartbeatAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn start(&self) -> Result<()> {
        let name = self.name.clone();
        let interval_ms = self.interval_ms;
        let event_bus = Arc::clone(&self.event_bus);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_millis(interval_ms));
            let mut count: u64 = 0;

            loop {
                ticker.tick().await;
                count += 1;

                let event = Event::new(
                    &name,
                    "agent.heartbeat.v1",
                    Payload::inline(count.to_le_bytes().to_vec()),
                );

                info!(
                    agent = %name,
                    count = count,
                    event_id = %event.id,
                    "💓 Heartbeat"
                );

                let _ = event_bus.publish(event);
            }
        });

        Ok(())
    }

    async fn handle(&self, _event: Event) -> Result<()> {
        Ok(())
    }
}

struct ListenerAgent {
    name: String,
    caps: CapabilitySet,
}

impl ListenerAgent {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            caps: CapabilitySet::new(),
        }
    }
}

#[async_trait]
impl Agent for ListenerAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn handle(&self, event: Event) -> Result<()> {
        let count = if let Payload::Inline(bytes) = &event.payload {
            if bytes.len() >= 8 {
                u64::from_le_bytes(bytes[..8].try_into().unwrap_or([0; 8]))
            } else {
                0
            }
        } else {
            0
        };

        info!(
            listener = %self.name,
            from = %event.source,
            count = count,
            "👂 Received heartbeat"
        );

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 CortexOS Heartbeat Demo");
    info!("   Demonstrating event-driven agent communication");
    info!("");

    let runtime = Runtime::new();
    let event_bus = runtime.event_bus();

    let heartbeat1 = HeartbeatAgent::new("heart-1", 1000, Arc::clone(&event_bus));
    let heartbeat2 = HeartbeatAgent::new("heart-2", 1500, Arc::clone(&event_bus));
    let listener = ListenerAgent::new("listener-1");

    runtime
        .spawn_agent(heartbeat1)
        .await
        .expect("Failed to spawn heartbeat1");
    runtime
        .spawn_agent(heartbeat2)
        .await
        .expect("Failed to spawn heartbeat2");
    runtime
        .spawn_agent(listener)
        .await
        .expect("Failed to spawn listener");

    let mut subscription = runtime.subscribe("agent.heartbeat.*");

    info!("Agents spawned. Press Ctrl+C to stop.\n");

    tokio::spawn(async move {
        while let Some(event) = subscription.recv().await {
            if let Some(agent) = runtime.get_agent("listener-1") {
                let _ = agent.send(event).await;
            }
        }
    });

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    info!("\n🛑 Shutting down...");
}
