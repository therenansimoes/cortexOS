# WASM/WASI Build Guide

This document describes how to build CortexOS for WebAssembly/WASI targets.

## Prerequisites

Install the WASM target:
```bash
rustup target add wasm32-wasip1
```

## Building cortex-core for WASM

The `cortex-core` crate is designed to be WASM-compatible and includes target-specific dependency configurations.

### Debug Build
```bash
cargo build --target wasm32-wasip1 -p cortex-core
```

### Release Build (Optimized for Size)
```bash
cargo build --target wasm32-wasip1 -p cortex-core --release
```

## Size Optimization

The release build is configured with aggressive size optimizations:
- `opt-level = "z"` - Optimize for binary size
- `lto = true` - Link-time optimization
- `codegen-units = 1` - Single codegen unit for better optimization

### Current Binary Sizes

| Build Type | Size | Notes |
|------------|------|-------|
| Debug (.rlib) | ~1.8MB | Unoptimized with debug symbols |
| Release (.rlib) | ~770KB | Size-optimized release build |
| Release Binary | ~93KB | Minimal demo binary |

## WASM-Specific Adaptations

### Limited Tokio Features

On WASM targets, tokio is configured with only WASM-compatible features:
- `sync` - Synchronization primitives
- `macros` - Async macros
- `io-util` - IO utilities
- `rt` - Runtime
- `time` - Time utilities

Features like `fs`, `net`, and `process` are excluded as they're not supported on WASM.

### Storage Backend

RocksDB is not available on WASM. The storage layer automatically uses in-memory implementations:
- `MemoryEventStore` instead of `RocksEventStore`
- `MemoryGraphStore` instead of `RocksGraphStore`

These are selected at compile time based on the target architecture.

## Building Examples

To build WASM examples, build from the example directory to avoid workspace feature unification:

```bash
cd examples/wasm-demo
cargo build --target wasm32-wasip1 --release
```

Or use cargo's `--package` flag with `--target-dir`:

```bash
cargo build --target wasm32-wasip1 --package wasm-demo --release --target-dir examples/wasm-demo/target
```

## Running WASM Binaries

WASM binaries can be run using a WASI runtime like wasmtime:

```bash
# Install wasmtime
curl https://wasmtime.dev/install.sh -sSf | bash

# Run the WASM binary
wasmtime target/wasm32-wasip1/release/wasm-demo.wasm
```

## Performance Benchmarks

### Build Performance
- Debug build: ~16s (cold), ~2s (incremental)
- Release build: ~9s (cold), ~2s (incremental)

### Binary Performance
The WASM runtime provides near-native performance for compute-intensive operations while maintaining full portability across platforms.

## Troubleshooting

### Feature Unification Issues

If you encounter errors about unsupported tokio features when building from the workspace root:
```
error: Only features sync,macros,io-util,rt,time are supported on wasm.
```

This is due to Cargo's workspace-level feature unification. Build from the specific crate directory instead:
```bash
cd crates/core
cargo build --target wasm32-wasip1 --release
```

### Size Concerns

If the binary size exceeds 1MB:
1. Ensure you're building with `--release`
2. Check that `opt-level = "z"` is set in `Cargo.toml`
3. Use `wasm-opt` for additional size reduction:
   ```bash
   wasm-opt -Oz -o optimized.wasm target/wasm32-wasip1/release/your-binary.wasm
   ```

## Browser Compatibility

While this guide focuses on WASI (server-side WASM), cortex-core can also be compiled for browser environments using `wasm32-unknown-unknown` target with appropriate JavaScript bindings.
