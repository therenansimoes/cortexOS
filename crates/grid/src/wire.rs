use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::peer::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionParams {
    pub session_id: [u8; 32],
    pub heartbeat_interval_ms: u32,
    pub max_message_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Accepted,
    Rejected,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Handshake
    Hello {
        protocol_version: u32,
        node_id: NodeId,
        pubkey: [u8; 32],
        capabilities: Vec<u8>,
        x25519_pubkey: [u8; 32],
        timestamp: u64,
        signature: Vec<u8>,
    },
    Challenge {
        nonce: [u8; 32],
        x25519_pubkey: [u8; 32],
    },
    Prove {
        #[serde(with = "BigArray")]
        response: [u8; 64],
    },
    Welcome {
        session_params: SessionParams,
    },

    // Liveness
    Ping {
        seq: u64,
    },
    Pong {
        seq: u64,
    },

    // Capabilities
    CapsGet,
    CapsSet {
        caps: Vec<u8>,
    },

    // Tasks
    TaskRequest {
        task_id: [u8; 32],
        payload: Vec<u8>,
    },
    TaskAck {
        task_id: [u8; 32],
        status: TaskStatus,
    },

    // Event sync
    EventChunkGet {
        hash: [u8; 32],
    },
    EventChunkPut {
        hash: [u8; 32],
        data: Vec<u8>,
    },

    // Artifacts
    ArtifactGet {
        hash: [u8; 32],
    },
    ArtifactPut {
        hash: [u8; 32],
        data: Vec<u8>,
    },

    // Relay mesh (AirTag-style)
    RelayBeacon {
        recipient_pubkey_hash: [u8; 8],
        ttl: u8,
        hop_count: u8,
        encrypted_payload: Vec<u8>,
    },
    RelayForward {
        beacon: Box<Message>,
    },
    RelayDeliver {
        beacon_hash: [u8; 32],
    },
    RelayFetch {
        pubkey_prefix: [u8; 8],
    },

    // Error
    Error {
        code: u32,
        message: String,
    },
}

impl Message {
    pub fn encode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn decode(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }

    pub fn message_type(&self) -> u8 {
        match self {
            Message::Hello { .. } => 0x01,
            Message::Challenge { .. } => 0x02,
            Message::Prove { .. } => 0x03,
            Message::Welcome { .. } => 0x04,
            Message::Ping { .. } => 0x10,
            Message::Pong { .. } => 0x11,
            Message::CapsGet => 0x20,
            Message::CapsSet { .. } => 0x21,
            Message::TaskRequest { .. } => 0x30,
            Message::TaskAck { .. } => 0x31,
            Message::EventChunkGet { .. } => 0x40,
            Message::EventChunkPut { .. } => 0x41,
            Message::ArtifactGet { .. } => 0x50,
            Message::ArtifactPut { .. } => 0x51,
            Message::RelayBeacon { .. } => 0x60,
            Message::RelayForward { .. } => 0x61,
            Message::RelayDeliver { .. } => 0x62,
            Message::RelayFetch { .. } => 0x63,
            Message::Error { .. } => 0xFF,
        }
    }
}

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16 MB
