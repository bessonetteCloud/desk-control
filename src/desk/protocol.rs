use uuid::Uuid;

/// Linak BLE Service and Characteristic UUIDs
/// Based on reverse engineering of Linak DPG (Desk Panel Gateway) protocol

// Main control service UUID
pub const CONTROL_SERVICE_UUID: Uuid =
    Uuid::from_u128(0x99fa0001_338a_1024_8a49_009c0215f78a);

// Characteristic for reading current height (in 0.1mm units)
pub const HEIGHT_CHARACTERISTIC_UUID: Uuid =
    Uuid::from_u128(0x99fa0021_338a_1024_8a49_009c0215f78a);

// Characteristic for sending movement commands
pub const CONTROL_CHARACTERISTIC_UUID: Uuid =
    Uuid::from_u128(0x99fa0002_338a_1024_8a49_009c0215f78a);

// Characteristic for position reference
pub const REFERENCE_INPUT_UUID: Uuid =
    Uuid::from_u128(0x99fa0031_338a_1024_8a49_009c0215f78a);

/// Movement commands
#[derive(Debug, Clone, Copy)]
pub enum MovementCommand {
    /// Stop all movement
    Stop,
    /// Move desk up
    Up,
    /// Move desk down
    Down,
    /// Move to specific height (in 0.1mm units)
    MoveToHeight(u16),
}

impl MovementCommand {
    /// Convert command to bytes for BLE transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Stop => vec![0xFF, 0x00],
            Self::Up => vec![0x47, 0x00],   // Move up command
            Self::Down => vec![0x46, 0x00], // Move down command
            Self::MoveToHeight(height) => {
                // Move to position command
                // Format: [0x05, low_byte, high_byte]
                let height_bytes = height.to_le_bytes();
                vec![0x05, height_bytes[0], height_bytes[1]]
            }
        }
    }
}

/// Parse height from BLE characteristic data
/// Height is transmitted as 16-bit little-endian in 0.1mm units
pub fn parse_height(data: &[u8]) -> Option<u16> {
    if data.len() >= 2 {
        Some(u16::from_le_bytes([data[0], data[1]]))
    } else {
        None
    }
}

/// Convert millimeters to the desk's internal format (0.1mm units)
pub fn mm_to_desk_units(mm: u16) -> u16 {
    mm * 10
}

/// Convert desk's internal format (0.1mm units) to millimeters
pub fn desk_units_to_mm(units: u16) -> u16 {
    units / 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_commands() {
        assert_eq!(MovementCommand::Stop.to_bytes(), vec![0xFF, 0x00]);
        assert_eq!(MovementCommand::Up.to_bytes(), vec![0x47, 0x00]);
        assert_eq!(MovementCommand::Down.to_bytes(), vec![0x46, 0x00]);

        let height = MovementCommand::MoveToHeight(10500); // 1050mm = 105cm
        assert_eq!(height.to_bytes(), vec![0x05, 0x04, 0x29]);
    }

    #[test]
    fn test_height_conversion() {
        assert_eq!(mm_to_desk_units(1050), 10500);
        assert_eq!(desk_units_to_mm(10500), 1050);
    }

    #[test]
    fn test_parse_height() {
        assert_eq!(parse_height(&[0x04, 0x29]), Some(10500));
        assert_eq!(parse_height(&[0x00]), None);
    }
}
