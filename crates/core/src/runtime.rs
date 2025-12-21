use crate::capability::CapabilitySet;
use crate::error::{CoreError, Result};
use crate::event::Event;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::future::BoxFuture;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

pub type EventHandler = Box<dyn Fn(Event) -> BoxFuture<'static, ()> + Send + Sync>;

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &CapabilitySet;
    async fn handle(&self, event: Event) -> Result<()>;
    async fn start(&self) -> Result<()> {
        Ok(())
    }
    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

pub struct AgentHandle {
    pub name: String,
    pub capabilities: CapabilitySet,
    sender: mpsc::Sender<Event>,
    shutdown: mpsc::Sender<()>,
}

impl AgentHandle {
    pub async fn send(&self, event: Event) -> Result<()> {
        self.sender
            .send(event)
            .await
            .map_err(|_| CoreError::ChannelClosed)
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown
            .send(())
            .await
            .map_err(|_| CoreError::ChannelClosed)
    }
}

struct Subscription {
    pattern: String,
    sender: mpsc::Sender<Event>,
}

pub struct EventBus {
    broadcast: broadcast::Sender<Event>,
    subscriptions: RwLock<Vec<Subscription>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (broadcast, _) = broadcast::channel(capacity);
        Self {
            broadcast,
            subscriptions: RwLock::new(Vec::new()),
        }
    }

    pub fn publish(&self, event: Event) -> Result<()> {
        let _ = self.broadcast.send(event.clone());

        let subscriptions = self.subscriptions.read();
        for sub in subscriptions.iter() {
            if pattern_matches(&sub.pattern, event.kind()) {
                let _ = sub.sender.try_send(event.clone());
            }
        }
        Ok(())
    }

    pub fn subscribe(&self, pattern: &str) -> mpsc::Receiver<Event> {
        let (tx, rx) = mpsc::channel(256);
        self.subscriptions.write().push(Subscription {
            pattern: pattern.to_string(),
            sender: tx,
        });
        rx
    }

    pub fn subscribe_all(&self) -> broadcast::Receiver<Event> {
        self.broadcast.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}

fn pattern_matches(pattern: &str, kind: &str) -> bool {
    if pattern == "*" || pattern == kind {
        return true;
    }
    if pattern.ends_with(".*") {
        let prefix = &pattern[..pattern.len() - 2];
        return kind.starts_with(prefix);
    }
    if pattern.ends_with("*") {
        let prefix = &pattern[..pattern.len() - 1];
        return kind.starts_with(prefix);
    }
    false
}

pub struct Runtime {
    agents: DashMap<String, AgentHandle>,
    event_bus: Arc<EventBus>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: RwLock<Option<mpsc::Receiver<()>>>,
}

impl Runtime {
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        Self {
            agents: DashMap::new(),
            event_bus: Arc::new(EventBus::default()),
            shutdown_tx,
            shutdown_rx: RwLock::new(Some(shutdown_rx)),
        }
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    pub async fn spawn_agent<A: Agent + 'static>(&self, agent: A) -> Result<()> {
        let name = agent.name().to_string();

        if self.agents.contains_key(&name) {
            return Err(CoreError::AgentAlreadyRegistered(name));
        }

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(256);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let handle = AgentHandle {
            name: name.clone(),
            capabilities: agent.capabilities().clone(),
            sender: event_tx,
            shutdown: shutdown_tx,
        };

        self.agents.insert(name.clone(), handle);

        let agent = Arc::new(agent);
        let agent_clone = Arc::clone(&agent);

        tokio::spawn(async move {
            if let Err(e) = agent_clone.start().await {
                tracing::error!(agent = %name, error = %e, "Agent failed to start");
                return;
            }

            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        if let Err(e) = agent_clone.handle(event).await {
                            tracing::warn!(agent = %name, error = %e, "Agent failed to handle event");
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!(agent = %name, "Agent shutting down");
                        break;
                    }
                }
            }

            if let Err(e) = agent_clone.stop().await {
                tracing::error!(agent = %name, error = %e, "Agent failed to stop gracefully");
            }
        });

        Ok(())
    }

    pub fn get_agent(
        &self,
        name: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, String, AgentHandle>> {
        self.agents.get(name)
    }

    pub async fn send_to_agent(&self, name: &str, event: Event) -> Result<()> {
        let agent = self
            .agents
            .get(name)
            .ok_or_else(|| CoreError::AgentNotFound(name.to_string()))?;
        agent.send(event).await
    }

    pub fn publish(&self, event: Event) -> Result<()> {
        self.event_bus.publish(event)
    }

    pub fn subscribe(&self, pattern: &str) -> mpsc::Receiver<Event> {
        self.event_bus.subscribe(pattern)
    }

    pub async fn shutdown(&self) -> Result<()> {
        for entry in self.agents.iter() {
            let _ = entry.value().shutdown().await;
        }
        self.shutdown_tx
            .send(())
            .await
            .map_err(|_| CoreError::RuntimeShutdown)
    }

    pub async fn run(&self) -> Result<()> {
        let mut shutdown_rx = self
            .shutdown_rx
            .write()
            .take()
            .ok_or(CoreError::RuntimeShutdown)?;

        shutdown_rx.recv().await;
        Ok(())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Payload;

    struct TestAgent {
        name: String,
        caps: CapabilitySet,
    }

    #[async_trait]
    impl Agent for TestAgent {
        fn name(&self) -> &str {
            &self.name
        }

        fn capabilities(&self) -> &CapabilitySet {
            &self.caps
        }

        async fn handle(&self, event: Event) -> Result<()> {
            tracing::info!(agent = %self.name, kind = %event.kind(), "Received event");
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_spawn_agent() {
        let runtime = Runtime::new();
        let agent = TestAgent {
            name: "test-agent".to_string(),
            caps: CapabilitySet::new(),
        };

        runtime.spawn_agent(agent).await.unwrap();
        assert!(runtime.get_agent("test-agent").is_some());
    }

    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe("test.*");

        let event = Event::new("source", "test.event", Payload::inline(b"data".to_vec()));
        bus.publish(event).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.kind(), "test.event");
    }

    #[test]
    fn test_pattern_matching() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("test.*", "test.event"));
        assert!(pattern_matches("test.event", "test.event"));
        assert!(!pattern_matches("test.*", "other.event"));
    }
}
