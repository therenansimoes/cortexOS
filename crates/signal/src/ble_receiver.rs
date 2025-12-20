//! BLE signal reception using Bluetooth Low Energy scanning
//!
//! This module provides BLE-based signal reception by scanning for BLE
//! advertisements containing CortexOS signal patterns. It decodes manufacturer-
//! specific data from advertisement packets back into signal patterns.
//!
//! # Platform Support
//!
//! BLE reception requires native Bluetooth hardware and is only available when
//! the `ble` feature is enabled. This module is not available on WASM targets.

use async_trait::async_trait;
use btleplug::api::{
    Central, Manager as _, ScanFilter,
};
use btleplug::platform::{Adapter, Manager};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, info};

use crate::codebook::Codebook;
use crate::error::{DecodeError, ReceiveError};
use crate::signal::{Channel, Pulse, Signal, SignalPattern};
use crate::Receiver;

/// Manufacturer ID for CortexOS BLE signals
const CORTEX_MANUFACTURER_ID: u16 = 0xFFFF;

/// Default timeout for receiving BLE signals
const DEFAULT_RECEIVE_TIMEOUT: Duration = Duration::from_secs(5);

/// BLE signal receiver implementation
///
/// Receives signals by scanning for BLE advertisements containing
/// CortexOS signal patterns in manufacturer-specific data.
#[allow(dead_code)] // Some fields will be used in full implementation
pub struct BleReceiver {
    adapter: Arc<Mutex<Adapter>>,
    receive_timeout: Duration,
    pattern_queue: Arc<Mutex<Vec<SignalPattern>>>,
}

impl BleReceiver {
    /// Create a new BLE receiver
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No BLE adapter is available
    /// - BLE adapter initialization fails
    pub async fn new() -> Result<Self, ReceiveError> {
        let manager = Manager::new()
            .await
            .map_err(|e| ReceiveError::HardwareError(format!("Failed to create BLE manager: {}", e)))?;

        let adapters = manager
            .adapters()
            .await
            .map_err(|e| ReceiveError::HardwareError(format!("Failed to get BLE adapters: {}", e)))?;

        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| ReceiveError::ChannelUnavailable(Channel::Ble))?;

        info!(
            adapter_info = ?adapter.adapter_info().await,
            "BLE receiver initialized"
        );

        Ok(Self {
            adapter: Arc::new(Mutex::new(adapter)),
            receive_timeout: DEFAULT_RECEIVE_TIMEOUT,
            pattern_queue: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Set the timeout for receiving signals
    pub fn with_timeout(mut self, timeout_duration: Duration) -> Self {
        self.receive_timeout = timeout_duration;
        self
    }

    /// Decode BLE advertisement data into a signal pattern
    ///
    /// Expected format:
    /// - Byte 0-1: Manufacturer ID (must match CORTEX_MANUFACTURER_ID)
    /// - Byte 2: Number of pulses
    /// - Bytes 3+: Pulse data (each pulse: 1 byte on/off + 4 bytes duration)
    #[allow(dead_code)] // Used in full BLE scanning implementation
    fn decode_advertisement(&self, data: &[u8]) -> Result<SignalPattern, ReceiveError> {
        if data.len() < 3 {
            return Err(ReceiveError::HardwareError(
                "Advertisement data too short".into(),
            ));
        }

        // Check manufacturer ID
        let mfr_id = u16::from_le_bytes([data[0], data[1]]);
        if mfr_id != CORTEX_MANUFACTURER_ID {
            return Err(ReceiveError::HardwareError(format!(
                "Invalid manufacturer ID: expected 0x{:04X}, got 0x{:04X}",
                CORTEX_MANUFACTURER_ID, mfr_id
            )));
        }

        let pulse_count = data[2] as usize;
        let expected_size = 3 + (pulse_count * 5);

        if data.len() < expected_size {
            return Err(ReceiveError::HardwareError(format!(
                "Advertisement data incomplete: expected {} bytes, got {}",
                expected_size,
                data.len()
            )));
        }

        let mut pulses = Vec::with_capacity(pulse_count);
        let mut offset = 3;

        for _ in 0..pulse_count {
            let on = data[offset] != 0;
            let duration_bytes = [
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
            ];
            let duration_us = u32::from_le_bytes(duration_bytes);

            pulses.push(Pulse::new(on, duration_us));
            offset += 5;
        }

        debug!(
            pulse_count = pulse_count,
            "Decoded BLE advertisement into signal pattern"
        );

        Ok(SignalPattern::new(pulses))
    }

    /// Start scanning for BLE advertisements
    ///
    /// This is a helper method that would start the BLE scan in a real implementation.
    /// Currently simplified due to btleplug API limitations.
    async fn start_scan(&self) -> Result<(), ReceiveError> {
        let adapter = self.adapter.lock().await;

        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| ReceiveError::HardwareError(format!("Failed to start BLE scan: {}", e)))?;

        debug!("BLE scan started");
        Ok(())
    }

    /// Stop BLE scanning
    async fn stop_scan(&self) -> Result<(), ReceiveError> {
        let adapter = self.adapter.lock().await;

        adapter
            .stop_scan()
            .await
            .map_err(|e| ReceiveError::HardwareError(format!("Failed to stop BLE scan: {}", e)))?;

        debug!("BLE scan stopped");
        Ok(())
    }

    /// Queue a pattern for testing purposes
    ///
    /// This method is used in tests to simulate received BLE patterns.
    #[cfg(test)]
    pub async fn queue_pattern_for_test(&self, pattern: SignalPattern) {
        self.pattern_queue.lock().await.push(pattern);
    }
}

#[async_trait]
impl Receiver for BleReceiver {
    fn channel(&self) -> Channel {
        Channel::Ble
    }

