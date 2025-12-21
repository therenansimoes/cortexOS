//! Multi-hop routing for signal-based mesh networks.
//!
//! This module provides routing capabilities for CortexOS's signal layer,
//! enabling messages to be forwarded across multiple physical nodes in a
//! mesh network topology.
//!
//! # Overview
//!
//! Multi-hop communication allows nodes to send signals to destinations
//! beyond their direct communication range by relaying through intermediate
//! nodes. This is crucial for:
//!
//! - **Extended range**: Reach distant nodes via relay chains
//! - **Mesh networking**: Self-organizing, resilient topologies  
//! - **Swarm coordination**: Enable cooperation among distributed devices
//! - **Path diversity**: Multiple routes for reliability
//!
//! # Key Components
//!
//! - [`Route`]: A path from source to destination through intermediate hops
//! - [`RouteHop`]: Single hop in a route (node + channel + metrics)
//! - [`MultiHopRouter`]: Manages routing tables and message forwarding
//! - [`MultiHopMessage`]: Message with routing metadata (TTL, hop count, path)
//! - [`RouteDiscoveryRequest`]/[`RouteDiscoveryReply`]: Dynamic route finding
//!
//! # Example
//!
//! ```rust
//! use cortex_core::NodeId;
//! use cortex_signal::routing::{Route, RouteHop, MultiHopRouter, MultiHopMessage};
//! use cortex_signal::{Channel, Signal, SignalPattern};
//!
//! # async fn example() {
//! // Create a 3-node route: A -> B -> C
//! let node_a = NodeId::generate();
//! let node_b = NodeId::generate();
//! let node_c = NodeId::generate();
//!
//! let route = Route::new(
//!     node_a,
//!     node_c,
//!     vec![
//!         RouteHop::new(node_b, Channel::Ble).with_latency(1000),
//!         RouteHop::new(node_c, Channel::Light).with_latency(1500),
//!     ],
//! );
//!
//! // Create router and install route
//! let router = MultiHopRouter::new(node_a);
//! router.add_route(route).await;
//!
//! // Create and route a message
//! # let signal = Signal::new(
//! #     cortex_core::SymbolId::from_bytes(b"TEST"),
//! #     SignalPattern::empty(),
//! #     Channel::Ble
//! # );
//! let message = MultiHopMessage::new(node_a, node_c, signal);
//! let next_hop = router.route_message(&message).await.ok();
//! # }
//! ```
//!
//! # Route Quality
//!
//! Routes are scored based on:
//! - **Success rate** (60%): Delivery success vs. failures
//! - **Hop count** (30%): Shorter paths preferred
//! - **Age** (10%): Fresher routes preferred
//!
//! Quality score ranges from 0.0 (worst) to 1.0 (best).
//!
//! # TTL and Loop Prevention
//!
//! Messages have a TTL (Time To Live) that decrements at each hop.
//! When TTL reaches 0, the message is dropped. This prevents infinite
//! loops in case of routing errors or topology changes.
//!
//! Default TTL: 7 hops  
//! Maximum hop count: 15 hops
//!
//! # Route Discovery
//!
//! Dynamic route discovery allows nodes to find paths to destinations:
//!
//! ```rust
//! # use cortex_core::NodeId;
//! # use cortex_signal::routing::MultiHopRouter;
//! # async fn example() {
//! # let node_a = NodeId::generate();
//! # let node_d = NodeId::generate();
//! let router = MultiHopRouter::new(node_a);
//! let request = router.discover_route(node_d).await;
//!
//! // Request propagates through network
//! // Nodes along the path respond with RouteDiscoveryReply
//! // containing the discovered path
//! # }
//! ```

use cortex_core::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::SignalError;
use crate::signal::{Channel, Signal};

const DEFAULT_MAX_HOPS: u8 = 7;
const MAX_HOP_LIMIT: u8 = 15;
const ROUTE_EXPIRY: Duration = Duration::from_secs(300);

// Route quality scoring weights
const QUALITY_SUCCESS_WEIGHT: f32 = 0.6;
const QUALITY_HOP_WEIGHT: f32 = 0.3;
const QUALITY_AGE_WEIGHT: f32 = 0.1;

