use crate::{ModbusMessage, NetworkStatus};
use horus_core::error::HorusResult;
use horus_core::{Hub, Node, NodeInfo};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Modbus Node - Industrial protocol handler for Modbus TCP/RTU communication
///
/// Handles Modbus protocol communication with industrial equipment.
/// Supports reading/writing coils, discrete inputs, holding registers, and input registers.
pub struct ModbusNode {
    publisher: Hub<ModbusMessage>,
    status_publisher: Hub<NetworkStatus>,
    request_subscriber: Hub<ModbusMessage>,

    // Configuration
    server_address: String,
    server_port: u16,
    slave_id: u8,
    timeout_ms: u64,

    // State
    is_connected: bool,
    connection_attempts: u32,
    last_activity: u64,
    device_cache: HashMap<u16, u16>, // address -> value cache
}

impl ModbusNode {
    /// Create a new Modbus node with default topics
    pub fn new() -> HorusResult<Self> {
        Self::new_with_topics("modbus_request", "modbus_response", "modbus_status")
    }

    /// Create a new Modbus node with custom topics
    pub fn new_with_topics(
        request_topic: &str,
        response_topic: &str,
        status_topic: &str,
    ) -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new(response_topic)?,
            status_publisher: Hub::new(status_topic)?,
            request_subscriber: Hub::new(request_topic)?,

            server_address: "127.0.0.1".to_string(),
            server_port: 502,
            slave_id: 1,
            timeout_ms: 5000,

            is_connected: false,
            connection_attempts: 0,
            last_activity: 0,
            device_cache: HashMap::new(),
        })
    }

    /// Set Modbus server connection parameters
    pub fn set_connection(&mut self, address: &str, port: u16, slave_id: u8) {
        self.server_address = address.to_string();
        self.server_port = port;
        self.slave_id = slave_id;
    }

    /// Set communication timeout in milliseconds
    pub fn set_timeout(&mut self, timeout_ms: u64) {
        self.timeout_ms = timeout_ms;
    }

    /// Check if connected to Modbus server
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> (u32, u64, usize) {
        (
            self.connection_attempts,
            self.last_activity,
            self.device_cache.len(),
        )
    }

    fn simulate_connection(&mut self) -> bool {
        // Simulate connection logic (would use tokio-modbus in real implementation)
        self.connection_attempts += 1;

        // Simulate connection success after a few attempts
        if self.connection_attempts > 2 {
            self.is_connected = true;
            true
        } else {
            self.is_connected = false;
            false
        }
    }

    fn handle_modbus_request(&mut self, request: ModbusMessage) -> ModbusMessage {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        self.last_activity = current_time;

        // Simulate processing the request
        let mut response = request.clone();
        response.is_request = false;
        response.timestamp = current_time;

        // Simulate reading values (in real implementation, this would read from actual device)
        match request.function_code {
            1 | 2 => {
                // Read Coils / Discrete Inputs
                response.data_length = request.quantity as u8;
                for i in 0..response.data_length as usize {
                    response.data[i] = ((request.start_address as usize + i) % 2) as u16;
                    // Alternating pattern
                }
            }
            3 | 4 => {
                // Read Holding / Input Registers
                response.data_length = request.quantity as u8;
                for i in 0..response.data_length as usize {
                    let addr = request.start_address + i as u16;
                    let value = self.device_cache.get(&addr).cloned().unwrap_or(addr * 10); // Default values
                    response.data[i] = value;
                }
            }
            5 | 6 => {
                // Write Single Coil / Register
                if request.data_length > 0 {
                    self.device_cache
                        .insert(request.start_address, request.data[0]);
                }
                response.data[0] = request.data[0]; // Echo written value
            }
            15 | 16 => {
                // Write Multiple Coils / Registers
                for i in 0..request.data_length as usize {
                    let addr = request.start_address + i as u16;
                    self.device_cache.insert(addr, request.data[i]);
                }
            }
            _ => {
                // Unsupported function code
                response.function_code = request.function_code | 0x80; // Error response
            }
        }

        response
    }

    fn publish_network_status(&self) {
        let _current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut status = NetworkStatus::new("modbus");
        status.link_up = self.is_connected;
        status.tx_packets = self.connection_attempts as u64;
        status.rx_packets = if self.last_activity > 0 { 1 } else { 0 };
        status.tx_errors = 0; // Would track actual errors
        status.rx_errors = 0;

        let _ = self.status_publisher.send(status, None);
    }
}

impl Node for ModbusNode {
    fn name(&self) -> &'static str {
        "ModbusNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Try to connect if not connected
        if !self.is_connected {
            self.simulate_connection();
        }

        // Handle incoming requests
        if let Some(request) = self.request_subscriber.recv(None) {
            if self.is_connected {
                let response = self.handle_modbus_request(request);
                let _ = self.publisher.send(response, None);
            }
        }

        // Publish network status periodically
        self.publish_network_status();
    }
}

// Default impl removed - use Node::new() instead which returns HorusResult
