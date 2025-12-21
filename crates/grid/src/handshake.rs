use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use x25519_dalek::{PublicKey, StaticSecret};

use crate::error::{GridError, Result};
use crate::peer::{Capabilities, NodeId};
use crate::wire::{Message, SessionParams, PROTOCOL_VERSION};

/// Maximum allowed time drift for timestamp validation (5 minutes)
const MAX_TIMESTAMP_DRIFT_SECS: u64 = 300;

/// Handshake timeout in milliseconds
const HANDSHAKE_TIMEOUT_MS: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    Initial,
    HelloSent,
    ChallengeReceived,
    ProveSent,
    Completed,
    Failed,
}

/// Session keys for post-handshake encryption
#[derive(Debug, Clone)]
pub struct SessionKeys {
    pub session_id: [u8; 32],
    pub encryption_key: [u8; 32],
    pub initiated_at: SystemTime,
}

impl SessionKeys {
    pub fn new(session_id: [u8; 32], encryption_key: [u8; 32]) -> Self {
        Self {
            session_id,
            encryption_key,
            initiated_at: SystemTime::now(),
        }
    }

    /// Encrypt data using the session key
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = ChaCha20Poly1305::new_from_slice(&self.encryption_key)
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

        Ok(result)
    }

    /// Decrypt data using the session key
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 12 {
            return Err(GridError::DecryptionError("Ciphertext too short".to_string()));
        }

        let cipher = ChaCha20Poly1305::new_from_slice(&self.encryption_key)
            .map_err(|e| GridError::DecryptionError(e.to_string()))?;

        let nonce = Nonce::from_slice(&ciphertext[..12]);
        let encrypted = &ciphertext[12..];

        cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| GridError::DecryptionError(e.to_string()))
    }
}

pub struct HandshakeContext {
    pub state: HandshakeState,
    pub local_node_id: NodeId,
    pub local_signing_key: SigningKey,
    pub remote_node_id: Option<NodeId>,
    pub remote_pubkey: Option<[u8; 32]>,
    pub nonce: Option<[u8; 32]>,
    pub capabilities: Capabilities,
    // X25519 keys for session encryption
    pub x25519_secret: StaticSecret,
    pub x25519_public: PublicKey,
    pub remote_x25519_public: Option<PublicKey>,
    pub session_keys: Option<SessionKeys>,
    // Replay attack prevention
    pub used_nonces: Vec<[u8; 32]>,
    pub handshake_started_at: Option<SystemTime>,
}

impl HandshakeContext {
    pub fn new(local_node_id: NodeId, local_signing_key: SigningKey, capabilities: Capabilities) -> Self {
        let x25519_secret = StaticSecret::random_from_rng(rand::thread_rng());
        let x25519_public = PublicKey::from(&x25519_secret);

        Self {
            state: HandshakeState::Initial,
            local_node_id,
            local_signing_key,
            remote_node_id: None,
            remote_pubkey: None,
            nonce: None,
            capabilities,
            x25519_secret,
            x25519_public,
            remote_x25519_public: None,
            session_keys: None,
            used_nonces: Vec::new(),
            handshake_started_at: None,
        }
    }

    pub fn create_hello(&self) -> Message {
        let pubkey = self.local_signing_key.verifying_key().to_bytes();
        let caps_encoded = self.capabilities.encode();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is set before UNIX epoch")
            .as_secs();

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
        sign_data.extend_from_slice(&self.local_node_id.0);
        sign_data.extend_from_slice(&pubkey);
        sign_data.extend_from_slice(&caps_encoded);
        sign_data.extend_from_slice(self.x25519_public.as_bytes());
        sign_data.extend_from_slice(&timestamp.to_be_bytes());

        let signature = self.local_signing_key.sign(&sign_data);

        Message::Hello {
            protocol_version: PROTOCOL_VERSION,
            node_id: self.local_node_id,
            pubkey,
            capabilities: caps_encoded,
            x25519_pubkey: *self.x25519_public.as_bytes(),
            timestamp,
            signature: signature.to_bytes().to_vec(),
        }
    }

