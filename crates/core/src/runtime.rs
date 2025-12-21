use crate::capability::CapabilitySet;
use crate::error::{CoreError, Result};
use crate::event::Event;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::future::BoxFuture;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

/// Runtime metrics for monitoring event processing
#[derive(Debug, Default)]
pub struct RuntimeMetrics {
    /// Total number of events published
    pub events_published: AtomicU64,
    /// Total number of events dropped due to backpressure
    pub events_dropped: AtomicU64,
    /// Total number of events delivered to subscribers
    pub events_delivered: AtomicU64,
    /// Number of active subscriptions
    pub active_subscriptions: AtomicU64,
    /// Number of active agents
    pub active_agents: AtomicU64,
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_publish(&self) {
        self.events_published.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_drop(&self) {
        self.events_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_delivery(&self) {
        self.events_delivered.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            events_published: self.events_published.load(Ordering::Relaxed),
            events_dropped: self.events_dropped.load(Ordering::Relaxed),
            events_delivered: self.events_delivered.load(Ordering::Relaxed),
            active_subscriptions: self.active_subscriptions.load(Ordering::Relaxed),
            active_agents: self.active_agents.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, Copy)]
pub struct MetricsSnapshot {
    pub events_published: u64,
    pub events_dropped: u64,
    pub events_delivered: u64,
    pub active_subscriptions: u64,
    pub active_agents: u64,
}

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
        self.metrics.record_publish();
        
        let _ = self.broadcast.send(event.clone());

        let subscriptions = self.subscriptions.read();
        for sub in subscriptions.iter() {
            if pattern_matches(&sub.pattern, event.kind()) {
                match sub.sender.try_send(event.clone()) {
                    Ok(_) => self.metrics.record_delivery(),
                    Err(_) => self.metrics.record_drop(),
                }
            }
        }
        Ok(())
    }

    /// Publish multiple events in a batch for improved performance.
    /// This reduces lock contention by acquiring the subscriptions lock once.
    pub fn publish_batch(&self, events: &[Event]) -> Result<usize> {
        let mut published = 0;
        let subscriptions = self.subscriptions.read();
        
        for event in events {
            self.metrics.record_publish();
            let _ = self.broadcast.send(event.clone());
            
            for sub in subscriptions.iter() {
                if pattern_matches(&sub.pattern, event.kind()) {
                    match sub.sender.try_send(event.clone()) {
                        Ok(_) => self.metrics.record_delivery(),
                        Err(_) => self.metrics.record_drop(),
                    }
                }
            }
            published += 1;
        }
        
        Ok(published)
    }

    pub fn subscribe(&self, pattern: &str) -> mpsc::Receiver<Event> {
        let (tx, rx) = mpsc::channel(256);
        self.subscriptions.write().push(Subscription {
            pattern: pattern.to_string(),
            sender: tx,
        });
        self.metrics.active_subscriptions.fetch_add(1, Ordering::Relaxed);
        rx
    }

    pub fn subscribe_all(&self) -> broadcast::Receiver<Event> {
        self.broadcast.subscribe()
    }

