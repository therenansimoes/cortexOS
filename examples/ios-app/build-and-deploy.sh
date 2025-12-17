#!/bin/bash
set -e

PROJECT_ROOT="$(cd ../.. && pwd)"
FFI_LIB="$PROJECT_ROOT/target/aarch64-apple-ios/release/libcortex_ios_ffi.a"
APP_DIR="$(pwd)/ios/CortexOS"

echo "📱 Building CortexOS for iPhone 16 Pro Max..."

# Ensure Rust lib is built
if [ ! -f "$FFI_LIB" ]; then
    echo "🔨 Building Rust FFI..."
    cargo build -p cortex-ios-ffi --target aarch64-apple-ios --release \
        --manifest-path "$PROJECT_ROOT/Cargo.toml"
fi

echo "✅ Binary ready: $(ls -lh "$FFI_LIB" | awk '{print $5}')"
echo ""
echo "📲 Opening Xcode..."
echo ""
echo "Setup in Xcode:"
echo "1. Product → Scheme → Edit Scheme"
echo "2. Run → Info tab"
echo "3. Build Configuration: Release"
echo "4. Then Product → Run (⌘R)"
echo ""

open "$APP_DIR"