    pub fn create_challenge(&mut self) -> Message {
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);
        self.nonce = Some(nonce);

        Message::Challenge { 
            nonce,
            x25519_pubkey: *self.x25519_public.as_bytes(),
        }
    }

    pub fn create_prove(&self, nonce: &[u8; 32]) -> Message {
        let signature = self.local_signing_key.sign(nonce);
        Message::Prove {
            response: signature.to_bytes(),
        }
    }

    pub fn create_welcome(&mut self) -> Result<Message> {
        let mut session_id = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut session_id);

        // Derive session key from X25519 shared secret
        let remote_x25519 = self.remote_x25519_public
            .ok_or_else(|| GridError::HandshakeFailed("No remote X25519 pubkey".to_string()))?;

        let shared_secret = self.x25519_secret.diffie_hellman(&remote_x25519);
        let encryption_key = blake3::derive_key("cortex-session-v1", shared_secret.as_bytes());

        self.session_keys = Some(SessionKeys::new(session_id, encryption_key));

        Ok(Message::Welcome {
            session_params: SessionParams {
                session_id,
                heartbeat_interval_ms: 30000,
                max_message_size: 16 * 1024 * 1024,
            },
        })
    }

    /// Check if nonce has been used (replay attack prevention)
    fn is_nonce_used(&self, nonce: &[u8; 32]) -> bool {
        self.used_nonces.contains(nonce)
    }

    /// Mark nonce as used
    fn mark_nonce_used(&mut self, nonce: [u8; 32]) {
        self.used_nonces.push(nonce);
        // Keep only last 100 nonces to prevent memory bloat
        if self.used_nonces.len() > 100 {
            // Remove oldest nonces efficiently
            self.used_nonces.drain(0..self.used_nonces.len() - 100);
        }
    }

    /// Validate timestamp to prevent replay attacks
    fn validate_timestamp(&self, timestamp: u64) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is set before UNIX epoch")
            .as_secs();

        let diff = if now > timestamp {
            now - timestamp
        } else {
            timestamp - now
        };

        if diff > MAX_TIMESTAMP_DRIFT_SECS {
            return Err(GridError::HandshakeFailed(
                format!("Timestamp too far from current time: {} seconds", diff)
            ));
        }

        Ok(())
    }

    /// Check handshake timeout
    fn check_timeout(&self) -> Result<()> {
        if let Some(started_at) = self.handshake_started_at {
            let elapsed = started_at.elapsed()
                .map_err(|e| GridError::HandshakeFailed(format!("Time error: {}", e)))?;

            let timeout_duration = Duration::from_millis(HANDSHAKE_TIMEOUT_MS);
            if elapsed > timeout_duration {
                warn!("Handshake timeout: {:?} > {:?}", elapsed, timeout_duration);
                return Err(GridError::Timeout);
            }
        }
        Ok(())
    }
}

pub struct Handshaker {
    context: HandshakeContext,
}

