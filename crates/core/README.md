# CortexOS Core

The core runtime and event system for CortexOS - a decentralized, portable AI agent platform.

## Overview

`cortex-core` provides the foundational building blocks for CortexOS:

- **Event System**: Content-addressable events with trace propagation
- **Backpressure Policies**: Configurable queue management (DropNew, DropOld, Coalesce, Sample, Persist)
- **Runtime**: Async event bus and agent lifecycle management
- **Capability System**: Fine-grained permission control
- **Metrics**: Built-in monitoring for event throughput and system health

## Features

### Event System

Events are the primary communication mechanism in CortexOS. Each event contains:

```rust
use cortex_core::event::{Event, Payload};

// Create a simple event
let event = Event::new(
    "sensor-node-1",           // source
    "sensor.temperature.v1",   // kind (versioned)
    Payload::inline(vec![25])  // payload
);

// Create a validated event with bounds checking
let event = Event::new_validated(
    "sensor-node-1",
    "sensor.temperature.v1",
    Payload::inline(data)
)?;

// Add distributed tracing context
let event = event.with_trace("trace-id-123", "span-id-456");
```

**Event Validation**:
- Source and kind length limits (256 chars)
- Kind format validation (must be dot-separated, e.g., `sensor.mic.v1`)
- Payload size limit (1MB for inline data)
- Control character sanitization

### Backpressure Policies

Control how queues handle load with different backpressure strategies:

```rust
use cortex_core::backpressure::{BackpressurePolicy, PolicyQueue};

// Drop new events when queue is full
let queue = PolicyQueue::new(BackpressurePolicy::DropNew, 1000);

// Drop oldest events when queue is full (FIFO eviction)
let queue = PolicyQueue::new(BackpressurePolicy::DropOld, 1000);

// Coalesce events by key (keep only latest per key)
let queue = PolicyQueue::new(BackpressurePolicy::Coalesce("sensor_id".to_string()), 1000);

// Sample every Nth event (reduce event rate)
let queue = PolicyQueue::new(BackpressurePolicy::Sample(10), 1000);

// Persist to storage when full (future: event log)
let queue = PolicyQueue::new(BackpressurePolicy::Persist, 1000);
```

**Policy Selection Guidelines**:
- **DropNew**: Real-time systems where recent data is critical
- **DropOld**: Logging/audit where you want most recent events
- **Coalesce**: Sensor updates where only latest value matters
- **Sample**: High-frequency events that can be downsampled
- **Persist**: Important events that must not be lost

### Runtime & Agent System

The runtime manages agent lifecycle and event routing:

```rust
use cortex_core::runtime::{Runtime, Agent};
use cortex_core::event::Event;
use cortex_core::capability::CapabilitySet;

#[async_trait]
impl Agent for MyAgent {
    fn name(&self) -> &str { "my-agent" }
    fn capabilities(&self) -> &CapabilitySet { &self.caps }
    
    async fn handle(&self, event: Event) -> Result<()> {
        // Process event
        Ok(())
    }
}

let runtime = Runtime::new();

// Spawn an agent
runtime.spawn_agent(my_agent).await?;

// Publish events
runtime.publish(event)?;

// Subscribe to events by pattern
let mut rx = runtime.subscribe("sensor.*");
while let Some(event) = rx.recv().await {
    // Handle events matching pattern
}
```

### Metrics & Monitoring

Built-in metrics for observability:

```rust
let metrics = runtime.metrics();

println!("Events published: {}", metrics.events_published);
println!("Events delivered: {}", metrics.events_delivered);
println!("Events dropped: {}", metrics.events_dropped);
println!("Active agents: {}", metrics.active_agents);
println!("Active subscriptions: {}", metrics.active_subscriptions);
```

### Capability System

Fine-grained permissions for agents:

```rust
use cortex_core::capability::{Capability, CapabilitySet};

let mut caps = CapabilitySet::new();

// File system access
caps.grant(Capability::FsRead("/var/data/*".to_string()));
caps.grant(Capability::FsWrite("/tmp/*".to_string()));

// Network access
caps.grant(Capability::NetworkTcp("api.example.com:443".to_string()));

// Sensor access
caps.grant(Capability::Sensor("camera".to_string()));

// Grid communication
caps.grant(Capability::GridSend);
caps.grant(Capability::GridReceive);

// Check permissions
if caps.has(&Capability::FsRead("/var/data/file.txt".to_string())) {
    // Allowed
}
```

## Platform Support

`cortex-core` is designed for portability:

- **Native**: Linux, macOS, Windows
- **WASM/WASI**: Runs in browsers and WASI runtimes
- **Embedded**: No OS-specific dependencies in core

Build for WASI:
```bash
cargo build --target wasm32-wasip1
```

## Performance

Event throughput benchmarks (see `benches/` and `PERFORMANCE.md`):
- Event creation: ~500ns per event
- Event bus publish: ~1-2μs per event
- Pattern matching: ~100-200ns per check
- High throughput: >100K events/sec on modern hardware

## Design Principles

1. **Event-log first**: All perceptions/actions/network messages are timestamped events
2. **Backpressure everywhere**: Every subscription defines load behavior
3. **Capability-based security**: Agents only act through explicit capability tokens
4. **OS-agnostic core**: No platform-specific APIs in core modules
5. **Zero-copy where possible**: Minimize allocations in hot paths

## Testing

Run tests:
```bash
cargo test -p cortex-core
```

Run benchmarks:
```bash
cargo bench -p cortex-core
```

## License

MIT OR Apache-2.0
