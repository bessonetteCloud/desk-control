mod config;
mod desk;
mod ui;

use anyhow::Result;
use btleplug::api::Peripheral;
use config::{Config, DrinkSize};
use desk::DeskController;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use ui::{TrayApp, MenuCallback};

/// Application state shared between UI and background tasks
struct AppState {
    config: Mutex<Config>,
    desk_controller: Mutex<Option<DeskController>>,
}

impl AppState {
    fn new(config: Config) -> Self {
        Self {
            config: Mutex::new(config),
            desk_controller: Mutex::new(None),
        }
    }

    /// Ensure we're connected to the desk
    async fn ensure_connected(&self) -> Result<()> {
        let mut controller = self.desk_controller.lock().await;

        if controller.is_none() {
            log::info!("Connecting to desk...");
            let config = self.config.lock().await;
            let desk_address = config.desk_address.clone();
            drop(config); // Release lock before async operation

            let desk = DeskController::connect(desk_address).await?;
            *controller = Some(desk);
            log::info!("Connected to desk successfully");
        }

        Ok(())
    }

    /// Move desk to a specific preset
    async fn move_to_preset(&self, preset: DrinkSize) -> Result<()> {
        self.ensure_connected().await?;

        let config = self.config.lock().await;
        let height_mm = config.get_preset(preset);
        drop(config);

        log::info!("Moving to {} preset ({}mm)", preset.name(), height_mm);

        let controller = self.desk_controller.lock().await;
        if let Some(desk) = controller.as_ref() {
            desk.move_to_height(height_mm).await?;
            log::info!("Successfully moved to {} preset", preset.name());
        }

        Ok(())
    }
}

/// Menu callback implementation
struct AppMenuCallback {
    state: Arc<AppState>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl MenuCallback for AppMenuCallback {
    fn on_preset_selected(&self, preset: DrinkSize) {
        let state = Arc::clone(&self.state);
        self.runtime.spawn(async move {
            if let Err(e) = state.move_to_preset(preset).await {
                log::error!("Failed to move to preset {}: {}", preset.name(), e);
                show_error_dialog(&format!(
                    "Failed to move desk: {}",
                    e
                ));
            }
        });
    }

    fn on_configure_desk(&self) {
        log::info!("Configure desk requested");
        let state = Arc::clone(&self.state);
        let runtime = Arc::clone(&self.runtime);

        runtime.spawn(async move {
            match scan_and_configure_desk(state).await {
                Ok(_) => {
                    show_info_dialog("Desk configured successfully!");
                }
                Err(e) => {
                    log::error!("Failed to configure desk: {}", e);
                    show_error_dialog(&format!("Failed to configure desk: {}", e));
                }
            }
        });
    }

    fn on_configure_presets(&self) {
        log::info!("Configure presets requested");
        show_info_dialog(
            "To configure presets, edit the config file at:\n~/.desk-control/config\n\n\
            Heights are in millimeters (e.g., 1050 = 105cm)"
        );
    }

    fn on_quit(&self) {
        log::info!("Quitting application");
        std::process::exit(0);
    }
}

/// Scan for desks and let user select one
async fn scan_and_configure_desk(state: Arc<AppState>) -> Result<()> {
    log::info!("Scanning for desks...");

    let desks = DeskController::scan_for_desks(10).await?;

    if desks.is_empty() {
        return Err(anyhow::anyhow!("No desks found"));
    }

    log::info!("Found {} desk(s)", desks.len());

    // For now, just connect to the first one
    // In a full implementation, you'd show a dialog to select
    let desk = desks.into_iter().next().unwrap();

    let address = if let Ok(Some(props)) = desk.properties().await {
        props.address.to_string()
    } else {
        return Err(anyhow::anyhow!("Could not get desk properties"));
    };

    // Update config with desk address
    let mut config = state.config.lock().await;
    config.desk_address = Some(address.clone());
    config.save()?;

    log::info!("Configured desk: {}", address);

    // Clear existing controller to force reconnect
    let mut controller = state.desk_controller.lock().await;
    *controller = None;

    Ok(())
}

/// Show an error dialog (macOS)
#[cfg(target_os = "macos")]
fn show_error_dialog(message: &str) {
    use cocoa::appkit::NSAlert;
    use cocoa::base::nil;
    use cocoa::foundation::NSString;

    unsafe {
        let alert = NSAlert::alloc(nil);
        let title = NSString::alloc(nil).init_str("Desk Control Error");
        let msg = NSString::alloc(nil).init_str(message);

        let _: () = objc::msg_send![alert, setMessageText: title];
        let _: () = objc::msg_send![alert, setInformativeText: msg];
        let _: () = objc::msg_send![alert, runModal];
    }
}

/// Show an info dialog (macOS)
#[cfg(target_os = "macos")]
fn show_info_dialog(message: &str) {
    use cocoa::appkit::NSAlert;
    use cocoa::base::nil;
    use cocoa::foundation::NSString;

    unsafe {
        let alert = NSAlert::alloc(nil);
        let title = NSString::alloc(nil).init_str("Desk Control");
        let msg = NSString::alloc(nil).init_str(message);

        let _: () = objc::msg_send![alert, setMessageText: title];
        let _: () = objc::msg_send![alert, setInformativeText: msg];
        let _: () = objc::msg_send![alert, runModal];
    }
}

/// Show an error notification (Linux)
#[cfg(target_os = "linux")]
fn show_error_dialog(message: &str) {
    use notify_rust::Notification;

    if let Err(e) = Notification::new()
        .summary("Desk Control Error")
        .body(message)
        .urgency(notify_rust::Urgency::Critical)
        .timeout(5000)
        .show()
    {
        log::error!("Failed to show notification: {}", e);
        eprintln!("Error: {}", message);
    }
}

/// Show an info notification (Linux)
#[cfg(target_os = "linux")]
fn show_info_dialog(message: &str) {
    use notify_rust::Notification;

    if let Err(e) = Notification::new()
        .summary("Desk Control")
        .body(message)
        .urgency(notify_rust::Urgency::Normal)
        .timeout(3000)
        .show()
    {
        log::error!("Failed to show notification: {}", e);
        println!("Info: {}", message);
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn show_error_dialog(message: &str) {
    eprintln!("Error: {}", message);
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn show_info_dialog(message: &str) {
    println!("Info: {}", message);
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Starting Desk Control application");

    // Initialize GTK on Linux (required by tray-icon)
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = gtk::init() {
            anyhow::bail!("Failed to initialize GTK: {}", e);
        }
        log::info!("GTK initialized successfully");
    }

    // Load configuration
    let config = Config::load()?;
    log::info!("Configuration loaded");

    // Create async runtime for background tasks
    let runtime = Arc::new(
        tokio::runtime::Runtime::new()
            .expect("Failed to create Tokio runtime"),
    );

    // Create application state
    let state = Arc::new(AppState::new(config.clone()));

    // Create menu callback
    let callback = Arc::new(AppMenuCallback {
        state,
        runtime,
    });

    // Create tray app
    let tray_app = TrayApp::new(config, callback)?;

    log::info!("System tray app started");

    // Event loop to process tray events
    loop {
        tray_app.process_events();
        std::thread::sleep(Duration::from_millis(100));
    }
}
