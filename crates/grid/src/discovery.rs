use async_trait::async_trait;
use futures::StreamExt;
use libp2p::mdns;
use libp2p::swarm::SwarmEvent;
use libp2p::{PeerId, Swarm};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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
        let multicast_addr: SocketAddr = format!("{}:{}", MULTICAST_ADDR, MULTICAST_PORT)
            .parse()
            .unwrap();

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

        let multicast_addr: std::net::Ipv4Addr = MULTICAST_ADDR.parse().unwrap();
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
        let event_tx = self.event_tx.take().unwrap();
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
        Self::new().expect("Failed to create MdnsDiscovery")
    }
}
