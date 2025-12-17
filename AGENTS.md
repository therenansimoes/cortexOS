# CortexOS Development Guide

## Quick Commands

```bash
# Build all crates
cargo build

# Check all crates (faster, no codegen)
cargo check

# Run all tests
cargo test --workspace

# Build WASM (core only)
cargo build --target wasm32-wasip1 -p cortex-core

# Build WASM release (optimized)
cargo build --target wasm32-wasip1 -p cortex-core --release

# Run heartbeat demo
cargo run --example heartbeat

# Run relay mesh demo
cargo run --example relay-demo

# Format code
cargo fmt

# Lint
cargo clippy --workspace
```

## Project Structure

```
cortexos/
├── crates/
│   ├── core/       # Event system, runtime, backpressure, capabilities
│   ├── grid/       # P2P networking, wire protocol, relay mesh (AirTag-style)
│   ├── signal/     # Physical signal layer (BLE, audio, light)
│   ├── storage/    # Event store, Thought Graph, persistence
│   ├── agent/      # Agent framework, lifecycle, built-in agents
│   ├── sensor/     # Hardware abstraction for sensors
│   ├── lang/       # MindLang parser and VM
│   ├── reputation/ # P2P reputation system (EigenTrust)
│   ├── skill/      # Skill framework, routing, task execution
│   └── inference/  # Local LLM inference (llama.cpp)
├── examples/
│   ├── heartbeat/     # Basic event-driven agents demo
│   ├── relay-demo/    # AirTag-style relay mesh demo
│   └── skill-network/ # Decentralized skill-based AI network demo
```

## Crate Dependencies

```
cortex-core (base)
    ↓
cortex-storage, cortex-signal, cortex-grid
    ↓
cortex-agent, cortex-sensor
    ↓
cortex-lang
```

## Key Concepts

### Event Envelope
All communication uses `Event` with: id, timestamp, source, kind, payload, trace.

### Backpressure Policies
- `DropNew`: drop incoming when full
- `DropOld`: drop oldest when full
- `Coalesce(key)`: keep latest per key
- `Sample(n)`: keep 1 of every n
- `Persist`: spill to storage

### Relay Mesh (AirTag-style)
Anonymous message relay with:
- E2E encryption (X25519 + ChaCha20-Poly1305)
- Rotating identities
- TTL/hop count limits
- DHT bulletin board

## Testing

```bash
# Test specific crate
cargo test -p cortex-core

# Test with output
cargo test -- --nocapture

# Test single function
cargo test test_function_name
```

## WASM Notes

- Core compiles to WASM (~760KB release)
- Uses limited tokio features on WASM
- RocksDB not available on WASM (uses MemoryStore)
