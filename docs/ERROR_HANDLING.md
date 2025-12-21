# Error Handling in CortexOS

This document describes the error handling patterns and best practices used throughout the CortexOS codebase.

## Philosophy

CortexOS follows Rust best practices for error handling:

1. **No panics in production code** - All fallible operations return `Result` types
2. **Descriptive errors** - Each error variant includes context about what went wrong
3. **Type safety** - Each crate has its own error type with domain-specific variants
4. **Propagation** - Use the `?` operator for clean error propagation
5. **Documentation** - All error types and variants are documented

## Error Types by Crate

### Core (`cortex-core`)

**Error Type:** `CoreError`  
**Result Type:** `cortex_core::Result<T>`

Covers runtime operations, event system, agent management, and capability checks.

Key variants:
- `QueueFull` / `QueueEmpty` - Backpressure queue capacity issues
- `AgentNotFound` / `AgentAlreadyRegistered` - Agent lifecycle
- `CapabilityDenied` - Security/permission violations
- `ChannelClosed` - Communication channel failures

### Grid (`cortex-grid`)

**Error Type:** `GridError`  
**Result Type:** `cortex_grid::Result<T>`

Covers P2P networking, peer discovery, handshakes, and message relay.

Key variants:
- `HandshakeFailed` / `InvalidSignature` - Peer authentication
- `PeerNotFound` / `NoPeersAvailable` - Peer management
- `EncryptionError` / `DecryptionError` - Cryptographic operations
- `RelayError` - AirTag-style mesh relay issues

### Signal (`cortex-signal`)

**Error Types:** Multiple specialized error types  
**Result Types:** `SignalResult<T>`, `EmitResult<T>`, `ReceiveResult<T>`, etc.

The signal layer has multiple error types for different subsystems:

- `SignalError` - General signal processing (encoding, decoding, patterns)
- `EmitError` - Signal emission through hardware actuators
- `ReceiveError` - Signal reception through sensors
- `DecodeError` - Pattern decoding and recognition
- `NegotiationError` - Channel quality negotiation
- `RoutingError` - Multi-hop routing through signal mesh

### Storage (`cortex-storage`)

**Error Type:** `StoreError`  
**Result Type:** `cortex_storage::Result<T>`

Covers event log, thought graph persistence, and privacy enforcement.

Key variants:
- `Serialization` / `Deserialization` - Data encoding issues
- `Backend` - RocksDB or other storage backend errors
- `NotFound` - Missing data
- `Integrity` - Hash mismatches or corruption
- `PrivacyViolation` - Attempted breach of privacy rules

### Agent (`cortex-agent`)

**Error Types:** `AgentError`, `IntentionError`  
**Result Types:** `cortex_agent::Result<T>`, `IntentionResult<T>`

Covers agent lifecycle, event handling, and intention management.

Key variants:
- `InitFailed` / `SpawnFailed` - Agent startup issues
- `EventHandlingFailed` - Event processing errors
- `AgentPanicked` - Agent task panic recovery
- `IntentionError::NotFound` - Missing intention

### Sensor (`cortex-sensor`)

**Error Types:** `SensorError`, `BleError`  
**Result Types:** `cortex_sensor::Result<T>`, `BleResult<T>`

Covers hardware sensor abstraction and BLE operations.

Key variants:
- `NotFound` / `AlreadyRunning` - Sensor lifecycle
- `Hardware` - Low-level device errors
- `PermissionDenied` - OS permission issues
- `BleError::AdapterNotAvailable` - BLE adapter status

### Lang (`cortex-lang`)

**Error Types:** `LexError`, `ParseError`, `VMError`, `CompileError`  
**Result Types:** `LexResult<T>`, `ParseResult<T>`, etc.

Covers MindLang language processing from lexing to execution.

Key variants:
- `LexError::UnexpectedChar` - Tokenization errors
- `ParseError::UnexpectedToken` - Grammar violations
- `VMError::UndefinedVariable` - Runtime variable issues
- `CompileError::UnsupportedConstruct` - Code generation limits

### Reputation (`cortex-reputation`)

**Error Type:** `ReputationError`  
**Result Type:** `cortex_reputation::Result<T>`

Covers reputation tracking, trust scores, and gossip protocol.

Key variants:
- `NodeNotFound` / `SkillNotFound` - Missing entities
- `SelfRatingNotAllowed` - Gaming prevention
- `TrustComputationFailed` - Algorithm errors

### Skill (`cortex-skill`)

**Error Type:** `SkillError`  
**Result Type:** `cortex_skill::Result<T>`

Covers skill registration, discovery, and network execution.

Key variants:
- `SkillNotFound` / `NoCapableNode` - Skill discovery
- `ExecutionFailed` / `Timeout` - Remote execution
- `InvalidInput` - Parameter validation

### Inference (`cortex-inference`)

**Error Type:** `InferenceError`  
**Result Type:** `cortex_inference::Result<T>`

Covers local LLM operations and model management.

