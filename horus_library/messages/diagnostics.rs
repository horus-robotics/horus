use horus_core::core::LogSummary;
// Diagnostic and system health message types
//
// This module provides messages for system monitoring, health checks,
// error reporting, and general diagnostics.

use serde::{Deserialize, Serialize};
use serde_arrays;

/// System heartbeat message
///
/// Periodic signal indicating a node is alive and operational.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Node name (null-terminated string)
    pub node_name: [u8; 32],
    /// Node ID
    pub node_id: u32,
    /// Sequence number (increments each heartbeat)
    pub sequence: u64,
    /// Node is alive and responding
    pub alive: bool,
    /// Time since startup in seconds
    pub uptime: f64,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for Heartbeat {
    fn default() -> Self {
        Self {
            node_name: [0; 32],
            node_id: 0,
            sequence: 0,
            alive: true,
            uptime: 0.0,
            timestamp: 0,
        }
    }
}

impl Heartbeat {
    /// Create a new heartbeat message
    pub fn new(node_name: &str, node_id: u32) -> Self {
        let mut hb = Self {
            node_id,
            alive: true,
            sequence: 0,
            uptime: 0.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        };

        // Copy node name
        let name_bytes = node_name.as_bytes();
        let len = name_bytes.len().min(31);
        hb.node_name[..len].copy_from_slice(&name_bytes[..len]);
        hb.node_name[len] = 0;

        hb
    }

    /// Update for next heartbeat
    pub fn update(&mut self, uptime: f64) {
        self.sequence += 1;
        self.uptime = uptime;
        self.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
    }

    /// Get node name as string
    pub fn name(&self) -> String {
        let end = self.node_name.iter().position(|&b| b == 0).unwrap_or(32);
        String::from_utf8_lossy(&self.node_name[..end]).into_owned()
    }
}

/// Status level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum StatusLevel {
    /// Everything is OK
    Ok = 0,
    /// Warning condition
    Warn = 1,
    /// Error condition (recoverable)
    Error = 2,
    /// Fatal error (system should stop)
    Fatal = 3,
}

impl Default for StatusLevel {
    fn default() -> Self {
        Self::Ok
    }
}

/// System status message
///
/// General-purpose status reporting for any component.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Status {
    /// Severity level
    pub level: StatusLevel,
    /// Error/status code (component-specific)
    pub code: u32,
    /// Human-readable message (null-terminated)
    #[serde(with = "serde_arrays")]
    pub message: [u8; 128],
    /// Component name reporting the status
    pub component: [u8; 32],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            level: StatusLevel::Ok,
            code: 0,
            message: [0; 128],
            component: [0; 32],
            timestamp: 0,
        }
    }
}

impl Status {
    /// Create a new status message
    pub fn new(level: StatusLevel, code: u32, message: &str) -> Self {
        let mut status = Self {
            level,
            code,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        };

        // Copy message
        let msg_bytes = message.as_bytes();
        let len = msg_bytes.len().min(127);
        status.message[..len].copy_from_slice(&msg_bytes[..len]);
        status.message[len] = 0;

        status
    }

    /// Create an OK status
    pub fn ok(message: &str) -> Self {
        Self::new(StatusLevel::Ok, 0, message)
    }

    /// Create a warning status
    pub fn warn(code: u32, message: &str) -> Self {
        Self::new(StatusLevel::Warn, code, message)
    }

    /// Create an error status
    pub fn error(code: u32, message: &str) -> Self {
        Self::new(StatusLevel::Error, code, message)
    }

    /// Create a fatal status
    pub fn fatal(code: u32, message: &str) -> Self {
        Self::new(StatusLevel::Fatal, code, message)
    }

    /// Set the component name
    pub fn with_component(mut self, component: &str) -> Self {
        let comp_bytes = component.as_bytes();
        let len = comp_bytes.len().min(31);
        self.component[..len].copy_from_slice(&comp_bytes[..len]);
        self.component[len] = 0;
        self
    }

    /// Get message as string
    pub fn message_str(&self) -> String {
        let end = self.message.iter().position(|&b| b == 0).unwrap_or(128);
        String::from_utf8_lossy(&self.message[..end]).into_owned()
    }

    /// Get component as string
    pub fn component_str(&self) -> String {
        let end = self.component.iter().position(|&b| b == 0).unwrap_or(32);
        String::from_utf8_lossy(&self.component[..end]).into_owned()
    }
}

