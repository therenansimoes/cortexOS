use cortex_core::event::{Event, Payload};
use cortex_core::runtime::EventBus;
use std::time::{Duration, Instant};

fn benchmark_event_bus_throughput(num_events: usize) -> Duration {
    let bus = EventBus::new(10000);
    let _rx = bus.subscribe("*");

    let start = Instant::now();
    for i in 0..num_events {
        let event = Event::new(
            "bench-source",
            "bench.event",
            Payload::inline(format!("event-{}", i).into_bytes()),
        );
        let _ = bus.publish(event);
    }
    start.elapsed()
}

fn benchmark_pattern_matching(num_events: usize) -> Duration {
    let bus = EventBus::new(10000);
    let _rx1 = bus.subscribe("sensor.*");
    let _rx2 = bus.subscribe("grid.*");
    let _rx3 = bus.subscribe("agent.*");

    let start = Instant::now();
    for i in 0..num_events {
        let kind = match i % 3 {
            0 => "sensor.mic.v1",
            1 => "grid.msg.v1",
            _ => "agent.intent.v1",
        };
        let event = Event::new(
            "bench-source",
            kind,
            Payload::inline(format!("event-{}", i).into_bytes()),
        );
        let _ = bus.publish(event);
    }
    start.elapsed()
}

fn main() {
    println!("Event Bus Throughput Benchmark");
    println!("================================\n");

    // Warm up
    let _ = benchmark_event_bus_throughput(1000);

    // Benchmark different event counts
    for &count in &[10_000, 50_000, 100_000, 500_000] {
        let duration = benchmark_event_bus_throughput(count);
        let throughput = (count as f64) / duration.as_secs_f64();
        println!(
            "{:7} events in {:8.3}ms = {:12.0} events/sec",
            count,
            duration.as_millis(),
            throughput
        );
    }

    println!("\nPattern Matching Benchmark");
    println!("==========================\n");

    // Benchmark with pattern matching
    for &count in &[10_000, 50_000, 100_000, 500_000] {
        let duration = benchmark_pattern_matching(count);
        let throughput = (count as f64) / duration.as_secs_f64();
        println!(
            "{:7} events in {:8.3}ms = {:12.0} events/sec",
            count,
            duration.as_millis(),
            throughput
        );
    }
}