    pub fn metrics(&self) -> Arc<RuntimeMetrics> {
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
        
        // Update metrics
        self.event_bus.metrics().active_agents.fetch_add(1, Ordering::Relaxed);

        let agent = Arc::new(agent);
        let agent_clone = Arc::clone(&agent);
        let metrics = self.event_bus.metrics();

        tokio::spawn(async move {
            if let Err(e) = agent_clone.start().await {
                tracing::error!(agent = %name, error = %e, "Agent failed to start");
                metrics.active_agents.fetch_sub(1, Ordering::Relaxed);
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
            
            metrics.active_agents.fetch_sub(1, Ordering::Relaxed);
        });

        Ok(())
    }

    pub fn get_agent(&self, name: &str) -> Option<dashmap::mapref::one::Ref<'_, String, AgentHandle>> {
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

    /// Publish multiple events in a batch for improved performance
    pub fn publish_batch(&self, events: &[Event]) -> Result<usize> {
        self.event_bus.publish_batch(events)
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

    /// Get current runtime metrics snapshot
    pub fn metrics(&self) -> MetricsSnapshot {
        self.event_bus.metrics().snapshot()
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

    #[tokio::test]
    async fn test_batch_publishing() {
        let bus = EventBus::new(1024);
        let mut rx = bus.subscribe("batch.*");

        // Create batch of events
        let events: Vec<Event> = (0..10)
            .map(|i| {
                Event::new(
                    "batch-source",
                    "batch.event",
                    Payload::inline(format!("event-{}", i).into_bytes()),
                )
            })
            .collect();

        let published = bus.publish_batch(&events).unwrap();
        assert_eq!(published, 10);

        // Verify all events were received
        for _ in 0..10 {
            let received = rx.recv().await.unwrap();
            assert_eq!(received.kind(), "batch.event");
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new(1024);
        let mut rx1 = bus.subscribe("multi.*");
        let mut rx2 = bus.subscribe("multi.*");
        let mut rx3 = bus.subscribe("*");

        let event = Event::new("source", "multi.test", Payload::inline(b"data".to_vec()));
        bus.publish(event).unwrap();

        // All subscribers should receive the event
        let r1 = rx1.recv().await.unwrap();
        let r2 = rx2.recv().await.unwrap();
        let r3 = rx3.recv().await.unwrap();

        assert_eq!(r1.kind(), "multi.test");
        assert_eq!(r2.kind(), "multi.test");
        assert_eq!(r3.kind(), "multi.test");
    }

    #[tokio::test]
    async fn test_pattern_filtering() {
        let bus = EventBus::new(1024);
        let mut sensor_rx = bus.subscribe("sensor.*");
        let mut grid_rx = bus.subscribe("grid.*");

        // Publish different event types
        bus.publish(Event::new("s1", "sensor.data", Payload::inline(b"s".to_vec())))
            .unwrap();
        bus.publish(Event::new("g1", "grid.msg", Payload::inline(b"g".to_vec())))
            .unwrap();
        bus.publish(Event::new("a1", "agent.action", Payload::inline(b"a".to_vec())))
            .unwrap();

        // Check sensor subscriber only gets sensor events
        let sensor_event = sensor_rx.recv().await.unwrap();
        assert_eq!(sensor_event.kind(), "sensor.data");

        // Check grid subscriber only gets grid events
        let grid_event = grid_rx.recv().await.unwrap();
        assert_eq!(grid_event.kind(), "grid.msg");

        // Verify no more events in queues
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(sensor_rx.try_recv().is_err());
        assert!(grid_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_high_throughput() {
        let bus = EventBus::new(10000);
        
        // Use a smaller batch that fits in the channel buffer
        let batch_size = 100;
        
        let mut rx = bus.subscribe("*");

        // Publish events
        let events: Vec<Event> = (0..batch_size)
            .map(|i| {
                Event::new(
                    "throughput-test",
                    "throughput.event",
                    Payload::inline(format!("event-{}", i).into_bytes()),
                )
            })
            .collect();

        let published = bus.publish_batch(&events).unwrap();
        assert_eq!(published, batch_size);

        // Verify we can receive events
        let mut received = 0;
        for _ in 0..batch_size {
            if rx.recv().await.is_some() {
                received += 1;
            }
        }
        assert_eq!(received, batch_size);
    }

    #[test]
    fn test_pattern_matching() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("test.*", "test.event"));
        assert!(pattern_matches("test.event", "test.event"));
        assert!(!pattern_matches("test.*", "other.event"));
    }

    #[tokio::test]
    async fn test_agent_lifecycle() {
        struct LifecycleAgent {
            name: String,
            caps: CapabilitySet,
            started: Arc<RwLock<bool>>,
            stopped: Arc<RwLock<bool>>,
        }

        #[async_trait]
        impl Agent for LifecycleAgent {
            fn name(&self) -> &str {
                &self.name
            }

            fn capabilities(&self) -> &CapabilitySet {
                &self.caps
            }

            async fn handle(&self, _event: Event) -> Result<()> {
                Ok(())
            }

            async fn start(&self) -> Result<()> {
                *self.started.write() = true;
                Ok(())
            }

            async fn stop(&self) -> Result<()> {
                *self.stopped.write() = true;
                Ok(())
            }
        }

        let started = Arc::new(RwLock::new(false));
        let stopped = Arc::new(RwLock::new(false));
        
        let agent = LifecycleAgent {
            name: "lifecycle-test".to_string(),
            caps: CapabilitySet::new(),
            started: Arc::clone(&started),
            stopped: Arc::clone(&stopped),
        };

        let runtime = Runtime::new();
        runtime.spawn_agent(agent).await.unwrap();

        // Give start() time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(*started.read());

        runtime.shutdown().await.unwrap();
        
        // Give stop() time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        assert!(*stopped.read());
    }

    #[tokio::test]
    async fn test_send_to_agent() {
        let runtime = Runtime::new();
        let agent = TestAgent {
            name: "target-agent".to_string(),
            caps: CapabilitySet::new(),
        };

        runtime.spawn_agent(agent).await.unwrap();

        let event = Event::new("test", "test.event", Payload::inline(b"data".to_vec()));
        runtime.send_to_agent("target-agent", event).await.unwrap();
    }

    #[tokio::test]
    async fn test_send_to_nonexistent_agent() {
        let runtime = Runtime::new();
        let event = Event::new("test", "test.event", Payload::inline(b"data".to_vec()));
        let result = runtime.send_to_agent("nonexistent", event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_duplicate_agent_registration() {
        let runtime = Runtime::new();
        let agent1 = TestAgent {
            name: "duplicate".to_string(),
            caps: CapabilitySet::new(),
        };
        let agent2 = TestAgent {
            name: "duplicate".to_string(),
            caps: CapabilitySet::new(),
        };

        runtime.spawn_agent(agent1).await.unwrap();
        let result = runtime.spawn_agent(agent2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_runtime_publish() {
        let runtime = Runtime::new();
        let mut rx = runtime.subscribe("test.*");

        let event = Event::new("runtime", "test.publish", Payload::inline(b"data".to_vec()));
        runtime.publish(event).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.kind(), "test.publish");
    }

    #[tokio::test]
    async fn test_runtime_publish_batch() {
        let runtime = Runtime::new();
        let mut rx = runtime.subscribe("batch.*");

        let events: Vec<Event> = (0..5)
            .map(|i| {
                Event::new(
                    "runtime",
                    "batch.event",
                    Payload::inline(format!("batch-{}", i).into_bytes()),
                )
            })
            .collect();

        let count = runtime.publish_batch(&events).unwrap();
        assert_eq!(count, 5);

        for _ in 0..5 {
            assert!(rx.recv().await.is_some());
        }
    }

    #[tokio::test]
    async fn test_subscribe_all() {
        let bus = EventBus::new(1024);
        let mut rx = bus.subscribe_all();

        bus.publish(Event::new("s1", "type1", Payload::inline(vec![]))).unwrap();
        bus.publish(Event::new("s2", "type2", Payload::inline(vec![]))).unwrap();

        let e1 = rx.recv().await.unwrap();
        let e2 = rx.recv().await.unwrap();
        
        assert_eq!(e1.kind(), "type1");
        assert_eq!(e2.kind(), "type2");
    }

    #[tokio::test]
    async fn test_eventbus_default() {
        let bus = EventBus::default();
        let mut rx = bus.subscribe("*");
        
        bus.publish(Event::new("test", "event", Payload::inline(vec![]))).unwrap();
        assert!(rx.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_runtime_default() {
        let runtime = Runtime::default();
        assert!(runtime.get_agent("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_runtime_metrics() {
        let runtime = Runtime::new();
        let mut rx = runtime.subscribe("metrics.*");

        // Publish some events
        for i in 0..10 {
            let event = Event::new(
                "test",
                "metrics.event",
                Payload::inline(format!("event-{}", i).into_bytes()),
            );
            runtime.publish(event).unwrap();
        }

        // Receive some events
        for _ in 0..10 {
            let _ = rx.recv().await;
        }

        // Check metrics
        let metrics = runtime.metrics();
        assert_eq!(metrics.events_published, 10);
        assert!(metrics.events_delivered >= 10);
        assert_eq!(metrics.active_subscriptions, 1);
    }

    #[tokio::test]
    async fn test_metrics_agent_tracking() {
        let runtime = Runtime::new();
        
        let initial_metrics = runtime.metrics();
        assert_eq!(initial_metrics.active_agents, 0);

        let agent = TestAgent {
            name: "metrics-agent".to_string(),
            caps: CapabilitySet::new(),
        };

        runtime.spawn_agent(agent).await.unwrap();
        
        // Give agent time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        let metrics = runtime.metrics();
        assert_eq!(metrics.active_agents, 1);

        runtime.shutdown().await.unwrap();
        
        // Give agent time to stop
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        let final_metrics = runtime.metrics();
        assert_eq!(final_metrics.active_agents, 0);
    }

    #[tokio::test]
    async fn test_metrics_drop_tracking() {
        let bus = EventBus::new(1024);
        let rx = bus.subscribe("test.*");
        
        // Fill the subscription channel
        for i in 0..300 {
            bus.publish(Event::new("test", "test.event", Payload::inline(vec![i as u8]))).unwrap();
        }
        
        drop(rx); // Drop receiver to cause more drops
        
        for i in 0..10 {
            bus.publish(Event::new("test", "test.event", Payload::inline(vec![i as u8]))).unwrap();
        }
        
        let metrics = bus.metrics().snapshot();
        assert!(metrics.events_published > 0);
        assert!(metrics.events_dropped > 0);
    }
}
