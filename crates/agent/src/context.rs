use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

use crate::error::AgentError;
use crate::intention::IntentionManager;
use crate::lifecycle::AgentHandle;
use crate::traits::{Agent, Emitter};
use crate::types::{
    Event, EventId, EventPattern, GraphQuery, IntentionId, NodeId, ThoughtContent, ThoughtNode,
    Timestamp,
};

pub struct EventBusHandle {
    sender: broadcast::Sender<Event>,
}

impl EventBusHandle {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: Event) -> Result<(), AgentError> {
        self.sender
            .send(event)
            .map_err(|e| AgentError::EventBusError(e.to_string()))?;
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

impl Clone for EventBusHandle {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

pub struct GraphStoreHandle {
    nodes: Arc<RwLock<HashMap<NodeId, ThoughtNode>>>,
}

impl GraphStoreHandle {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_node(&self, content: ThoughtContent) -> NodeId {
        let id = NodeId::new();
        let node = ThoughtNode {
            id,
            content,
            created_at: Timestamp::now(),
        };
        self.nodes.write().await.insert(id, node);
        id
    }

    pub async fn get_node(&self, id: &NodeId) -> Option<ThoughtNode> {
        self.nodes.read().await.get(id).cloned()
    }

    pub async fn query(&self, query: &GraphQuery) -> Vec<ThoughtNode> {
        let nodes = self.nodes.read().await;
        let mut results: Vec<_> = nodes
            .values()
            .filter(|node| {
                query
                    .node_type
                    .as_ref()
                    .map(|t| node.content.node_type == *t)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        results
    }
}

impl Default for GraphStoreHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GraphStoreHandle {
    fn clone(&self) -> Self {
        Self {
            nodes: Arc::clone(&self.nodes),
        }
    }
}

pub struct Subscription {
    pattern: EventPattern,
    receiver: broadcast::Receiver<Event>,
}

impl Subscription {
    pub fn new(pattern: EventPattern, receiver: broadcast::Receiver<Event>) -> Self {
        Self { pattern, receiver }
    }

    pub async fn next(&mut self) -> Option<Event> {
        loop {
            match self.receiver.recv().await {
                Ok(event) if self.pattern.matches(&event) => return Some(event),
                Ok(_) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    }

    pub fn pattern(&self) -> &EventPattern {
        &self.pattern
    }
}

pub struct AgentContext {
    event_bus: EventBusHandle,
    graph: GraphStoreHandle,
    intentions: IntentionManager,
    emitters: Vec<Box<dyn Emitter>>,
    agent_spawn_tx: mpsc::Sender<Box<dyn Agent>>,
}

impl AgentContext {
    pub fn new(
        event_bus: EventBusHandle,
        graph: GraphStoreHandle,
        intentions: IntentionManager,
        agent_spawn_tx: mpsc::Sender<Box<dyn Agent>>,
    ) -> Self {
        Self {
            event_bus,
            graph,
            intentions,
            emitters: Vec::new(),
            agent_spawn_tx,
        }
    }

    pub fn add_emitter(&mut self, emitter: Box<dyn Emitter>) {
        self.emitters.push(emitter);
    }

    pub async fn emit_event(&self, kind: &str, payload: Vec<u8>) -> Result<EventId, AgentError> {
        let event = Event::new(kind, payload);
        let id = event.id;

        for emitter in &self.emitters {
            emitter.emit(&event);
        }

        self.event_bus.publish(event)?;
        Ok(id)
    }

    pub async fn subscribe(&self, pattern: EventPattern) -> Result<Subscription, AgentError> {
        let receiver = self.event_bus.subscribe();
        Ok(Subscription::new(pattern, receiver))
    }

    pub async fn query_graph(&self, query: GraphQuery) -> Result<Vec<ThoughtNode>, AgentError> {
        Ok(self.graph.query(&query).await)
    }

    pub async fn add_thought(&self, content: ThoughtContent) -> Result<NodeId, AgentError> {
        Ok(self.graph.add_node(content).await)
    }

    pub async fn set_intention(&self, goal: &str) -> Result<IntentionId, AgentError> {
        Ok(self.intentions.create_intention(goal).await)
    }

    pub async fn spawn_subagent(&self, agent: Box<dyn Agent>) -> Result<AgentHandle, AgentError> {
        let handle = AgentHandle::new(*agent.id());
        self.agent_spawn_tx
            .send(agent)
            .await
            .map_err(|e| AgentError::SpawnFailed(e.to_string()))?;
        Ok(handle)
    }

    pub fn event_bus(&self) -> &EventBusHandle {
        &self.event_bus
    }

    pub fn graph(&self) -> &GraphStoreHandle {
        &self.graph
    }

    pub fn intentions(&self) -> &IntentionManager {
        &self.intentions
    }
}
