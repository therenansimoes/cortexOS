//! Physical signal layer for CortexOS
//!
//! This crate provides abstractions for low-level signal communication across
//! multiple physical channels: light, audio, BLE, vibration, and radio.
//!
//! ## Feature Flags
//!
//! - `ble`: Enable Bluetooth Low Energy signal support (native platforms only)
//!
//! ## Architecture
//!
//! The signal layer uses an emitter/receiver pattern with pluggable transports:
//!
//! - **Emitter**: Converts signal patterns to physical emissions
//! - **Receiver**: Detects physical signals and converts to patterns
//! - **Codebook**: Maps semantic symbols to signal patterns
//! - **Negotiation**: Selects best channel based on quality metrics

pub mod codebook;
pub mod emitter;
pub mod error;
pub mod negotiation;
pub mod receiver;
pub mod signal;

#[cfg(feature = "ble")]
pub mod ble_emitter;

#[cfg(feature = "ble")]
pub mod ble_receiver;

// Re-export commonly used types
pub use codebook::{Codebook, CodebookEntry, StandardSymbol};
pub use emitter::{ConsoleEmitter, Emitter, MockEmitter};
pub use error::{DecodeError, EmitError, NegotiationError, ReceiveError, SignalError};
pub use negotiation::{ChannelNegotiator, ChannelQuality};
pub use receiver::{MockReceiver, Receiver};
pub use signal::{Channel, Pulse, Signal, SignalPattern};

#[cfg(feature = "ble")]
pub use ble_emitter::BleEmitter;

#[cfg(feature = "ble")]
pub use ble_receiver::BleReceiver;
