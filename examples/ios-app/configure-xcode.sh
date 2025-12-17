#!/bin/bash
set -e

PROJECT_DIR="$(pwd)/ios/CortexOS"
RUST_LIB="/Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios/release"
BRIDGING_HEADER="CortexOS-Bridging-Header.h"

echo "⚙️  Configurando Xcode..."
echo ""

# 1. Create bridging header
cat > "$PROJECT_DIR/CortexOS/$BRIDGING_HEADER" << 'EOF'
#ifndef CortexOS_Bridging_Header_h
#define CortexOS_Bridging_Header_h

void cortex_init(void);
char* cortex_start_agent(const char* name);
char* cortex_send_event(const char* agent_id, const char* payload);

#endif
EOF

echo "✅ Bridging header criado"

# 2. Instructions for manual Xcode config
cat > "$PROJECT_DIR/SETUP.md" << 'EOF'
# Configure Xcode Project

## 1. Build Settings
- Product → Scheme → Edit Scheme
- Build → Pre-actions
- Add script:
```bash
cd /Users/renansimoes/Desktop/Projetos/cortexOS
cargo build -p cortex-ios-ffi --target aarch64-apple-ios --release
```

## 2. Link Rust Library
- Select Project → Build Phases
- Link Binary With Libraries (+)
- Add: `/Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios/release/libcortex_ios_ffi.a`

## 3. Search Paths
- Select Project → Build Settings
- Search: "Library Search Paths"
- Add: `/Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios/release`

## 4. Bridging Header
- Build Settings → Search: "Bridging Header"
- Set: `CortexOS/CortexOS-Bridging-Header.h`

## 5. Run
- Plug iPhone via USB-C
- Select device (top left)
- Click ▶️
EOF

echo "📋 Instruções em: $PROJECT_DIR/SETUP.md"
echo ""
echo "Próximo passo no Xcode:"
echo "1. Select Project 'CortexOS' (left sidebar)"
echo "2. Select Target 'CortexOS'"
echo "3. Go to Build Phases tab"
echo "4. Click + → New Run Script Phase"
echo "5. Paste:"
echo "   cargo build -p cortex-ios-ffi --target aarch64-apple-ios --release --manifest-path /Users/renansimoes/Desktop/Projetos/cortexOS/Cargo.toml"
