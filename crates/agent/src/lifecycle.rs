use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::context::{AgentContext, EventBusHandle, GraphStoreHandle};
use crate::error::AgentError;
use crate::intention::IntentionManager;
use crate::traits::Agent;
use crate::types::AgentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

pub struct AgentHandle {
    pub id: AgentId,
    state: Arc<RwLock<AgentState>>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl AgentHandle {
    pub fn new(id: AgentId) -> Self {
        Self {
            id,
            state: Arc::new(RwLock::new(AgentState::Starting)),
            stop_tx: None,
        }
    }

    fn with_stop_channel(id: AgentId, stop_tx: oneshot::Sender<()>) -> Self {
        Self {
            id,
            state: Arc::new(RwLock::new(AgentState::Starting)),
            stop_tx: Some(stop_tx),
        }
    }

    pub async fn state(&self) -> AgentState {
        *self.state.read().await
    }

    pub async fn is_running(&self) -> bool {
        *self.state.read().await == AgentState::Running
    }

    pub fn request_stop(mut self) -> bool {
        if let Some(tx) = self.stop_tx.take() {
            tx.send(()).is_ok()
        } else {
            false
        }
    }
}

struct RunningAgent {
    handle: AgentHandle,
    task: JoinHandle<()>,
}

pub struct AgentManagerConfig {
    pub tick_interval: Duration,
    pub event_bus_capacity: usize,
    pub spawn_channel_capacity: usize,
}

impl Default for AgentManagerConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::from_secs(1),
            event_bus_capacity: 1024,
            spawn_channel_capacity: 64,
        }
    }
}

pub struct AgentManager {
    agents: Arc<RwLock<HashMap<AgentId, RunningAgent>>>,
    event_bus: EventBusHandle,
    graph: GraphStoreHandle,
    intentions: IntentionManager,
    config: AgentManagerConfig,
    spawn_tx: mpsc::Sender<Box<dyn Agent>>,
    spawn_rx: Arc<RwLock<Option<mpsc::Receiver<Box<dyn Agent>>>>>,
}

