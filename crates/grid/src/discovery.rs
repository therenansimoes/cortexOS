use async_trait::async_trait;
use futures::StreamExt;
use libp2p::mdns;
use libp2p::swarm::SwarmEvent;
use libp2p::{PeerId, Swarm};
use libp2p::kad;
use libp2p::kad::{store::MemoryStore, Mode};
use libp2p::identity::Keypair;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, Transport};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{GridError, Result};
use crate::peer::{NodeId, PeerInfo};

#[async_trait]
pub trait Discovery: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn discovered_peers(&self) -> Vec<PeerInfo>;
}

pub struct DiscoveryEvent {
    pub peer_id: NodeId,
    pub addresses: Vec<SocketAddr>,
}

const MULTICAST_ADDR: &str = "239.255.70.77";
const MULTICAST_PORT: u16 = 7077;
const ANNOUNCE_INTERVAL: Duration = Duration::from_secs(30);

pub struct LanDiscovery {
    local_node_id: NodeId,
    local_pubkey: [u8; 32],
    port: u16,
    discovered: Arc<RwLock<HashSet<NodeId>>>,
    running: Arc<RwLock<bool>>,
    event_tx: Option<mpsc::Sender<DiscoveryEvent>>,
}

impl LanDiscovery {
    pub fn new(
        local_node_id: NodeId,
        local_pubkey: [u8; 32],
        port: u16,
    ) -> (Self, mpsc::Receiver<DiscoveryEvent>) {
        let (tx, rx) = mpsc::channel(64);
        (
            Self {
                local_node_id,
                local_pubkey,
                port,
                discovered: Arc::new(RwLock::new(HashSet::new())),
                running: Arc::new(RwLock::new(false)),
                event_tx: Some(tx),
            },
            rx,
        )
    }

    fn create_announce_packet(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(64);
        packet.extend_from_slice(b"CORTEX");
        packet.extend_from_slice(&self.local_node_id.0);
        packet.extend_from_slice(&self.local_pubkey);
        packet.extend_from_slice(&self.port.to_be_bytes());
        packet
    }

    fn parse_announce_packet(data: &[u8]) -> Option<(NodeId, [u8; 32], u16)> {
        if data.len() < 70 || &data[..6] != b"CORTEX" {
            return None;
        }

        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&data[6..38]);

        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(&data[38..70]);

        let port = u16::from_be_bytes([data[70], data[71]]);

