# Core Event System Performance

## Overview

The CortexOS core event system is designed for high-throughput, low-latency event processing with comprehensive backpressure policies to ensure reliability under load.

## Performance Metrics

### Event Throughput

Based on benchmark results (see `benches/event_throughput.rs`):

- **Single Event Publishing**: ~1.4M events/sec
- **Batch Publishing**: ~1.4M events/sec
- **With Pattern Matching**: ~1.3M events/sec

All measurements exceed the requirement of **100K events/sec** by over 10x.

### Test Coverage

- **Overall Core Coverage**: 87.1%
- **Event Module**: 93.9%
- **Backpressure Module**: 84.7%
- **Capability Module**: 93.2%
- **Runtime Module**: 80.7%
- **ID Module**: 100%

Coverage exceeds the **80% requirement**.

## Key Features

### Batch Publishing

The event system supports batch publishing for improved performance:

```rust
let events: Vec<Event> = /* create events */;
let published = runtime.publish_batch(&events)?;
```

Batch publishing reduces lock contention by acquiring the subscriptions lock once for multiple events.

### Backpressure Policies

The system implements multiple backpressure policies to handle load:

- **DropNew**: Drop incoming events when queue is full
- **DropOld**: Drop oldest events to make room for new ones
- **Coalesce**: Keep latest event per key (useful for sensor data)
- **Sample(n)**: Keep 1 out of every n events
- **Persist**: Spill to storage when queue is full (planned)

### Pattern Matching

Efficient pattern matching for event subscriptions:

- Exact match: `"sensor.mic.v1"`
- Prefix match: `"sensor.*"`
- Wildcard: `"*"`

## Optimization Techniques

1. **Lock-free where possible**: Uses `DashMap` for concurrent agent access
2. **Channel-based communication**: Tokio mpsc/broadcast channels for async event delivery
3. **Minimal allocations**: Reuses data structures where possible
4. **Batch operations**: Single lock acquisition for multiple events

## Reliability Features

1. **Comprehensive error handling**: All failure modes return typed errors
2. **Agent lifecycle management**: Proper start/stop hooks
3. **Graceful shutdown**: Coordinated cleanup of all agents
4. **Backpressure policies**: Configurable behavior under load
5. **Event tracing**: Optional trace IDs for debugging

## Usage Recommendations

### For High Throughput

```rust
// Use batch publishing for bursts of events
let events = collect_events();
runtime.publish_batch(&events)?;
```

### For Memory-Constrained Systems

```rust
// Use sampling or coalescing to reduce memory usage
let queue = PolicyQueue::new(BackpressurePolicy::Sample(10), 1000);
```

### For Reliability

```rust
// Use DropOld to never lose the latest data
let queue = PolicyQueue::new(BackpressurePolicy::DropOld, capacity);
```

## Future Improvements

- [ ] Zero-copy event passing where possible
- [ ] Lock-free subscription registry
- [ ] Event batching at the publisher side
- [ ] Persistent queue implementation for Persist policy
- [ ] Performance profiling tools