impl AgentManager {
    pub fn new(config: AgentManagerConfig) -> Self {
        let (spawn_tx, spawn_rx) = mpsc::channel(config.spawn_channel_capacity);

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            event_bus: EventBusHandle::new(config.event_bus_capacity),
            graph: GraphStoreHandle::new(),
            intentions: IntentionManager::new(),
            config,
            spawn_tx,
            spawn_rx: Arc::new(RwLock::new(Some(spawn_rx))),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(AgentManagerConfig::default())
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

    pub async fn start_agent(&self, mut agent: Box<dyn Agent>) -> Result<AgentId, AgentError> {
        let agent_id = *agent.id();
        let agent_name = agent.name().to_string();

        {
            let agents = self.agents.read().await;
            if agents.contains_key(&agent_id) {
                return Err(AgentError::AgentAlreadyExists(agent_id));
            }
        }

        info!(agent_id = %agent_id, agent_name = %agent_name, "Starting agent");

        self.intentions
            .register_agent_capabilities(agent_id, agent.capabilities().clone())
            .await;

        let (stop_tx, stop_rx) = oneshot::channel();
        let handle = AgentHandle::with_stop_channel(agent_id, stop_tx);
        let state = Arc::clone(&handle.state);

        let event_bus = self.event_bus.clone();
        let graph = self.graph.clone();
        let intentions = self.intentions.clone();
        let spawn_tx = self.spawn_tx.clone();
        let tick_interval = self.config.tick_interval;

        let task = tokio::spawn(async move {
            let result = run_agent_loop(
                &mut *agent,
                event_bus,
                graph,
                intentions,
                spawn_tx,
                stop_rx,
                tick_interval,
                Arc::clone(&state),
            )
            .await;

            if let Err(e) = result {
                error!(agent_id = %agent_id, error = %e, "Agent failed");
                *state.write().await = AgentState::Failed;
            } else {
                *state.write().await = AgentState::Stopped;
            }
        });

        let running = RunningAgent { handle, task };

        self.agents.write().await.insert(agent_id, running);

        Ok(agent_id)
    }

    pub async fn stop_agent(&self, agent_id: &AgentId) -> Result<(), AgentError> {
        let running = self
            .agents
            .write()
            .await
            .remove(agent_id)
            .ok_or_else(|| AgentError::AgentNotFound(*agent_id))?;

        info!(agent_id = %agent_id, "Stopping agent");

        running.handle.request_stop();

        match tokio::time::timeout(Duration::from_secs(5), running.task).await {
            Ok(Ok(())) => {
                self.intentions.unregister_agent(agent_id).await;
                Ok(())
            }
            Ok(Err(e)) => {
                self.intentions.unregister_agent(agent_id).await;
                if e.is_panic() {
                    Err(AgentError::AgentPanicked(format!("{:?}", e)))
                } else {
                    Err(AgentError::ShutdownFailed(e.to_string()))
                }
            }
            Err(_) => {
                warn!(agent_id = %agent_id, "Agent stop timed out");
                self.intentions.unregister_agent(agent_id).await;
                Err(AgentError::ShutdownFailed("Timeout".to_string()))
            }
        }
    }

    pub async fn restart_agent(&self, agent_id: &AgentId) -> Result<(), AgentError> {
        self.stop_agent(agent_id).await?;
        Ok(())
    }

    pub async fn get_agent_state(&self, agent_id: &AgentId) -> Option<AgentState> {
        let agents = self.agents.read().await;
        if let Some(running) = agents.get(agent_id) {
            Some(*running.handle.state.read().await)
        } else {
            None
        }
    }

    pub async fn list_agents(&self) -> Vec<AgentId> {
        self.agents.read().await.keys().copied().collect()
    }

    pub async fn running_count(&self) -> usize {
        self.agents.read().await.len()
    }

    pub async fn stop_all(&self) -> Vec<Result<(), AgentError>> {
        let agent_ids: Vec<_> = self.agents.read().await.keys().copied().collect();
        let mut results = Vec::with_capacity(agent_ids.len());

        for id in agent_ids {
            results.push(self.stop_agent(&id).await);
        }

        results
    }

    pub async fn run_spawn_listener(&self) {
        let mut rx = self.spawn_rx.write().await.take();

        if let Some(ref mut receiver) = rx {
            while let Some(agent) = receiver.recv().await {
                if let Err(e) = self.start_agent(agent).await {
                    error!(error = %e, "Failed to spawn agent");
                }
            }
        }
    }
}

async fn run_agent_loop(
    agent: &mut dyn Agent,
    event_bus: EventBusHandle,
    graph: GraphStoreHandle,
    intentions: IntentionManager,
    spawn_tx: mpsc::Sender<Box<dyn Agent>>,
    mut stop_rx: oneshot::Receiver<()>,
    tick_interval: Duration,
    state: Arc<RwLock<AgentState>>,
) -> Result<(), AgentError> {
    let mut ctx = AgentContext::new(event_bus.clone(), graph, intentions, spawn_tx);

    agent.init(&mut ctx).await?;
    *state.write().await = AgentState::Running;

    debug!(agent_id = %agent.id(), "Agent initialized and running");

    let mut event_rx = event_bus.subscribe();
    let mut tick_interval = tokio::time::interval(tick_interval);

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                *state.write().await = AgentState::Stopping;
                agent.shutdown(&mut ctx).await?;
                break;
            }

            _ = tick_interval.tick() => {
                if let Err(e) = agent.tick(&mut ctx).await {
                    warn!(agent_id = %agent.id(), error = %e, "Tick error");
                }
            }

            event_result = event_rx.recv() => {
                match event_result {
                    Ok(event) => {
                        if let Err(e) = agent.on_event(&event, &mut ctx).await {
                            warn!(agent_id = %agent.id(), error = %e, "Event handling error");
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(agent_id = %agent.id(), skipped = n, "Event receiver lagged");
                    }
                }
            }
        }
    }

    Ok(())
}
