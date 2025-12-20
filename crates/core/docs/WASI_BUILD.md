# WASI Build Guide

This document describes how to build and optimize CortexOS for WebAssembly/WASI targets.

## Overview

CortexOS core is designed to compile to WASM/WASI for maximum portability. This enables running the same agent logic in browsers, serverless environments, and embedded systems.

## Current Status

✅ **cortex-core compiles to WASM32-WASI**
- Release build size: ~800KB (rlib)
- WASM demo size: 71KB
- All core features work in WASI
- Tests pass in native and WASI environments

## Building for WASI

### Prerequisites

```bash
# Add WASI target
rustup target add wasm32-wasip1
```

### Build Core Library

```bash
# Development build
cargo build --package cortex-core --target wasm32-wasip1

# Release build (optimized for size)
cargo build --package cortex-core --target wasm32-wasip1 --release
```

### Build WASM Example

```bash
cd examples/wasm-demo
cargo build --target wasm32-wasip1 --release

# Output: target/wasm32-wasip1/release/wasm_demo.wasm
```

## Size Optimization

The workspace is configured for minimal WASM size:

```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
opt-level = "z"      # Optimize for size
```

### Size Metrics

| Component | Size | Target |
|-----------|------|--------|
| cortex-core (rlib) | 809KB | < 1MB |
| wasm-demo (wasm) | 71KB | < 100KB |

## WASI Limitations

### Platform-Specific Features Not Available

1. **Native Threading**: WASI has limited threading support
   - Use async/await instead of OS threads
   - Tokio configured for WASI compatibility

2. **File System**: Restricted access
   - Only WASI-exposed directories are accessible
   - Use capability-based file access

3. **Networking**: Limited socket support
   - Some libp2p transports may not work
   - Grid discovery may need WASI-specific adapters

### Workarounds

**Async Runtime:**
```rust
#[cfg(target_arch = "wasm32")]
use tokio with minimal features: sync, macros, io-util, rt, time
```

**Random Number Generation:**
```rust
getrandom crate with "js" feature for browser compatibility
```

## Testing WASM Builds

### Unit Tests

Tests run in the native environment by default:

```bash
cargo test --package cortex-core
```

### WASM-Specific Testing

Use `wasm-pack` or `wasmtime` for testing:

```bash
# Install wasmtime
curl https://wasmtime.dev/install.sh -sSf | bash

# Run WASM binary
wasmtime target/wasm32-wasip1/release/wasm_demo.wasm
```

## CI Integration

Add WASI build check to CI:

```yaml
- name: Check WASI build
  run: |
    rustup target add wasm32-wasip1
    cargo build --package cortex-core --target wasm32-wasip1 --release
    
- name: Check WASM size
  run: |
    SIZE=$(stat -f%z target/wasm32-wasip1/release/libcortex_core.rlib)
    if [ $SIZE -gt 1048576 ]; then
      echo "Error: WASM build too large: $SIZE bytes (max 1MB)"
      exit 1
    fi
```

## Best Practices

1. **Keep Dependencies Minimal**
   - Avoid OS-specific crates in core
   - Use `#[cfg(not(target_arch = "wasm32"))]` for native-only code

2. **Size-Conscious Development**
   - Monitor binary size after adding features
   - Use `cargo bloat` to analyze size contributors

3. **Feature Flags**
   - Separate native and WASM features
   - Default to minimal feature set for WASM

4. **Testing**
   - Test critical paths in WASI environment
   - Use feature flags to skip unsupported tests

## Future Improvements

- [ ] WASM Component Model support
- [ ] Browser-specific optimizations
- [ ] Streaming compilation for large modules
- [ ] WASM SIMD for performance
- [ ] Further size reduction (target: < 500KB)

## Resources

- [WASI Documentation](https://wasi.dev/)
- [Rust WASM Book](https://rustwasm.github.io/docs/book/)
- [wasmtime Runtime](https://wasmtime.dev/)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)
