//! BLE Signal Reception Example
//!
//! This example demonstrates how to use the BLE receiver to listen for signals
//! transmitted via Bluetooth Low Energy advertisements.
//!
//! # Requirements
//!
//! - BLE hardware (Bluetooth adapter)
//! - The `ble` feature must be enabled
//! - Appropriate permissions for BLE scanning
//!
//! # Usage
//!
//! ```bash
//! cargo run --example ble_receive --features ble
//! ```

use cortex_signal::{BleReceiver, Codebook, Receiver};
use std::time::Duration;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("BLE Signal Reception Example");
    info!("============================");

    // Create a BLE receiver
    match BleReceiver::new().await {
        Ok(receiver) => {
            info!("BLE receiver initialized successfully");
            info!("Scanning for CortexOS BLE signals...");
            info!("(Press Ctrl+C to stop)\n");

            // Create a codebook for decoding signals
            let codebook = Codebook::new();

            // Continuously listen for signals
            loop {
                match tokio::time::timeout(Duration::from_secs(10), receiver.receive()).await {
                    Ok(Ok(pattern)) => {
                        info!("Received signal pattern:");
                        info!("  Pulse count: {}", pattern.pulse_count());
                        info!("  Total duration: {}µs", pattern.total_duration_us());

                        // Try to decode the pattern
                        match codebook.decode(&pattern) {
                            Ok(symbol) => {
                                info!("  Decoded symbol: {:?}", symbol);
                            }
                            Err(e) => {
                                warn!("  Could not decode pattern: {}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("Error receiving signal: {}", e);
                    }
                    Err(_) => {
                        info!("No signals received in the last 10 seconds...");
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to initialize BLE receiver: {}", e);
            info!("Make sure you have:");
            info!("  1. BLE hardware available");
            info!("  2. Appropriate permissions for BLE scanning");
            info!("  3. The 'ble' feature enabled");
        }
    }
}
