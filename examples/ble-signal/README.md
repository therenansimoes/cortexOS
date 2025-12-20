# BLE Signal Example

This example demonstrates how to use CortexOS's BLE signal layer to emit and receive signals via Bluetooth Low Energy.

## Overview

The BLE signal layer allows CortexOS nodes to communicate using Bluetooth Low Energy advertisements. This enables:

- **Device-to-device communication** without internet connectivity
- **Low-power signaling** suitable for battery-powered devices
- **Proximity-based networking** for local swarm coordination

## Requirements

### Hardware
- Bluetooth Low Energy capable adapter (Bluetooth 4.0 or later)
- Linux with BlueZ stack (for Linux systems)
- Appropriate BLE support for your platform

### Software
- Rust 1.70 or later
- System packages:
  ```bash
  # On Ubuntu/Debian
  sudo apt install libdbus-1-dev pkg-config
  
  # On Fedora
  sudo dnf install dbus-devel pkgconf-pkg-config
  ```

### Permissions
On Linux, you may need to run with elevated privileges or add your user to the `bluetooth` group:
```bash
sudo usermod -a -G bluetooth $USER
```

## Usage

### Emitting BLE Signals

The `ble_emit` example shows how to broadcast signals via BLE:

```bash
cargo run --example ble_emit --features ble
```

This will:
1. Initialize a BLE emitter
2. Emit various signal patterns
3. Demonstrate both direct pattern emission and symbol-based emission

### Receiving BLE Signals

The `ble_receive` example shows how to scan for BLE signals:

```bash
cargo run --example ble_receive --features ble
```

This will:
1. Initialize a BLE receiver
2. Continuously scan for CortexOS BLE signals
3. Decode and display received patterns

### Running Both

For a complete demonstration, run both examples in separate terminals:

```bash
# Terminal 1
cargo run --example ble_receive --features ble

# Terminal 2
cargo run --example ble_emit --features ble
```

## Signal Format

BLE signals use manufacturer-specific data in advertisement packets:

```
Byte 0-1:  Manufacturer ID (0xFFFF for CortexOS)
Byte 2:    Pulse count (0-255)
Bytes 3+:  Pulse data (5 bytes per pulse)
           - Byte 0: On/off (1/0)
           - Bytes 1-4: Duration in microseconds (little-endian u32)
```

### Size Limits

Due to BLE advertisement size constraints (27 bytes for manufacturer data):
- Maximum of 4 pulses per signal
- Total payload: 3 bytes header + (4 pulses × 5 bytes) = 23 bytes

For more complex signals, use multiple advertisements or implement fragmentation.

## Platform-Specific Notes

### Linux
- Uses BlueZ via D-Bus
- Requires `libdbus-1-dev`
- May require root or bluetooth group membership

### macOS/iOS
- Uses CoreBluetooth framework
- Requires appropriate entitlements in production
- Advertising API is more restricted

### Windows
- Uses Windows BLE APIs
- Requires Windows 10 or later
- May need developer mode enabled

### WASM
- BLE is **not available** on WASM targets
- The `ble` feature is automatically disabled for `target_arch = "wasm32"`

## Implementation Status

⚠️ **Current Status**: The implementation provides the encoding/decoding infrastructure and basic BLE adapter initialization. Full BLE advertising/scanning requires platform-specific extensions:

- **Encoding/Decoding**: ✅ Complete
- **BLE Adapter Init**: ✅ Complete
- **Linux Advertising**: ⚠️ Requires BlueZ D-Bus integration
- **Scanning**: ⚠️ Simplified implementation
- **macOS/iOS**: ⚠️ Requires CoreBluetooth peripheral mode
- **Windows**: ⚠️ Requires Windows BLE advertising APIs

## Next Steps

To complete the BLE implementation:

1. **Platform-Specific Advertising**:
   - Linux: Use BlueZ D-Bus API for LE advertising
   - macOS/iOS: Implement CBPeripheralManager
   - Windows: Use Windows.Devices.Bluetooth.Advertisement

2. **Full Scanning Support**:
   - Process `CentralEvent` stream from btleplug
   - Filter manufacturer data
   - Decode advertisements in real-time

3. **Power Management**:
   - Implement advertisement intervals
   - Add scan duty cycling
   - Support for connectable advertisements

## Troubleshooting

### "No BLE adapter found"
- Ensure Bluetooth is enabled
- Check `bluetoothctl` (Linux) or System Preferences (macOS)
- Verify hardware with `hciconfig` (Linux)

### "Permission denied"
- Add user to bluetooth group (Linux)
- Run with `sudo` (not recommended for production)
- Check D-Bus permissions

### "libdbus-1-dev not found"
- Install development packages (see Requirements above)

## Related Documentation

- [CortexOS Signal Layer](../../crates/signal/README.md)
- [BLE Specification](https://www.bluetooth.com/specifications/specs/)
- [btleplug Documentation](https://docs.rs/btleplug/)
