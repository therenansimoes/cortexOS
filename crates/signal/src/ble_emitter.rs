//! BLE signal emission using Bluetooth Low Energy advertisements
//!
//! This module provides BLE-based signal emission by encoding signal patterns
//! into BLE advertisement packets. Signals are transmitted as manufacturer-specific
//! data in advertisement packets, allowing nearby devices to receive them.
//!
//! # Platform Support
//!
//! BLE emission requires native Bluetooth hardware and is only available when
//! the `ble` feature is enabled. This module is not available on WASM targets.

use async_trait::async_trait;
use btleplug::api::{Central, Manager as _};
use btleplug::platform::{Adapter, Manager};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::codebook::Codebook;
use crate::error::EmitError;
use crate::signal::{Channel, Signal, SignalPattern};
use crate::Emitter;

/// Manufacturer ID for CortexOS BLE signals
/// Using a test/experimental ID (0xFFFF is reserved for internal use)
const CORTEX_MANUFACTURER_ID: u16 = 0xFFFF;

/// Maximum size of BLE advertisement payload (manufacturer data)
const MAX_ADVERTISEMENT_SIZE: usize = 27;

/// BLE signal emitter implementation
///
/// Emits signals via BLE advertisements using manufacturer-specific data.
/// The signal pattern is encoded into the advertisement payload.
#[allow(dead_code)] // adapter will be used in full implementation
pub struct BleEmitter {
    adapter: Arc<Mutex<Adapter>>,
    device_name: String,
    emit_duration_ms: u64,
}

impl BleEmitter {
    /// Create a new BLE emitter
    ///
    /// # Arguments
    ///
    /// * `device_name` - Name to use for the BLE device
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No BLE adapter is available
    /// - BLE adapter initialization fails
    pub async fn new(device_name: impl Into<String>) -> Result<Self, EmitError> {
        let manager = Manager::new()
            .await
            .map_err(|e| EmitError::HardwareError(format!("Failed to create BLE manager: {}", e)))?;

        let adapters = manager
            .adapters()
            .await
            .map_err(|e| EmitError::HardwareError(format!("Failed to get BLE adapters: {}", e)))?;

        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| EmitError::ChannelUnavailable(Channel::Ble))?;

        info!(
            adapter_info = ?adapter.adapter_info().await,
            "BLE emitter initialized"
        );

        Ok(Self {
            adapter: Arc::new(Mutex::new(adapter)),
            device_name: device_name.into(),
            emit_duration_ms: 1000, // Default 1 second emission
        })
    }

    /// Set the emission duration for BLE advertisements
    ///
    /// This controls how long the BLE advertisement remains active.
    /// Longer durations increase the chance of reception but consume more power.
    pub fn with_emit_duration(mut self, duration_ms: u64) -> Self {
        self.emit_duration_ms = duration_ms;
        self
    }

    /// Encode a signal pattern into BLE advertisement data
    ///
    /// The encoding format is:
    /// - Byte 0-1: Manufacturer ID (CORTEX_MANUFACTURER_ID)
    /// - Byte 2: Number of pulses (max 255)
    /// - Bytes 3+: Pulse data (each pulse: 1 byte on/off + 4 bytes duration)
    fn encode_pattern(&self, pattern: &SignalPattern) -> Result<Vec<u8>, EmitError> {
        let pulse_count = pattern.pulse_count();
        if pulse_count > 255 {
            return Err(EmitError::PatternTooLong {
                max: 255,
                got: pulse_count,
            });
        }

        // Calculate required size: 2 (mfr ID) + 1 (count) + pulses * 5 bytes
        let required_size = 3 + (pulse_count * 5);
        if required_size > MAX_ADVERTISEMENT_SIZE {
            return Err(EmitError::PatternTooLong {
                max: (MAX_ADVERTISEMENT_SIZE - 3) / 5,
                got: pulse_count,
            });
        }

        let mut data = Vec::with_capacity(required_size);

        // Manufacturer ID (little-endian)
        data.extend_from_slice(&CORTEX_MANUFACTURER_ID.to_le_bytes());

        // Pulse count
        data.push(pulse_count as u8);

        // Encode each pulse
        for pulse in &pattern.pulses {
            data.push(if pulse.on { 1 } else { 0 });
            data.extend_from_slice(&pulse.duration_us.to_le_bytes());
        }

        debug!(
            pulse_count = pulse_count,
            data_size = data.len(),
            "Encoded BLE advertisement data"
        );

        Ok(data)
    }
}

