use cortex_core::event::{Event, EventMetrics, Payload};
use std::hint::black_box;
use std::time::Instant;

fn bench_event_creation(n: usize) -> f64 {
    let start = Instant::now();
    for i in 0..n {
        black_box(Event::new("test-source", "test.v1", Payload::inline(vec![i as u8])));
    }
    let duration = start.elapsed();
    duration.as_secs_f64()
}

fn bench_event_validation(n: usize) -> f64 {
    let events: Vec<_> = (0..n)
        .map(|i| Event::new("test-source", "test.v1", Payload::inline(vec![i as u8])))
        .collect();
    
    let start = Instant::now();
    for event in &events {
        black_box(event.validate().unwrap());
    }
    let duration = start.elapsed();
    duration.as_secs_f64()
}

fn bench_trace_propagation(n: usize) -> f64 {
    let parent = Event::new_with_trace("parent", "test.v1", Payload::inline(vec![]));
    
    let start = Instant::now();
    for i in 0..n {
        black_box(parent.new_child("child", "test.v2", Payload::inline(vec![i as u8])));
    }
    let duration = start.elapsed();
    duration.as_secs_f64()
}

fn bench_payload_size_calculation(n: usize) -> f64 {
    let payloads: Vec<_> = (0..n)
        .map(|i| Payload::inline(vec![0u8; i % 1000]))
        .collect();
    
    let start = Instant::now();
    for payload in &payloads {
        black_box(payload.size());
    }
    let duration = start.elapsed();
    duration.as_secs_f64()
}

fn main() {
    println!("CortexOS Event System Benchmarks");
    println!("=================================\n");

    let iterations = 100_000;

    // Event creation benchmark
    let duration = bench_event_creation(iterations);
    let throughput = iterations as f64 / duration;
    println!("Event Creation:");
    println!("  Iterations: {}", iterations);
    println!("  Time: {:.3}s", duration);
    println!("  Throughput: {:.0} events/sec\n", throughput);

    // Event validation benchmark
    let duration = bench_event_validation(iterations);
    let throughput = iterations as f64 / duration;
    println!("Event Validation:");
    println!("  Iterations: {}", iterations);
    println!("  Time: {:.3}s", duration);
    println!("  Throughput: {:.0} validations/sec\n", throughput);

    // Trace propagation benchmark
    let duration = bench_trace_propagation(iterations);
    let throughput = iterations as f64 / duration;
    println!("Trace Propagation:");
    println!("  Iterations: {}", iterations);
    println!("  Time: {:.3}s", duration);
    println!("  Throughput: {:.0} events/sec\n", throughput);

    // Payload size calculation benchmark
    let duration = bench_payload_size_calculation(iterations);
    let throughput = iterations as f64 / duration;
    println!("Payload Size Calculation:");
    println!("  Iterations: {}", iterations);
    println!("  Time: {:.3}s", duration);
    println!("  Throughput: {:.0} ops/sec\n", throughput);

    // Print metrics
    let metrics = EventMetrics::snapshot();
    println!("Event Metrics:");
    println!("  Events created: {}", metrics.events_created);
    println!("  Events validated: {}", metrics.events_validated);
    println!("  Validation failures: {}", metrics.validation_failures);
}
