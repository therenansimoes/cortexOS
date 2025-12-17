#!/bin/bash
set -e

# Build script for iOS app
# Compiles Rust FFI and generates Swift bindings

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."
FFI_CRATE="$PROJECT_ROOT/crates/ios-ffi"

echo "🔨 Building CortexOS iOS FFI..."

# Install iOS targets
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim

# Build for device
cargo build -p cortex-ios-ffi --target aarch64-apple-ios --release

# Build for simulator
cargo build -p cortex-ios-ffi --target aarch64-apple-ios-sim --release

# Generate Swift bindings with UniFFI
cargo run --manifest-path "$FFI_CRATE/Cargo.toml" \
  --example cortex_generate_bindings -- \
  generate --language swift \
  "$FFI_CRATE/cortex.udl" \
  --out-dir "$SCRIPT_DIR/ios/CortexOS/Generated"

echo "✅ FFI compiled and bindings generated!"
echo ""
echo "Next steps:"
echo "1. Open $SCRIPT_DIR/ios/CortexOS/CortexOS.xcodeproj in Xcode"
echo "2. Build and run on simulator or device"
echo "3. Configure EAS for publishing (optional)"
