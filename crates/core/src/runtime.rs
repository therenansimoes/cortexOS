use crate::capability::CapabilitySet;
use crate::error::{CoreError, Result};
use crate::event::Event;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::future::BoxFuture;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

pub type EventHandler = Box<dyn Fn(Event) -> BoxFuture<'static, ()> + Send + Sync>;

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum number of events in the event bus buffer
    pub event_bus_capacity: usize,
    /// Maximum number of events per agent queue
    pub agent_queue_capacity: usize,
    /// Shutdown timeout duration
    pub shutdown_timeout: Duration,
    /// Enable runtime statistics collection
    pub enable_stats: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            event_bus_capacity: 1024,
            agent_queue_capacity: 256,
            shutdown_timeout: Duration::from_secs(30),
            enable_stats: true,
        }
    }
}

/// Runtime statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Number of agents currently registered
    pub active_agents: usize,
    /// Total events published through event bus
    pub events_published: u64,
    /// Total events delivered to agents
    pub events_delivered: u64,
    /// Total events dropped (agent queue full)
    pub events_dropped: u64,
    /// Runtime uptime in seconds
    pub uptime_secs: u64,
}

struct RuntimeMetrics {
    events_published: AtomicU64,
    events_delivered: AtomicU64,
    events_dropped: AtomicU64,
    start_time: Instant,
}

impl RuntimeMetrics {
    fn new() -> Self {
        Self {
            events_published: AtomicU64::new(0),
            events_delivered: AtomicU64::new(0),
            events_dropped: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn snapshot(&self, active_agents: usize) -> RuntimeStats {
        RuntimeStats {
            active_agents,
            events_published: self.events_published.load(Ordering::Relaxed),
            events_delivered: self.events_delivered.load(Ordering::Relaxed),
            events_dropped: self.events_dropped.load(Ordering::Relaxed),
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }
}

/// Agent health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Agent health check result
#[derive(Debug, Clone)]
pub struct AgentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub events_processed: u64,
    pub last_event_time: Option<Instant>,
}

struct AgentMetrics {
    events_processed: AtomicU64,
    last_event_time: RwLock<Option<Instant>>,
}

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
    metrics: Arc<AgentMetrics>,
}

impl AgentHandle {
    pub async fn send(&self, event: Event) -> Result<()> {
        match self.sender.send(event).await {
            Ok(_) => {
                self.metrics.events_processed.fetch_add(1, Ordering::Relaxed);
                *self.metrics.last_event_time.write() = Some(Instant::now());
                Ok(())
            }
            Err(_) => Err(CoreError::ChannelClosed),
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown
            .send(())
            .await
            .map_err(|_| CoreError::ChannelClosed)
    }

    pub fn health(&self) -> AgentHealth {
        AgentHealth {
            name: self.name.clone(),
            status: self.calculate_health_status(),
            events_processed: self.metrics.events_processed.load(Ordering::Relaxed),
            last_event_time: *self.metrics.last_event_time.read(),
        }
    }

    fn calculate_health_status(&self) -> HealthStatus {
        let last_event = *self.metrics.last_event_time.read();
        match last_event {
            None => HealthStatus::Healthy, // Just started, no events yet
            Some(last) => {
                let elapsed = last.elapsed();
                if elapsed > Duration::from_secs(300) {
                    HealthStatus::Unhealthy
                } else if elapsed > Duration::from_secs(60) {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                }
            }
        }
    }
}

struct Subscription {
    pattern: String,
    sender: mpsc::Sender<Event>,
}

pub struct EventBus {
    broadcast: broadcast::Sender<Event>,
    subscriptions: RwLock<Vec<Subscription>>,
    metrics: Arc<RuntimeMetrics>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (broadcast, _) = broadcast::channel(capacity);
        Self {
            broadcast,
            subscriptions: RwLock::new(Vec::new()),
            metrics: Arc::new(RuntimeMetrics::new()),
        }
    }

