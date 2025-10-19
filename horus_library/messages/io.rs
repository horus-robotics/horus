//! Industrial I/O and communication message types for robotics
//!
//! This module provides messages for digital/analog I/O, industrial protocols,
//! and integration with PLCs, SCADA systems, and factory automation.

use serde::{Deserialize, Serialize};
use serde_arrays;

/// Digital I/O state message
///
/// Represents the state of digital input/output pins, typically used
/// for interfacing with sensors, actuators, and industrial equipment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[derive(Default)]
pub struct DigitalIO {
    /// Pin states (true = high/on, false = low/off)
    #[serde(with = "serde_arrays")]
    pub pins: [bool; 32],
    /// Number of active pins
    pub pin_count: u8,
    /// Pin direction mask (true = output, false = input)
    #[serde(with = "serde_arrays")]
    pub pin_directions: [bool; 32],
    /// Pull-up resistor enable mask
    #[serde(with = "serde_arrays")]
    pub pullup_enable: [bool; 32],
    /// Pin labels for identification
    pub pin_labels: [[u8; 16]; 32],
    /// I/O board identifier
    pub board_id: [u8; 32],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}


impl DigitalIO {
    /// Create a new digital I/O message
    pub fn new(pin_count: u8) -> Self {
        Self {
            pin_count,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        }
    }

    /// Set pin state
    pub fn set_pin(&mut self, pin: u8, state: bool) -> bool {
        if pin < self.pin_count && (pin as usize) < self.pins.len() {
            self.pins[pin as usize] = state;
            true
        } else {
            false
        }
    }

    /// Get pin state
    pub fn get_pin(&self, pin: u8) -> Option<bool> {
        if pin < self.pin_count && (pin as usize) < self.pins.len() {
            Some(self.pins[pin as usize])
        } else {
            None
        }
    }

    /// Set pin direction (true = output, false = input)
    pub fn set_pin_direction(&mut self, pin: u8, is_output: bool) -> bool {
        if pin < self.pin_count && (pin as usize) < self.pin_directions.len() {
            self.pin_directions[pin as usize] = is_output;
            true
        } else {
            false
        }
    }

    /// Set pin label
    pub fn set_pin_label(&mut self, pin: u8, label: &str) -> bool {
        if pin < self.pin_count && (pin as usize) < self.pin_labels.len() {
            let label_bytes = label.as_bytes();
            let len = label_bytes.len().min(15);
            self.pin_labels[pin as usize][..len].copy_from_slice(&label_bytes[..len]);
            self.pin_labels[pin as usize][len] = 0;
            true
        } else {
            false
        }
    }

    /// Get pin label as string
    pub fn get_pin_label(&self, pin: u8) -> Option<String> {
        if pin < self.pin_count && (pin as usize) < self.pin_labels.len() {
            let label_bytes = &self.pin_labels[pin as usize];
            let end = label_bytes.iter().position(|&b| b == 0).unwrap_or(16);
            Some(String::from_utf8_lossy(&label_bytes[..end]).into_owned())
        } else {
            None
        }
    }

    /// Count active (high) pins
    pub fn count_active(&self) -> u8 {
        (0..self.pin_count)
            .filter(|&pin| self.pins[pin as usize])
            .count() as u8
    }

    /// Get bitmask representation
    pub fn as_bitmask(&self) -> u32 {
        let mut mask = 0u32;
        for i in 0..self.pin_count.min(32) {
            if self.pins[i as usize] {
                mask |= 1 << i;
            }
        }
        mask
    }

    /// Set from bitmask
    pub fn from_bitmask(&mut self, mask: u32) {
        for i in 0..self.pin_count.min(32) {
            self.pins[i as usize] = (mask & (1 << i)) != 0;
        }
        self.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
    }
}

