use thiserror::Error;

#[derive(Error, Debug)]
pub enum GridError {
    #[error("handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("invalid signature")]
    InvalidSignature,

    #[error("invalid node id")]
    InvalidNodeId,

    #[error("peer not found: {0}")]
    PeerNotFound(String),

    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("protocol error: {0}")]
    ProtocolError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("encryption error: {0}")]
    EncryptionError(String),

    #[error("decryption error: {0}")]
    DecryptionError(String),

    #[error("relay error: {0}")]
    RelayError(String),

    #[error("discovery error: {0}")]
    DiscoveryError(String),

    #[error("invalid multicast address: {0}")]
    InvalidMulticastAddr(String),

    #[error("event bus error: {0}")]
    EventBusError(String),

    #[error("no peers available")]
    NoPeersAvailable,

    #[error("timeout")]
    Timeout,

    #[error("channel closed")]
    ChannelClosed,

    #[error("message sender not initialized")]
    MessageSenderNotInitialized,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GridError>;
