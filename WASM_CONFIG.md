# CortexOS WASM Build Configuration

This file documents WASM-specific build configurations and optimizations

## Cargo Configuration

The following configurations are applied for WASM builds in `Cargo.toml`:

### Release Profile Optimizations
```toml
[profile.release]
lto = true              # Link-time optimization for smaller binaries
codegen-units = 1       # Single codegen unit for better optimization
opt-level = "z"         # Optimize for size
```

### WASM-Specific Release Profile
```toml
[profile.wasm-release]
inherits = "release"
strip = true            # Strip debug symbols
opt-level = "z"         # Maximum size optimization
lto = true
codegen-units = 1
panic = "abort"         # Smaller panic handler
```

## Target-Specific Dependencies

### Tokio (Async Runtime)

For WASM targets, tokio is configured with minimal features:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
tokio = { version = "1.35", default-features = false, features = [
    "sync",      # Synchronization primitives
    "macros",    # Async macros
    "io-util",   # IO utilities
    "rt",        # Basic runtime
    "time"       # Time utilities
] }
```

Excluded features (not WASM-compatible):
- `fs` - Filesystem operations
- `net` - Network sockets
- `process` - Process spawning
- `signal` - Signal handling

### Storage Backend

RocksDB is not available on WASM. The storage crate automatically uses in-memory implementations:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
# No rocksdb for wasm - uses MemoryEventStore and MemoryGraphStore
```

## Build Commands

### Library Build
```bash
# Debug
cargo build --target wasm32-wasip1 -p cortex-core

# Release (size-optimized)
cargo build --target wasm32-wasip1 -p cortex-core --release

# With custom profile
cargo build --target wasm32-wasip1 -p cortex-core --profile wasm-release
```

### Binary Build

To avoid workspace feature unification issues, build from the crate directory:

```bash
cd examples/wasm-demo
cargo build --target wasm32-wasip1 --release
```

## Size Targets

| Component | Target Size | Actual Size |
|-----------|-------------|-------------|
| cortex-core (library) | ~760KB | 769KB ✓ |
| wasm-demo (binary) | <1MB | 115KB ✓ |

## Further Optimization

If additional size reduction is needed, use `wasm-opt`:

```bash
# Install binaryen
cargo install wasm-opt

# Optimize WASM binary
wasm-opt -Oz -o optimized.wasm target/wasm32-wasip1/release/your-binary.wasm

# Check size
ls -lh optimized.wasm
```

Expected additional size reduction: 10-30%

## Feature Flags

### Recommended Feature Configuration

For WASM builds, disable heavy features:

```toml
[features]
default = ["native"]
native = ["dep:rocksdb"]  # Native-only features
wasm = []                  # WASM-compatible features only
```

Build with WASM features only:
```bash
cargo build --target wasm32-wasip1 --no-default-features --features wasm
```

## Known Limitations

1. **No filesystem access**: WASM sandbox doesn't provide direct filesystem access
2. **No network sockets**: Use HTTP APIs via JavaScript bindings instead
3. **No process spawning**: WASM runs in a single process
4. **No signal handling**: Not applicable in WASM environment
5. **Limited threading**: WASM doesn't support full threading (yet)

## Performance Considerations

### Memory Usage
- WASM has linear memory model
- Initial memory can be specified at build time
- Memory grows but cannot shrink

### Startup Time
- Smaller binaries = faster startup
- Consider lazy initialization for large data structures

### Runtime Performance
- Near-native performance for compute
- Some overhead for cross-boundary calls (WASM ↔ JavaScript)

## Testing

Run WASM binaries with wasmtime:

```bash
# Install wasmtime
curl https://wasmtime.dev/install.sh -sSf | bash

# Run binary
wasmtime target/wasm32-wasip1/release/wasm-demo.wasm
```

## Continuous Integration

For CI/CD pipelines, add WASM build checks:

```yaml
- name: Install WASM target
  run: rustup target add wasm32-wasip1

- name: Build WASM
  run: cargo build --target wasm32-wasip1 -p cortex-core --release

- name: Check size
  run: |
    SIZE=$(stat -c%s target/wasm32-wasip1/release/libcortex_core.rlib)
    if [ $SIZE -gt 1048576 ]; then
      echo "Binary too large: $SIZE bytes"
      exit 1
    fi
```
