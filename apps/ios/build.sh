#!/bin/bash
# Build CortexOS for iOS

set -e

echo "🔧 Building CortexOS for iOS..."

cd "$(dirname "$0")/../.."

# Build for device (ARM64)
echo "📱 Building for iOS Device (aarch64-apple-ios)..."
# Enable Metal for GPU acceleration on device
cargo build -p cortex-ios-ffi --release --target aarch64-apple-ios --features metal

# Build for simulator (ARM64 - M1/M2 Mac)
echo "🖥️  Building for iOS Simulator (aarch64-apple-ios-sim)..."
# Enable Metal for simulator (works on Apple Silicon)
cargo build -p cortex-ios-ffi --release --target aarch64-apple-ios-sim --features metal

# Copy libraries
echo "📦 Copying libraries..."

mkdir -p apps/ios/lib/device
mkdir -p apps/ios/lib/simulator

cp target/aarch64-apple-ios/release/libcortex_ios_ffi.a apps/ios/lib/device/
cp target/aarch64-apple-ios-sim/release/libcortex_ios_ffi.a apps/ios/lib/simulator/

echo ""
echo "✅ Build complete!"
echo ""
echo "📁 Libraries are in:"
echo "   Device:    apps/ios/lib/device/libcortex_ios_ffi.a"
echo "   Simulator: apps/ios/lib/simulator/libcortex_ios_ffi.a"
echo ""
echo "📋 Next steps:"
echo "   1. Open Xcode"
echo "   2. Create new iOS App project named 'CortexOS'"
echo "   3. Copy Swift files from apps/ios/CortexOS/ to your project"
echo "   4. Copy cortex.h to your project"
echo "   5. Add cortex.h to Bridging Header"
echo "   6. Link libcortex_ios_ffi.a (device or simulator)"
echo "   7. Add 'libresolv.tbd' to Frameworks"
echo "   8. Build and run!"
echo ""
