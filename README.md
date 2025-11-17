# Desk Control

A cross-platform system tray application to control Linak Bluetooth Low Energy standing desks with Starbucks-themed height presets.

Supports **macOS** and **Linux** (including Wayland).

## Features

- **Cross-Platform System Tray**: Lives in your system tray/notification area for quick access
  - macOS: Menu bar integration
  - Linux: System tray with Wayland and X11 support
- **Starbucks Drink Size Presets**: Configure 4 height presets themed as coffee sizes:
  - ☕ **Short** - Typically sitting height (~65cm)
  - 🥤 **Tall** - Mid-level height (~85cm)
  - 🍺 **Grande** - Standing height (~105cm)
  - 🏺 **Venti** - Maximum height (~125cm)
- **Bluetooth LE Control**: Direct communication with Linak desk motors
- **Persistent Configuration**: Settings saved to `~/.desk-control/config`
- **Auto-reconnect**: Automatically connects to your configured desk
- **Native Notifications**: Desktop notifications on both macOS and Linux

## Requirements

### General
- Bluetooth LE support
- Rust 1.70+ (for building from source)
- A Linak-compatible standing desk (e.g., DPG series)

### Platform-Specific

#### macOS
- macOS 10.15+

#### Linux (Arch, Ubuntu, Fedora, etc.)
- Bluetooth daemon (`bluez`)
- D-Bus
- System tray support:
  - **Wayland**: Compositor with system tray support (GNOME with AppIndicator extension, KDE Plasma, Sway, etc.)
  - **X11**: Any desktop environment with system tray
- libayatana-appindicator3 (or libappindicator3)
- notification daemon (for desktop notifications)

## Installation

### Linux (Arch Linux)

#### Install System Dependencies

```bash
# On Arch Linux - Runtime and build dependencies
sudo pacman -S bluez bluez-utils libappindicator-gtk3 dbus gtk3 pkg-config

# Start and enable Bluetooth service
sudo systemctl start bluetooth
sudo systemctl enable bluetooth

# Add your user to the bluetooth group
sudo usermod -a -G bluetooth $USER
# Log out and back in for group changes to take effect
```

**For other distributions:**

Ubuntu/Debian:
```bash
sudo apt install bluez libbluetooth-dev libdbus-1-dev libappindicator3-dev libgtk-3-dev pkg-config
```

Fedora:
```bash
sudo dnf install bluez bluez-libs-devel dbus-devel libappindicator-gtk3-devel gtk3-devel pkgconf-pkg-config
```

For **GNOME on Wayland**, install the AppIndicator extension:
```bash
# Install GNOME Shell extension for AppIndicator support
yay -S gnome-shell-extension-appindicator
# Or install via https://extensions.gnome.org/extension/615/appindicator-support/
```

#### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd desk-control

# Build the project
cargo build --release

# The binary will be at target/release/desk-control
```

### macOS

#### Build from Source

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

The app will appear in your system tray:
- **macOS**: Menu bar with a blue circle icon
- **Linux**: System tray with a blue circle icon

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

**Cross-platform:**
- `btleplug` - Bluetooth LE communication
- `tokio` - Async runtime
- `tray-icon` - Cross-platform system tray
- `serde` / `serde_json` - Configuration serialization

**macOS-specific:**
- `cocoa` / `objc` - macOS UI framework bindings

**Linux-specific:**
- `notify-rust` - Desktop notifications

## Troubleshooting

### Desk Not Found

- Ensure your desk is powered on and Bluetooth is enabled
- Make sure you're not connected to the desk from another application
- Try the "Configure Desk..." option to rescan

### Permission Issues

**On macOS:**
1. Go to System Preferences → Security & Privacy → Privacy → Bluetooth
2. Ensure the application has permission

**On Linux:**
- Ensure your user is in the `bluetooth` group: `groups | grep bluetooth`
- If not, add yourself: `sudo usermod -a -G bluetooth $USER` (then log out and back in)
- Ensure the Bluetooth service is running: `sudo systemctl status bluetooth`
- Check permissions: `ls -l /dev/rfkill` (you may need to add udev rules)

### System Tray Not Visible (Linux)

**GNOME on Wayland:**
- Install the AppIndicator extension: https://extensions.gnome.org/extension/615/appindicator-support/
- Enable it in GNOME Extensions app

**KDE Plasma:**
- Right-click on the panel → Configure Panel → Add Widgets → System Tray
- Ensure "Status Notifier Items" is enabled

**Sway/i3:**
- Ensure you have a status bar configured (waybar, i3status, etc.)
- Add system tray module to your bar configuration

### Notifications Not Working (Linux)

- Ensure a notification daemon is running
- For GNOME: Should work out of the box
- For other WMs: Install `dunst`, `mako`, or similar notification daemon

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
