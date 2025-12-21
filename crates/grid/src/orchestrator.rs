use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::error::{GridError, Result};
use crate::peer::{NodeId, PeerStore};
use crate::wire::{Message, TaskStatus};
use cortex_core::event::{Event, Payload};
use cortex_core::runtime::EventBus;

const TASK_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RETRIES: u32 = 3;

#[derive(Debug, Clone)]
struct PendingTask {
    #[allow(dead_code)] // Used for debugging and future implementations
    task_id: [u8; 32],
    #[allow(dead_code)] // May be used for retry logic
    payload: Vec<u8>,
    #[allow(dead_code)] // Used for routing retries
    target_node: NodeId,
    created_at: Instant,
    retries: u32,
    last_status: TaskStatus,
}

pub struct GridOrchestrator {
    local_node_id: NodeId,
    peer_store: PeerStore,
    event_bus: Arc<EventBus>,
    pending_tasks: Arc<RwLock<HashMap<[u8; 32], PendingTask>>>,
    message_tx: Option<mpsc::Sender<(NodeId, Message)>>,
    message_rx: Option<mpsc::Receiver<(NodeId, Message)>>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

impl GridOrchestrator {
    pub fn new(local_node_id: NodeId, peer_store: PeerStore, event_bus: Arc<EventBus>) -> Self {
        let (message_tx, message_rx) = mpsc::channel(256);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Self {
            local_node_id,
            peer_store,
            event_bus,
            pending_tasks: Arc::new(RwLock::new(HashMap::new())),
            message_tx: Some(message_tx),
            message_rx: Some(message_rx),
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
        }
    }

    /// Get the message sender for sending grid messages
    pub fn message_sender(&self) -> mpsc::Sender<(NodeId, Message)> {
        self.message_tx
            .as_ref()
            .expect("Message sender already taken")
            .clone()
    }

    /// Receive a grid message from a peer
    pub async fn handle_message(&self, from: NodeId, message: Message) -> Result<()> {
        match message {
            Message::TaskRequest { task_id, payload } => {
                self.handle_task_request(from, task_id, payload).await
            }
            Message::TaskAck { task_id, status } => self.handle_task_ack(task_id, status).await,
            _ => {
                debug!("Ignoring non-task message: {:?}", message);
                Ok(())
            }
        }
    }

    async fn handle_task_request(
        &self,
        from: NodeId,
        task_id: [u8; 32],
        payload: Vec<u8>,
    ) -> Result<()> {
        info!("Received task request {} from {}", hex_id(&task_id), from);

        // Publish task as event to local event bus
        let event = Event::new(
            "grid.orchestrator",
            "grid.task.received",
            Payload::inline(payload.clone()),
        );

        if let Err(e) = self.event_bus.publish(event) {
            error!("Failed to publish task event: {}", e);
            // Send rejection
            if let Some(tx) = &self.message_tx {
                let _ = tx
                    .send((
                        from,
                        Message::TaskAck {
                            task_id,
                            status: TaskStatus::Rejected,
                        },
                    ))
                    .await;
            }
            return Err(GridError::EventBusError(e.to_string()));
        }

        // Send acceptance
        if let Some(tx) = &self.message_tx {
            let _ = tx
                .send((
                    from,
                    Message::TaskAck {
                        task_id,
                        status: TaskStatus::Accepted,
                    },
                ))
                .await;
        }

        // TODO: Actually process the task and send completion/failure later
        // For now, we just accept it

        Ok(())
    }

