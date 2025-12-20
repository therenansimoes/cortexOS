# CortexOS Development Guide

## ⚠️ ZERO MOCK POLICY

**This project has a strict NO MOCK policy.**

- All code must use real implementations
- No fake data, no stubs, no simulations
- If a feature can't run on a platform, use compile-time feature flags to exclude it
- Tests should use real components or be marked as integration tests
- iOS/Android FFI must call real Rust code, never return hardcoded strings

When in doubt: **make it real or don't ship it.**

---

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

# Run compiler agent demo
cargo run -p compiler-demo

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
│   ├── skill-network/ # Decentralized skill-based AI network demo
│   └── compiler-demo/ # AI-assisted code generation demo
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

## Built-in Agents

CortexOS includes several built-in agents in the `cortex-agent` crate:

### HeartbeatAgent
Simple agent that emits periodic heartbeat events for health monitoring and testing event-driven communication.

**Capabilities**: `heartbeat`, `health-check`

### LoggerAgent
Records and logs events from the event bus, useful for debugging and monitoring.

**Capabilities**: `logging`, `monitoring`

### RelayAgent
Handles AirTag-style relay mesh communication for offline message propagation.

**Capabilities**: `relay`, `mesh-networking`

### CompilerAgent (NEW in PR #25)
AI-assisted code generation agent that can generate, validate, and check compilation of code in multiple languages.

**Capabilities**: `code-generation`, `compilation`, `code-validation`, `syntax-checking`

**Supported Languages**: Rust, Python, JavaScript, TypeScript

**Quality Metrics**:
- Code quality scoring (>80% target)
- Compilation success checking (>90% target)
- Validation across syntax, documentation, error handling, and conventions

**Usage Example**:
```rust
use cortex_agent::prelude::*;
use cortex_agent::builtin::compiler::{CodeGenRequest, CodeGenResponse};

let mut compiler = CompilerAgent::new()
    .with_name("code-generator");

let request = CodeGenRequest {
    task_description: "Create an HTTP server".to_string(),
    language: "rust".to_string(),
    context: Some("Using standard library".to_string()),
    constraints: vec!["Must handle errors".to_string()],
};

// Send request via event bus
let event = Event::new("compiler.generate", serde_json::to_vec(&request).unwrap());
compiler.on_event(&event, &mut ctx).await?;

// Listen for compiler.response events
```

See `examples/compiler-demo` for a complete demonstration.

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
