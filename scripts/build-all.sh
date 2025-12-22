#!/bin/bash
# CortexOS Cross-Platform Build Script
# Builds peer binaries for all supported platforms

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║          🔧 CortexOS Cross-Platform Build                    ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Create output directory
mkdir -p target/releases

# Current platform build (fastest, most reliable)
echo "📦 Building for current platform..."
cargo build --release -p cortex-peer
cp target/release/cortex-peer target/releases/

# Detect current OS
OS=$(uname -s)
ARCH=$(uname -m)
echo "   Current: ${OS}-${ARCH}"

# macOS builds
if [[ "$OS" == "Darwin" ]]; then
    echo ""
    echo "🍎 macOS builds..."
    
    # Native (already built above)
    if [[ "$ARCH" == "arm64" ]]; then
        cp target/release/cortex-peer "target/releases/cortex-peer-macos-arm64"
        echo "   ✅ macOS ARM64 (Apple Silicon)"
        
        # Build for Intel Mac (cross-compile)
        if rustup target list | grep -q "x86_64-apple-darwin (installed)"; then
            echo "   Building macOS x86_64..."
            cargo build --release -p cortex-peer --target x86_64-apple-darwin
            cp target/x86_64-apple-darwin/release/cortex-peer "target/releases/cortex-peer-macos-x64"
            echo "   ✅ macOS x86_64 (Intel)"
        else
            echo "   ⚠️ Run: rustup target add x86_64-apple-darwin"
        fi
    else
        cp target/release/cortex-peer "target/releases/cortex-peer-macos-x64"
        echo "   ✅ macOS x86_64 (Intel)"
        
        # Build for Apple Silicon (cross-compile)
        if rustup target list | grep -q "aarch64-apple-darwin (installed)"; then
            echo "   Building macOS ARM64..."
            cargo build --release -p cortex-peer --target aarch64-apple-darwin
            cp target/aarch64-apple-darwin/release/cortex-peer "target/releases/cortex-peer-macos-arm64"
            echo "   ✅ macOS ARM64 (Apple Silicon)"
        else
            echo "   ⚠️ Run: rustup target add aarch64-apple-darwin"
        fi
    fi
fi

# Linux builds
echo ""
echo "🐧 Linux builds..."

# Linux x86_64
if rustup target list | grep -q "x86_64-unknown-linux-gnu (installed)"; then
    echo "   Building Linux x86_64..."
    cargo build --release -p cortex-peer --target x86_64-unknown-linux-gnu 2>/dev/null || echo "   ⚠️ Linux x86_64 build failed (may need cross-linker)"
    if [ -f target/x86_64-unknown-linux-gnu/release/cortex-peer ]; then
        cp target/x86_64-unknown-linux-gnu/release/cortex-peer "target/releases/cortex-peer-linux-x64"
        echo "   ✅ Linux x86_64"
    fi
else
    echo "   ⚠️ Run: rustup target add x86_64-unknown-linux-gnu"
fi

# Linux ARM64
if rustup target list | grep -q "aarch64-unknown-linux-gnu (installed)"; then
    echo "   Building Linux ARM64..."
    cargo build --release -p cortex-peer --target aarch64-unknown-linux-gnu 2>/dev/null || echo "   ⚠️ Linux ARM64 build failed (may need cross-linker)"
    if [ -f target/aarch64-unknown-linux-gnu/release/cortex-peer ]; then
        cp target/aarch64-unknown-linux-gnu/release/cortex-peer "target/releases/cortex-peer-linux-arm64"
        echo "   ✅ Linux ARM64"
    fi
else
    echo "   ⚠️ Run: rustup target add aarch64-unknown-linux-gnu"
fi

# Windows builds
echo ""
echo "🪟 Windows builds..."

if rustup target list | grep -q "x86_64-pc-windows-gnu (installed)"; then
    echo "   Building Windows x86_64..."
    cargo build --release -p cortex-peer --target x86_64-pc-windows-gnu 2>/dev/null || echo "   ⚠️ Windows build failed (may need mingw-w64)"
    if [ -f target/x86_64-pc-windows-gnu/release/cortex-peer.exe ]; then
        cp target/x86_64-pc-windows-gnu/release/cortex-peer.exe "target/releases/cortex-peer-windows-x64.exe"
        echo "   ✅ Windows x86_64"
    fi
else
    echo "   ⚠️ Run: rustup target add x86_64-pc-windows-gnu"
fi

# iOS library (for Swift/Objective-C integration)
echo ""
echo "📱 iOS builds..."

if rustup target list | grep -q "aarch64-apple-ios (installed)"; then
    echo "   Building iOS ARM64..."
    cargo build --release -p cortex-ios-ffi --target aarch64-apple-ios 2>/dev/null || echo "   ⚠️ iOS build failed"
    if [ -f target/aarch64-apple-ios/release/libcortex_ios_ffi.a ]; then
        cp target/aarch64-apple-ios/release/libcortex_ios_ffi.a "target/releases/libcortex-ios-arm64.a"
        echo "   ✅ iOS ARM64 (device)"
    fi
else
    echo "   ⚠️ Run: rustup target add aarch64-apple-ios"
fi

if rustup target list | grep -q "aarch64-apple-ios-sim (installed)"; then
    echo "   Building iOS Simulator..."
    cargo build --release -p cortex-ios-ffi --target aarch64-apple-ios-sim 2>/dev/null || echo "   ⚠️ iOS Simulator build failed"
    if [ -f target/aarch64-apple-ios-sim/release/libcortex_ios_ffi.a ]; then
        cp target/aarch64-apple-ios-sim/release/libcortex_ios_ffi.a "target/releases/libcortex-ios-sim-arm64.a"
        echo "   ✅ iOS Simulator ARM64"
    fi
else
    echo "   ⚠️ Run: rustup target add aarch64-apple-ios-sim"
fi

# Android builds
echo ""
echo "🤖 Android builds..."

if rustup target list | grep -q "aarch64-linux-android (installed)"; then
    echo "   Building Android ARM64..."
    # Requires Android NDK setup
    cargo build --release -p cortex-peer --target aarch64-linux-android 2>/dev/null || echo "   ⚠️ Android build failed (may need NDK)"
    if [ -f target/aarch64-linux-android/release/cortex-peer ]; then
        cp target/aarch64-linux-android/release/cortex-peer "target/releases/cortex-peer-android-arm64"
        echo "   ✅ Android ARM64"
    fi
else
    echo "   ⚠️ Run: rustup target add aarch64-linux-android"
fi

# Summary
echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                    📦 Build Summary                          ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "Built binaries in target/releases/:"
ls -la target/releases/ 2>/dev/null || echo "  No binaries found"
echo ""
echo "To add missing targets, run:"
echo "  rustup target add <target-triple>"
echo ""
echo "For cross-compilation, you may need:"
echo "  • Linux builds on macOS: brew install FiloSottile/musl-cross/musl-cross"
echo "  • Windows builds: brew install mingw-w64"
echo "  • Android builds: Set up Android NDK and ANDROID_NDK_HOME"
echo ""

