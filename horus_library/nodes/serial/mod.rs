use crate::SerialData;
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo, NodeInfoExt};
use std::time::{SystemTime, UNIX_EPOCH};

/// Serial/UART Communication Node
///
/// Handles serial/UART communication with devices like GPS modules,
/// Arduino boards, sensors, and other serial peripherals.
/// Supports various baud rates, data formats, and flow control.
pub struct SerialNode {
    rx_publisher: Hub<SerialData>,   // Received data
    tx_subscriber: Hub<SerialData>,  // Data to transmit

    // Configuration
    port_path: String,
    baud_rate: u32,
    data_bits: u8,
    stop_bits: u8,
    parity: u8,
    flow_control: bool,
    read_timeout_ms: u64,

    // State
    port_open: bool,
    bytes_received: u64,
    bytes_transmitted: u64,
    errors: u64,

    // Buffering (for simulation)
    receive_buffer: Vec<u8>,
}

impl SerialNode {
    /// Create a new serial node with default port and topics
    pub fn new() -> Result<Self> {
        Self::new_with_config("/dev/ttyUSB0", 9600, "serial/rx", "serial/tx")
    }

    /// Create a new serial node with custom configuration
    pub fn new_with_config(
        port: &str,
        baud_rate: u32,
        rx_topic: &str,
        tx_topic: &str,
    ) -> Result<Self> {
        Ok(Self {
            rx_publisher: Hub::new(rx_topic)?,
            tx_subscriber: Hub::new(tx_topic)?,
            port_path: port.to_string(),
            baud_rate,
            data_bits: 8,
            stop_bits: 1,
            parity: SerialData::PARITY_NONE,
            flow_control: false,
            read_timeout_ms: 100,
            port_open: false,
            bytes_received: 0,
            bytes_transmitted: 0,
            errors: 0,
            receive_buffer: Vec::new(),
        })
    }

    /// Set serial port path
    pub fn set_port(&mut self, port: &str) {
        self.port_path = port.to_string();
    }

    /// Set baud rate
    pub fn set_baud_rate(&mut self, baud_rate: u32) {
        self.baud_rate = baud_rate;
    }

    /// Set data format (data_bits, stop_bits, parity)
    pub fn set_format(&mut self, data_bits: u8, stop_bits: u8, parity: u8) {
        self.data_bits = data_bits;
        self.stop_bits = stop_bits;
        self.parity = parity;
    }

    /// Enable/disable hardware flow control
    pub fn set_flow_control(&mut self, enabled: bool) {
        self.flow_control = enabled;
    }

    /// Set read timeout in milliseconds
    pub fn set_read_timeout(&mut self, timeout_ms: u64) {
        self.read_timeout_ms = timeout_ms;
    }

    /// Check if port is open
    pub fn is_open(&self) -> bool {
        self.port_open
    }

    /// Get statistics
    pub fn get_stats(&self) -> (u64, u64, u64) {
        (self.bytes_received, self.bytes_transmitted, self.errors)
    }

    /// Open the serial port
    fn open_port(&mut self, mut ctx: Option<&mut NodeInfo>) -> bool {
        // In real implementation, this would open actual serial port
        // using a library like serialport-rs
        ctx.log_info(&format!(
            "Opening serial port {} @ {} baud",
            self.port_path, self.baud_rate
        ));

        // Simulate successful open
        self.port_open = true;
        true
    }

    /// Close the serial port
    fn close_port(&mut self, mut ctx: Option<&mut NodeInfo>) {
        if self.port_open {
            ctx.log_info(&format!("Closing serial port {}", self.port_path));
            self.port_open = false;
        }
    }

    /// Read data from serial port
    fn read_serial(&mut self, mut ctx: Option<&mut NodeInfo>) {
        if !self.port_open {
            return;
        }

        // In real implementation, this would read from actual hardware
        // For simulation, generate some test data periodically
        if self.receive_buffer.is_empty() {
            return;
        }

        // Create message with received data
        let mut msg = SerialData::new(&self.port_path);
        msg.baud_rate = self.baud_rate;
        msg.data_bits = self.data_bits;
        msg.stop_bits = self.stop_bits;
        msg.parity = self.parity;

        // Copy data from buffer (up to 1024 bytes)
        let available = self.receive_buffer.len().min(1024);
        if msg.set_data(&self.receive_buffer[..available]) {
            self.bytes_received += available as u64;
            self.receive_buffer.drain(..available);

            // Publish received data
            let _ = self.rx_publisher.send(msg, None);

            ctx.log_debug(&format!("Received {} bytes from serial port", available));
        }
    }

    /// Write data to serial port
    fn write_serial(&mut self, data: &SerialData, mut ctx: Option<&mut NodeInfo>) {
        if !self.port_open {
            ctx.log_warning("Cannot write: serial port not open");
            return;
        }

        let bytes = data.get_data();
        if bytes.is_empty() {
            return;
        }

        // In real implementation, this would write to actual hardware
        self.bytes_transmitted += bytes.len() as u64;

        ctx.log_debug(&format!("Transmitted {} bytes to serial port", bytes.len()));

        // Log as string if valid UTF-8
        if let Some(text) = data.get_string() {
            ctx.log_debug(&format!("TX: {}", text.trim()));
        }
    }

    /// Simulate receiving data (for testing without hardware)
    pub fn simulate_receive(&mut self, data: &[u8]) {
        self.receive_buffer.extend_from_slice(data);
    }
}

impl Node for SerialNode {
    fn name(&self) -> &'static str {
        "SerialNode"
    }

    fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        // Open serial port on initialization
        ctx.log_info(&format!(
            "Opening serial port {} @ {} baud",
            self.port_path, self.baud_rate
        ));

        // Simulate successful open
        self.port_open = true;
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Read incoming data
        self.read_serial(ctx.as_deref_mut());

        // Process outgoing data
        while let Some(tx_data) = self.tx_subscriber.recv(None) {
            self.write_serial(&tx_data, ctx.as_deref_mut());
        }
    }

    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        if self.port_open {
            ctx.log_info(&format!("Closing serial port {}", self.port_path));
            self.port_open = false;
        }
        Ok(())
    }
}