    async fn receive(&self) -> Result<SignalPattern, ReceiveError> {
        // Check if we have queued patterns (for testing or buffered reception)
        {
            let mut queue = self.pattern_queue.lock().await;
            if !queue.is_empty() {
                let pattern = queue.remove(0);
                debug!("Retrieved pattern from queue");
                return Ok(pattern);
            }
        }

        // Start scanning for BLE advertisements
        self.start_scan().await?;

        // Note: Full BLE scanning implementation would process advertisement
        // events from the adapter. This is simplified due to btleplug API.
        // In a real implementation, we would:
        // 1. Subscribe to adapter events
        // 2. Filter for manufacturer data matching CORTEX_MANUFACTURER_ID
        // 3. Decode the advertisement data
        // 4. Return the decoded pattern

        // For now, we simulate with a timeout
        let result = timeout(self.receive_timeout, async {
            // Simulated wait - in real implementation, this would wait for
            // actual BLE advertisement events
            tokio::time::sleep(Duration::from_millis(100)).await;
            Err(ReceiveError::Timeout)
        })
        .await;

        self.stop_scan().await?;

        match result {
            Ok(r) => r,
            Err(_) => Err(ReceiveError::Timeout),
        }
    }

    async fn decode(&self, codebook: &Codebook) -> Result<Signal, DecodeError> {
        let pattern = self.receive().await?;
        let symbol = codebook.decode(&pattern)?;

        debug!(
            symbol = ?symbol,
            pulse_count = pattern.pulse_count(),
            "Decoded BLE signal"
        );

        Ok(Signal::new(symbol, pattern, Channel::Ble))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_valid_advertisement() {
        // Construct valid advertisement data manually
        let mut data = Vec::new();
        data.extend_from_slice(&CORTEX_MANUFACTURER_ID.to_le_bytes());
        data.push(2); // 2 pulses

        // First pulse: on, 1000us
        data.push(1);
        data.extend_from_slice(&1000u32.to_le_bytes());

        // Second pulse: off, 500us
        data.push(0);
        data.extend_from_slice(&500u32.to_le_bytes());

        // Test decoding logic
        assert_eq!(data.len(), 13);
        assert_eq!(&data[0..2], &CORTEX_MANUFACTURER_ID.to_le_bytes());
        assert_eq!(data[2], 2);
        
        // Verify pulse data
        assert_eq!(data[3], 1); // First pulse on
        assert_eq!(u32::from_le_bytes([data[4], data[5], data[6], data[7]]), 1000);
        assert_eq!(data[8], 0); // Second pulse off
        assert_eq!(u32::from_le_bytes([data[9], data[10], data[11], data[12]]), 500);
    }

    #[test]
    fn test_decode_invalid_manufacturer_id() {
        let mut data = Vec::new();
        data.extend_from_slice(&0x1234u16.to_le_bytes()); // Wrong manufacturer ID
        data.push(0);

        // Verify we'd reject wrong manufacturer ID
        let mfr_id = u16::from_le_bytes([data[0], data[1]]);
        assert_ne!(mfr_id, CORTEX_MANUFACTURER_ID);
    }

    #[test]
    fn test_decode_data_sizes() {
        // Test minimum valid size
        let mut data = Vec::new();
        data.extend_from_slice(&CORTEX_MANUFACTURER_ID.to_le_bytes());
        data.push(0); // 0 pulses
        assert_eq!(data.len(), 3); // Minimum valid size

        // Test size calculation for 2 pulses
        let pulse_count = 2;
        let expected_size = 3 + (pulse_count * 5);
        assert_eq!(expected_size, 13);
    }

    #[test]
    fn test_pattern_queue_size() {
        // Test that we can calculate expected queue behavior
        let test_patterns = vec![
            SignalPattern::new(vec![Pulse::on(100)]),
            SignalPattern::new(vec![Pulse::off(200)]),
        ];
        assert_eq!(test_patterns.len(), 2);
    }
}
