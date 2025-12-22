# CortexOS for iOS

Run CortexOS distributed AI on your iPhone!

## Quick Setup

### 1. Build the Rust library
```bash
cd apps/ios
./build.sh
```

### 2. Create Xcode Project

1. Open **Xcode** → **Create new project** → **iOS App**
2. Name: `CortexOS`
3. Interface: **SwiftUI**
4. Language: **Swift**

### 3. Add Swift Files

Copy these files to your Xcode project:
- `CortexOS/CortexOSApp.swift`
- `CortexOS/ContentView.swift`
- `CortexOS/CortexBridge.swift`
- `CortexOS/cortex.h`

### 4. Create Bridging Header

1. In Xcode: **File → New → Header File**
2. Name it: `CortexOS-Bridging-Header.h`
3. Add this content:
```c
#import "cortex.h"
```

4. In **Build Settings** → **Swift Compiler - General**:
   - Set **Objective-C Bridging Header** to: `$(PROJECT_DIR)/CortexOS-Bridging-Header.h`

### 5. Link the Library

1. Drag `lib/simulator/libcortex_ios_ffi.a` to your project (for Simulator)
   - Or `lib/device/libcortex_ios_ffi.a` for real device

2. In **Build Phases** → **Link Binary With Libraries**, add:
   - `libcortex_ios_ffi.a`
   - `libresolv.tbd`
   - `Security.framework`

### 6. Build Settings

In **Build Settings**:
- **Library Search Paths**: Add `$(PROJECT_DIR)/lib/simulator` (or `/device`)
- **Enable Bitcode**: `No`

### 7. Run!

1. Select your iPhone or Simulator
2. Build and Run (⌘R)

## Features

- 🧠 **AI Chat** - Query the distributed LLM swarm
- 🌐 **Network** - See connected peers
- ⚙️ **Settings** - Configure contribution

## Architecture

```
┌─────────────────────────────────┐
│  Swift UI (ContentView.swift)   │
├─────────────────────────────────┤
│  CortexBridge.swift             │
├─────────────────────────────────┤
│  cortex.h (C FFI)               │
├─────────────────────────────────┤
│  libcortex_ios_ffi.a (Rust)     │
│  - Discovery                    │
│  - Tensor Processing            │
│  - Agent System                 │
└─────────────────────────────────┘
```

## Troubleshooting

### "Undefined symbols for architecture arm64"
- Make sure you're linking the correct library (simulator vs device)

### "No such module 'cortex'"
- Check that the Bridging Header is configured correctly

### Build fails with linker errors
- Add `libresolv.tbd` to Link Binary With Libraries