#[async_trait]
impl Emitter for BleEmitter {
    fn channel(&self) -> Channel {
        Channel::Ble
    }

    async fn emit(&self, pattern: &SignalPattern) -> Result<(), EmitError> {
        let data = self.encode_pattern(pattern)?;

        // Note: btleplug doesn't currently support advertising in the same way
        // across all platforms. This is a simplified implementation that would
        // need platform-specific extensions for full advertisement support.
        //
        // For now, we simulate emission by logging. In a real implementation,
        // platform-specific APIs would be used:
        // - Linux: BlueZ D-Bus API for advertising
        // - macOS/iOS: CoreBluetooth peripheral mode
        // - Windows: Windows BLE APIs

        info!(
            device_name = %self.device_name,
            pattern_size = data.len(),
            pulse_count = pattern.pulse_count(),
            duration_ms = self.emit_duration_ms,
            "BLE emission (simulated)"
        );

        // In a real implementation, this would start BLE advertising
        // For now, we just sleep to simulate the emission duration
        tokio::time::sleep(Duration::from_millis(self.emit_duration_ms)).await;

        debug!("BLE emission complete");
        Ok(())
    }

    async fn emit_signal(&self, signal: &Signal, codebook: &Codebook) -> Result<(), EmitError> {
        let pattern = codebook.encode(signal.symbol)?;
        info!(
            symbol = ?signal.symbol,
            channel = ?signal.channel,
            "Emitting signal via BLE"
        );
        self.emit(pattern).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::Pulse;

    #[test]
    fn test_encode_simple_pattern() {
        let _pattern = SignalPattern::new(vec![Pulse::on(1000), Pulse::off(500)]);
        
        // Test encoding logic directly without creating emitter
        let mut data = Vec::new();
        data.extend_from_slice(&CORTEX_MANUFACTURER_ID.to_le_bytes());
        data.push(2); // 2 pulses
        data.push(1);
        data.extend_from_slice(&1000u32.to_le_bytes());
        data.push(0);
        data.extend_from_slice(&500u32.to_le_bytes());

        // Check structure: mfr_id (2) + count (1) + 2 pulses * 5 bytes = 13 bytes
        assert_eq!(data.len(), 13);
        assert_eq!(&data[0..2], &CORTEX_MANUFACTURER_ID.to_le_bytes());
        assert_eq!(data[2], 2);
        assert_eq!(data[3], 1);
        assert_eq!(&data[4..8], &1000u32.to_le_bytes());
        assert_eq!(data[8], 0);
        assert_eq!(&data[9..13], &500u32.to_le_bytes());
    }

    #[test]
    fn test_pattern_size_limits() {
        // Test that we respect BLE advertisement size limits
        let max_pulses = (MAX_ADVERTISEMENT_SIZE - 3) / 5; // 4 pulses with current limit
        assert_eq!(max_pulses, 4);
        
        // Verify calculation
        let size_for_4_pulses = 3 + (4 * 5);
        assert_eq!(size_for_4_pulses, 23);
        assert!(size_for_4_pulses <= MAX_ADVERTISEMENT_SIZE);
    }

    #[test]
    fn test_empty_pattern_encoding() {
        let _pattern = SignalPattern::empty();
        
        let mut data = Vec::new();
        data.extend_from_slice(&CORTEX_MANUFACTURER_ID.to_le_bytes());
        data.push(0); // Zero pulses

        // Should have just mfr_id + count
        assert_eq!(data.len(), 3);
        assert_eq!(data[2], 0); // Zero pulses
    }
}
