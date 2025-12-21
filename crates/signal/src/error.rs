use thiserror::Error;

use crate::Channel;

/// Errors that can occur in the Subnet signal layer.
///
/// The signal layer handles low-level physical communication through
/// light pulses, audio chirps, BLE beacons, and other modalities.
/// These errors cover symbol encoding/decoding, channel management,
/// emission/reception, negotiation, and routing.

/// General signal processing errors.
#[derive(Debug, Error)]
pub enum SignalError {
    /// Symbol ID is not recognized in the codebook
    #[error("unknown symbol: {0}")]
    UnknownSymbol(String),

    /// Requested communication channel is not available
    #[error("channel not available: {0:?}")]
    ChannelUnavailable(Channel),

    /// Signal pattern is malformed or invalid
    #[error("invalid pattern: {0}")]
    InvalidPattern(String),

    /// Codec operation failed (encoding/decoding)
    #[error("codec error: {0}")]
    CodecError(String),
}

/// Convenience Result type for signal operations
pub type SignalResult<T> = std::result::Result<T, SignalError>;

/// Errors that occur during signal emission.
#[derive(Debug, Error)]
pub enum EmitError {
    /// Communication channel is unavailable for emission
    #[error("channel unavailable: {0:?}")]
    ChannelUnavailable(Channel),

    /// Hardware actuator error (LED, speaker, etc.)
    #[error("hardware error: {0}")]
    HardwareError(String),

    /// Signal pattern exceeds hardware limits
    #[error("pattern too long: max {max} pulses, got {got}")]
    PatternTooLong { max: usize, got: usize },

    /// Emission operation timed out
    #[error("timeout during emission")]
    Timeout,

    /// Signal encoding or general signal error
    #[error("signal error: {0}")]
    Signal(#[from] SignalError),
}

/// Convenience Result type for emission operations
pub type EmitResult<T> = std::result::Result<T, EmitError>;

/// Errors that occur during signal reception.
#[derive(Debug, Error)]
pub enum ReceiveError {
    /// Communication channel is unavailable for reception
    #[error("channel unavailable: {0:?}")]
    ChannelUnavailable(Channel),

    /// Hardware sensor error (light sensor, microphone, etc.)
    #[error("hardware error: {0}")]
    HardwareError(String),

    /// Reception operation timed out waiting for signal
    #[error("timeout waiting for signal")]
    Timeout,

    /// Too much noise, signal could not be distinguished
    #[error("noise detected, signal unclear")]
    NoiseError,
}

/// Convenience Result type for reception operations
pub type ReceiveResult<T> = std::result::Result<T, ReceiveError>;

/// Errors that occur during signal decoding.
#[derive(Debug, Error)]
pub enum DecodeError {
    /// Received pattern does not match any known symbol
    #[error("unknown pattern")]
    UnknownPattern,

    /// Signal was corrupted during transmission
    #[error("corrupted signal")]
    CorruptedSignal,

    /// Error during signal reception
    #[error("receive error: {0}")]
    Receive(#[from] ReceiveError),

    /// General signal processing error
    #[error("signal error: {0}")]
    Signal(#[from] SignalError),
}

/// Convenience Result type for decoding operations
pub type DecodeResult<T> = std::result::Result<T, DecodeError>;

/// Errors during channel negotiation between peers.
///
/// Peers negotiate to select the best available channel based on
/// signal quality, latency, and mutual capability.
#[derive(Debug, Error)]
pub enum NegotiationError {
    /// No mutually available channels between peers
    #[error("no channels available")]
    NoChannelsAvailable,

    /// Channel negotiation timed out
    #[error("negotiation timeout")]
    Timeout,

    /// Peer rejected the proposed channel
    #[error("peer rejected channel: {0:?}")]
    ChannelRejected(Channel),

    /// Signal quality is below acceptable threshold
    #[error("quality below threshold: snr={snr}, min={min}")]
    QualityBelowThreshold { snr: f32, min: f32 },
}

/// Convenience Result type for negotiation operations
pub type NegotiationResult<T> = std::result::Result<T, NegotiationError>;

/// Errors in multi-hop routing through the signal mesh.
///
/// The routing layer enables messages to hop through intermediate
/// nodes when direct communication is not possible.
#[derive(Debug, Error)]
pub enum RoutingError {
    /// No route could be found to the destination
    #[error("no route available to destination")]
    NoRouteAvailable,

    /// Route is malformed or invalid
    #[error("invalid route")]
    InvalidRoute,

    /// Route discovery process timed out
    #[error("route discovery timeout")]
    DiscoveryTimeout,

    /// Message exceeded maximum hop count
    #[error("max hops exceeded")]
    MaxHopsExceeded,

    /// Routing loop detected in path
    #[error("routing loop detected")]
    LoopDetected,
}

/// Convenience Result type for routing operations
pub type RoutingResult<T> = std::result::Result<T, RoutingError>;
