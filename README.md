# Desk Control

A macOS menu bar application to control Linak Bluetooth Low Energy standing desks with Starbucks-themed height presets.

## Features

- **macOS Menu Bar Integration**: Lives in your menu bar for quick access
- **Starbucks Drink Size Presets**: Configure 4 height presets themed as coffee sizes:
  - ☕ **Short** - Typically sitting height (~65cm)
  - 🥤 **Tall** - Mid-level height (~85cm)
  - 🍺 **Grande** - Standing height (~105cm)
  - 🏺 **Venti** - Maximum height (~125cm)
- **Bluetooth LE Control**: Direct communication with Linak desk motors
- **Persistent Configuration**: Settings saved to `~/.desk-control/config`
- **Auto-reconnect**: Automatically connects to your configured desk

## Requirements

- macOS (tested on macOS 10.15+)
- Bluetooth LE support
- Rust 1.70+ (for building from source)
- A Linak-compatible standing desk (e.g., DPG series)

## Installation

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd desk-control

# Build the project
cargo build --release

# The binary will be at target/release/desk-control
```

### Running

```bash
cargo run --release
```

Or run the compiled binary:

```bash
./target/release/desk-control
```

The app will appear in your macOS menu bar as a chair icon (🪑).

## Configuration

### First Time Setup

1. Launch the application
2. Click the menu bar icon
3. Select "Configure Desk..." to scan for and connect to your desk
4. The app will automatically save the desk address to `~/.desk-control/config`

### Customizing Height Presets

Edit the configuration file at `~/.desk-control/config`:

```json
{
  "desk_address": "XX:XX:XX:XX:XX:XX",
  "presets": {
    "short": 650,
    "tall": 850,
    "grande": 1050,
    "venti": 1250
  }
}
```

Heights are in millimeters (e.g., 1050 = 105.0cm).

## Usage

1. **Move to Preset**: Click the menu bar icon and select any drink size
2. **Configure Desk**: Scan for and connect to a new desk
3. **Configure Presets**: View instructions for editing preset heights
4. **Quit**: Exit the application

## Technical Details

### Project Structure

```
src/
├── main.rs           # Application entry point
├── config.rs         # Configuration management
├── desk/
│   ├── mod.rs        # Desk module
│   ├── bluetooth.rs  # BLE communication
│   └── protocol.rs   # Linak protocol implementation
└── ui/
    ├── mod.rs        # UI module
    ├── menu_bar.rs   # macOS menu bar implementation
    └── icons.rs      # Icon management
```

### Linak BLE Protocol

The application communicates with Linak desks using the following BLE characteristics:

- **Service UUID**: `99fa0001-338a-1024-8a49-009c0215f78a`
- **Height Characteristic**: `99fa0021-338a-1024-8a49-009c0215f78a` (read current height)
- **Control Characteristic**: `99fa0002-338a-1024-8a49-009c0215f78a` (send movement commands)

Heights are transmitted in 0.1mm units (e.g., 10500 = 1050mm = 105cm).

### Dependencies

- `btleplug` - Bluetooth LE communication
- `tokio` - Async runtime
- `cocoa` / `objc` - macOS UI framework bindings
- `serde` / `serde_json` - Configuration serialization

## Troubleshooting

### Desk Not Found

- Ensure your desk is powered on and Bluetooth is enabled on your Mac
- Make sure you're not connected to the desk from another application
- Try the "Configure Desk..." option to rescan

### Permission Issues

On macOS, you may need to grant Bluetooth permissions:
1. Go to System Preferences → Security & Privacy → Privacy → Bluetooth
2. Ensure the application has permission

### Connection Timeouts

- The desk may take a few seconds to respond
- If movement commands timeout, try stopping and restarting the desk
- Check that you're within Bluetooth range (~10 meters)

## Development

### Running Tests

```bash
cargo test
```

### Debug Logging

Set the `RUST_LOG` environment variable for verbose output:

```bash
RUST_LOG=debug cargo run
```

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]

## Acknowledgments

- Linak BLE protocol reverse engineering based on community efforts
- Starbucks drink sizes used for fun preset naming (not affiliated with Starbucks)