/// Analog I/O measurements message
///
/// Represents analog input/output channels, typically used for
/// sensors, actuators, and continuous control signals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalogIO {
    /// Channel values (in volts or engineering units)
    #[serde(with = "serde_arrays")]
    pub channels: [f64; 16],
    /// Number of active channels
    pub channel_count: u8,
    /// Channel ranges [min, max] for each channel
    pub channel_ranges: [[f64; 2]; 16],
    /// Engineering unit labels ("V", "mA", "°C", etc.)
    pub unit_labels: [[u8; 8]; 16],
    /// Channel names for identification
    pub channel_labels: [[u8; 16]; 16],
    /// ADC resolution in bits
    pub resolution_bits: u8,
    /// Sampling frequency in Hz
    pub sampling_frequency: f32,
    /// I/O board identifier
    pub board_id: [u8; 32],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for AnalogIO {
    fn default() -> Self {
        Self {
            channels: [0.0; 16],
            channel_count: 0,
            channel_ranges: [[-10.0, 10.0]; 16], // Default ±10V range
            unit_labels: [[0; 8]; 16],
            channel_labels: [[0; 16]; 16],
            resolution_bits: 16,
            sampling_frequency: 1000.0,
            board_id: [0; 32],
            timestamp: 0,
        }
    }
}

impl AnalogIO {
    /// Create a new analog I/O message
    pub fn new(channel_count: u8) -> Self {
        Self {
            channel_count,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        }
    }

    /// Set channel value
    pub fn set_channel(&mut self, channel: u8, value: f64) -> bool {
        if channel < self.channel_count && (channel as usize) < self.channels.len() {
            self.channels[channel as usize] = value;
            true
        } else {
            false
        }
    }

    /// Get channel value
    pub fn get_channel(&self, channel: u8) -> Option<f64> {
        if channel < self.channel_count && (channel as usize) < self.channels.len() {
            Some(self.channels[channel as usize])
        } else {
            None
        }
    }

    /// Set channel range
    pub fn set_channel_range(&mut self, channel: u8, min_val: f64, max_val: f64) -> bool {
        if channel < self.channel_count && (channel as usize) < self.channel_ranges.len() {
            self.channel_ranges[channel as usize] = [min_val, max_val];
            true
        } else {
            false
        }
    }

    /// Convert raw ADC value to engineering units
    pub fn raw_to_engineering(&self, channel: u8, raw_value: u16) -> Option<f64> {
        if channel < self.channel_count && (channel as usize) < self.channel_ranges.len() {
            let max_raw = (1 << self.resolution_bits) - 1;
            let normalized = raw_value as f64 / max_raw as f64;
            let range = &self.channel_ranges[channel as usize];
            Some(range[0] + normalized * (range[1] - range[0]))
        } else {
            None
        }
    }

    /// Convert engineering units to raw ADC value
    pub fn engineering_to_raw(&self, channel: u8, eng_value: f64) -> Option<u16> {
        if channel < self.channel_count && (channel as usize) < self.channel_ranges.len() {
            let range = &self.channel_ranges[channel as usize];
            let normalized = (eng_value - range[0]) / (range[1] - range[0]);
            let max_raw = (1 << self.resolution_bits) - 1;
            Some((normalized * max_raw as f64).clamp(0.0, max_raw as f64) as u16)
        } else {
            None
        }
    }

    /// Set channel label and unit
    pub fn set_channel_info(&mut self, channel: u8, label: &str, unit: &str) -> bool {
        if channel < self.channel_count && (channel as usize) < self.channel_labels.len() {
            // Set label
            let label_bytes = label.as_bytes();
            let len = label_bytes.len().min(15);
            self.channel_labels[channel as usize][..len].copy_from_slice(&label_bytes[..len]);
            self.channel_labels[channel as usize][len] = 0;

            // Set unit
            let unit_bytes = unit.as_bytes();
            let len = unit_bytes.len().min(7);
            self.unit_labels[channel as usize][..len].copy_from_slice(&unit_bytes[..len]);
            self.unit_labels[channel as usize][len] = 0;

            true
        } else {
            false
        }
    }
}