    async fn handle_task_ack(&self, task_id: [u8; 32], status: TaskStatus) -> Result<()> {
        info!(
            "Received task ack {} with status {:?}",
            hex_id(&task_id),
            status
        );

        let mut pending = self.pending_tasks.write().await;
        if let Some(task) = pending.get_mut(&task_id) {
            task.last_status = status;

            match status {
                TaskStatus::Accepted | TaskStatus::InProgress => {
                    debug!("Task {} is being processed", hex_id(&task_id));
                }
                TaskStatus::Completed => {
                    info!("Task {} completed successfully", hex_id(&task_id));
                    // Publish completion event
                    let event = Event::new(
                        "grid.orchestrator",
                        "grid.task.completed",
                        Payload::inline(task_id.to_vec()),
                    );
                    let _ = self.event_bus.publish(event);
                    pending.remove(&task_id);
                }
                TaskStatus::Failed | TaskStatus::Rejected => {
                    warn!("Task {} failed or rejected", hex_id(&task_id));
                    // Publish failure event
                    let event = Event::new(
                        "grid.orchestrator",
                        "grid.task.failed",
                        Payload::inline(task_id.to_vec()),
                    );
                    let _ = self.event_bus.publish(event);

                    // Check for retries
                    if task.retries < MAX_RETRIES {
                        task.retries += 1;
                        info!(
                            "Will retry task {} (attempt {}/{})",
                            hex_id(&task_id),
                            task.retries,
                            MAX_RETRIES
                        );
                    } else {
                        warn!("Task {} exceeded max retries", hex_id(&task_id));
                        pending.remove(&task_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// Delegate a task to a remote peer with the can_compute capability
    pub async fn delegate_task(&self, task_id: [u8; 32], payload: Vec<u8>) -> Result<NodeId> {
        // Find peers with compute capability
        let peers = self
            .peer_store
            .find_by_capability(|caps| caps.can_compute)
            .await;

        if peers.is_empty() {
            return Err(GridError::NoPeersAvailable);
        }

        // Select peer with lowest latency (or first if no latency info)
        let target_peer = peers
            .iter()
            .min_by_key(|p| p.latency_ms.unwrap_or(u32::MAX))
            .ok_or(GridError::NoPeersAvailable)?;

        let target_node = target_peer.node_id;

        // Store as pending task
        let task = PendingTask {
            task_id,
            payload: payload.clone(),
            target_node,
            created_at: Instant::now(),
            retries: 0,
            last_status: TaskStatus::Accepted,
        };

        self.pending_tasks.write().await.insert(task_id, task);

        // Send task request
        if let Some(tx) = &self.message_tx {
            tx.send((target_node, Message::TaskRequest { task_id, payload }))
                .await
                .map_err(|_| GridError::ChannelClosed)?;
        }

        info!(
            "Delegated task {} to peer {}",
            hex_id(&task_id),
            target_node
        );

        Ok(target_node)
    }

    /// Start the orchestrator
    pub async fn start(&mut self) -> Result<()> {
        let event_bus = Arc::clone(&self.event_bus);
        let peer_store = self.peer_store.clone();

        // Subscribe to local events that should be delegated
        let mut task_events = event_bus.subscribe("agent.task.*");

        // Take the message receiver
        let mut message_rx = self.message_rx.take().ok_or_else(|| {
            GridError::DiscoveryError("Message receiver already taken".to_string())
        })?;

        let shutdown_rx = self.shutdown_rx.take().ok_or_else(|| {
            GridError::DiscoveryError("Shutdown receiver already taken".to_string())
        })?;

        // Clone for the first task
        let pending_tasks_events = Arc::clone(&self.pending_tasks);
        let message_tx_events = self.message_tx.clone();

        // Spawn event handler task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(event) = task_events.recv() => {
                        // Check if this is a task that should be delegated
                        if event.kind().starts_with("agent.task.delegate") {
                            if let Some(payload_bytes) = event.payload.as_bytes() {
                                // Generate task ID from content
                                let task_id_hash = blake3::hash(payload_bytes);
                                let task_id: [u8; 32] = *task_id_hash.as_bytes();

                                // Find a compute peer
                                let peers = peer_store
                                    .find_by_capability(|caps| caps.can_compute)
                                    .await;

                                if let Some(peer) = peers.first() {
                                    let target_node = peer.node_id;

                                    // Store as pending
                                    let task = PendingTask {
                                        task_id,
                                        payload: payload_bytes.to_vec(),
                                        target_node,
                                        created_at: Instant::now(),
                                        retries: 0,
                                        last_status: TaskStatus::Accepted,
                                    };
                                    pending_tasks_events.write().await.insert(task_id, task);

                                    // Send task request
                                    if let Some(tx) = &message_tx_events {
                                        let _ = tx.send((
                                            target_node,
                                            Message::TaskRequest {
                                                task_id,
                                                payload: payload_bytes.to_vec(),
                                            },
                                        )).await;
                                    }

                                    info!("Auto-delegated task {} to {}", hex_id(&task_id), target_node);
                                } else {
                                    warn!("No compute peers available for delegation");
                                }
                            }
                        }
                    }
                }
            }
        });

        // Spawn timeout checker
        let pending_tasks_timeout = Arc::clone(&self.pending_tasks);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;

                let mut tasks = pending_tasks_timeout.write().await;
                let now = Instant::now();
                let mut to_remove = Vec::new();

                for (task_id, task) in tasks.iter() {
                    if now.duration_since(task.created_at) > TASK_TIMEOUT {
                        warn!("Task {} timed out", hex_id(task_id));
                        to_remove.push(*task_id);
                    }
                }

                for task_id in to_remove {
                    tasks.remove(&task_id);
                }
            }
        });

