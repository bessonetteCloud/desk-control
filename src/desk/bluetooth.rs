use anyhow::{anyhow, Context, Result};
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::time::Duration;
use tokio::time::sleep;

use super::protocol::{
    parse_height, CONTROL_CHARACTERISTIC_UUID, CONTROL_SERVICE_UUID, HEIGHT_CHARACTERISTIC_UUID,
    MovementCommand,
};

pub struct DeskController {
    peripheral: Peripheral,
    control_char: Option<Characteristic>,
    height_char: Option<Characteristic>,
}

impl DeskController {
    /// Scan for available Linak desks
    pub async fn scan_for_desks(timeout_secs: u64) -> Result<Vec<Peripheral>> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;

        let central = adapters
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No Bluetooth adapters found"))?;

        log::info!("Starting BLE scan for Linak desks...");
        central.start_scan(ScanFilter::default()).await?;

        sleep(Duration::from_secs(timeout_secs)).await;

        let peripherals = central.peripherals().await?;
        log::info!("Found {} BLE devices", peripherals.len());

        // Filter for Linak desks (they advertise the control service)
        let mut desks = Vec::new();
        for peripheral in peripherals {
            if let Ok(Some(properties)) = peripheral.properties().await {
                if let Some(name) = properties.local_name {
                    // Linak desks usually have "Desk" or "DPG" in their name
                    if name.to_lowercase().contains("desk")
                        || name.to_lowercase().contains("dpg")
                        || name.to_lowercase().contains("linak")
                    {
                        log::info!("Found potential Linak desk: {}", name);
                        desks.push(peripheral);
                    }
                }
            }
        }

