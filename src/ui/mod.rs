pub mod icons;
pub mod tray_app;

#[cfg(target_os = "macos")]
pub mod menu_bar;

// Export the cross-platform tray app
pub use tray_app::{TrayApp, MenuCallback};
