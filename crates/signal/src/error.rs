use thiserror::Error;

use crate::signal::Channel;

#[derive(Debug, Error)]
pub enum SignalError {
    #[error("unknown symbol: {0}")]
    UnknownSymbol(String),

    #[error("channel not available: {0:?}")]
    ChannelUnavailable(Channel),

    #[error("invalid pattern: {0}")]
    InvalidPattern(String),

    #[error("codec error: {0}")]
    CodecError(String),
}

#[derive(Debug, Error)]
pub enum EmitError {
    #[error("channel unavailable: {0:?}")]
    ChannelUnavailable(Channel),

    #[error("hardware error: {0}")]
    HardwareError(String),

    #[error("pattern too long: max {max} pulses, got {got}")]
    PatternTooLong { max: usize, got: usize },

    #[error("timeout during emission")]
    Timeout,

    #[error("signal error: {0}")]
    Signal(#[from] SignalError),
}

#[derive(Debug, Error)]
pub enum ReceiveError {
    #[error("channel unavailable: {0:?}")]
    ChannelUnavailable(Channel),

    #[error("hardware error: {0}")]
    HardwareError(String),

    #[error("timeout waiting for signal")]
    Timeout,

    #[error("noise detected, signal unclear")]
    NoiseError,
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("unknown pattern")]
    UnknownPattern,

    #[error("corrupted signal")]
    CorruptedSignal,

    #[error("receive error: {0}")]
    Receive(#[from] ReceiveError),

    #[error("signal error: {0}")]
    Signal(#[from] SignalError),
}

impl From<EmitError> for DecodeError {
    fn from(e: EmitError) -> Self {
        DecodeError::Signal(SignalError::CodecError(e.to_string()))
    }
}

#[derive(Debug, Error)]
pub enum NegotiationError {
    #[error("no channels available")]
    NoChannelsAvailable,

    #[error("negotiation timeout")]
    Timeout,

    #[error("peer rejected channel: {0:?}")]
    ChannelRejected(Channel),

    #[error("quality below threshold: snr={snr}, min={min}")]
    QualityBelowThreshold { snr: f32, min: f32 },
}
