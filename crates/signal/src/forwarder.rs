use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use cortex_core::NodeId;

use crate::codebook::{Codebook, StandardSymbol};
use crate::emitter::Emitter;
use crate::error::{EmitError, RoutingError};
use crate::receiver::Receiver;
use crate::routing::{MultiHopRouter, RouteQuality};
use crate::signal::Channel;

#[derive(Debug, Clone)]
pub struct ForwardedMessage {
    pub source: NodeId,
    pub destination: NodeId,
    pub payload: Vec<u8>,
    pub hop_count: u8,
    pub max_hops: u8,
}

impl ForwardedMessage {
    pub fn new(source: NodeId, destination: NodeId, payload: Vec<u8>, max_hops: u8) -> Self {
        Self {
            source,
            destination,
            payload,
            hop_count: 0,
            max_hops,
        }
    }

    pub fn can_forward(&self) -> bool {
        self.hop_count < self.max_hops
    }

    pub fn increment_hop(&mut self) {
        self.hop_count = self.hop_count.saturating_add(1);
    }
}

pub struct SignalForwarder {
    local_node: NodeId,
    router: Arc<MultiHopRouter>,
    codebook: Arc<RwLock<Codebook>>,
    emitters: Arc<RwLock<HashMap<Channel, Arc<dyn Emitter>>>>,
    pending_forwards: Arc<RwLock<Vec<ForwardedMessage>>>,
}

impl SignalForwarder {
    pub fn new(local_node: NodeId, router: Arc<MultiHopRouter>) -> Self {
        Self {
            local_node,
            router,
            codebook: Arc::new(RwLock::new(Codebook::new())),
            emitters: Arc::new(RwLock::new(HashMap::new())),
            pending_forwards: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_emitter(&self, channel: Channel, emitter: Arc<dyn Emitter>) {
        let mut emitters = self.emitters.write().await;
        info!(channel = ?channel, "Registered emitter for multi-hop forwarding");
        emitters.insert(channel, emitter);
    }

    pub async fn forward_message(&self, mut message: ForwardedMessage) -> Result<(), RoutingError> {
        if message.destination == self.local_node {
            info!(source = %message.source, "Message reached destination");
            return Ok(());
        }

        if !message.can_forward() {
            warn!(
                source = %message.source,
                destination = %message.destination,
                hop_count = message.hop_count,
                "Message exceeded max hops, dropping"
            );
            return Err(RoutingError::MaxHopsExceeded);
        }

        message.increment_hop();

        let next_hop = self.router.next_hop(&message.destination).await?;
        
        debug!(
            source = %message.source,
            destination = %message.destination,
            next_hop = %next_hop,
            hop_count = message.hop_count,
            "Forwarding message to next hop"
        );

        self.pending_forwards.write().await.push(message);
        
        Ok(())
    }

    pub async fn send_via_signal(
        &self,
        destination: NodeId,
        payload: Vec<u8>,
        preferred_channel: Option<Channel>,
    ) -> Result<(), EmitError> {
        let route = self.router.find_route(&destination).await
            .map_err(|_| EmitError::ChannelUnavailable(Channel::Ble))?;

        let channel = preferred_channel.unwrap_or(route.quality.channel);
        
        let emitters = self.emitters.read().await;
        let emitter = emitters
            .get(&channel)
            .ok_or_else(|| EmitError::ChannelUnavailable(channel.clone()))?;

        let codebook = self.codebook.read().await;
        let beacon_symbol = StandardSymbol::Beacon.to_symbol_id();
        let pattern = codebook.encode(beacon_symbol)?;
        
        info!(
            destination = %destination,
            channel = ?channel,
            payload_size = payload.len(),
            "Sending signal via multi-hop route"
        );

        emitter.emit(pattern).await?;
        
        Ok(())
    }

    pub async fn process_received_signal<R: Receiver>(
        &self,
        receiver: &R,
    ) -> Result<Option<ForwardedMessage>, RoutingError> {
        let codebook = self.codebook.read().await;
        
        match receiver.decode(&codebook).await {
            Ok(signal) => {
                debug!(
                    channel = ?receiver.channel(),
                    symbol = ?signal.symbol,
                    "Received signal on multi-hop network"
                );
                
                Ok(None)
            }
            Err(_) => {
                Ok(None)
            }
        }
    }

    pub async fn announce_route_discovery(
        &self,
        destination: NodeId,
        channel: Channel,
    ) -> Result<(), EmitError> {
        let emitters = self.emitters.read().await;
        let emitter = emitters
            .get(&channel)
            .ok_or_else(|| EmitError::ChannelUnavailable(channel.clone()))?;

        let codebook = self.codebook.read().await;
        let beacon_symbol = StandardSymbol::Beacon.to_symbol_id();
        let pattern = codebook.encode(beacon_symbol)?;

        info!(
            destination = %destination,
            channel = ?channel,
            "Broadcasting route discovery"
        );

        emitter.emit(pattern).await?;
        
        Ok(())
    }

    pub async fn update_route_from_signal(
        &self,
        source: NodeId,
        path: Vec<NodeId>,
        quality: RouteQuality,
    ) {
        self.router.add_discovered_route(source, path, quality).await;
    }

    pub async fn pending_forward_count(&self) -> usize {
        self.pending_forwards.read().await.len()
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
    fn test_forwarded_message_creation() {
        let source = test_node_id(1);
        let dest = test_node_id(3);
        let payload = vec![1, 2, 3, 4];
        
        let msg = ForwardedMessage::new(source, dest, payload.clone(), 5);
        
        assert_eq!(msg.source, source);
        assert_eq!(msg.destination, dest);
        assert_eq!(msg.payload, payload);
        assert_eq!(msg.hop_count, 0);
        assert_eq!(msg.max_hops, 5);
        assert!(msg.can_forward());
    }

    #[test]
    fn test_forwarded_message_hop_limit() {
        let source = test_node_id(1);
        let dest = test_node_id(3);
        let mut msg = ForwardedMessage::new(source, dest, vec![], 2);
        
        msg.increment_hop();
        assert!(msg.can_forward());
        
        msg.increment_hop();
        assert!(!msg.can_forward());
    }

    #[tokio::test]
    async fn test_signal_forwarder_creation() {
        let local = test_node_id(1);
        let router = Arc::new(MultiHopRouter::new(local));
        let forwarder = SignalForwarder::new(local, router);
        
        assert_eq!(forwarder.pending_forward_count().await, 0);
    }

    #[tokio::test]
    async fn test_forward_message_max_hops() {
        let local = test_node_id(2);
        let router = Arc::new(MultiHopRouter::new(local));
        let forwarder = SignalForwarder::new(local, router);
        
        let mut msg = ForwardedMessage::new(test_node_id(1), test_node_id(3), vec![], 1);
        msg.hop_count = 1;
        
        let result = forwarder.forward_message(msg).await;
        assert!(matches!(result, Err(RoutingError::MaxHopsExceeded)));
    }
}
