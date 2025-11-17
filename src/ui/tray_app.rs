use anyhow::Result;
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};
use image::Rgba;

use crate::config::{Config, DrinkSize};

/// Callback handler for menu item actions
pub trait MenuCallback: Send + Sync {
    fn on_preset_selected(&self, preset: DrinkSize);
    fn on_configure_desk(&self);
    fn on_configure_presets(&self);
    fn on_quit(&self);
}

/// Cross-platform system tray application
pub struct TrayApp {
    _tray_icon: TrayIcon,
    menu_rx: std::sync::mpsc::Receiver<MenuEvent>,
    callback: Arc<dyn MenuCallback>,
    preset_items: Vec<(DrinkSize, MenuItem)>,
    configure_desk_item: MenuItem,
    configure_presets_item: MenuItem,
    quit_item: MenuItem,
}

impl TrayApp {
    /// Create a new system tray application
    pub fn new(config: Config, callback: Arc<dyn MenuCallback>) -> Result<Self> {
        // Create the menu
        let menu = Menu::new();

        // Add title (disabled item)
        let title = MenuItem::new("Desk Control", false, None);
        menu.append(&title)?;

        menu.append(&PredefinedMenuItem::separator())?;

        // Add preset menu items and store them
        let mut preset_items = Vec::new();
        for preset in DrinkSize::all() {
            let height_mm = config.get_preset(preset);
            let height_cm = height_mm as f32 / 10.0;

            let label = format!(
                "{} {} - {:.1}cm",
                get_icon_emoji(preset),
                preset.name(),
                height_cm
            );

            let item = MenuItem::new(label, true, None);
            menu.append(&item)?;
            preset_items.push((preset, item));
        }

        menu.append(&PredefinedMenuItem::separator())?;

        // Add configuration items
        let configure_desk_item = MenuItem::new("Configure Desk...", true, None);
        menu.append(&configure_desk_item)?;

        let configure_presets_item = MenuItem::new("Configure Presets...", true, None);
        menu.append(&configure_presets_item)?;

        menu.append(&PredefinedMenuItem::separator())?;

        // Add quit item
        let quit_item = MenuItem::new("Quit", true, None);
        menu.append(&quit_item)?;

        // Create the tray icon
        let icon = create_tray_icon();
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Desk Control")
            .with_icon(icon)
            .build()?;

        // Get the menu event receiver
        let menu_rx = MenuEvent::receiver();

        Ok(Self {
            _tray_icon: tray_icon,
            menu_rx,
            callback,
            preset_items,
            configure_desk_item,
            configure_presets_item,
            quit_item,
        })
    }

    /// Process menu events (call this in your event loop)
    pub fn process_events(&self) {
        while let Ok(event) = self.menu_rx.try_recv() {
            let item_id = event.id;

            // Check if it's a preset item
            if let Some((preset, _)) = self.preset_items.iter().find(|(_, item)| item.id() == item_id) {
                self.callback.on_preset_selected(*preset);
                continue;
            }

            // Check other items
            if item_id == self.configure_desk_item.id() {
                self.callback.on_configure_desk();
            } else if item_id == self.configure_presets_item.id() {
                self.callback.on_configure_presets();
            } else if item_id == self.quit_item.id() {
                self.callback.on_quit();
            }
        }
    }
}

/// Create a simple tray icon (chair emoji as icon)
fn create_tray_icon() -> tray_icon::Icon {
    // Create a simple 32x32 icon with a chair emoji
    // For a better icon, you'd load a PNG file, but for now we'll create a simple colored square
    let size = 32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];

    // Create a simple blue circle as the icon
    let center = size / 2;
    let radius = size / 3;

    for y in 0..size {
        for x in 0..size {
            let dx = (x as i32 - center as i32).abs();
            let dy = (y as i32 - center as i32).abs();
            let dist_sq = dx * dx + dy * dy;
            let radius_sq = (radius * radius) as i32;

            let idx = ((y * size + x) * 4) as usize;
            if dist_sq < radius_sq {
                // Blue color
                rgba[idx] = 66;      // R
                rgba[idx + 1] = 135; // G
                rgba[idx + 2] = 245; // B
                rgba[idx + 3] = 255; // A
            } else {
                // Transparent
                rgba[idx + 3] = 0;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}

/// Get emoji icon for drink size
fn get_icon_emoji(preset: DrinkSize) -> &'static str {
    match preset {
        DrinkSize::Short => "☕",
        DrinkSize::Tall => "🥤",
        DrinkSize::Grande => "🍺",
        DrinkSize::Venti => "🏺",
    }
}
