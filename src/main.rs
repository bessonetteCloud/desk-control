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
            log::info!("No active desk connection, initiating new connection...");
            let config = self.config.lock().await;
            let desk_address = config.desk_address.clone();
            drop(config); // Release lock before async operation

            if desk_address.is_none() {
                return Err(anyhow::anyhow!("No desk configured. Please configure a desk first."));
            }

            log::info!("Connecting to desk at address: {:?}", desk_address);
            let desk = DeskController::connect(desk_address).await?;
            *controller = Some(desk);
            log::info!("Desk connection established and cached");
        } else {
            log::info!("Using existing desk connection (no scan needed)");
        }

        Ok(())
    }

    /// Move desk to a specific preset
    async fn move_to_preset(&self, preset: DrinkSize) -> Result<()> {
        log::info!("=== Starting move to {} preset ===", preset.name());

        self.ensure_connected().await?;

        let config = self.config.lock().await;
        let height_mm = config.get_preset(preset);
        drop(config);

        log::info!("Target height: {}mm ({:.1}cm)", height_mm, height_mm as f32 / 10.0);

        let controller = self.desk_controller.lock().await;
        if let Some(desk) = controller.as_ref() {
            log::info!("Sending move command to desk...");
            desk.move_to_height(height_mm).await?;
            log::info!("=== Successfully moved to {} preset ===", preset.name());
        } else {
            log::error!("Controller was None after ensure_connected succeeded - this should not happen!");
            return Err(anyhow::anyhow!("Desk controller unavailable"));
        }

        Ok(())
    }

    /// Get current desk height in millimeters (returns None if not connected)
    async fn get_current_height(&self) -> Option<u16> {
        let controller = self.desk_controller.lock().await;
        if let Some(desk) = controller.as_ref() {
            desk.get_height().await.ok()
        } else {
            None
        }
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
            log::info!("Moving desk to {} preset", preset.name());
            match state.move_to_preset(preset).await {
                Ok(_) => {
                    log::info!("Successfully moved to {} preset", preset.name());
                    show_info_dialog(&format!("Desk moved to {} preset", preset.name()));
                }
                Err(e) => {
                    log::error!("Failed to move to preset {}: {}", preset.name(), e);
                    show_error_dialog(&format!(
                        "Failed to move desk: {}",
                        e
                    ));
                }
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
        #[cfg(target_os = "linux")]
        {
            gtk::main_quit();
        }
        #[cfg(not(target_os = "linux"))]
        {
            std::process::exit(0);
        }
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
    drop(config);

    log::info!("Configured desk: {}", address);

    // Connect to the desk and keep the connection alive for future use
    log::info!("Establishing connection to configured desk...");
    let new_controller = DeskController::connect(Some(address)).await?;
    let mut controller = state.desk_controller.lock().await;
    *controller = Some(new_controller);

    log::info!("Desk connected and ready to use");

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

    // Clone state and runtime for callback (they will be moved)
    let state_for_callback = Arc::clone(&state);
    let runtime_for_callback = Arc::clone(&runtime);

    // Create menu callback
    let callback = Arc::new(AppMenuCallback {
        state: state_for_callback,
        runtime: runtime_for_callback,
    });

    // Create tray app
    let tray_app = TrayApp::new(config, callback)?;

    log::info!("System tray app started");

    // Event loop to process tray events
    #[cfg(target_os = "linux")]
    {
        // On Linux, we need to use GTK's main loop for the tray icon to work
        use std::cell::RefCell;
        use std::rc::Rc;

        let tray_app_rc = Rc::new(RefCell::new(tray_app));
        let tray_app_clone = Rc::clone(&tray_app_rc);

        // Process tray events periodically using GTK's timeout mechanism
        glib::timeout_add_local(Duration::from_millis(100), move || {
            tray_app_clone.borrow().process_events();
            glib::ControlFlow::Continue
        });

        // Update current height display periodically
        let tray_app_height = Rc::clone(&tray_app_rc);
        let state_height = Arc::clone(&state);
        let runtime_height = Arc::clone(&runtime);

        glib::timeout_add_local(Duration::from_secs(5), move || {
            let state = Arc::clone(&state_height);
            let tray_app = Rc::clone(&tray_app_height);

            // Spawn async task to get height (don't capture Rc in the async block)
            let (tx, rx) = std::sync::mpsc::channel();
            runtime_height.spawn(async move {
                if let Some(height_mm) = state.get_current_height().await {
                    let _ = tx.send(height_mm);
                }
            });

            // Schedule UI update on main thread using received height
            glib::timeout_add_local_once(Duration::from_millis(100), move || {
                if let Ok(height_mm) = rx.try_recv() {
                    let height_cm = height_mm as f32 / 10.0;
                    tray_app.borrow().update_current_height(height_cm);
                }
            });

            glib::ControlFlow::Continue
        });

        log::info!("Starting GTK main loop");
        gtk::main();
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        use std::time::Instant;

        // On other platforms, use simple polling loop
        let mut last_height_update = Instant::now();

        loop {
            tray_app.process_events();

            // Update height display every 5 seconds
            if last_height_update.elapsed() >= Duration::from_secs(5) {
                last_height_update = Instant::now();

                let state_clone = Arc::clone(&state);
                let height_future = async move {
                    state_clone.get_current_height().await
                };

                if let Some(height_mm) = runtime.block_on(height_future) {
                    let height_cm = height_mm as f32 / 10.0;
                    tray_app.update_current_height(height_cm);
                    log::debug!("Updated current height: {:.1}cm", height_cm);
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