/// Modbus communication message
///
/// Standard industrial protocol message for communicating with
/// PLCs, sensors, and other Modbus-compatible devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusMessage {
    /// Slave/unit address (1-255)
    pub unit_id: u8,
    /// Function code (1=read coils, 3=read holding registers, etc.)
    pub function_code: u8,
    /// Starting register/coil address
    pub start_address: u16,
    /// Number of registers/coils to read/write
    pub quantity: u16,
    /// Data payload (registers for function codes 3,4,6,16)
    #[serde(with = "serde_arrays")]
    pub data: [u16; 32],
    /// Data length (number of valid entries in data array)
    pub data_length: u8,
    /// Exception code if error occurred
    pub exception_code: u8,
    /// Transaction ID for matching requests/responses
    pub transaction_id: u16,
    /// Message direction (true = request, false = response)
    pub is_request: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for ModbusMessage {
    fn default() -> Self {
        Self {
            unit_id: 1,
            function_code: 0,
            start_address: 0,
            quantity: 0,
            data: [0; 32],
            data_length: 0,
            exception_code: 0,
            transaction_id: 0,
            is_request: true,
            timestamp: 0,
        }
    }
}

impl ModbusMessage {
    // Standard Modbus function codes
    pub const FUNC_READ_COILS: u8 = 1;
    pub const FUNC_READ_DISCRETE_INPUTS: u8 = 2;
    pub const FUNC_READ_HOLDING_REGISTERS: u8 = 3;
    pub const FUNC_READ_INPUT_REGISTERS: u8 = 4;
    pub const FUNC_WRITE_SINGLE_COIL: u8 = 5;
    pub const FUNC_WRITE_SINGLE_REGISTER: u8 = 6;
    pub const FUNC_WRITE_MULTIPLE_COILS: u8 = 15;
    pub const FUNC_WRITE_MULTIPLE_REGISTERS: u8 = 16;

    /// Create a read holding registers request
    pub fn read_holding_registers(unit_id: u8, start_addr: u16, count: u16) -> Self {
        Self {
            unit_id,
            function_code: Self::FUNC_READ_HOLDING_REGISTERS,
            start_address: start_addr,
            quantity: count,
            is_request: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        }
    }

    /// Create a write single register request
    pub fn write_single_register(unit_id: u8, address: u16, value: u16) -> Self {
        let mut msg = Self {
            unit_id,
            function_code: Self::FUNC_WRITE_SINGLE_REGISTER,
            start_address: address,
            quantity: 1,
            data_length: 1,
            is_request: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        };
        msg.data[0] = value;
        msg
    }

    /// Create a write multiple registers request
    pub fn write_multiple_registers(unit_id: u8, start_addr: u16, values: &[u16]) -> Self {
        let mut msg = Self {
            unit_id,
            function_code: Self::FUNC_WRITE_MULTIPLE_REGISTERS,
            start_address: start_addr,
            quantity: values.len() as u16,
            data_length: values.len().min(32) as u8,
            is_request: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        };

        let copy_len = values.len().min(32);
        msg.data[..copy_len].copy_from_slice(&values[..copy_len]);
        msg
    }

    /// Create a response message
    pub fn create_response(&self, data: &[u16]) -> Self {
        let mut response = self.clone();
        response.is_request = false;
        response.data_length = data.len().min(32) as u8;
        response.data[..response.data_length as usize]
            .copy_from_slice(&data[..response.data_length as usize]);
        response.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        response
    }

    /// Create an exception response
    pub fn create_exception(&self, exception_code: u8) -> Self {
        Self {
            unit_id: self.unit_id,
            function_code: self.function_code | 0x80, // Set exception bit
            exception_code,
            transaction_id: self.transaction_id,
            is_request: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        }
    }

