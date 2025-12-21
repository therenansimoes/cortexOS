use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use x25519_dalek::{EphemeralSecret, PublicKey, ReusableSecret};

use crate::error::{GridError, Result};
use crate::peer::NodeId;
use crate::wire::Message;

const DEFAULT_TTL: u8 = 7;
const MAX_HOP_COUNT: u8 = 15;
const BEACON_EXPIRY: Duration = Duration::from_secs(3600);
const IDENTITY_ROTATION_INTERVAL: Duration = Duration::from_secs(900);

#[derive(Debug, Clone)]
pub struct RelayBeacon {
    pub recipient_pubkey_hash: [u8; 8],
    pub ttl: u8,
    pub hop_count: u8,
    pub encrypted_payload: Vec<u8>,
    pub created_at: Instant,
}

impl RelayBeacon {
    pub fn new(recipient_pubkey_hash: [u8; 8], encrypted_payload: Vec<u8>) -> Self {
        Self {
            recipient_pubkey_hash,
            ttl: DEFAULT_TTL,
            hop_count: 0,
            encrypted_payload,
            created_at: Instant::now(),
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut data = Vec::new();
        data.extend_from_slice(&self.recipient_pubkey_hash);
        data.extend_from_slice(&[self.ttl, self.hop_count]);
        data.extend_from_slice(&self.encrypted_payload);
        *blake3::hash(&data).as_bytes()
    }

    pub fn can_forward(&self) -> bool {
        self.ttl > 0 && self.hop_count < MAX_HOP_COUNT
    }

    pub fn forward(&self) -> Option<Self> {
        if !self.can_forward() {
            return None;
        }

        Some(Self {
            recipient_pubkey_hash: self.recipient_pubkey_hash,
            ttl: self.ttl.saturating_sub(1),
            hop_count: self.hop_count.saturating_add(1),
            encrypted_payload: self.encrypted_payload.clone(),
            created_at: self.created_at,
        })
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > BEACON_EXPIRY
    }

    pub fn to_message(&self) -> Message {
        Message::RelayBeacon {
            recipient_pubkey_hash: self.recipient_pubkey_hash,
            ttl: self.ttl,
            hop_count: self.hop_count,
            encrypted_payload: self.encrypted_payload.clone(),
        }
    }

    pub fn from_message(msg: &Message) -> Option<Self> {
        match msg {
            Message::RelayBeacon {
                recipient_pubkey_hash,
                ttl,
                hop_count,
                encrypted_payload,
            } => Some(Self {
                recipient_pubkey_hash: *recipient_pubkey_hash,
                ttl: *ttl,
                hop_count: *hop_count,
                encrypted_payload: encrypted_payload.clone(),
                created_at: Instant::now(),
            }),
            _ => None,
        }
    }
}

pub struct RotatingIdentity {
    current_secret: ReusableSecret,
    current_public: PublicKey,
    pubkey_hash: [u8; 8],
    rotated_at: Instant,
}

impl RotatingIdentity {
    pub fn new() -> Self {
        let secret = ReusableSecret::random_from_rng(rand::thread_rng());
        let public = PublicKey::from(&secret);
        let pubkey_hash = Self::compute_hash(&public);

        Self {
            current_secret: secret,
            current_public: public,
            pubkey_hash,
            rotated_at: Instant::now(),
        }
    }

    fn compute_hash(pubkey: &PublicKey) -> [u8; 8] {
        let hash = blake3::hash(pubkey.as_bytes());
        let mut result = [0u8; 8];
        result.copy_from_slice(&hash.as_bytes()[..8]);
        result
    }

    pub fn should_rotate(&self) -> bool {
        self.rotated_at.elapsed() > IDENTITY_ROTATION_INTERVAL
    }

    pub fn rotate(&mut self) {
        self.current_secret = ReusableSecret::random_from_rng(rand::thread_rng());
        self.current_public = PublicKey::from(&self.current_secret);
        self.pubkey_hash = Self::compute_hash(&self.current_public);
        self.rotated_at = Instant::now();
        info!("Rotated relay identity");
    }

