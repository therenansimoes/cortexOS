use thiserror::Error;

/// Errors that can occur in Grid P2P networking operations.
///
/// These errors cover peer discovery, handshakes, message relay,
/// encryption/decryption, and general network communication.
#[derive(Error, Debug)]
pub enum GridError {
    /// Handshake protocol failed between peers
    #[error("handshake failed: {0}")]
    HandshakeFailed(String),

    /// Cryptographic signature verification failed
    #[error("invalid signature")]
    InvalidSignature,

    /// Node ID format or content is invalid
    #[error("invalid node id")]
    InvalidNodeId,

    /// Requested peer is not in the peer table
    #[error("peer not found: {0}")]
    PeerNotFound(String),

    /// Failed to establish connection to peer
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Wire protocol violation or unexpected message
    #[error("protocol error: {0}")]
    ProtocolError(String),

    /// Failed to serialize or deserialize network message
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Encryption operation failed
    #[error("encryption error: {0}")]
    EncryptionError(String),

    /// Decryption operation failed
    #[error("decryption error: {0}")]
    DecryptionError(String),

    /// Relay mesh operation failed
    #[error("relay error: {0}")]
    RelayError(String),

    /// Peer discovery mechanism failed
    #[error("discovery error: {0}")]
    DiscoveryError(String),

    /// Multicast address parsing or validation failed
    #[error("invalid multicast address: {0}")]
    InvalidMulticastAddr(String),

    /// Event bus operation failed
    #[error("event bus error: {0}")]
    EventBusError(String),

    /// No peers available to handle request
    #[error("no peers available")]
    NoPeersAvailable,

    /// Operation timed out waiting for response
    #[error("timeout")]
    Timeout,

    /// Communication channel was closed
    #[error("channel closed")]
    ChannelClosed,

    /// Message sender was not properly initialized
    #[error("message sender not initialized")]
    MessageSenderNotInitialized,

    /// I/O operation failed
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience Result type for Grid operations
pub type Result<T> = std::result::Result<T, GridError>;
