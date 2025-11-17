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
        let desks = Self::scan_for_desks(5).await?;

        if desks.is_empty() {
            return Err(anyhow!("No Linak desks found"));
        }

        let peripheral = if let Some(addr) = desk_address {
            desks
                .into_iter()
                .find(|p| {
                    if let Ok(Some(props)) = futures::executor::block_on(p.properties()) {
                        props.address.to_string() == addr
                    } else {
                        false
                    }
                })
                .ok_or_else(|| anyhow!("Desk with address {} not found", addr))?
        } else {
            log::info!("No desk address specified, connecting to first available desk");
            desks
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("No desks available"))?
        };

        // Connect to the peripheral
        if !peripheral.is_connected().await? {
            log::info!("Connecting to desk...");
            peripheral.connect().await?;
            log::info!("Connected successfully");
        }

        // Discover services
        peripheral.discover_services().await?;

        // Find the control service and characteristics
        let chars = peripheral.characteristics();
        let control_char = chars
            .iter()
            .find(|c| c.uuid == CONTROL_CHARACTERISTIC_UUID)
            .cloned();

        let height_char = chars
            .iter()
            .find(|c| c.uuid == HEIGHT_CHARACTERISTIC_UUID)
            .cloned();

        if control_char.is_none() {
            return Err(anyhow!("Could not find control characteristic on desk"));
        }

        if height_char.is_none() {
            return Err(anyhow!("Could not find height characteristic on desk"));
        }

        log::info!("Desk controller initialized");

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

        let data = self.peripheral.read(height_char).await?;

        let height_units = parse_height(&data)
            .ok_or_else(|| anyhow!("Failed to parse height data"))?;

        let height_mm = super::protocol::desk_units_to_mm(height_units);
        log::debug!("Current height: {}mm", height_mm);

        Ok(height_mm)
    }

    /// Send a movement command to the desk
    pub async fn send_command(&self, command: MovementCommand) -> Result<()> {
        let control_char = self
            .control_char
            .as_ref()
            .ok_or_else(|| anyhow!("Control characteristic not available"))?;

        let bytes = command.to_bytes();
        log::debug!("Sending command: {:?} -> {:?}", command, bytes);

        self.peripheral
            .write(control_char, &bytes, WriteType::WithoutResponse)
            .await?;

        Ok(())
    }

    /// Move desk to a specific height in millimeters
    pub async fn move_to_height(&self, height_mm: u16) -> Result<()> {
        let height_units = super::protocol::mm_to_desk_units(height_mm);
        log::info!("Moving desk to {}mm ({}units)", height_mm, height_units);

        self.send_command(MovementCommand::MoveToHeight(height_units))
            .await?;

        // Wait for movement to start
        sleep(Duration::from_millis(100)).await;

        // Poll until we reach the target height (with tolerance)
        const TOLERANCE_MM: u16 = 5; // 5mm tolerance
        const MAX_WAIT_SECS: u64 = 30;
        const POLL_INTERVAL_MS: u64 = 200;

        let start = std::time::Instant::now();

        loop {
            if start.elapsed().as_secs() > MAX_WAIT_SECS {
                return Err(anyhow!("Timeout waiting for desk to reach target height"));
            }

            let current = self.get_height().await?;
            let diff = if current > height_mm {
                current - height_mm
            } else {
                height_mm - current
            };

            if diff <= TOLERANCE_MM {
                log::info!("Reached target height: {}mm", current);
                break;
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