impl Handshaker {
    pub fn new_initiator(
        local_node_id: NodeId,
        local_signing_key: SigningKey,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            context: HandshakeContext::new(local_node_id, local_signing_key, capabilities),
        }
    }

    pub fn new_responder(
        local_node_id: NodeId,
        local_signing_key: SigningKey,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            context: HandshakeContext::new(local_node_id, local_signing_key, capabilities),
        }
    }

    pub fn state(&self) -> HandshakeState {
        self.context.state
    }

    pub fn start(&mut self) -> Message {
        debug!("Starting handshake as initiator");
        self.context.state = HandshakeState::HelloSent;
        self.context.handshake_started_at = Some(SystemTime::now());
        self.context.create_hello()
    }

    pub fn process(&mut self, msg: Message) -> Result<Option<Message>> {
        // Check timeout before processing
        self.context.check_timeout()?;

        match (&self.context.state, msg) {
            (HandshakeState::Initial, Message::Hello { 
                protocol_version, 
                node_id, 
                pubkey, 
                capabilities, 
                x25519_pubkey,
                timestamp,
                signature 
            }) => {
                // Start timeout tracking when receiving first message
                if self.context.handshake_started_at.is_none() {
                    self.context.handshake_started_at = Some(SystemTime::now());
                }

                // Validate timestamp for replay attack prevention
                self.context.validate_timestamp(timestamp)?;

                self.verify_hello(protocol_version, node_id, pubkey, &capabilities, x25519_pubkey, timestamp, &signature)?;
                self.context.remote_node_id = Some(node_id);
                self.context.remote_pubkey = Some(pubkey);
                self.context.remote_x25519_public = Some(PublicKey::from(x25519_pubkey));
                info!("Received valid HELLO from {}", node_id);

                let challenge = self.context.create_challenge();
                Ok(Some(challenge))
            }

            (HandshakeState::HelloSent, Message::Challenge { nonce, x25519_pubkey }) => {
                debug!("Received CHALLENGE");

                // Check for nonce reuse (replay attack)
                if self.context.is_nonce_used(&nonce) {
                    return Err(GridError::HandshakeFailed("Nonce reuse detected".to_string()));
                }

                self.context.mark_nonce_used(nonce);
                self.context.state = HandshakeState::ChallengeReceived;
                self.context.nonce = Some(nonce);
                self.context.remote_x25519_public = Some(PublicKey::from(x25519_pubkey));

                let prove = self.context.create_prove(&nonce);
                self.context.state = HandshakeState::ProveSent;
                Ok(Some(prove))
            }

            (HandshakeState::Initial, Message::Prove { response }) => {
                let nonce = self.context.nonce.ok_or_else(|| {
                    GridError::HandshakeFailed("No nonce set".to_string())
                })?;

                let remote_pubkey = self.context.remote_pubkey.ok_or_else(|| {
                    GridError::HandshakeFailed("No remote pubkey".to_string())
                })?;

                self.verify_prove(&response, &nonce, &remote_pubkey)?;
                info!("PROVE verified successfully");

                let welcome = self.context.create_welcome()?;
                self.context.state = HandshakeState::Completed;
                Ok(Some(welcome))
            }

            (HandshakeState::ProveSent, Message::Welcome { session_params }) => {
                info!("Received WELCOME, session_id: {:?}", &session_params.session_id[..8]);

                // Derive session keys on initiator side
                let remote_x25519 = self.context.remote_x25519_public
                    .ok_or_else(|| GridError::HandshakeFailed("No remote X25519 pubkey".to_string()))?;

                let shared_secret = self.context.x25519_secret.diffie_hellman(&remote_x25519);
                let encryption_key = blake3::derive_key("cortex-session-v1", shared_secret.as_bytes());

                self.context.session_keys = Some(SessionKeys::new(session_params.session_id, encryption_key));
                self.context.state = HandshakeState::Completed;
                Ok(None)
            }

            (_, msg) => {
                self.context.state = HandshakeState::Failed;
                Err(GridError::HandshakeFailed(format!(
                    "Unexpected message {:?} in state {:?}",
                    msg.message_type(),
                    self.context.state
                )))
            }
        }
    }

    fn verify_hello(
        &self,
        protocol_version: u32,
        node_id: NodeId,
        pubkey: [u8; 32],
        capabilities: &[u8],
        x25519_pubkey: [u8; 32],
        timestamp: u64,
        signature: &[u8],
    ) -> Result<()> {
        if protocol_version != PROTOCOL_VERSION {
            return Err(GridError::HandshakeFailed(format!(
                "Protocol version mismatch: expected {}, got {}",
                PROTOCOL_VERSION, protocol_version
            )));
        }

        let expected_node_id = NodeId::from_pubkey(&pubkey);
        if node_id != expected_node_id {
            return Err(GridError::InvalidNodeId);
        }

        let verifying_key = VerifyingKey::from_bytes(&pubkey)
            .map_err(|_| GridError::InvalidSignature)?;

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(&protocol_version.to_be_bytes());
        sign_data.extend_from_slice(&node_id.0);
        sign_data.extend_from_slice(&pubkey);
        sign_data.extend_from_slice(capabilities);
        sign_data.extend_from_slice(&x25519_pubkey);
        sign_data.extend_from_slice(&timestamp.to_be_bytes());

        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| GridError::InvalidSignature)?;
        let sig = Signature::from_bytes(&sig_bytes);

        verifying_key
            .verify(&sign_data, &sig)
            .map_err(|_| GridError::InvalidSignature)?;

        Ok(())
    }

    fn verify_prove(
        &self,
        response: &[u8; 64],
        nonce: &[u8; 32],
        remote_pubkey: &[u8; 32],
    ) -> Result<()> {
        let verifying_key = VerifyingKey::from_bytes(remote_pubkey)
            .map_err(|_| GridError::InvalidSignature)?;

        let sig = Signature::from_bytes(response);

        verifying_key
            .verify(nonce, &sig)
            .map_err(|_| GridError::InvalidSignature)?;

        Ok(())
    }

    pub fn is_completed(&self) -> bool {
        self.context.state == HandshakeState::Completed
    }

    pub fn remote_node_id(&self) -> Option<NodeId> {
        self.context.remote_node_id
    }

    /// Get session keys after successful handshake
    pub fn session_keys(&self) -> Option<&SessionKeys> {
        self.context.session_keys.as_ref()
    }

    /// Get handshake duration in milliseconds
    pub fn handshake_duration_ms(&self) -> Option<u128> {
        self.context.handshake_started_at
            .and_then(|start| start.elapsed().ok())
            .map(|d| d.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn test_handshake_flow() {
        let initiator_key = SigningKey::generate(&mut OsRng);
        let responder_key = SigningKey::generate(&mut OsRng);

        let initiator_pubkey = initiator_key.verifying_key().to_bytes();
        let responder_pubkey = responder_key.verifying_key().to_bytes();

        let initiator_node_id = NodeId::from_pubkey(&initiator_pubkey);
        let responder_node_id = NodeId::from_pubkey(&responder_pubkey);

        let mut initiator = Handshaker::new_initiator(
            initiator_node_id,
            initiator_key,
            Capabilities::default(),
        );

        let mut responder = Handshaker::new_responder(
            responder_node_id,
            responder_key,
            Capabilities::default(),
        );

        let hello = initiator.start();
        let challenge = responder.process(hello).unwrap().unwrap();
        let prove = initiator.process(challenge).unwrap().unwrap();
        let welcome = responder.process(prove).unwrap().unwrap();
        let result = initiator.process(welcome).unwrap();

        assert!(result.is_none());
        assert!(initiator.is_completed());
        assert!(responder.is_completed());

        // Verify session keys are established
        assert!(initiator.session_keys().is_some());
        assert!(responder.session_keys().is_some());

        // Verify both parties derived the same session key
        let init_keys = initiator.session_keys().unwrap();
        let resp_keys = responder.session_keys().unwrap();
        assert_eq!(init_keys.session_id, resp_keys.session_id);
        assert_eq!(init_keys.encryption_key, resp_keys.encryption_key);

        // Verify handshake completed within timeout
        if let Some(duration_ms) = initiator.handshake_duration_ms() {
            assert!(duration_ms < HANDSHAKE_TIMEOUT_MS as u128, 
                "Handshake took {}ms, exceeds {}ms limit", duration_ms, HANDSHAKE_TIMEOUT_MS);
        }
    }

    #[test]
    fn test_session_encryption() {
        let initiator_key = SigningKey::generate(&mut OsRng);
        let responder_key = SigningKey::generate(&mut OsRng);

        let initiator_pubkey = initiator_key.verifying_key().to_bytes();
        let responder_pubkey = responder_key.verifying_key().to_bytes();

        let initiator_node_id = NodeId::from_pubkey(&initiator_pubkey);
        let responder_node_id = NodeId::from_pubkey(&responder_pubkey);

        let mut initiator = Handshaker::new_initiator(
            initiator_node_id,
            initiator_key,
            Capabilities::default(),
        );

        let mut responder = Handshaker::new_responder(
            responder_node_id,
            responder_key,
            Capabilities::default(),
        );

        // Complete handshake
        let hello = initiator.start();
        let challenge = responder.process(hello).unwrap().unwrap();
        let prove = initiator.process(challenge).unwrap().unwrap();
        let welcome = responder.process(prove).unwrap().unwrap();
        initiator.process(welcome).unwrap();

        // Test encryption/decryption
        let plaintext = b"Hello, secure Grid!";
        let init_keys = initiator.session_keys().unwrap();
        let resp_keys = responder.session_keys().unwrap();

        let ciphertext = init_keys.encrypt(plaintext).unwrap();
        let decrypted = resp_keys.decrypt(&ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_replay_attack_prevention() {
        let initiator_key = SigningKey::generate(&mut OsRng);
        let responder_key = SigningKey::generate(&mut OsRng);

        let initiator_pubkey = initiator_key.verifying_key().to_bytes();
        let responder_pubkey = responder_key.verifying_key().to_bytes();

        let initiator_node_id = NodeId::from_pubkey(&initiator_pubkey);
        let responder_node_id = NodeId::from_pubkey(&responder_pubkey);

        let mut initiator = Handshaker::new_initiator(
            initiator_node_id,
            initiator_key,
            Capabilities::default(),
        );

        let mut responder = Handshaker::new_responder(
            responder_node_id,
            responder_key,
            Capabilities::default(),
        );

        let hello = initiator.start();
        let challenge = responder.process(hello).unwrap().unwrap();
        
        // Complete handshake normally
        let _prove = initiator.process(challenge.clone()).unwrap().unwrap();
        
        // Try to replay the challenge - this should fail because nonce is already used
        let result = initiator.process(challenge);
        assert!(result.is_err(), "Should detect nonce reuse");
    }

    #[test]
    fn test_timestamp_validation() {
        let responder_key = SigningKey::generate(&mut OsRng);
        let responder_pubkey = responder_key.verifying_key().to_bytes();
        let responder_node_id = NodeId::from_pubkey(&responder_pubkey);

        let mut responder = Handshaker::new_responder(
            responder_node_id,
            responder_key,
            Capabilities::default(),
        );

        // Create a HELLO message with a very old timestamp
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - MAX_TIMESTAMP_DRIFT_SECS - 100;

        let fake_node_id = NodeId::random();
        let fake_key = SigningKey::generate(&mut OsRng);
        let fake_pubkey = fake_key.verifying_key().to_bytes();
        let caps_encoded = Capabilities::default().encode();
        let x25519_secret = StaticSecret::random_from_rng(rand::thread_rng());
        let x25519_public = PublicKey::from(&x25519_secret);

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
        sign_data.extend_from_slice(&fake_node_id.0);
        sign_data.extend_from_slice(&fake_pubkey);
        sign_data.extend_from_slice(&caps_encoded);
        sign_data.extend_from_slice(x25519_public.as_bytes());
        sign_data.extend_from_slice(&old_timestamp.to_be_bytes());

        let signature = fake_key.sign(&sign_data);

        let old_hello = Message::Hello {
            protocol_version: PROTOCOL_VERSION,
            node_id: NodeId::from_pubkey(&fake_pubkey),
            pubkey: fake_pubkey,
            capabilities: caps_encoded,
            x25519_pubkey: *x25519_public.as_bytes(),
            timestamp: old_timestamp,
            signature: signature.to_bytes().to_vec(),
        };

        let result = responder.process(old_hello);
        assert!(result.is_err(), "Should reject message with old timestamp");
    }

    #[test]
    fn test_nonce_reuse_detection() {
        let initiator_key = SigningKey::generate(&mut OsRng);
        let responder_key = SigningKey::generate(&mut OsRng);

        let initiator_pubkey = initiator_key.verifying_key().to_bytes();
        let responder_pubkey = responder_key.verifying_key().to_bytes();

        let initiator_node_id = NodeId::from_pubkey(&initiator_pubkey);
        let responder_node_id = NodeId::from_pubkey(&responder_pubkey);

        let mut initiator = Handshaker::new_initiator(
            initiator_node_id,
            initiator_key,
            Capabilities::default(),
        );

        let mut responder = Handshaker::new_responder(
            responder_node_id,
            responder_key,
            Capabilities::default(),
        );

        let hello = initiator.start();
        let challenge = responder.process(hello).unwrap().unwrap();

        // Process challenge once
        let _prove = initiator.process(challenge.clone()).unwrap();

        // Try to replay the challenge
        let result = initiator.process(challenge);
        assert!(result.is_err(), "Should detect nonce reuse");
    }

    #[test]
    fn test_protocol_version_mismatch() {
        let responder_key = SigningKey::generate(&mut OsRng);
        let responder_pubkey = responder_key.verifying_key().to_bytes();
        let responder_node_id = NodeId::from_pubkey(&responder_pubkey);

        let mut responder = Handshaker::new_responder(
            responder_node_id,
            responder_key,
            Capabilities::default(),
        );

        // Create HELLO with wrong protocol version
        let wrong_version = PROTOCOL_VERSION + 1;
        let fake_key = SigningKey::generate(&mut OsRng);
        let fake_pubkey = fake_key.verifying_key().to_bytes();
        let caps_encoded = Capabilities::default().encode();
        let x25519_secret = StaticSecret::random_from_rng(rand::thread_rng());
        let x25519_public = PublicKey::from(&x25519_secret);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(&wrong_version.to_be_bytes());
        sign_data.extend_from_slice(&NodeId::from_pubkey(&fake_pubkey).0);
        sign_data.extend_from_slice(&fake_pubkey);
        sign_data.extend_from_slice(&caps_encoded);
        sign_data.extend_from_slice(x25519_public.as_bytes());
        sign_data.extend_from_slice(&timestamp.to_be_bytes());

        let signature = fake_key.sign(&sign_data);

        let wrong_hello = Message::Hello {
            protocol_version: wrong_version,
            node_id: NodeId::from_pubkey(&fake_pubkey),
            pubkey: fake_pubkey,
            capabilities: caps_encoded,
            x25519_pubkey: *x25519_public.as_bytes(),
            timestamp,
            signature: signature.to_bytes().to_vec(),
        };

        let result = responder.process(wrong_hello);
        assert!(result.is_err(), "Should reject wrong protocol version");
    }

    #[test]
    fn test_invalid_signature() {
        let responder_key = SigningKey::generate(&mut OsRng);
        let responder_pubkey = responder_key.verifying_key().to_bytes();
        let responder_node_id = NodeId::from_pubkey(&responder_pubkey);

        let mut responder = Handshaker::new_responder(
            responder_node_id,
            responder_key,
            Capabilities::default(),
        );

        let fake_key = SigningKey::generate(&mut OsRng);
        let fake_pubkey = fake_key.verifying_key().to_bytes();
        let caps_encoded = Capabilities::default().encode();
        let x25519_secret = StaticSecret::random_from_rng(rand::thread_rng());
        let x25519_public = PublicKey::from(&x25519_secret);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create invalid signature (random bytes)
        let mut invalid_sig = [0u8; 64];
        rand::thread_rng().fill_bytes(&mut invalid_sig);

        let invalid_hello = Message::Hello {
            protocol_version: PROTOCOL_VERSION,
            node_id: NodeId::from_pubkey(&fake_pubkey),
            pubkey: fake_pubkey,
            capabilities: caps_encoded,
            x25519_pubkey: *x25519_public.as_bytes(),
            timestamp,
            signature: invalid_sig.to_vec(),
        };

        let result = responder.process(invalid_hello);
        assert!(result.is_err(), "Should reject invalid signature");
    }
}