        // Spawn message receiver handler
        let pending_tasks_msg = Arc::clone(&self.pending_tasks);
        let event_bus_msg = Arc::clone(&event_bus);
        let message_tx_clone = self.message_tx.clone();

        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            loop {
                tokio::select! {
                    Some((from, message)) = message_rx.recv() => {
                        match message {
                            Message::TaskRequest { task_id, payload } => {
                                info!("Received task request {} from {}", hex_id(&task_id), from);

                                // Publish to event bus
                                let event = Event::new(
                                    "grid.orchestrator",
                                    "grid.task.received",
                                    Payload::inline(payload.clone()),
                                );

                                let accepted = event_bus_msg.publish(event).is_ok();

                                // Send response
                                if let Some(tx) = &message_tx_clone {
                                    let status = if accepted {
                                        TaskStatus::Accepted
                                    } else {
                                        TaskStatus::Rejected
                                    };
                                    let _ = tx.send((from, Message::TaskAck { task_id, status })).await;
                                }
                            }
                            Message::TaskAck { task_id, status } => {
                                info!("Received task ack {} with status {:?}", hex_id(&task_id), status);

                                let mut tasks = pending_tasks_msg.write().await;
                                if let Some(task) = tasks.get_mut(&task_id) {
                                    task.last_status = status;

                                    match status {
                                        TaskStatus::Completed => {
                                            let event = Event::new(
                                                "grid.orchestrator",
                                                "grid.task.completed",
                                                Payload::inline(task_id.to_vec()),
                                            );
                                            let _ = event_bus_msg.publish(event);
                                            tasks.remove(&task_id);
                                        }
                                        TaskStatus::Failed | TaskStatus::Rejected => {
                                            let event = Event::new(
                                                "grid.orchestrator",
                                                "grid.task.failed",
                                                Payload::inline(task_id.to_vec()),
                                            );
                                            let _ = event_bus_msg.publish(event);

                                            if task.retries < MAX_RETRIES {
                                                task.retries += 1;
                                                info!("Will retry task {} (attempt {}/{})", hex_id(&task_id), task.retries, MAX_RETRIES);
                                            } else {
                                                tasks.remove(&task_id);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {
                                debug!("Ignoring non-task message");
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Orchestrator shutting down");
                        break;
                    }
                }
            }
        });

        info!("Grid orchestrator started");
        Ok(())
    }

    /// Stop the orchestrator
    pub async fn stop(&self) -> Result<()> {
        self.shutdown_tx
            .send(())
            .await
            .map_err(|_| GridError::ChannelClosed)?;
        info!("Grid orchestrator stopped");
        Ok(())
    }

    /// Get pending task count
    pub async fn pending_count(&self) -> usize {
        self.pending_tasks.read().await.len()
    }
}

fn hex_id(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::{Capabilities, PeerInfo};
    use std::time::Duration;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let node_id = NodeId::random();
        let peer_store = PeerStore::new(Duration::from_secs(60));
        let event_bus = Arc::new(EventBus::default());

        let orchestrator = GridOrchestrator::new(node_id, peer_store, event_bus);
        assert_eq!(orchestrator.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_task_delegation() {
        let node_id = NodeId::random();
        let peer_store = PeerStore::new(Duration::from_secs(60));
        let event_bus = Arc::new(EventBus::default());

        // Add a compute peer
        let peer_id = NodeId::random();
        let mut peer = PeerInfo::new(peer_id, [0u8; 32]);
        peer.capabilities = Capabilities {
            can_compute: true,
            ..Default::default()
        };
        peer_store.insert(peer).await;

        let orchestrator = GridOrchestrator::new(node_id, peer_store, event_bus);

        let task_id = [1u8; 32];
        let payload = b"test task".to_vec();

        let result = orchestrator.delegate_task(task_id, payload).await;
        assert!(result.is_ok());
        assert_eq!(orchestrator.pending_count().await, 1);
    }
}
