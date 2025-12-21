use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use cortex_core::NodeId;

use crate::error::RoutingError;
use crate::signal::Channel;

const MAX_HOP_COUNT: u8 = 10;
const ROUTE_EXPIRY: Duration = Duration::from_secs(300);
const MAX_ROUTES_PER_DESTINATION: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteId {
    source: NodeId,
    destination: NodeId,
    sequence: u32,
}

impl RouteId {
    pub fn new(source: NodeId, destination: NodeId, sequence: u32) -> Self {
        Self {
            source,
            destination,
            sequence,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteQuality {
    pub latency_us: u64,
    pub reliability: f32,
    pub signal_strength: f32,
    pub hop_count: u8,
    pub channel: Channel,
}

impl RouteQuality {
    pub fn new(channel: Channel) -> Self {
        Self {
            latency_us: 0,
            reliability: 1.0,
            signal_strength: 1.0,
            hop_count: 0,
            channel,
        }
    }

    pub fn score(&self) -> f32 {
        let latency_score = 1.0 - (self.latency_us as f32 / 1_000_000.0).clamp(0.0, 1.0);
        let hop_penalty = (1.0 - (self.hop_count as f32 / MAX_HOP_COUNT as f32)).max(0.1);

        (self.reliability * 0.4)
            + (latency_score * 0.3)
            + (self.signal_strength * 0.2)
            + (hop_penalty * 0.1)
    }

    pub fn degrade_for_hop(&self) -> Self {
        Self {
            latency_us: self.latency_us + 50_000,
            reliability: self.reliability * 0.95,
            signal_strength: self.signal_strength * 0.90,
            hop_count: self.hop_count.saturating_add(1),
            channel: self.channel.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Route {
    pub path: Vec<NodeId>,
    pub quality: RouteQuality,
    pub discovered_at: Instant,
    pub last_used: Instant,
}

impl Route {
    pub fn new(path: Vec<NodeId>, quality: RouteQuality) -> Self {
        let now = Instant::now();
        Self {
            path,
            quality,
            discovered_at: now,
            last_used: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.discovered_at.elapsed() > ROUTE_EXPIRY
    }

    pub fn hop_count(&self) -> usize {
        self.path.len().saturating_sub(1)
    }

    pub fn next_hop(&self, current: &NodeId) -> Option<NodeId> {
        self.path
            .iter()
            .position(|n| n == current)
            .and_then(|idx| self.path.get(idx + 1))
            .copied()
    }

    pub fn contains(&self, node: &NodeId) -> bool {
        self.path.contains(node)
    }
}

#[derive(Debug)]
pub struct RoutingTable {
    routes: HashMap<NodeId, Vec<Route>>,
    local_node: NodeId,
}

impl RoutingTable {
    pub fn new(local_node: NodeId) -> Self {
        Self {
            routes: HashMap::new(),
            local_node,
        }
    }

    pub fn add_route(&mut self, destination: NodeId, route: Route) {
        if route.contains(&self.local_node) && route.path[0] != self.local_node {
            warn!("Rejecting route that would create a loop");
            return;
        }

        let routes = self.routes.entry(destination).or_default();

        routes.retain(|r| !r.is_expired());

        routes.push(route);
        routes.sort_by(|a, b| b.quality.score().partial_cmp(&a.quality.score()).unwrap());
        routes.truncate(MAX_ROUTES_PER_DESTINATION);

        debug!(
            destination = %destination,
            route_count = routes.len(),
            "Updated routing table"
        );
    }

    pub fn best_route(&self, destination: &NodeId) -> Option<&Route> {
        self.routes
            .get(destination)
            .and_then(|routes| routes.iter().find(|r| !r.is_expired()))
    }

    pub fn all_routes(&self, destination: &NodeId) -> Vec<&Route> {
        self.routes
            .get(destination)
            .map(|routes| routes.iter().filter(|r| !r.is_expired()).collect())
            .unwrap_or_default()
    }

    pub fn prune_expired(&mut self) -> usize {
        let mut pruned = 0;
        for routes in self.routes.values_mut() {
            let before = routes.len();
            routes.retain(|r| !r.is_expired());
            pruned += before - routes.len();
        }

        self.routes.retain(|_, routes| !routes.is_empty());

        if pruned > 0 {
            debug!(pruned_count = pruned, "Pruned expired routes");
        }
        pruned
    }

    pub fn remove_route(&mut self, destination: &NodeId, path: &[NodeId]) {
        if let Some(routes) = self.routes.get_mut(destination) {
            routes.retain(|r| r.path != path);
            if routes.is_empty() {
                self.routes.remove(destination);
            }
        }
    }

    pub fn known_destinations(&self) -> Vec<NodeId> {
        self.routes.keys().copied().collect()
    }
}

#[derive(Debug)]
pub struct RouteDiscovery {
    pending_requests: HashMap<RouteId, Instant>,
    seen_requests: HashSet<RouteId>,
}

impl RouteDiscovery {
    pub fn new() -> Self {
        Self {
            pending_requests: HashMap::new(),
            seen_requests: HashSet::new(),
        }
    }

    pub fn initiate_discovery(
        &mut self,
        source: NodeId,
        destination: NodeId,
        sequence: u32,
    ) -> RouteId {
        let route_id = RouteId::new(source, destination, sequence);
        self.pending_requests
            .insert(route_id.clone(), Instant::now());
        info!(
            source = %source,
            destination = %destination,
            sequence = sequence,
            "Initiated route discovery"
        );
        route_id
    }

    pub fn should_forward(&mut self, route_id: &RouteId) -> bool {
        if self.seen_requests.contains(route_id) {
            return false;
        }

        self.seen_requests.insert(route_id.clone());
        true
    }

    pub fn complete_discovery(&mut self, route_id: &RouteId) {
        self.pending_requests.remove(route_id);
    }

    pub fn prune_old_requests(&mut self, timeout: Duration) {
        let now = Instant::now();
        self.pending_requests
            .retain(|_, timestamp| now.duration_since(*timestamp) < timeout);

        if self.seen_requests.len() > 1000 {
            self.seen_requests.clear();
        }
    }
}

impl Default for RouteDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MultiHopRouter {
    local_node: NodeId,
    routing_table: Arc<RwLock<RoutingTable>>,
    route_discovery: Arc<RwLock<RouteDiscovery>>,
    sequence_counter: Arc<RwLock<u32>>,
}

impl MultiHopRouter {
    pub fn new(local_node: NodeId) -> Self {
        Self {
            local_node,
            routing_table: Arc::new(RwLock::new(RoutingTable::new(local_node))),
            route_discovery: Arc::new(RwLock::new(RouteDiscovery::new())),
            sequence_counter: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn find_route(&self, destination: &NodeId) -> Result<Route, RoutingError> {
        let table = self.routing_table.read().await;

        if let Some(route) = table.best_route(destination) {
            return Ok(route.clone());
        }

        drop(table);

        Err(RoutingError::NoRouteAvailable)
    }

    pub async fn add_discovered_route(
        &self,
        destination: NodeId,
        path: Vec<NodeId>,
        quality: RouteQuality,
    ) {
        let route = Route::new(path, quality);
        let mut table = self.routing_table.write().await;
        table.add_route(destination, route);
    }

    pub async fn next_hop(&self, destination: &NodeId) -> Result<NodeId, RoutingError> {
        let route = self.find_route(destination).await?;

        route
            .next_hop(&self.local_node)
            .ok_or(RoutingError::InvalidRoute)
    }

    pub async fn initiate_discovery(&self, destination: NodeId) -> RouteId {
        let mut seq = self.sequence_counter.write().await;
        *seq = seq.wrapping_add(1);
        let sequence = *seq;
        drop(seq);

        let mut discovery = self.route_discovery.write().await;
        discovery.initiate_discovery(self.local_node, destination, sequence)
    }

    pub async fn should_forward_request(&self, route_id: &RouteId) -> bool {
        let mut discovery = self.route_discovery.write().await;
        discovery.should_forward(route_id)
    }

    pub async fn prune_stale_data(&self) {
        let mut table = self.routing_table.write().await;
        table.prune_expired();

        let mut discovery = self.route_discovery.write().await;
        discovery.prune_old_requests(Duration::from_secs(30));
    }

    pub async fn remove_node_from_routes(&self, node: &NodeId) {
        let mut table = self.routing_table.write().await;
        let destinations: Vec<NodeId> = table.known_destinations();

        for dest in destinations {
            let routes_to_remove: Vec<Vec<NodeId>> = table
                .all_routes(&dest)
                .iter()
                .filter(|r| r.contains(node))
                .map(|r| r.path.clone())
                .collect();

            for path in routes_to_remove {
                table.remove_route(&dest, &path);
            }
        }
    }

    pub async fn get_route_stats(&self) -> HashMap<NodeId, usize> {
        let table = self.routing_table.read().await;
        table
            .known_destinations()
            .into_iter()
            .map(|dest| (dest, table.all_routes(&dest).len()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(n: u8) -> NodeId {
        let mut bytes = [0u8; 16];
        bytes[0] = n;
        NodeId::from_bytes(bytes)
    }

    #[test]
    fn test_route_quality_score() {
        let quality = RouteQuality {
            latency_us: 10_000,
            reliability: 0.95,
            signal_strength: 0.9,
            hop_count: 2,
            channel: Channel::Ble,
        };

        let score = quality.score();
        assert!(score > 0.5 && score < 1.0);
    }

    #[test]
    fn test_route_quality_degradation() {
        let quality = RouteQuality::new(Channel::Light);
        let degraded = quality.degrade_for_hop();

        assert_eq!(degraded.hop_count, 1);
        assert!(degraded.reliability < quality.reliability);
        assert!(degraded.signal_strength < quality.signal_strength);
        assert!(degraded.latency_us > quality.latency_us);
    }

    #[test]
    fn test_route_next_hop() {
        let node1 = test_node_id(1);
        let node2 = test_node_id(2);
        let node3 = test_node_id(3);

        let path = vec![node1, node2, node3];
        let route = Route::new(path, RouteQuality::new(Channel::Ble));

        assert_eq!(route.next_hop(&node1), Some(node2));
        assert_eq!(route.next_hop(&node2), Some(node3));
        assert_eq!(route.next_hop(&node3), None);
    }

    #[test]
    fn test_routing_table_add_route() {
        let local = test_node_id(1);
        let dest = test_node_id(3);

        let mut table = RoutingTable::new(local);

        let route = Route::new(
            vec![local, test_node_id(2), dest],
            RouteQuality::new(Channel::Ble),
        );

        table.add_route(dest, route);

        assert!(table.best_route(&dest).is_some());
    }

    #[test]
    fn test_routing_table_best_route() {
        let local = test_node_id(1);
        let dest = test_node_id(4);

        let mut table = RoutingTable::new(local);

        let mut good_quality = RouteQuality::new(Channel::Ble);
        good_quality.reliability = 0.95;
        good_quality.latency_us = 1000;

        let mut bad_quality = RouteQuality::new(Channel::Light);
        bad_quality.reliability = 0.5;
        bad_quality.latency_us = 100_000;

        table.add_route(
            dest,
            Route::new(vec![local, test_node_id(2), dest], good_quality),
        );
        table.add_route(
            dest,
            Route::new(vec![local, test_node_id(3), dest], bad_quality),
        );

        let best = table.best_route(&dest).unwrap();
        assert!(best.quality.reliability > 0.9);
    }

    #[tokio::test]
    async fn test_multi_hop_router_find_route() {
        let local = test_node_id(1);
        let dest = test_node_id(3);

        let router = MultiHopRouter::new(local);

        let path = vec![local, test_node_id(2), dest];
        router
            .add_discovered_route(dest, path, RouteQuality::new(Channel::Ble))
            .await;

        let route = router.find_route(&dest).await.unwrap();
        assert_eq!(route.path.len(), 3);
    }

    #[tokio::test]
    async fn test_multi_hop_router_next_hop() {
        let local = test_node_id(1);
        let intermediate = test_node_id(2);
        let dest = test_node_id(3);

        let router = MultiHopRouter::new(local);

        let path = vec![local, intermediate, dest];
        router
            .add_discovered_route(dest, path, RouteQuality::new(Channel::Ble))
            .await;

        let next = router.next_hop(&dest).await.unwrap();
        assert_eq!(next, intermediate);
    }

    #[test]
    fn test_route_discovery_forward_once() {
        let mut discovery = RouteDiscovery::new();
        let route_id = RouteId::new(test_node_id(1), test_node_id(3), 1);

        assert!(discovery.should_forward(&route_id));
        assert!(!discovery.should_forward(&route_id));
    }
}