    pub fn publish(&self, event: Event) -> Result<()> {
        self.metrics.events_published.fetch_add(1, Ordering::Relaxed);
        
        let _ = self.broadcast.send(event.clone());

        let subscriptions = self.subscriptions.read();
        let mut delivered = 0;
        let mut dropped = 0;
        
        for sub in subscriptions.iter() {
            if pattern_matches(&sub.pattern, event.kind()) {
                match sub.sender.try_send(event.clone()) {
                    Ok(_) => delivered += 1,
                    Err(_) => dropped += 1,
                }
            }
        }
        
        self.metrics.events_delivered.fetch_add(delivered, Ordering::Relaxed);
        self.metrics.events_dropped.fetch_add(dropped, Ordering::Relaxed);
        
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

    fn metrics(&self) -> Arc<RuntimeMetrics> {
        Arc::clone(&self.metrics)
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
    config: RuntimeConfig,
}

impl Runtime {
    pub fn new() -> Self {
        Self::with_config(RuntimeConfig::default())
    }

    pub fn with_config(config: RuntimeConfig) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        Self {
            agents: DashMap::new(),
            event_bus: Arc::new(EventBus::new(config.event_bus_capacity)),
            shutdown_tx,
            shutdown_rx: RwLock::new(Some(shutdown_rx)),
            config,
        }
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Get runtime statistics
    pub fn stats(&self) -> RuntimeStats {
        let metrics = self.event_bus.metrics();
        metrics.snapshot(self.agents.len())
    }

    /// Get health status of all agents
    pub fn agent_health(&self) -> Vec<AgentHealth> {
        self.agents
            .iter()
            .map(|entry| entry.value().health())
            .collect()
    }

    /// Get health status of a specific agent
    pub fn agent_health_by_name(&self, name: &str) -> Option<AgentHealth> {
        self.agents.get(name).map(|handle| handle.health())
    }

    pub async fn spawn_agent<A: Agent + 'static>(&self, agent: A) -> Result<()> {
        let name = agent.name().to_string();

        if self.agents.contains_key(&name) {
            return Err(CoreError::AgentAlreadyRegistered(name));
        }

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(self.config.agent_queue_capacity);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let metrics = Arc::new(AgentMetrics {
            events_processed: AtomicU64::new(0),
            last_event_time: RwLock::new(None),
        });

        let handle = AgentHandle {
            name: name.clone(),
            capabilities: agent.capabilities().clone(),
            sender: event_tx,
            shutdown: shutdown_tx,
            metrics: Arc::clone(&metrics),
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

    pub fn get_agent(&self, name: &str) -> Option<dashmap::mapref::one::Ref<'_, String, AgentHandle>> {
        self.agents.get(name)
    }

    /// List all registered agent names
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.iter().map(|entry| entry.key().clone()).collect()
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

    /// Graceful shutdown with timeout
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Runtime shutting down gracefully");
        
        // Collect agent names and create shutdown tasks
        let agent_names: Vec<String> = self.agents.iter().map(|e| e.key().clone()).collect();
        
        let shutdown_timeout = self.config.shutdown_timeout;
        let agents_ref = &self.agents;
        
        // Signal all agents to shutdown
        for name in agent_names {
            if let Some(handle) = agents_ref.get(&name) {
                let agent_name = name.clone();
                let shutdown_result = tokio::time::timeout(
                    shutdown_timeout,
                    handle.shutdown()
                ).await;
                
                match shutdown_result {
                    Ok(Ok(_)) => {
                        tracing::debug!(agent = %agent_name, "Agent shutdown successfully");
                    }
                    Ok(Err(e)) => {
                        tracing::warn!(agent = %agent_name, error = %e, "Agent shutdown failed");
                    }
                    Err(_) => {
                        tracing::warn!(agent = %agent_name, "Agent shutdown timed out");
                    }
                }
            }
        }

        // Signal runtime to shutdown
        self.shutdown_tx
            .send(())
            .await
            .map_err(|_| CoreError::RuntimeShutdown)?;

        tracing::info!("Runtime shutdown complete");
        Ok(())
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

    #[tokio::test]
    async fn test_runtime_config() {
        let config = RuntimeConfig {
            event_bus_capacity: 512,
            agent_queue_capacity: 128,
            shutdown_timeout: Duration::from_secs(10),
            enable_stats: true,
        };

        let runtime = Runtime::with_config(config.clone());
        assert_eq!(runtime.config().event_bus_capacity, 512);
        assert_eq!(runtime.config().agent_queue_capacity, 128);
    }

    #[tokio::test]
    async fn test_runtime_stats() {
        let runtime = Runtime::new();
        let agent = TestAgent {
            name: "stats-agent".to_string(),
            caps: CapabilitySet::new(),
        };

        runtime.spawn_agent(agent).await.unwrap();

        let event = Event::new("test", "test.v1", Payload::inline(vec![1, 2, 3]));
        runtime.publish(event).unwrap();

        // Give time for event to be processed
        tokio::time::sleep(Duration::from_millis(10)).await;

        let stats = runtime.stats();
        assert_eq!(stats.active_agents, 1);
        assert!(stats.events_published > 0);
    }

    #[tokio::test]
    async fn test_agent_health() {
        let runtime = Runtime::new();
        let agent = TestAgent {
            name: "health-agent".to_string(),
            caps: CapabilitySet::new(),
        };

        runtime.spawn_agent(agent).await.unwrap();

        let event = Event::new("test", "test.v1", Payload::inline(vec![]));
        runtime.send_to_agent("health-agent", event).await.unwrap();

        // Give time for event to be processed
        tokio::time::sleep(Duration::from_millis(10)).await;

        let health = runtime.agent_health_by_name("health-agent").unwrap();
        assert_eq!(health.name, "health-agent");
        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.events_processed > 0);
    }

    #[tokio::test]
    async fn test_list_agents() {
        let runtime = Runtime::new();
        
        runtime.spawn_agent(TestAgent {
            name: "agent1".to_string(),
            caps: CapabilitySet::new(),
        }).await.unwrap();

        runtime.spawn_agent(TestAgent {
            name: "agent2".to_string(),
            caps: CapabilitySet::new(),
        }).await.unwrap();

        let agents = runtime.list_agents();
        assert_eq!(agents.len(), 2);
        assert!(agents.contains(&"agent1".to_string()));
        assert!(agents.contains(&"agent2".to_string()));
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let config = RuntimeConfig {
            shutdown_timeout: Duration::from_secs(1),
            ..Default::default()
        };
        let runtime = Runtime::with_config(config);

        runtime.spawn_agent(TestAgent {
            name: "shutdown-agent".to_string(),
            caps: CapabilitySet::new(),
        }).await.unwrap();

        // Shutdown should complete within timeout
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            runtime.shutdown()
        ).await;

        assert!(result.is_ok());
    }
}