    /// Check if this is an exception response
    pub fn is_exception(&self) -> bool {
        (self.function_code & 0x80) != 0
    }
}

/// Ethernet/IP communication message
///
/// Industrial Ethernet protocol commonly used with Allen-Bradley PLCs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtherNetIPMessage {
    /// Service code
    pub service: u8,
    /// Class ID
    pub class_id: u16,
    /// Instance ID
    pub instance_id: u16,
    /// Attribute ID
    pub attribute_id: u16,
    /// Data payload
    pub data: Vec<u8>,
    /// Session handle
    pub session_handle: u32,
    /// Context data
    pub context: [u8; 8],
    /// Status code
    pub status: u16,
    /// Message direction (true = request, false = response)
    pub is_request: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for EtherNetIPMessage {
    fn default() -> Self {
        Self {
            service: 0,
            class_id: 0,
            instance_id: 0,
            attribute_id: 0,
            data: Vec::new(),
            session_handle: 0,
            context: [0; 8],
            status: 0,
            is_request: true,
            timestamp: 0,
        }
    }
}

/// Industrial network status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct NetworkStatus {
    /// Network interface name
    pub interface_name: [u8; 16],
    /// IP address (IPv4 as u32)
    pub ip_address: u32,
    /// Subnet mask
    pub subnet_mask: u32,
    /// Gateway address
    pub gateway: u32,
    /// Link status (true = up, false = down)
    pub link_up: bool,
    /// Link speed in Mbps
    pub link_speed: u16,
    /// Duplex mode (true = full, false = half)
    pub full_duplex: bool,
    /// Packets transmitted
    pub tx_packets: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Transmission errors
    pub tx_errors: u32,
    /// Reception errors
    pub rx_errors: u32,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl NetworkStatus {
    /// Create new network status
    pub fn new(interface: &str) -> Self {
        let mut status = Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        };

        let interface_bytes = interface.as_bytes();
        let len = interface_bytes.len().min(15);
        status.interface_name[..len].copy_from_slice(&interface_bytes[..len]);
        status.interface_name[len] = 0;

        status
    }

    /// Convert IP address to string
    pub fn ip_to_string(&self) -> String {
        format!(
            "{}.{}.{}.{}",
            (self.ip_address >> 24) & 0xFF,
            (self.ip_address >> 16) & 0xFF,
            (self.ip_address >> 8) & 0xFF,
            self.ip_address & 0xFF
        )
    }

    /// Set IP address from string
    pub fn set_ip_from_string(&mut self, ip_str: &str) -> Result<(), &'static str> {
        let parts: Vec<&str> = ip_str.split('.').collect();
        if parts.len() != 4 {
            return Err("Invalid IP address format");
        }

        let mut ip = 0u32;
        for (i, part) in parts.iter().enumerate() {
            let octet: u8 = part.parse().map_err(|_| "Invalid IP octet")?;
            ip |= (octet as u32) << (24 - i * 8);
        }

        self.ip_address = ip;
        Ok(())
    }

    /// Calculate packet loss percentage
    pub fn packet_loss_percent(&self) -> f32 {
        if self.tx_packets == 0 {
            return 0.0;
        }
        (self.tx_errors as f32 / self.tx_packets as f32) * 100.0
    }
}