        Some((NodeId(node_id), pubkey, port))
    }

    async fn run_announcer(
        socket: Arc<UdpSocket>,
        packet: Vec<u8>,
        running: Arc<RwLock<bool>>,
    ) {
        let multicast_addr = match format!("{}:{}", MULTICAST_ADDR, MULTICAST_PORT)
            .parse::<SocketAddr>()
        {
            Ok(addr) => addr,
            Err(e) => {
                error!("Failed to parse multicast address: {}, announcer stopping", e);
                return;
            }
        };

        loop {
            {
                if !*running.read().await {
                    break;
                }
            }

            if let Err(e) = socket.send_to(&packet, multicast_addr).await {
                warn!("Failed to send multicast announce: {}", e);
            } else {
                debug!("Sent discovery announce");
            }

            tokio::time::sleep(ANNOUNCE_INTERVAL).await;
        }
    }

    async fn run_listener(
        socket: Arc<UdpSocket>,
        local_node_id: NodeId,
        discovered: Arc<RwLock<HashSet<NodeId>>>,
        event_tx: mpsc::Sender<DiscoveryEvent>,
        running: Arc<RwLock<bool>>,
    ) {
        let mut buf = [0u8; 1024];

        loop {
            {
                if !*running.read().await {
                    break;
                }
            }

            match tokio::time::timeout(Duration::from_secs(1), socket.recv_from(&mut buf)).await {
                Ok(Ok((len, src))) => {
                    if let Some((node_id, _pubkey, port)) = Self::parse_announce_packet(&buf[..len])
                    {
                        if node_id == local_node_id {
                            continue;
                        }

                        let mut discovered = discovered.write().await;
                        if discovered.insert(node_id) {
                            info!("Discovered new peer: {} at {:?}", node_id, src);

                            let peer_addr = SocketAddr::new(src.ip(), port);
                            let _ = event_tx
                                .send(DiscoveryEvent {
                                    peer_id: node_id,
                                    addresses: vec![peer_addr],
                                })
                                .await;
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("UDP recv error: {}", e);
                }
                Err(_) => {}
            }
        }
    }
}

#[async_trait]
impl Discovery for LanDiscovery {
    async fn start(&mut self) -> Result<()> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", MULTICAST_PORT))
            .await
            .map_err(|e| GridError::DiscoveryError(e.to_string()))?;

        let multicast_addr: std::net::Ipv4Addr = MULTICAST_ADDR
            .parse()
            .map_err(|e| GridError::InvalidMulticastAddr(format!("{}", e)))?;
        socket
            .join_multicast_v4(multicast_addr, std::net::Ipv4Addr::UNSPECIFIED)
            .map_err(|e| GridError::DiscoveryError(e.to_string()))?;

        let socket = Arc::new(socket);

        {
            *self.running.write().await = true;
        }

        let packet = self.create_announce_packet();
        let running = Arc::clone(&self.running);
        let socket_clone = Arc::clone(&socket);
        tokio::spawn(Self::run_announcer(socket_clone, packet, running));

        let local_node_id = self.local_node_id;
        let discovered = Arc::clone(&self.discovered);
        let event_tx = self
            .event_tx
            .take()
            .ok_or_else(|| GridError::DiscoveryError("Event sender already taken".to_string()))?;
        let running = Arc::clone(&self.running);
        tokio::spawn(Self::run_listener(
            socket,
            local_node_id,
            discovered,
            event_tx,
            running,
        ));

        info!("LAN discovery started on port {}", MULTICAST_PORT);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;
        info!("LAN discovery stopped");
        Ok(())
    }

    async fn discovered_peers(&self) -> Vec<PeerInfo> {
        let discovered = self.discovered.read().await;
        discovered
            .iter()
            .map(|node_id| PeerInfo::new(*node_id, [0u8; 32]))
            .collect()
    }
}

pub struct MdnsDiscovery {
    swarm: Option<Swarm<mdns::tokio::Behaviour>>,
    discovered: Arc<RwLock<HashSet<PeerId>>>,
}

