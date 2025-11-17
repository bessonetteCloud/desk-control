pub mod icons;

#[cfg(target_os = "macos")]
pub mod menu_bar;

#[cfg(target_os = "macos")]
pub use menu_bar::{MenuBarApp, MenuCallback};
