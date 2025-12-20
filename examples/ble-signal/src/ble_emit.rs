//! BLE Signal Emission Example
//!
//! This example demonstrates how to use the BLE emitter to send signals
//! via Bluetooth Low Energy advertisements.
//!
//! # Requirements
//!
//! - BLE hardware (Bluetooth adapter)
//! - The `ble` feature must be enabled
//! - Appropriate permissions for BLE access
//!
//! # Usage
//!
//! ```bash
//! cargo run --example ble_emit --features ble
//! ```

use cortex_signal::{BleEmitter, Codebook, Emitter, Pulse, Signal, SignalPattern, StandardSymbol};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("BLE Signal Emission Example");
    info!("===========================");

    // Create a BLE emitter
    match BleEmitter::new("CortexOS-Emitter").await {
        Ok(emitter) => {
            info!("BLE emitter initialized successfully");

            // Create a codebook for encoding signals
            let codebook = Codebook::new();

            // Example 1: Emit a simple pattern directly
            info!("\n--- Example 1: Direct Pattern Emission ---");
            let pattern = SignalPattern::new(vec![
                Pulse::on(1000),
                Pulse::off(500),
                Pulse::on(1000),
            ]);

            match emitter.emit(&pattern).await {
                Ok(_) => info!("Pattern emitted successfully"),
                Err(e) => error!("Failed to emit pattern: {}", e),
            }

            // Example 2: Emit a standard symbol
            info!("\n--- Example 2: Standard Symbol Emission ---");
            let beacon_signal = Signal::new(
                StandardSymbol::Beacon.to_symbol_id(),
                codebook
                    .encode(StandardSymbol::Beacon.to_symbol_id())
                    .unwrap()
                    .clone(),
                cortex_signal::Channel::Ble,
            );

            match emitter.emit_signal(&beacon_signal, &codebook).await {
                Ok(_) => info!("Beacon signal emitted successfully"),
                Err(e) => error!("Failed to emit beacon: {}", e),
            }

            // Example 3: Emit multiple signals
            info!("\n--- Example 3: Multiple Signal Emission ---");
            let signals = vec![
                StandardSymbol::Ping,
                StandardSymbol::Ack,
                StandardSymbol::Ready,
            ];

            for symbol in signals {
                let signal = Signal::new(
                    symbol.to_symbol_id(),
                    codebook.encode(symbol.to_symbol_id()).unwrap().clone(),
                    cortex_signal::Channel::Ble,
                );

                info!("Emitting {} signal...", symbol.as_str());
                match emitter.emit_signal(&signal, &codebook).await {
                    Ok(_) => info!("  ✓ {} emitted", symbol.as_str()),
                    Err(e) => error!("  ✗ Failed to emit {}: {}", symbol.as_str(), e),
                }
            }

            info!("\n--- Emission Complete ---");
            info!(
                "Note: This is a simplified implementation. Full BLE advertising \
                 requires platform-specific extensions."
            );
        }
        Err(e) => {
            error!("Failed to initialize BLE emitter: {}", e);
            info!("Make sure you have:");
            info!("  1. BLE hardware available");
            info!("  2. Appropriate permissions for BLE access");
            info!("  3. The 'ble' feature enabled");
        }
    }
}