Key variants:
- `ModelNotLoaded` / `ModelLoadFailed` - Model lifecycle
- `OutOfMemory` - Resource exhaustion
- `ContextLengthExceeded` - Input too long
- `TokenizationError` - Text encoding issues

## Best Practices

### 1. Always Return Results for Fallible Operations

❌ **Bad:**
```rust
pub fn load_config(path: &str) -> Config {
    let content = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&content).unwrap()
}
```

✅ **Good:**
```rust
pub fn load_config(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| StoreError::Io(e))?;
    serde_json::from_str(&content)
        .map_err(|e| StoreError::Deserialization(e.to_string()))
}
```

### 2. Provide Context with Errors

❌ **Bad:**
```rust
Err(CoreError::AgentNotFound)
```

✅ **Good:**
```rust
Err(CoreError::AgentNotFound(format!("Agent '{}' does not exist", agent_id)))
```

### 3. Use the `?` Operator for Propagation

❌ **Bad:**
```rust
match some_operation() {
    Ok(val) => val,
    Err(e) => return Err(e),
}
```

✅ **Good:**
```rust
some_operation()?
```

### 4. Document Error Conditions

```rust
/// Load a model from the specified path.
///
/// # Errors
///
/// Returns `InferenceError::ModelFileNotFound` if the file doesn't exist.
/// Returns `InferenceError::UnsupportedFormat` if the file format is invalid.
/// Returns `InferenceError::OutOfMemory` if loading exhausts available RAM.
pub fn load_model(path: &Path) -> Result<Model> {
    // ...
}
```

### 5. Test Error Cases

```rust
#[test]
fn test_agent_not_found() {
    let runtime = Runtime::new();
    let result = runtime.get_agent("nonexistent");
    assert!(result.is_none());
}

#[test]
fn test_duplicate_agent() {
    let runtime = Runtime::new();
    let agent = TestAgent::new("test");
    runtime.spawn_agent(agent.clone()).await.unwrap();
    let result = runtime.spawn_agent(agent).await;
    assert!(matches!(result, Err(CoreError::AgentAlreadyRegistered(_))));
}
```

### 6. Avoid Unwrap/Expect in Production Code

The only acceptable uses of `.unwrap()` or `.expect()`:

1. **Test code** - Always acceptable in `#[cfg(test)]` blocks
2. **Infallible operations** - When the operation is guaranteed to succeed by construction
   ```rust
   // OK: blake3 hash is always 32 bytes, taking first 16 is safe
   let bytes: [u8; 16] = hash.as_bytes()[..16]
       .try_into()
       .expect("blake3 hash slice is always 32 bytes");
   ```
3. **FFI boundaries** - In iOS/Android FFI code where panics are caught at boundary
4. **Initialization** - In `Default` impl or lazy statics where failure is critical
   ```rust
   impl Default for MdnsDiscovery {
       fn default() -> Self {
           Self::new().unwrap_or_else(|e| {
               panic!("Critical: MdnsDiscovery init failed: {}", e)
           })
       }
   }
   ```

### 7. Handle Partial Comparison (NaN) Safely

❌ **Bad:**
```rust
// Can panic if either score is NaN
routes.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
```

✅ **Good:**
```rust
routes.sort_by(|a, b| {
    a.score.partial_cmp(&b.score)
        .unwrap_or(std::cmp::Ordering::Equal)
});
```

### 8. Convert Between Error Types

Use `From` implementations and the `?` operator for automatic conversion:

```rust
#[derive(Error, Debug)]
pub enum SkillError {
    // Automatic conversion from ReputationError
    #[error("Reputation error: {0}")]
    ReputationError(#[from] cortex_reputation::ReputationError),
}
```

## WASM Compatibility

When targeting WASM/WASI, error handling is even more critical:

- **No panics** - Panics in WASM cause the entire module to trap
- **No `std::io`** - Some I/O operations are not available; check platform capabilities
- **Limited formatting** - Some error formatting may allocate; keep messages concise

## Migration Guide

If you find code that doesn't follow these patterns:

1. Add the appropriate error variant to the crate's error type
2. Replace `.unwrap()` or `.expect()` with proper error handling
3. Add context to the error (what failed, what was expected)
4. Update function signatures to return `Result`
5. Propagate errors using `?`
6. Add tests for the error case

Example migration:

```rust
// Before
fn parse_config(data: &str) -> Config {
    serde_json::from_str(data).unwrap()
}

// After
fn parse_config(data: &str) -> Result<Config> {
    serde_json::from_str(data)
        .map_err(|e| StoreError::Deserialization(
            format!("Failed to parse config: {}", e)
        ))
}
```

## Summary

Good error handling in CortexOS means:
- ✅ Production code returns `Result` for all fallible operations
- ✅ Each crate has its own typed error enum
- ✅ Error messages provide useful context
- ✅ Errors are documented in function docs
- ✅ Test both success and error paths
- ✅ No unwraps/panics except in justified cases (tests, FFI, infallible ops)