/// Emergency stop message
///
/// Critical safety message to immediately stop all robot motion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EmergencyStop {
    /// Emergency stop is active
    pub engaged: bool,
    /// Reason for emergency stop
    #[serde(with = "serde_arrays")]
    pub reason: [u8; 64],
    /// Source that triggered the stop
    pub source: [u8; 32],
    /// Auto-reset allowed after clearing
    pub auto_reset: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for EmergencyStop {
    fn default() -> Self {
        Self {
            engaged: false,
            reason: [0; 64],
            source: [0; 32],
            auto_reset: false,
            timestamp: 0,
        }
    }
}

impl EmergencyStop {
    /// Create an emergency stop signal
    pub fn engage(reason: &str) -> Self {
        let mut estop = Self {
            engaged: true,
            auto_reset: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        };

        // Copy reason
        let reason_bytes = reason.as_bytes();
        let len = reason_bytes.len().min(63);
        estop.reason[..len].copy_from_slice(&reason_bytes[..len]);
        estop.reason[len] = 0;

        estop
    }

    /// Create a release signal
    pub fn release() -> Self {
        Self {
            engaged: false,
            auto_reset: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        }
    }

    /// Set the source of the emergency stop
    pub fn with_source(mut self, source: &str) -> Self {
        let source_bytes = source.as_bytes();
        let len = source_bytes.len().min(31);
        self.source[..len].copy_from_slice(&source_bytes[..len]);
        self.source[len] = 0;
        self
    }

    /// Get reason as string
    pub fn reason_str(&self) -> String {
        let end = self.reason.iter().position(|&b| b == 0).unwrap_or(64);
        String::from_utf8_lossy(&self.reason[..end]).into_owned()
    }
}

/// System resource usage
///
/// Reports CPU, memory, and other resource utilization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    /// CPU usage percentage (0-100)
    pub cpu_percent: f32,
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// Memory usage percentage (0-100)
    pub memory_percent: f32,
    /// Disk usage in bytes
    pub disk_bytes: u64,
    /// Disk usage percentage (0-100)
    pub disk_percent: f32,
    /// Network bytes sent
    pub network_tx_bytes: u64,
    /// Network bytes received
    pub network_rx_bytes: u64,
    /// System temperature in celsius
    pub temperature: f32,
    /// Number of active threads
    pub thread_count: u32,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl ResourceUsage {
    /// Create a new resource usage message
    pub fn new() -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        }
    }

    /// Check if CPU usage is high
    pub fn is_cpu_high(&self, threshold: f32) -> bool {
        self.cpu_percent > threshold
    }

    /// Check if memory usage is high
    pub fn is_memory_high(&self, threshold: f32) -> bool {
        self.memory_percent > threshold
    }

    /// Check if temperature is high
    pub fn is_temperature_high(&self, threshold: f32) -> bool {
        self.temperature > threshold
    }
}

/// Diagnostic key-value pair
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DiagnosticValue {
    /// Key name (null-terminated)
    pub key: [u8; 32],
    /// Value as string (null-terminated)
    #[serde(with = "serde_arrays")]
    pub value: [u8; 64],
    /// Value type hint (0=string, 1=int, 2=float, 3=bool)
    pub value_type: u8,
}

impl Default for DiagnosticValue {
    fn default() -> Self {
        Self {
            key: [0; 32],
            value: [0; 64],
            value_type: Self::TYPE_STRING,
        }
    }
}

impl DiagnosticValue {
    pub const TYPE_STRING: u8 = 0;
    pub const TYPE_INT: u8 = 1;
    pub const TYPE_FLOAT: u8 = 2;
    pub const TYPE_BOOL: u8 = 3;

    /// Create a string diagnostic value
    pub fn string(key: &str, value: &str) -> Self {
        let mut diag = Self {
            value_type: Self::TYPE_STRING,
            ..Default::default()
        };

        // Copy key
        let key_bytes = key.as_bytes();
        let len = key_bytes.len().min(31);
        diag.key[..len].copy_from_slice(&key_bytes[..len]);
        diag.key[len] = 0;

        // Copy value
        let val_bytes = value.as_bytes();
        let len = val_bytes.len().min(63);
        diag.value[..len].copy_from_slice(&val_bytes[..len]);
        diag.value[len] = 0;

        diag
    }

    /// Create an integer diagnostic value
    pub fn int(key: &str, value: i64) -> Self {
        Self::string(key, &value.to_string()).with_type(Self::TYPE_INT)
    }

    /// Create a float diagnostic value
    pub fn float(key: &str, value: f64) -> Self {
        Self::string(key, &format!("{:.3}", value)).with_type(Self::TYPE_FLOAT)
    }

    /// Create a boolean diagnostic value
    pub fn bool(key: &str, value: bool) -> Self {
        Self::string(key, if value { "true" } else { "false" }).with_type(Self::TYPE_BOOL)
    }