    pub fn pubkey_hash(&self) -> &[u8; 8] {
        &self.pubkey_hash
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.current_public
    }

    pub fn matches_hash(&self, hash: &[u8; 8]) -> bool {
        &self.pubkey_hash == hash
    }
}

impl Default for RotatingIdentity {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RelayEncryption;

impl RelayEncryption {
    pub fn encrypt(recipient_pubkey: &PublicKey, plaintext: &[u8]) -> Result<(Vec<u8>, PublicKey)> {
        let ephemeral_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let ephemeral_public = PublicKey::from(&ephemeral_secret);

        let shared_secret = ephemeral_secret.diffie_hellman(recipient_pubkey);
        let key = blake3::derive_key("cortex-relay-v1", shared_secret.as_bytes());

        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| GridError::EncryptionError(e.to_string()))?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| GridError::EncryptionError(e.to_string()))?;

        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok((result, ephemeral_public))
    }

    pub fn decrypt(
        recipient_secret: &ReusableSecret,
        sender_pubkey: &PublicKey,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>> {
        if ciphertext.len() < 12 {
            return Err(GridError::DecryptionError(
                "Ciphertext too short".to_string(),
            ));
        }

        let shared_secret = recipient_secret.diffie_hellman(sender_pubkey);
        let key = blake3::derive_key("cortex-relay-v1", shared_secret.as_bytes());

        let cipher = ChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| GridError::DecryptionError(e.to_string()))?;

        let nonce = Nonce::from_slice(&ciphertext[..12]);
        let encrypted = &ciphertext[12..];

        cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| GridError::DecryptionError(e.to_string()))
    }
}

pub struct BeaconStore {
    beacons: HashMap<[u8; 32], RelayBeacon>,
    by_recipient: HashMap<[u8; 8], Vec<[u8; 32]>>,
}

impl BeaconStore {
    pub fn new() -> Self {
        Self {
            beacons: HashMap::new(),
            by_recipient: HashMap::new(),
        }
    }

    pub fn insert(&mut self, beacon: RelayBeacon) -> [u8; 32] {
        let hash = beacon.hash();

        self.by_recipient
            .entry(beacon.recipient_pubkey_hash)
            .or_default()
            .push(hash);

        self.beacons.insert(hash, beacon);
        hash
    }

    pub fn get(&self, hash: &[u8; 32]) -> Option<&RelayBeacon> {
        self.beacons.get(hash)
    }