/// Safety relay status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct SafetyRelayStatus {
    /// Relay identifier
    pub relay_id: [u8; 16],
    /// Safety output states
    pub safety_outputs: [bool; 8],
    /// Input channel states
    pub input_channels: [bool; 16],
    /// Diagnostic information
    pub diagnostics: u16,
    /// Safety function active
    pub safety_active: bool,
    /// Reset required
    pub reset_required: bool,
    /// Fault present
    pub fault_present: bool,
    /// Test mode active
    pub test_mode: bool,
    /// Operating hours
    pub operating_hours: u32,
    /// Switch cycles count
    pub switch_cycles: u32,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl SafetyRelayStatus {
    /// Create new safety relay status
    pub fn new(relay_id: &str) -> Self {
        let mut status = Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        };

        let id_bytes = relay_id.as_bytes();
        let len = id_bytes.len().min(15);
        status.relay_id[..len].copy_from_slice(&id_bytes[..len]);
        status.relay_id[len] = 0;

        status
    }

    /// Check if system is in safe state
    pub fn is_safe_state(&self) -> bool {
        !self.fault_present && self.safety_active && !self.reset_required
    }

    /// Get active safety output count
    pub fn active_output_count(&self) -> u8 {
        self.safety_outputs.iter().filter(|&&state| state).count() as u8
    }

    /// Get active input count
    pub fn active_input_count(&self) -> u8 {
        self.input_channels.iter().filter(|&&state| state).count() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digital_io() {
        let mut dio = DigitalIO::new(8);

        // Test pin operations
        assert!(dio.set_pin(0, true));
        assert!(dio.set_pin(7, true));
        assert!(!dio.set_pin(8, true)); // Out of range

        assert_eq!(dio.get_pin(0), Some(true));
        assert_eq!(dio.get_pin(1), Some(false));
        assert_eq!(dio.count_active(), 2);

        // Test bitmask
        let mask = dio.as_bitmask();
        assert_eq!(mask, 0x81); // Bits 0 and 7 set

        let mut dio2 = DigitalIO::new(8);
        dio2.from_bitmask(0x81);
        assert!(dio2.get_pin(0).unwrap());
        assert!(dio2.get_pin(7).unwrap());
    }

    #[test]
    fn test_analog_io() {
        let mut aio = AnalogIO::new(4);

        assert!(aio.set_channel(0, 5.0));
        assert!(aio.set_channel(3, -2.5));
        assert!(!aio.set_channel(4, 1.0)); // Out of range

        assert_eq!(aio.get_channel(0), Some(5.0));
        assert_eq!(aio.get_channel(1), Some(0.0));

        // Test ADC conversion
        aio.set_channel_range(0, 0.0, 10.0);
        aio.resolution_bits = 12; // 12-bit ADC

        if let Some(raw) = aio.engineering_to_raw(0, 5.0) {
            // 5V should be about half scale (2047 for 12-bit)
            assert!((raw as i32 - 2047).abs() <= 1);
        }
    }

    #[test]
    fn test_modbus_message() {
        let msg = ModbusMessage::read_holding_registers(1, 100, 10);
        assert_eq!(msg.unit_id, 1);
        assert_eq!(
            msg.function_code,
            ModbusMessage::FUNC_READ_HOLDING_REGISTERS
        );
        assert_eq!(msg.start_address, 100);
        assert_eq!(msg.quantity, 10);
        assert!(msg.is_request);

        let write_msg = ModbusMessage::write_single_register(1, 200, 1234);
        assert_eq!(
            write_msg.function_code,
            ModbusMessage::FUNC_WRITE_SINGLE_REGISTER
        );
        assert_eq!(write_msg.data[0], 1234);
    }

    #[test]
    fn test_network_status() {
        let mut status = NetworkStatus::new("eth0");
        status.set_ip_from_string("192.168.1.100").unwrap();

        assert_eq!(status.ip_to_string(), "192.168.1.100");
        assert!(status.set_ip_from_string("invalid").is_err());

        status.tx_packets = 1000;
        status.tx_errors = 10;
        assert_eq!(status.packet_loss_percent(), 1.0);
    }

    #[test]
    fn test_safety_relay() {
        let mut relay = SafetyRelayStatus::new("SR001");
        assert!(relay.is_safe_state()); // Default state should be safe

        relay.fault_present = true;
        assert!(!relay.is_safe_state());

        relay.fault_present = false;
        relay.safety_outputs[0] = true;
        relay.safety_outputs[2] = true;
        assert_eq!(relay.active_output_count(), 2);
    }
}
