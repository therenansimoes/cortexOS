/// Signal Evolution Example
///
/// Demonstrates the signal evolution framework's ability to adaptively
/// learn and optimize communication patterns through reinforcement learning.

use cortex_core::SymbolId;
use cortex_signal::{
    Channel, CommunicationOutcome, ConsoleEmitter, Emitter, LearningConfig, LearningStats,
    LearningStrategy, LearningSystem, Pulse, SignalPattern,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🧬 Signal Evolution Framework Demo");
    info!("====================================\n");

    // Create a learning system with hybrid strategy
    let mut config = LearningConfig::default();
    config.strategy = LearningStrategy::Hybrid;
    config.auto_evolve = true;
    config.evaluation_interval = 5;

    let learning_system = LearningSystem::new(config);

    // Define symbols for different communication scenarios
    let ack_symbol = SymbolId::from_bytes(b"ACK");
    let beacon_symbol = SymbolId::from_bytes(b"BEACON");
    let data_symbol = SymbolId::from_bytes(b"DATA");

    info!("📡 Initializing learning for communication symbols...");
    learning_system.initialize_symbol(ack_symbol).await?;
    learning_system.initialize_symbol(beacon_symbol).await?;
    learning_system.initialize_symbol(data_symbol).await?;

    // Create emitter for visualization
    let emitter = ConsoleEmitter::new(Channel::Light, "EvolutionDemo");

    info!("\n🔄 Phase 1: Initial Pattern Evolution");
    info!("--------------------------------------");

    // Simulate communication attempts with feedback
    for round in 0..15 {
        info!("\nRound {}: Testing patterns...", round + 1);

        // Get best pattern for ACK
        let ack_pattern = learning_system.get_best_pattern(ack_symbol).await?;
        emitter.emit(&ack_pattern).await?;

        // Simulate varying success based on pattern characteristics
        let success = simulate_communication_success(&ack_pattern, round);
        let snr = if success { 75.0 + (round as f32 * 2.0) } else { 30.0 };
        let latency_us = if success { 5000 } else { 50000 };

        let outcome = CommunicationOutcome::success(ack_symbol, ack_pattern.clone())
            .with_snr(snr)
            .with_latency(latency_us);

        if !success {
            info!("❌ Communication failed (poor pattern)");
        } else {
            info!("✅ Communication successful");
        }

        learning_system.record_outcome(outcome).await?;

        // Display stats every few rounds
        if (round + 1) % 5 == 0 {
            display_stats(&learning_system, ack_symbol, "ACK").await;
        }
    }

    info!("\n🎯 Phase 2: Pattern Recognition");
    info!("--------------------------------");

    // Learn a specific high-performing pattern
    let learned_pattern = SignalPattern::new(vec![
        Pulse::on(100),
        Pulse::off(50),
        Pulse::on(100),
        Pulse::off(50),
        Pulse::on(100),
    ]);

    info!("Teaching system a known-good pattern...");
    learning_system
        .learn_pattern(beacon_symbol, learned_pattern.clone())
        .await?;

    // Test the learned pattern
    for _ in 0..3 {
        let outcome = CommunicationOutcome::success(beacon_symbol, learned_pattern.clone())
            .with_snr(85.0)
            .with_latency(3000);
        learning_system.record_outcome(outcome).await?;
    }

    info!("Best BEACON pattern after learning:");
    let best_beacon = learning_system.get_best_pattern(beacon_symbol).await?;
    emitter.emit(&best_beacon).await?;

    display_stats(&learning_system, beacon_symbol, "BEACON").await;

    info!("\n🔬 Phase 3: Multi-Symbol Evolution");
    info!("------------------------------------");

    // Evolve patterns for data transmission
    for _round in 0..10 {
        let data_pattern = learning_system.get_best_pattern(data_symbol).await?;

        // Simulate more complex success criteria for data
        let success = data_pattern.pulse_count() >= 4 && data_pattern.total_duration_us() < 2000;
        let outcome = if success {
            CommunicationOutcome::success(data_symbol, data_pattern)
                .with_snr(80.0)
                .with_latency(8000)
                .with_energy_cost(0.3)
        } else {
            CommunicationOutcome::failure(data_symbol, data_pattern)
                .with_snr(40.0)
                .with_latency(100000)
                .with_energy_cost(0.9)
        };

        learning_system.record_outcome(outcome).await?;
    }

    info!("\n📊 Final Evolution Results");
    info!("===========================\n");

    // Display final statistics for all symbols
    display_stats(&learning_system, ack_symbol, "ACK").await;
    display_stats(&learning_system, beacon_symbol, "BEACON").await;
    display_stats(&learning_system, data_symbol, "DATA").await;

    info!("\n🎓 Evolved Patterns:");
    info!("-------------------");

    let final_ack = learning_system.get_best_pattern(ack_symbol).await?;
    info!("\nACK Signal:");
    emitter.emit(&final_ack).await?;

    let final_beacon = learning_system.get_best_pattern(beacon_symbol).await?;
    info!("\nBEACON Signal:");
    emitter.emit(&final_beacon).await?;

    let final_data = learning_system.get_best_pattern(data_symbol).await?;
    info!("\nDATA Signal:");
    emitter.emit(&final_data).await?;

    info!("\n✨ Evolution complete! The system has learned optimal patterns.");
    info!("   Patterns can now be used for efficient communication across channels.\n");

    Ok(())
}

/// Simulate communication success based on pattern characteristics
fn simulate_communication_success(pattern: &SignalPattern, round: usize) -> bool {
    // Patterns with balanced on/off durations tend to succeed
    let on_duration: u32 = pattern
        .pulses
        .iter()
        .filter(|p| p.on)
        .map(|p| p.duration_us)
        .sum();
    let off_duration: u32 = pattern
        .pulses
        .iter()
        .filter(|p| !p.on)
        .map(|p| p.duration_us)
        .sum();

    let total = on_duration + off_duration;
    if total == 0 {
        return false;
    }

    let balance = (on_duration as f32 / total as f32 - 0.5).abs();

    // Success probability increases with balanced patterns and over time
    let base_prob = 1.0 - (balance * 2.0);
    let learning_bonus = (round as f32 * 0.03).min(0.3);
    let success_prob = (base_prob + learning_bonus).clamp(0.0, 1.0);

    // Also require reasonable pattern length
    let length_ok = pattern.pulse_count() >= 2 && pattern.pulse_count() <= 10;

    // Note: Using rand::random() for demo simplicity. For reproducible testing,
    // use a seeded RNG like `StdRng::seed_from_u64()`
    length_ok && rand::random::<f32>() < success_prob
}

async fn display_stats(system: &LearningSystem, symbol: SymbolId, name: &str) {
    let stats: LearningStats = system.get_stats(symbol).await;

    info!("\n{} Statistics:", name);
    info!("  Generation: {}", stats.current_generation);
    info!("  Success Rate: {:.1}%", stats.success_rate() * 100.0);
    info!("  Total Sent: {}", stats.total_signals_sent);
    info!(
        "  Successful: {} / Failed: {}",
        stats.successful_communications, stats.failed_communications
    );
    info!("  Patterns Learned: {}", stats.patterns_learned);
}
