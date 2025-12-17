#!/bin/bash
set -e

echo "🔨 Building Rust library..."
cd /Users/renansimoes/Desktop/Projetos/cortexOS
cargo build -p cortex-ios-ffi --target aarch64-apple-ios --release 2>&1 | grep -E "Compiling cortex-ios|Finished|error" || true

echo ""
echo "📱 Opening Xcode..."
cd /Users/renansimoes/Desktop/Projetos/cortexOS/examples/ios-app

# Use Xcode's new project feature
open -a Xcode ios/CortexOS

echo ""
echo "⚠️  No Xcode, você precisa:"
echo "1. File → New → Project..."
echo "2. Selecione iOS → App"
echo "3. Configure:"
echo "   - Product Name: CortexOS"
echo "   - Bundle ID: com.cortexos.app"
echo "   - Language: Swift"
echo "4. Depois clique ▶️"