    fn with_type(mut self, value_type: u8) -> Self {
        self.value_type = value_type;
        self
    }
}

/// Diagnostic report with multiple key-value pairs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// Component name
    pub component: [u8; 32],
    /// Diagnostic values (max 16)
    #[serde(with = "serde_arrays")]
    pub values: [DiagnosticValue; 16],
    /// Number of valid values
    pub value_count: u8,
    /// Overall status level
    pub level: StatusLevel,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for DiagnosticReport {
    fn default() -> Self {
        Self {
            component: [0; 32],
            values: [DiagnosticValue::default(); 16],
            value_count: 0,
            level: StatusLevel::Ok,
            timestamp: 0,
        }
    }
}

impl DiagnosticReport {
    /// Create a new diagnostic report
    pub fn new(component: &str) -> Self {
        let mut report = Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        };

        // Copy component name
        let comp_bytes = component.as_bytes();
        let len = comp_bytes.len().min(31);
        report.component[..len].copy_from_slice(&comp_bytes[..len]);
        report.component[len] = 0;

        report
    }

    /// Add a diagnostic value
    pub fn add_value(&mut self, value: DiagnosticValue) -> Result<(), &'static str> {
        if self.value_count >= 16 {
            return Err("Maximum 16 diagnostic values");
        }

        self.values[self.value_count as usize] = value;
        self.value_count += 1;
        Ok(())
    }

    /// Add a string value
    pub fn add_string(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
        self.add_value(DiagnosticValue::string(key, value))
    }

    /// Add an integer value
    pub fn add_int(&mut self, key: &str, value: i64) -> Result<(), &'static str> {
        self.add_value(DiagnosticValue::int(key, value))
    }

    /// Add a float value
    pub fn add_float(&mut self, key: &str, value: f64) -> Result<(), &'static str> {
        self.add_value(DiagnosticValue::float(key, value))
    }

    /// Add a boolean value
    pub fn add_bool(&mut self, key: &str, value: bool) -> Result<(), &'static str> {
        self.add_value(DiagnosticValue::bool(key, value))
    }

    /// Set the overall status level
    pub fn set_level(&mut self, level: StatusLevel) {
        self.level = level;
    }
}

/// Node execution state
///
/// Represents the current execution state of a HORUS node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NodeState {
    /// Node created but not started
    Idle = 0,
    /// Running initialization
    Initializing = 1,
    /// Active and executing tick()
    Running = 2,
    /// Temporarily suspended
    Paused = 3,
    /// Cleanly shut down
    Stopped = 4,
    /// Error or crashed state
    Error = 5,
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Idle
    }
}

impl NodeState {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Initializing => "Initializing",
            Self::Running => "Running",
            Self::Paused => "Paused",
            Self::Stopped => "Stopped",
            Self::Error => "Error",
        }
    }
}

/// Node health status
///
/// Indicates the operational health of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum HealthStatus {
    /// Operating normally
    Healthy = 0,
    /// Degraded performance (slow ticks, missed deadlines)
    Warning = 1,
    /// Errors occurring but still running
    Error = 2,
    /// Fatal errors, about to crash or unresponsive
    Critical = 3,
    /// Status unknown (no heartbeat received)
    Unknown = 4,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl HealthStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::Critical => "Critical",
            Self::Unknown => "Unknown",
        }
    }

    /// Get color code for dashboard display
    pub fn color(&self) -> &'static str {
        match self {
            Self::Healthy => "green",
            Self::Warning => "yellow",
            Self::Error => "orange",
            Self::Critical => "red",
            Self::Unknown => "gray",
        }
    }
}

/// Node status heartbeat with health information
///
/// Written to /dev/shm/horus/heartbeats/{node_name} for monitoring.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NodeHeartbeat {
    /// Node execution state
    pub state: NodeState,
    /// Health status
    pub health: HealthStatus,
    /// Tick count
    pub tick_count: u64,
    /// Target tick rate (Hz)
    pub target_rate_hz: u32,
    /// Actual measured tick rate (Hz)
    pub actual_rate_hz: u32,
    /// Error count
    pub error_count: u32,
    /// Last successful tick timestamp (unix epoch seconds)
    pub last_tick_timestamp: u64,
    /// Heartbeat timestamp (unix epoch seconds)
    pub heartbeat_timestamp: u64,
}

impl Default for NodeHeartbeat {
    fn default() -> Self {
        Self {
            state: NodeState::Idle,
            health: HealthStatus::Unknown,
            tick_count: 0,
            target_rate_hz: 0,
            actual_rate_hz: 0,
            error_count: 0,
            last_tick_timestamp: 0,
            heartbeat_timestamp: 0,
        }
    }
}

