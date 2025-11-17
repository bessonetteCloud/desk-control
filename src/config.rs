use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration for the desk control application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Bluetooth MAC address or device name of the desk
    pub desk_address: Option<String>,

    /// Height presets mapped to Starbucks drink sizes (in millimeters)
    pub presets: HeightPresets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightPresets {
    /// Short (8 oz) - typically sitting height
    pub short: u16,

    /// Tall (12 oz) - mid-level height
    pub tall: u16,

    /// Grande (16 oz) - standing height
    pub grande: u16,

    /// Venti (20 oz) - maximum height
    pub venti: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            desk_address: None,
            presets: HeightPresets {
                short: 650,   // 65.0 cm - typical sitting height
                tall: 850,    // 85.0 cm - mid-level
                grande: 1050, // 105.0 cm - standing height
                venti: 1250,  // 125.0 cm - maximum height
            },
        }
    }
}

impl Config {
    /// Get the configuration directory path (~/.desk-control)
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Could not find home directory")?;
        Ok(home.join(".desk-control"))
    }

    /// Get the configuration file path (~/.desk-control/config)
    pub fn config_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config"))
    }

    /// Load configuration from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let config_file = Self::config_file()?;

        if config_file.exists() {
            let content = fs::read_to_string(&config_file)
                .context("Failed to read config file")?;
            let config: Config = serde_json::from_str(&content)
                .context("Failed to parse config file")?;
            Ok(config)
        } else {
            log::info!("Config file not found, creating default");
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;
        }

        let config_file = Self::config_file()?;
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&config_file, content)
            .context("Failed to write config file")?;

        log::info!("Configuration saved to {:?}", config_file);
        Ok(())
    }

    /// Get height for a specific preset
    pub fn get_preset(&self, preset: DrinkSize) -> u16 {
        match preset {
            DrinkSize::Short => self.presets.short,
            DrinkSize::Tall => self.presets.tall,
            DrinkSize::Grande => self.presets.grande,
            DrinkSize::Venti => self.presets.venti,
        }
    }

    /// Set height for a specific preset
    pub fn set_preset(&mut self, preset: DrinkSize, height_mm: u16) {
        match preset {
            DrinkSize::Short => self.presets.short = height_mm,
            DrinkSize::Tall => self.presets.tall = height_mm,
            DrinkSize::Grande => self.presets.grande = height_mm,
            DrinkSize::Venti => self.presets.venti = height_mm,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrinkSize {
    Short,
    Tall,
    Grande,
    Venti,
}

impl DrinkSize {
    pub fn all() -> Vec<Self> {
        vec![Self::Short, Self::Tall, Self::Grande, Self::Venti]
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Short => "Short",
            Self::Tall => "Tall",
            Self::Grande => "Grande",
            Self::Venti => "Venti",
        }
    }
}