// Default values for route quality calculations
const HOP_PENALTY_FACTOR: f32 = 0.2;
const DEFAULT_SUCCESS_RATE: f32 = 0.5;

// Protocol versioning
const ROUTING_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteId([u8; 16]);

impl RouteId {
    pub fn new() -> Self {
        let mut bytes = [0u8; 16];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Default for RouteId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHop {
    pub node_id: NodeId,
    pub channel: Channel,
    pub latency_us: Option<u32>,
}

impl RouteHop {
    pub fn new(node_id: NodeId, channel: Channel) -> Self {
        Self {
            node_id,
            channel,
            latency_us: None,
        }
    }

    pub fn with_latency(mut self, latency_us: u32) -> Self {
        self.latency_us = Some(latency_us);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub id: RouteId,
    pub source: NodeId,
    pub destination: NodeId,
    pub hops: Vec<RouteHop>,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
    #[serde(skip, default = "Instant::now")]
    pub last_used: Instant,
    pub success_count: u32,
    pub failure_count: u32,
}

impl Route {
    pub fn new(source: NodeId, destination: NodeId, hops: Vec<RouteHop>) -> Self {
        let now = Instant::now();
        Self {
            id: RouteId::new(),
            source,
            destination,
            hops,
            created_at: now,
            last_used: now,
            success_count: 0,
            failure_count: 0,
        }
    }
    
    pub fn with_id(mut self, id: RouteId) -> Self {
        self.id = id;
        self
    }

    pub fn hop_count(&self) -> usize {
        self.hops.len()
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > ROUTE_EXPIRY
    }

    pub fn total_latency_us(&self) -> Option<u32> {
        self.hops
            .iter()
            .try_fold(0u32, |acc, hop| {
                hop.latency_us.map(|l| acc + l)
            })
    }

    pub fn quality_score(&self) -> f32 {
        if self.is_expired() {
            return 0.0;
        }

        let success_rate = self.calculate_success_rate();
        let hop_penalty = self.calculate_hop_penalty();
        let age_penalty = self.calculate_age_penalty();

        success_rate * QUALITY_SUCCESS_WEIGHT 
            + hop_penalty * QUALITY_HOP_WEIGHT 
            + age_penalty * QUALITY_AGE_WEIGHT
    }

    fn calculate_success_rate(&self) -> f32 {
        if self.success_count + self.failure_count > 0 {
            self.success_count as f32 / (self.success_count + self.failure_count) as f32
        } else {
            DEFAULT_SUCCESS_RATE
        }
    }

    fn calculate_hop_penalty(&self) -> f32 {
        1.0 / (1.0 + self.hop_count() as f32 * HOP_PENALTY_FACTOR)
    }

    fn calculate_age_penalty(&self) -> f32 {
        let age_ratio = self.created_at.elapsed().as_secs() as f32 / ROUTE_EXPIRY.as_secs() as f32;
        1.0 - age_ratio.min(1.0)
    }

    pub fn mark_used(&mut self) {
        self.last_used = Instant::now();
    }

    pub fn mark_success(&mut self) {
        self.success_count = self.success_count.saturating_add(1);
        self.mark_used();
    }

    pub fn mark_failure(&mut self) {
        self.failure_count = self.failure_count.saturating_add(1);
        self.mark_used();
    }

    pub fn next_hop(&self, current_node: &NodeId) -> Option<&RouteHop> {
        if current_node == &self.source {
            return self.hops.first();
        }

        for (i, hop) in self.hops.iter().enumerate() {
            if &hop.node_id == current_node && i + 1 < self.hops.len() {
                return Some(&self.hops[i + 1]);
            }
        }

        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiHopMessage {
    pub protocol_version: u32,
    pub route_id: RouteId,
    pub source: NodeId,
    pub destination: NodeId,
    pub ttl: u8,
    pub hop_count: u8,
    pub signal: Signal,
    pub route_record: Vec<NodeId>,
}

impl MultiHopMessage {
    pub fn new(source: NodeId, destination: NodeId, signal: Signal) -> Self {
        Self {
            protocol_version: ROUTING_PROTOCOL_VERSION,
            route_id: RouteId::new(),
            source,
            destination,
            ttl: DEFAULT_MAX_HOPS,
            hop_count: 0,
            signal,
            route_record: vec![source],
        }
    }

    pub fn with_ttl(mut self, ttl: u8) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn can_forward(&self) -> bool {
        self.ttl > 0 && self.hop_count < MAX_HOP_LIMIT
    }

    pub fn forward(&mut self, current_node: NodeId) -> Result<(), SignalError> {
        if !self.can_forward() {
            // Provide accurate error messages depending on which forwarding constraint failed
            if self.ttl == 0 && self.hop_count >= MAX_HOP_LIMIT {
                return Err(SignalError::InvalidPattern(
                    "Cannot forward: TTL expired and max hop limit reached".to_string(),
                ));
            } else if self.ttl == 0 {
                return Err(SignalError::InvalidPattern(
                    "Cannot forward: TTL expired".to_string(),
                ));
            } else {
                return Err(SignalError::InvalidPattern(
                    "Cannot forward: max hop limit reached".to_string(),
                ));
            }
        }

        self.ttl = self.ttl.saturating_sub(1);
        self.hop_count = self.hop_count.saturating_add(1);
        self.route_record.push(current_node);

        Ok(())
    }

    pub fn has_visited(&self, node_id: &NodeId) -> bool {
        self.route_record.contains(node_id)
    }
}

#[derive(Debug, Clone)]
pub struct RouteDiscoveryRequest {
    pub protocol_version: u32,
    pub id: RouteId,
    pub source: NodeId,
    pub destination: NodeId,
    pub ttl: u8,
    pub hop_count: u8,
    pub path: Vec<NodeId>,
    pub created_at: Instant,
}

impl RouteDiscoveryRequest {
    pub fn new(source: NodeId, destination: NodeId) -> Self {
        Self {
            protocol_version: ROUTING_PROTOCOL_VERSION,
            id: RouteId::new(),
            source,
            destination,
            ttl: DEFAULT_MAX_HOPS,
            hop_count: 0,
            path: vec![source],
            created_at: Instant::now(),
        }
    }

    pub fn can_forward(&self) -> bool {
        self.ttl > 0 && self.hop_count < MAX_HOP_LIMIT
    }

    pub fn forward(&mut self, node_id: NodeId) {
        self.ttl = self.ttl.saturating_sub(1);
        self.hop_count = self.hop_count.saturating_add(1);
        self.path.push(node_id);
    }

    pub fn has_visited(&self, node_id: &NodeId) -> bool {
        self.path.contains(node_id)
    }
}

#[derive(Debug, Clone)]
pub struct RouteDiscoveryReply {
    pub protocol_version: u32,
    pub request_id: RouteId,
    pub source: NodeId,
    pub destination: NodeId,
    pub path: Vec<RouteHop>,
    pub total_latency_us: u32,
}

pub struct RoutingTable {
    routes: HashMap<(NodeId, NodeId), Vec<Route>>,
    by_id: HashMap<RouteId, Route>,
    pending_discoveries: HashMap<RouteId, RouteDiscoveryRequest>,
    max_routes_per_pair: usize,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            by_id: HashMap::new(),
            pending_discoveries: HashMap::new(),
            max_routes_per_pair: 3,
        }
    }

    pub fn with_max_routes(mut self, max: usize) -> Self {
        self.max_routes_per_pair = max;
        self
    }

    pub fn add_route(&mut self, route: Route) {
        let key = (route.source, route.destination);
        let route_id = route.id.clone();
        
        // Insert into routes vector
        let routes = self.routes.entry(key).or_insert_with(Vec::new);
        routes.push(route);

        routes.sort_by(|a, b| {
            b.quality_score().partial_cmp(&a.quality_score()).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Trim excess routes and remove from by_id
        if routes.len() > self.max_routes_per_pair {
            if let Some(removed) = routes.pop() {
                self.by_id.remove(&removed.id);
            }
        }
        
        // Update by_id with reference to route in vector
        if let Some(route_ref) = routes.iter().find(|r| r.id == route_id) {
            self.by_id.insert(route_id, route_ref.clone());
        }
    }

    pub fn get_best_route(&mut self, source: &NodeId, destination: &NodeId) -> Option<&mut Route> {
        let key = (*source, *destination);
        let routes = self.routes.get_mut(&key)?;
        
        let mut best_idx = None;
        let mut best_score = 0.0f32;
        
        for (idx, route) in routes.iter().enumerate() {
            if route.is_expired() {
                continue;
            }
            let score = route.quality_score();
            if best_idx.is_none() || score > best_score {
                best_idx = Some(idx);
                best_score = score;
            }
        }
        
        best_idx.and_then(|idx| routes.get_mut(idx))
    }

    pub fn get_route_by_id(&mut self, id: &RouteId) -> Option<&mut Route> {
        self.by_id.get_mut(id)
    }

    pub fn prune_expired(&mut self) -> usize {
        let mut removed = 0;

        let expired_route_ids: Vec<RouteId> = self.by_id
            .iter()
            .filter(|(_, r)| r.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired_route_ids {
            self.by_id.remove(id);
        }

        self.routes.retain(|_, routes| {
            let before = routes.len();
            routes.retain(|r| !r.is_expired());
            removed += before - routes.len();
            !routes.is_empty()
        });

        if removed > 0 {
            debug!("Pruned {} expired routes", removed);
        }

        removed
    }

    pub fn start_discovery(&mut self, request: RouteDiscoveryRequest) {
        self.pending_discoveries.insert(request.id.clone(), request);
    }

    pub fn get_discovery(&self, id: &RouteId) -> Option<&RouteDiscoveryRequest> {
        self.pending_discoveries.get(id)
    }

    pub fn complete_discovery(&mut self, id: &RouteId) -> Option<RouteDiscoveryRequest> {
        self.pending_discoveries.remove(id)
    }

    pub fn route_count(&self) -> usize {
        self.by_id.len()
    }

    pub fn all_routes(&self) -> Vec<&Route> {
        self.by_id.values().collect()
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MultiHopRouter {
    node_id: NodeId,
    routing_table: Arc<RwLock<RoutingTable>>,
    message_queue: Arc<RwLock<VecDeque<MultiHopMessage>>>,
    max_queue_size: usize,
}

impl MultiHopRouter {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            routing_table: Arc::new(RwLock::new(RoutingTable::new())),
            message_queue: Arc::new(RwLock::new(VecDeque::new())),
            max_queue_size: 100,
        }
    }

    pub fn with_max_queue_size(mut self, size: usize) -> Self {
        self.max_queue_size = size;
        self
    }

    pub async fn route_message(&self, message: &MultiHopMessage) -> Result<Option<RouteHop>, SignalError> {
        let mut table = self.routing_table.write().await;

        if message.destination == self.node_id {
            info!("Message reached destination");
            return Ok(None);
        }

        if let Some(route) = table.get_route_by_id(&message.route_id) {
            if let Some(next_hop) = route.next_hop(&self.node_id) {
                return Ok(Some(next_hop.clone()));
            }
        }

        if let Some(route) = table.get_best_route(&self.node_id, &message.destination) {
            if let Some(next_hop) = route.next_hop(&self.node_id) {
                return Ok(Some(next_hop.clone()));
            }
        }

        warn!("No route found for message to {:?}", message.destination);
        Err(SignalError::InvalidPattern("No route available".to_string()))
    }

    pub async fn queue_message(&self, message: MultiHopMessage) -> Result<(), SignalError> {
        let mut queue = self.message_queue.write().await;
        
        if queue.len() >= self.max_queue_size {
            queue.pop_front();
            warn!("Message queue full, dropping oldest message");
        }

        queue.push_back(message);
        Ok(())
    }

    pub async fn dequeue_message(&self) -> Option<MultiHopMessage> {
        let mut queue = self.message_queue.write().await;
        queue.pop_front()
    }

    pub async fn add_route(&self, route: Route) {
        let mut table = self.routing_table.write().await;
        info!(
            "Adding route from {:?} to {:?} with {} hops",
            route.source,
            route.destination,
            route.hop_count()
        );
        table.add_route(route);
    }

    pub async fn discover_route(&self, destination: NodeId) -> RouteDiscoveryRequest {
        let request = RouteDiscoveryRequest::new(self.node_id, destination);
        let mut table = self.routing_table.write().await;
        table.start_discovery(request.clone());
        request
    }

    pub async fn handle_discovery_request(&self, request: &mut RouteDiscoveryRequest, channel: Channel) -> Option<RouteDiscoveryReply> {
        if request.destination == self.node_id {
            let hops: Vec<RouteHop> = request.path.iter()
                .zip(request.path.iter().skip(1))
                .enumerate()
                .map(|(idx, (_, next))| {
                    // Use the provided channel for discovered routes
                    // In a real implementation, this would query node capabilities
                    RouteHop::new(*next, channel.clone()).with_latency(1000 * (idx + 1) as u32)
                })
                .collect();

            // Calculate total latency from all hops
            let total_latency_us = hops.iter()
                .filter_map(|hop| hop.latency_us)
                .sum();

            return Some(RouteDiscoveryReply {
                protocol_version: ROUTING_PROTOCOL_VERSION,
                request_id: request.id.clone(),
                source: request.source,
                destination: self.node_id,
                path: hops,
                total_latency_us,
            });
        }

        if request.has_visited(&self.node_id) || !request.can_forward() {
            return None;
        }

        request.forward(self.node_id);
        None
    }

    pub async fn handle_discovery_reply(&self, reply: &RouteDiscoveryReply) {
        let mut table = self.routing_table.write().await;
        
        if let Some(_request) = table.get_discovery(&reply.request_id) {
            let route = Route::new(reply.source, reply.destination, reply.path.clone());
            table.add_route(route);
            table.complete_discovery(&reply.request_id);
            info!("Route discovery completed: {:?} -> {:?}", reply.source, reply.destination);
        }
    }

    pub async fn prune_routes(&self) -> usize {
        let mut table = self.routing_table.write().await;
        table.prune_expired()
    }

    pub async fn route_count(&self) -> usize {
        let table = self.routing_table.read().await;
        table.route_count()
    }

    pub async fn queue_size(&self) -> usize {
        let queue = self.message_queue.read().await;
        queue.len()
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::SymbolId;
    use crate::signal::SignalPattern;

    fn create_test_signal() -> Signal {
        Signal::new(
            SymbolId::from_bytes(b"TEST"),
            SignalPattern::empty(),
            Channel::Ble,
        )
    }

    #[test]
    fn test_route_creation() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let node3 = NodeId::generate();

        let hops = vec![
            RouteHop::new(node2, Channel::Ble).with_latency(1000),
            RouteHop::new(node3, Channel::Ble).with_latency(1500),
        ];

        let route = Route::new(node1, node3, hops);
        assert_eq!(route.hop_count(), 2);
        assert_eq!(route.total_latency_us(), Some(2500));
    }

    #[test]
    fn test_route_quality_score() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let hops = vec![RouteHop::new(node2, Channel::Ble)];

        let mut route = Route::new(node1, node2, hops);
        route.mark_success();
        route.mark_success();
        route.mark_failure();

        let score = route.quality_score();
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn test_multi_hop_message() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let signal = create_test_signal();

        let mut message = MultiHopMessage::new(node1, node2, signal);
        assert!(message.can_forward());
        assert_eq!(message.hop_count, 0);

        message.forward(NodeId::generate()).unwrap();
        assert_eq!(message.hop_count, 1);
        assert_eq!(message.ttl, DEFAULT_MAX_HOPS - 1);
    }

    #[test]
    fn test_multi_hop_message_ttl_expiry() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let signal = create_test_signal();

        let mut message = MultiHopMessage::new(node1, node2, signal).with_ttl(1);
        message.forward(NodeId::generate()).unwrap();
        assert!(!message.can_forward());
    }

    #[test]
    fn test_routing_table() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let node3 = NodeId::generate();

        let mut table = RoutingTable::new();
        let hops = vec![RouteHop::new(node2, Channel::Ble)];
        let route = Route::new(node1, node3, hops);

        table.add_route(route);
        assert_eq!(table.route_count(), 1);

        let found = table.get_best_route(&node1, &node3);
        assert!(found.is_some());
    }

    #[test]
    fn test_routing_table_max_routes() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();

        let mut table = RoutingTable::new().with_max_routes(2);

        for _ in 0..5 {
            let hops = vec![RouteHop::new(NodeId::generate(), Channel::Ble)];
            let route = Route::new(node1, node2, hops);
            table.add_route(route);
        }

        let routes = table.routes.get(&(node1, node2)).unwrap();
        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn test_route_discovery_request() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();

        let mut request = RouteDiscoveryRequest::new(node1, node2);
        assert!(request.can_forward());
        assert_eq!(request.hop_count, 0);

        request.forward(NodeId::generate());
        assert_eq!(request.hop_count, 1);
        assert_eq!(request.path.len(), 2);
    }

    #[tokio::test]
    async fn test_multi_hop_router() {
        let node1 = NodeId::generate();
        let router = MultiHopRouter::new(node1);

        assert_eq!(router.node_id(), node1);
        assert_eq!(router.route_count().await, 0);
    }

    #[tokio::test]
    async fn test_router_message_queue() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let router = MultiHopRouter::new(node1);

        let signal = create_test_signal();
        let message = MultiHopMessage::new(node1, node2, signal);

        router.queue_message(message.clone()).await.unwrap();
        assert_eq!(router.queue_size().await, 1);

        let dequeued = router.dequeue_message().await;
        assert!(dequeued.is_some());
        assert_eq!(router.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_route_discovery() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let router = MultiHopRouter::new(node1);

        let request = router.discover_route(node2).await;
        assert_eq!(request.source, node1);
        assert_eq!(request.destination, node2);
    }

    #[tokio::test]
    async fn test_handle_discovery_request_at_destination() {
        let node_dest = NodeId::generate();
        let node_src = NodeId::generate();
        let router = MultiHopRouter::new(node_dest);

        let mut request = RouteDiscoveryRequest::new(node_src, node_dest);
        request.path.push(NodeId::generate()); // Add intermediate node
        
        let reply = router.handle_discovery_request(&mut request, Channel::Ble).await;
        
        assert!(reply.is_some());
        let reply = reply.unwrap();
        assert_eq!(reply.source, node_src);
        assert_eq!(reply.destination, node_dest);
        assert_eq!(reply.protocol_version, ROUTING_PROTOCOL_VERSION);
        assert!(reply.total_latency_us > 0); // Should have calculated latency
    }

    #[tokio::test]
    async fn test_handle_discovery_request_at_intermediate() {
        let node_intermediate = NodeId::generate();
        let node_src = NodeId::generate();
        let node_dest = NodeId::generate();
        let router = MultiHopRouter::new(node_intermediate);

        let mut request = RouteDiscoveryRequest::new(node_src, node_dest);
        
        let reply = router.handle_discovery_request(&mut request, Channel::Ble).await;
        
        assert!(reply.is_none()); // Intermediate nodes don't reply
        assert_eq!(request.path.len(), 2); // Path should include intermediate node
        assert_eq!(request.path[1], node_intermediate);
    }

    #[tokio::test]
    async fn test_handle_discovery_reply() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let node3 = NodeId::generate();
        let router = MultiHopRouter::new(node1);

        // Start a discovery
        let request = router.discover_route(node3).await;
        let request_id = request.id.clone();

        // Simulate receiving a reply
        let hops = vec![
            RouteHop::new(node2, Channel::Ble).with_latency(1000),
            RouteHop::new(node3, Channel::Light).with_latency(1500),
        ];
        let reply = RouteDiscoveryReply {
            protocol_version: ROUTING_PROTOCOL_VERSION,
            request_id,
            source: node1,
            destination: node3,
            path: hops,
            total_latency_us: 2500,
        };

        router.handle_discovery_reply(&reply).await;

        // Verify route was added
        assert_eq!(router.route_count().await, 1);
    }

    #[test]
    fn test_protocol_versioning() {
        let node1 = NodeId::generate();
        let node2 = NodeId::generate();
        let signal = create_test_signal();

        let message = MultiHopMessage::new(node1, node2, signal);
        assert_eq!(message.protocol_version, ROUTING_PROTOCOL_VERSION);

        let request = RouteDiscoveryRequest::new(node1, node2);
        assert_eq!(request.protocol_version, ROUTING_PROTOCOL_VERSION);
    }
}