impl NodeHeartbeat {
    /// Create a new heartbeat
    pub fn new(state: NodeState, health: HealthStatus) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            state,
            health,
            heartbeat_timestamp: now,
            last_tick_timestamp: now,
            ..Default::default()
        }
    }

    /// Update the heartbeat timestamp
    pub fn update_timestamp(&mut self) {
        self.heartbeat_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Check if heartbeat is fresh (within last N seconds)
    pub fn is_fresh(&self, max_age_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now.saturating_sub(self.heartbeat_timestamp) <= max_age_secs
    }

    /// Serialize to bytes (for file writing)
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize from bytes (for file reading)
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }
}

/// Safety system status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct SafetyStatus {
    /// Safety system is active
    pub enabled: bool,
    /// Emergency stop is engaged
    pub estop_engaged: bool,
    /// Watchdog timer is OK
    pub watchdog_ok: bool,
    /// All limits are within bounds
    pub limits_ok: bool,
    /// Communication is healthy
    pub comms_ok: bool,
    /// Safety mode (0=normal, 1=reduced, 2=safe_stop)
    pub mode: u8,
    /// Fault code if any
    pub fault_code: u32,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl SafetyStatus {
    pub const MODE_NORMAL: u8 = 0;
    pub const MODE_REDUCED: u8 = 1;
    pub const MODE_SAFE_STOP: u8 = 2;

    /// Create a new safety status
    pub fn new() -> Self {
        Self {
            enabled: true,
            estop_engaged: false,
            watchdog_ok: true,
            limits_ok: true,
            comms_ok: true,
            mode: Self::MODE_NORMAL,
            fault_code: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Check if system is safe to operate
    pub fn is_safe(&self) -> bool {
        self.enabled
            && !self.estop_engaged
            && self.watchdog_ok
            && self.limits_ok
            && self.comms_ok
            && self.fault_code == 0
    }

    /// Set a fault condition
    pub fn set_fault(&mut self, code: u32) {
        self.fault_code = code;
        if code != 0 {
            self.mode = Self::MODE_SAFE_STOP;
        }
    }

    /// Clear all faults
    pub fn clear_faults(&mut self) {
        self.fault_code = 0;
        self.mode = Self::MODE_NORMAL;
    }
}


impl LogSummary for Heartbeat {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for Status {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for EmergencyStop {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for ResourceUsage {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for DiagnosticValue {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for DiagnosticReport {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for NodeHeartbeat {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for SafetyStatus {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for StatusLevel {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for NodeState {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}


impl LogSummary for HealthStatus {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat() {
        let mut hb = Heartbeat::new("test_node", 42);
        assert_eq!(hb.name(), "test_node");
        assert_eq!(hb.node_id, 42);
        assert_eq!(hb.sequence, 0);

        hb.update(10.5);
        assert_eq!(hb.sequence, 1);
        assert_eq!(hb.uptime, 10.5);
    }

    #[test]
    fn test_status_messages() {
        let status = Status::warn(100, "Temperature high").with_component("motor_controller");
        assert_eq!(status.level, StatusLevel::Warn);
        assert_eq!(status.code, 100);
        assert_eq!(status.message_str(), "Temperature high");
        assert_eq!(status.component_str(), "motor_controller");
    }

    #[test]
    fn test_emergency_stop() {
        let estop = EmergencyStop::engage("Obstacle detected").with_source("lidar_safety");
        assert!(estop.engaged);
        assert_eq!(estop.reason_str(), "Obstacle detected");

        let release = EmergencyStop::release();
        assert!(!release.engaged);
    }

    #[test]
    fn test_diagnostic_report() {
        let mut report = DiagnosticReport::new("motor_diagnostics");
        report.add_float("temperature", 45.6).unwrap();
        report.add_int("error_count", 3).unwrap();
        report.add_bool("enabled", true).unwrap();

        assert_eq!(report.value_count, 3);
        report.set_level(StatusLevel::Warn);
        assert_eq!(report.level, StatusLevel::Warn);
    }

    #[test]
    fn test_safety_status() {
        let mut safety = SafetyStatus::new();
        assert!(safety.is_safe());

        safety.set_fault(42);
        assert!(!safety.is_safe());
        assert_eq!(safety.mode, SafetyStatus::MODE_SAFE_STOP);

        safety.clear_faults();
        assert!(safety.is_safe());
        assert_eq!(safety.mode, SafetyStatus::MODE_NORMAL);
    }
}