        central.stop_scan().await?;
        Ok(desks)
    }

    /// Connect to a specific desk by address or first available desk
    pub async fn connect(desk_address: Option<String>) -> Result<Self> {
        // Scan for desks - do this once to find the peripheral
        let scan_duration = if desk_address.is_some() { 5u64 } else { 10u64 };
        log::info!("Scanning for desks for {} seconds...", scan_duration);

        let desks = Self::scan_for_desks(scan_duration).await?;

        if desks.is_empty() {
            return Err(anyhow!("No Linak desks found"));
        }

        // Find the peripheral matching the desk address
        let peripheral = if let Some(ref addr) = desk_address {
            log::info!("Searching for desk with address: {}", addr);
            let mut found_peripheral = None;

            for p in desks {
                match p.properties().await {
                    Ok(Some(props)) => {
                        let p_addr = props.address.to_string();
                        log::debug!("Checking peripheral with address: {}", p_addr);
                        if p_addr == *addr {
                            log::info!("Found matching desk with address: {}", p_addr);
                            found_peripheral = Some(p);
                            break;
                        }
                    }
                    Ok(None) => {
                        log::debug!("Peripheral has no properties");
                    }
                    Err(e) => {
                        log::debug!("Failed to get peripheral properties: {}", e);
                    }
                }
            }

            match found_peripheral {
                Some(p) => {
                    log::info!("Selected desk peripheral for connection");
                    p
                },
                None => {
                    return Err(anyhow!("Desk with address {} not found", addr));
                }
            }
        } else {
            log::info!("No desk address specified, connecting to first available desk");
            desks
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("No desks available"))?
        };

        // Wait a moment after scanning to let BLE stack settle
        log::info!("Waiting for BLE stack to settle after scan...");
        sleep(Duration::from_millis(1000)).await;

        // Try to connect to the peripheral with retries (but don't rescan)
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 1..=max_retries {
            if attempt > 1 {
                log::info!("Connection retry attempt {} of {}", attempt, max_retries);
                // Wait longer between retries to let BLE stack settle
                sleep(Duration::from_secs(2)).await;
            }

            // Try to connect to the peripheral
            log::info!("Attempting to connect to peripheral (attempt {})...", attempt);
            match Self::connect_to_peripheral(peripheral.clone()).await {
                Ok(controller) => {
                    log::info!("Successfully connected on attempt {}", attempt);
                    return Ok(controller);
                }
                Err(e) => {
                    log::error!("Connection attempt {} failed: {}", attempt, e);

                    // Try to ensure we're disconnected before retry
                    if let Ok(true) = peripheral.is_connected().await {
                        log::info!("Disconnecting before retry...");
                        let _ = peripheral.disconnect().await;
                        sleep(Duration::from_millis(500)).await;
                    }

                    last_error = Some(e);
                    if attempt < max_retries {
                        log::warn!("Will retry connection...");
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Failed to connect to desk after {} attempts", max_retries)))
    }

    /// Connect to a specific peripheral
    async fn connect_to_peripheral(peripheral: Peripheral) -> Result<Self> {
        use tokio::time::timeout;

        log::info!("Entered connect_to_peripheral function");

        // Check connection status
        log::info!("Checking desk connection status...");
        let is_connected = timeout(Duration::from_secs(5), peripheral.is_connected())
            .await
            .context("Timeout checking connection status")?
            .context("Failed to check connection status")?;
        log::info!("Connection status check completed: is_connected = {}", is_connected);

        // Connect to the peripheral if not connected
        if !is_connected {
            log::info!("Desk not connected, establishing connection...");
            match timeout(Duration::from_secs(15), peripheral.connect()).await {
                Ok(Ok(())) => {
                    log::info!("Bluetooth connection established successfully");
                }
                Ok(Err(e)) => {
                    log::error!("Bluetooth connection failed: {}", e);
                    return Err(anyhow!("Failed to connect to desk: {}", e));
                }
                Err(_) => {
                    log::error!("Bluetooth connection timed out after 15 seconds");
                    return Err(anyhow!("Timeout connecting to desk (15s)"));
                }
            }
        } else {
            log::info!("Desk already connected");
        }

        // Discover services
        log::info!("Discovering desk services and characteristics...");
        timeout(Duration::from_secs(10), peripheral.discover_services())
            .await
            .context("Timeout discovering services (10s)")?
            .context("Failed to discover services")?;
        log::info!("Services discovered successfully");

        // Find the control service and characteristics
        let chars = peripheral.characteristics();
        log::info!("Found {} characteristics total", chars.len());

        let control_char = chars
            .iter()
            .find(|c| c.uuid == CONTROL_CHARACTERISTIC_UUID)
            .cloned();

        let height_char = chars
            .iter()
            .find(|c| c.uuid == HEIGHT_CHARACTERISTIC_UUID)
            .cloned();

        if control_char.is_none() {
            log::error!("Could not find control characteristic (UUID: {})", CONTROL_CHARACTERISTIC_UUID);
            log::error!("Available characteristics: {:?}", chars.iter().map(|c| c.uuid).collect::<Vec<_>>());
            return Err(anyhow!("Could not find control characteristic on desk"));
        }

        if height_char.is_none() {
            log::error!("Could not find height characteristic (UUID: {})", HEIGHT_CHARACTERISTIC_UUID);
            return Err(anyhow!("Could not find height characteristic on desk"));
        }

        log::info!("Desk controller fully initialized and ready");

        Ok(Self {
            peripheral,
            control_char,
            height_char,
        })
    }

    /// Get the current desk height in millimeters
    pub async fn get_height(&self) -> Result<u16> {
        let height_char = self
            .height_char
            .as_ref()
            .ok_or_else(|| anyhow!("Height characteristic not available"))?;

        log::debug!("Reading height characteristic...");
        let data = self.peripheral.read(height_char).await
            .context("Failed to read height characteristic from BLE")?;

        log::info!("Read {} bytes from height characteristic: {:02X?}", data.len(), data);

        let height_units = parse_height(&data)
            .ok_or_else(|| anyhow!("Failed to parse height data from bytes: {:?}", data))?;

        let height_mm = super::protocol::desk_units_to_mm(height_units);
        log::info!("Parsed height: {} units = {}mm (bytes: {:02X?})", height_units, height_mm, data);

        Ok(height_mm)
    }

    /// Send a movement command to the desk
    pub async fn send_command(&self, command: MovementCommand) -> Result<()> {
        let control_char = self
            .control_char
            .as_ref()
            .ok_or_else(|| anyhow!("Control characteristic not available"))?;

        let bytes = command.to_bytes();
        log::info!("Sending command: {:?} -> bytes: {:02X?}", command, bytes);

        self.peripheral
            .write(control_char, &bytes, WriteType::WithoutResponse)
            .await
            .context("Failed to write command to BLE characteristic")?;

        log::info!("Command written to BLE characteristic successfully");

        Ok(())
    }

    /// Move desk to a specific height in millimeters
    pub async fn move_to_height(&self, height_mm: u16) -> Result<()> {
        let height_units = super::protocol::mm_to_desk_units(height_mm);
        log::info!("Moving desk to {}mm ({}units)", height_mm, height_units);

        self.send_command(MovementCommand::MoveToHeight(height_units))
            .await?;

        log::info!("Move command sent successfully, waiting for movement to start...");

        // Wait for movement to start
        sleep(Duration::from_millis(100)).await;

        // Poll until we reach the target height (with tolerance)
        const TOLERANCE_MM: u16 = 5; // 5mm tolerance
        const MAX_WAIT_SECS: u64 = 30;
        const POLL_INTERVAL_MS: u64 = 200;

        let start = std::time::Instant::now();
        let mut poll_count = 0;

        log::info!("Starting height polling (target: {}mm, tolerance: {}mm, max wait: {}s)",
                   height_mm, TOLERANCE_MM, MAX_WAIT_SECS);

        loop {
            poll_count += 1;

            if start.elapsed().as_secs() > MAX_WAIT_SECS {
                log::error!("Timeout after {} polls and {} seconds", poll_count, MAX_WAIT_SECS);
                return Err(anyhow!("Timeout waiting for desk to reach target height"));
            }

            match self.get_height().await {
                Ok(current) => {
                    let diff = if current > height_mm {
                        current - height_mm
                    } else {
                        height_mm - current
                    };

                    if poll_count <= 3 || poll_count % 10 == 0 {
                        log::info!("Poll #{}: Current height: {}mm, Target: {}mm, Diff: {}mm",
                                   poll_count, current, height_mm, diff);
                    }

                    if diff <= TOLERANCE_MM {
                        log::info!("Reached target height after {} polls: {}mm (target: {}mm, diff: {}mm)",
                                   poll_count, current, height_mm, diff);
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Failed to read height on poll #{}: {}", poll_count, e);
                    // Continue polling despite read error - desk might still be moving
                    if poll_count > 5 {
                        log::error!("Multiple height read failures, aborting");
                        return Err(anyhow!("Failed to read desk height: {}", e));
                    }
                }
            }

            sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }

        Ok(())
    }

    /// Stop desk movement
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping desk movement");
        self.send_command(MovementCommand::Stop).await
    }

    /// Disconnect from the desk
    pub async fn disconnect(&self) -> Result<()> {
        if self.peripheral.is_connected().await? {
            self.peripheral.disconnect().await?;
            log::info!("Disconnected from desk");
        }
        Ok(())
    }
}

impl Drop for DeskController {
    fn drop(&mut self) {
        // Best effort disconnect
        let _ = futures::executor::block_on(self.disconnect());
    }
}