impl MdnsDiscovery {
    pub fn new() -> Result<Self> {
        Ok(Self {
            swarm: None,
            discovered: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let swarm = self.swarm.as_mut().ok_or_else(|| {
            GridError::DiscoveryError("Swarm not initialized".to_string())
        })?;

        loop {
            match swarm.select_next_some().await {
                SwarmEvent::Behaviour(mdns::Event::Discovered(peers)) => {
                    for (peer_id, addr) in peers {
                        info!("mDNS discovered peer {} at {}", peer_id, addr);
                        self.discovered.write().await.insert(peer_id);
                    }
                }
                SwarmEvent::Behaviour(mdns::Event::Expired(peers)) => {
                    for (peer_id, _) in peers {
                        debug!("mDNS peer expired: {}", peer_id);
                        self.discovered.write().await.remove(&peer_id);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for MdnsDiscovery {
    fn default() -> Self {
        Self::new().expect("Failed to create MdnsDiscovery: critical initialization error")
    }
}

pub struct KademliaDiscovery {
    local_node_id: NodeId,
    local_pubkey: [u8; 32],
    listen_port: u16,
    discovered: Arc<RwLock<HashMap<NodeId, PeerInfo>>>,
    running: Arc<RwLock<bool>>,
}

impl KademliaDiscovery {
    pub fn new(
        local_node_id: NodeId,
        local_pubkey: [u8; 32],
        listen_port: u16,
    ) -> Result<(Self, mpsc::Receiver<DiscoveryEvent>)> {
        let (_tx, rx) = mpsc::channel(64);

        let discovery = Self {
            local_node_id,
            local_pubkey,
            listen_port,
            discovered: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        };

        Ok((discovery, rx))
    }

    async fn run_event_loop(
        _local_node_id: NodeId,
        _local_pubkey: [u8; 32],
        _listen_port: u16,
        discovered: Arc<RwLock<HashMap<NodeId, PeerInfo>>>,
        event_tx: mpsc::Sender<DiscoveryEvent>,
        running: Arc<RwLock<bool>>,
    ) -> Result<()> {
        // Create libp2p keypair
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        // Create a Kademlia behaviour with memory store
        let store = MemoryStore::new(local_peer_id);
        let kad_config = kad::Config::default();
        let behaviour = kad::Behaviour::with_config(local_peer_id, store, kad_config);

        // Build the swarm
        let transport = libp2p::tcp::tokio::Transport::default()
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(
                libp2p::noise::Config::new(&local_key)
                    .map_err(|e| GridError::DiscoveryError(format!("Noise config error: {}", e)))?,
            )
            .multiplex(libp2p::yamux::Config::default())
            .boxed();

        let mut swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor(),
        );

        // Listen on all interfaces
        let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0"
            .parse()
            .map_err(|e| GridError::DiscoveryError(format!("Invalid listen address: {}", e)))?;

        swarm
            .listen_on(listen_addr)
            .map_err(|e| GridError::DiscoveryError(format!("Failed to listen: {}", e)))?;

        // Set server mode for better DHT performance
        swarm.behaviour_mut().set_mode(Some(Mode::Server));

        loop {
            {
                if !*running.read().await {
                    break;
                }
            }

            tokio::select! {
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(kad::Event::RoutingUpdated {
                            peer,
                            addresses,
                            ..
                        }) => {
                            debug!("Kademlia: Routing updated for peer {} with {} addresses", peer, addresses.len());
                            
                            // Convert PeerId to NodeId
                            let peer_bytes = peer.to_bytes();
                            let mut node_id_bytes = [0u8; 32];
                            let hash = blake3::hash(&peer_bytes);
                            node_id_bytes.copy_from_slice(hash.as_bytes());
                            let node_id = NodeId(node_id_bytes);

                            // Convert libp2p Multiaddr to SocketAddr if possible
                            let socket_addrs: Vec<SocketAddr> = addresses
                                .iter()
                                .filter_map(|addr| {
                                    let mut ip = None;
                                    let mut port = None;
                                    for component in addr.iter() {
                                        match component {
                                            Protocol::Ip4(addr) => ip = Some(std::net::IpAddr::V4(addr)),
                                            Protocol::Ip6(addr) => ip = Some(std::net::IpAddr::V6(addr)),
                                            Protocol::Tcp(p) => port = Some(p),
                                            _ => {}
                                        }
                                    }
                                    match (ip, port) {
                                        (Some(ip), Some(port)) => Some(SocketAddr::new(ip, port)),
                                        _ => None,
                                    }
                                })
                                .collect();

                            if !socket_addrs.is_empty() {
                                let mut discovered_map = discovered.write().await;
                                let peer_info = discovered_map
                                    .entry(node_id)
                                    .or_insert_with(|| PeerInfo::new(node_id, [0u8; 32]));
                                
                                peer_info.addresses = socket_addrs.clone();
                                peer_info.touch();

                                info!("Kademlia discovered peer: {} at {:?}", node_id, socket_addrs);
                                let _ = event_tx
                                    .send(DiscoveryEvent {
                                        peer_id: node_id,
                                        addresses: socket_addrs,
                                    })
                                    .await;
                            }
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("Kademlia listening on {}", address);
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    // Keep the loop alive
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Discovery for KademliaDiscovery {
    async fn start(&mut self) -> Result<()> {
        {
            *self.running.write().await = true;
        }

        let local_node_id = self.local_node_id;
        let local_pubkey = self.local_pubkey;
        let listen_port = self.listen_port;
        let discovered = Arc::clone(&self.discovered);
        let running = Arc::clone(&self.running);
        
        // Create a channel for events
        let (tx, _rx) = mpsc::channel(64);
        
        // Spawn the event loop
        tokio::spawn(async move {
            if let Err(e) = Self::run_event_loop(
                local_node_id,
                local_pubkey,
                listen_port,
                discovered,
                tx,
                running,
            ).await {
                warn!("Kademlia event loop error: {}", e);
            }
        });

        info!("Kademlia discovery started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;
        info!("Kademlia discovery stopped");
        Ok(())
    }

    async fn discovered_peers(&self) -> Vec<PeerInfo> {
        let discovered = self.discovered.read().await;
        discovered.values().cloned().collect()
    }
}
