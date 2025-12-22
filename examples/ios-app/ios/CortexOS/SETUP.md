# CortexOS iOS Setup

## Quick Start (Just Build & Run!)

The project is already configured. Just:

1. **Build the Rust library** (if not already done):
```bash
cd /Users/renansimoes/Desktop/Projetos/cortexOS
cargo build -p cortex-ios-ffi --release --target aarch64-apple-ios      # For real iPhone
cargo build -p cortex-ios-ffi --release --target aarch64-apple-ios-sim  # For Simulator
```

2. **Open Xcode**:
```bash
open /Users/renansimoes/Desktop/Projetos/cortexOS/examples/ios-app/ios/CortexOS/CortexOS.xcodeproj
```

3. **Select target**:
   - For **iPhone Simulator**: Change library search path (see below)
   - For **Real iPhone**: Just plug in and run!

4. **Build and Run** (⌘R)

## For Simulator Testing (M1/M2/M3 Mac)

The default config is for real devices. To test on Simulator:

1. In Xcode → Select project in navigator
2. **Build Settings** → Search "Library Search Paths"
3. Change from:
   ```
   /Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios/release/**
   ```
   To:
   ```
   /Users/renansimoes/Desktop/Projetos/cortexOS/target/aarch64-apple-ios-sim/release/**
   ```

4. Select **iPhone 15 Pro** simulator (or any)
5. Build and Run (⌘R)

## For Real iPhone

1. Plug in your iPhone via USB-C
2. Trust the computer on your iPhone
3. In Xcode: Select your iPhone from the device list
4. Build and Run (⌘R)

## What's Already Configured

✅ Bridging Header: `CortexOS/CortexOS-Bridging-Header.h`
✅ Library linked: `libcortex_ios_ffi.a`
✅ Library search paths set
✅ All Swift files ready

## Troubleshooting

### "Library not found for -lcortex_ios_ffi"
Run: `cargo build -p cortex-ios-ffi --release --target aarch64-apple-ios`

### "Undefined symbols for architecture arm64"
Make sure you're using the right library:
- Simulator (M1/M2/M3): `aarch64-apple-ios-sim`
- Real device: `aarch64-apple-ios`

### App crashes on launch
Check Console.app for crash logs. Make sure `cortex_init()` is called first.
