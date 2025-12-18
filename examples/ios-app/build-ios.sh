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

# Copy the static library to the iOS project
# Note: We are using the Device build (aarch64-apple-ios). 
# If running on Simulator (M1/M2), you might need to switch this to aarch64-apple-ios-sim
cp "$PROJECT_ROOT/target/aarch64-apple-ios/release/libcortex_ios_ffi.a" "$SCRIPT_DIR/ios/CortexOS/CortexOS/"

echo "✅ FFI compiled and library copied!"
echo ""
echo "Next steps:"
echo "1. Open $SCRIPT_DIR/ios/CortexOS/CortexOS.xcodeproj in Xcode"
echo "2. Build and run on your iPhone"
echo "   (Note: If running on Simulator, you may need to link the simulator build instead)"

# Legacy UniFFI step removed (we use manual FFI now)
# cargo run --manifest-path "$FFI_CRATE/Cargo.toml" ...
