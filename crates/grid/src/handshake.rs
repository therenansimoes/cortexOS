use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use tracing::{debug, info};

use crate::error::{GridError, Result};
use crate::peer::{Capabilities, NodeId};
use crate::wire::{Message, SessionParams, PROTOCOL_VERSION};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeState {
    Initial,
    HelloSent,
    ChallengeReceived,
    ProveSent,
    Completed,
    Failed,
}

pub struct HandshakeContext {
    pub state: HandshakeState,
    pub local_node_id: NodeId,
    pub local_signing_key: SigningKey,
    pub remote_node_id: Option<NodeId>,
    pub remote_pubkey: Option<[u8; 32]>,
    pub nonce: Option<[u8; 32]>,
    pub capabilities: Capabilities,
}

impl HandshakeContext {
    pub fn new(
        local_node_id: NodeId,
        local_signing_key: SigningKey,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            state: HandshakeState::Initial,
            local_node_id,
            local_signing_key,
            remote_node_id: None,
            remote_pubkey: None,
            nonce: None,
            capabilities,
        }
    }

    pub fn create_hello(&self) -> Message {
        let pubkey = self.local_signing_key.verifying_key().to_bytes();
        let caps_encoded = self.capabilities.encode();

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
        sign_data.extend_from_slice(&self.local_node_id.0);
        sign_data.extend_from_slice(&pubkey);
        sign_data.extend_from_slice(&caps_encoded);

        let signature = self.local_signing_key.sign(&sign_data);

        Message::Hello {
            protocol_version: PROTOCOL_VERSION,
            node_id: self.local_node_id,
            pubkey,
            capabilities: caps_encoded,
            signature: signature.to_bytes().to_vec(),
        }
    }

    pub fn create_challenge(&mut self) -> Message {
        let mut nonce = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);
        self.nonce = Some(nonce);

        Message::Challenge { nonce }
    }

    pub fn create_prove(&self, nonce: &[u8; 32]) -> Message {
        let signature = self.local_signing_key.sign(nonce);
        Message::Prove {
            response: signature.to_bytes(),
        }
    }

    pub fn create_welcome(&self) -> Message {
        let mut session_id = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut session_id);

        Message::Welcome {
            session_params: SessionParams {
                session_id,
                heartbeat_interval_ms: 30000,
                max_message_size: 16 * 1024 * 1024,
            },
        }
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
        self.context.create_hello()
    }

    pub fn process(&mut self, msg: Message) -> Result<Option<Message>> {
        match (&self.context.state, msg) {
            (
                HandshakeState::Initial,
                Message::Hello {
                    protocol_version,
                    node_id,
                    pubkey,
                    capabilities,
                    signature,
                },
            ) => {
                self.verify_hello(protocol_version, node_id, pubkey, &capabilities, &signature)?;
                self.context.remote_node_id = Some(node_id);
                self.context.remote_pubkey = Some(pubkey);
                info!("Received valid HELLO from {}", node_id);

                let challenge = self.context.create_challenge();
                Ok(Some(challenge))
            }

            (HandshakeState::HelloSent, Message::Challenge { nonce }) => {
                debug!("Received CHALLENGE");
                self.context.state = HandshakeState::ChallengeReceived;
                self.context.nonce = Some(nonce);

                let prove = self.context.create_prove(&nonce);
                self.context.state = HandshakeState::ProveSent;
                Ok(Some(prove))
            }

            (HandshakeState::Initial, Message::Prove { response }) => {
                let nonce = self
                    .context
                    .nonce
                    .ok_or_else(|| GridError::HandshakeFailed("No nonce set".to_string()))?;

                let remote_pubkey = self
                    .context
                    .remote_pubkey
                    .ok_or_else(|| GridError::HandshakeFailed("No remote pubkey".to_string()))?;

                self.verify_prove(&response, &nonce, &remote_pubkey)?;
                info!("PROVE verified successfully");

                let welcome = self.context.create_welcome();
                self.context.state = HandshakeState::Completed;
                Ok(Some(welcome))
            }

            (HandshakeState::ProveSent, Message::Welcome { session_params }) => {
                info!(
                    "Received WELCOME, session_id: {:?}",
                    &session_params.session_id[..8]
                );
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

        let verifying_key =
            VerifyingKey::from_bytes(&pubkey).map_err(|_| GridError::InvalidSignature)?;

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(&protocol_version.to_be_bytes());
        sign_data.extend_from_slice(&node_id.0);
        sign_data.extend_from_slice(&pubkey);
        sign_data.extend_from_slice(capabilities);

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
        let verifying_key =
            VerifyingKey::from_bytes(remote_pubkey).map_err(|_| GridError::InvalidSignature)?;

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

        let mut initiator =
            Handshaker::new_initiator(initiator_node_id, initiator_key, Capabilities::default());

        let mut responder =
            Handshaker::new_responder(responder_node_id, responder_key, Capabilities::default());

        let hello = initiator.start();
        let challenge = responder.process(hello).unwrap().unwrap();
        let prove = initiator.process(challenge).unwrap().unwrap();
        let welcome = responder.process(prove).unwrap().unwrap();
        let result = initiator.process(welcome).unwrap();

        assert!(result.is_none());
        assert!(initiator.is_completed());
        assert!(responder.is_completed());
    }
}