    pub fn find_for_recipient(&self, pubkey_hash: &[u8; 8]) -> Vec<&RelayBeacon> {
        self.by_recipient
            .get(pubkey_hash)
            .map(|hashes| {
                hashes
                    .iter()
                    .filter_map(|h| self.beacons.get(h))
                    .filter(|b| !b.is_expired())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn prune_expired(&mut self) -> usize {
        let expired: Vec<_> = self
            .beacons
            .iter()
            .filter(|(_, b)| b.is_expired())
            .map(|(h, _)| *h)
            .collect();

        let count = expired.len();
        for hash in expired {
            if let Some(beacon) = self.beacons.remove(&hash) {
                if let Some(hashes) = self.by_recipient.get_mut(&beacon.recipient_pubkey_hash) {
                    hashes.retain(|h| h != &hash);
                }
            }
        }
        count
    }
}

impl Default for BeaconStore {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RelayNode {
    node_id: NodeId,
    identity: Arc<RwLock<RotatingIdentity>>,
    beacon_store: Arc<RwLock<BeaconStore>>,
    outbound_tx: mpsc::Sender<Message>,
    running: Arc<RwLock<bool>>,
}

impl RelayNode {
    pub fn new(node_id: NodeId) -> (Self, mpsc::Receiver<Message>) {
        let (tx, rx) = mpsc::channel(256);

        (
            Self {
                node_id,
                identity: Arc::new(RwLock::new(RotatingIdentity::new())),
                beacon_store: Arc::new(RwLock::new(BeaconStore::new())),
                outbound_tx: tx,
                running: Arc::new(RwLock::new(false)),
            },
            rx,
        )
    }

    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;
        info!("Relay node started for {}", self.node_id);

        let identity = Arc::clone(&self.identity);
        let beacon_store = Arc::clone(&self.beacon_store);
        let running = Arc::clone(&self.running);

        tokio::spawn(async move {
            loop {
                if !*running.read().await {
                    break;
                }

                {
                    let mut id = identity.write().await;
                    if id.should_rotate() {
                        id.rotate();
                    }
                }

                {
                    let pruned = beacon_store.write().await.prune_expired();
                    if pruned > 0 {
                        debug!("Pruned {} expired beacons", pruned);
                    }
                }

                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Relay node stopped");
    }

    pub async fn handle_beacon(&self, beacon: RelayBeacon) -> Result<()> {
        let identity = self.identity.read().await;

        if identity.matches_hash(&beacon.recipient_pubkey_hash) {
            info!("Beacon is for us, storing for pickup");
            self.beacon_store.write().await.insert(beacon);
            return Ok(());
        }

        drop(identity);

        if beacon.can_forward() {
            if let Some(forwarded) = beacon.forward() {
                debug!("Forwarding beacon, hop {}", forwarded.hop_count);
                self.beacon_store.write().await.insert(forwarded.clone());

                let msg = Message::RelayForward {
                    beacon: Box::new(forwarded.to_message()),
                };
                self.outbound_tx
                    .send(msg)
                    .await
                    .map_err(|_| GridError::ChannelClosed)?;
            }
        } else {
            debug!("Beacon expired, not forwarding");
        }

        Ok(())
    }

    pub async fn create_beacon(
        &self,
        recipient_pubkey: &PublicKey,
        payload: &[u8],
    ) -> Result<RelayBeacon> {
        let (encrypted, ephemeral_pubkey) = RelayEncryption::encrypt(recipient_pubkey, payload)?;

        let mut full_payload = Vec::with_capacity(32 + encrypted.len());
        full_payload.extend_from_slice(ephemeral_pubkey.as_bytes());
        full_payload.extend_from_slice(&encrypted);

        let recipient_hash = {
            let hash = blake3::hash(recipient_pubkey.as_bytes());
            let mut result = [0u8; 8];
            result.copy_from_slice(&hash.as_bytes()[..8]);
            result
        };

        Ok(RelayBeacon::new(recipient_hash, full_payload))
    }

    pub async fn fetch_beacons(&self) -> Vec<RelayBeacon> {
        let identity = self.identity.read().await;
        let store = self.beacon_store.read().await;
        store
            .find_for_recipient(identity.pubkey_hash())
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn current_pubkey_hash(&self) -> [u8; 8] {
        *self.identity.read().await.pubkey_hash()
    }
}

impl Clone for RelayNode {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            identity: Arc::clone(&self.identity),
            beacon_store: Arc::clone(&self.beacon_store),
            outbound_tx: self.outbound_tx.clone(),
            running: Arc::clone(&self.running),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beacon_forward() {
        let beacon = RelayBeacon::new([1u8; 8], vec![1, 2, 3]);
        assert!(beacon.can_forward());
        assert_eq!(beacon.hop_count, 0);
        assert_eq!(beacon.ttl, DEFAULT_TTL);

        let forwarded = beacon.forward().unwrap();
        assert_eq!(forwarded.hop_count, 1);
        assert_eq!(forwarded.ttl, DEFAULT_TTL - 1);
    }

    #[test]
    fn test_encryption_roundtrip() {
        let recipient_secret = ReusableSecret::random_from_rng(rand::thread_rng());
        let recipient_public = PublicKey::from(&recipient_secret);

        let plaintext = b"Hello, relay mesh!";
        let (ciphertext, ephemeral_public) =
            RelayEncryption::encrypt(&recipient_public, plaintext).unwrap();

        let decrypted =
            RelayEncryption::decrypt(&recipient_secret, &ephemeral_public, &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_rotating_identity() {
        let mut identity = RotatingIdentity::new();
        let original_hash = *identity.pubkey_hash();

        identity.rotate();
        let new_hash = *identity.pubkey_hash();

        assert_ne!(original_hash, new_hash);
    }
}
