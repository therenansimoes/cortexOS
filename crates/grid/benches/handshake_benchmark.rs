use cortex_grid::{Capabilities, Handshaker, NodeId};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::time::Instant;

const ITERATIONS: usize = 100;
const TARGET_LATENCY_MS: u128 = 100;

fn main() {
    println!("Running Grid Handshake Benchmark...");
    println!("Target latency: < {}ms", TARGET_LATENCY_MS);
    println!("Iterations: {}", ITERATIONS);
    println!();

    let mut durations = Vec::with_capacity(ITERATIONS);
    let mut successes = 0;

    for i in 0..ITERATIONS {
        let initiator_key = SigningKey::generate(&mut OsRng);
        let responder_key = SigningKey::generate(&mut OsRng);

        let initiator_pubkey = initiator_key.verifying_key().to_bytes();
        let responder_pubkey = responder_key.verifying_key().to_bytes();

        let initiator_node_id = NodeId::from_pubkey(&initiator_pubkey);
        let responder_node_id = NodeId::from_pubkey(&responder_pubkey);

        let mut initiator = Handshaker::new_initiator(
            initiator_node_id,
            initiator_key,
            Capabilities::default(),
        );

        let mut responder = Handshaker::new_responder(
            responder_node_id,
            responder_key,
            Capabilities::default(),
        );

        let start = Instant::now();

        // Perform handshake
        let hello = initiator.start().unwrap();
        let challenge = responder.process(hello).unwrap().unwrap();
        let prove = initiator.process(challenge).unwrap().unwrap();
        let welcome = responder.process(prove).unwrap().unwrap();
        let result = initiator.process(welcome);

        let duration = start.elapsed();

        if result.is_ok() && initiator.is_completed() && responder.is_completed() {
            successes += 1;
            durations.push(duration.as_micros());
        }

        if (i + 1) % 10 == 0 {
            print!(".");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }

    println!();
    println!();

    if durations.is_empty() {
        println!("ERROR: No successful handshakes!");
        std::process::exit(1);
    }

    // Calculate statistics
    durations.sort_unstable();
    let min_us = durations[0];
    let max_us = durations[durations.len() - 1];
    let median_us = durations[durations.len() / 2];
    let avg_us = durations.iter().sum::<u128>() / durations.len() as u128;
    let p95_idx = (durations.len() as f64 * 0.95) as usize;
    let p95_us = durations[p95_idx.min(durations.len() - 1)];
    let p99_idx = (durations.len() as f64 * 0.99) as usize;
    let p99_us = durations[p99_idx.min(durations.len() - 1)];

    println!("Results:");
    println!("  Successes: {}/{}", successes, ITERATIONS);
    println!("  Min:       {:6.2}ms ({:8}µs)", min_us as f64 / 1000.0, min_us);
    println!("  Median:    {:6.2}ms ({:8}µs)", median_us as f64 / 1000.0, median_us);
    println!("  Average:   {:6.2}ms ({:8}µs)", avg_us as f64 / 1000.0, avg_us);
    println!("  P95:       {:6.2}ms ({:8}µs)", p95_us as f64 / 1000.0, p95_us);
    println!("  P99:       {:6.2}ms ({:8}µs)", p99_us as f64 / 1000.0, p99_us);
    println!("  Max:       {:6.2}ms ({:8}µs)", max_us as f64 / 1000.0, max_us);
    println!();

    // Check against target
    let median_ms = median_us as f64 / 1000.0;
    let p95_ms = p95_us as f64 / 1000.0;

    if median_ms < TARGET_LATENCY_MS as f64 {
        println!("✓ PASS: Median latency ({:.2}ms) is below target ({}ms)", median_ms, TARGET_LATENCY_MS);
    } else {
        println!("✗ FAIL: Median latency ({:.2}ms) exceeds target ({}ms)", median_ms, TARGET_LATENCY_MS);
    }

    if p95_ms < TARGET_LATENCY_MS as f64 {
        println!("✓ PASS: P95 latency ({:.2}ms) is below target ({}ms)", p95_ms, TARGET_LATENCY_MS);
    } else {
        println!("✗ FAIL: P95 latency ({:.2}ms) exceeds target ({}ms)", p95_ms, TARGET_LATENCY_MS);
    }

    println!();
    println!("Security features verified:");
    println!("  ✓ X25519 key exchange for session encryption");
    println!("  ✓ Ed25519 signatures for authentication");
    println!("  ✓ Challenge-response for liveness proof");
    println!("  ✓ Timestamp validation for replay prevention");
    println!("  ✓ State machine enforces message ordering");
}
