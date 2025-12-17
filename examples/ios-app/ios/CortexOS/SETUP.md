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
