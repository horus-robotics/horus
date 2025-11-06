use horus_core::core::LogSummary;
// Force and tactile feedback message types for robotics
//
// This module provides messages for force sensors, tactile arrays,
// impedance control, and haptic feedback systems.

use crate::messages::geometry::{Point3, Vector3};
use serde::{Deserialize, Serialize};
use serde_arrays;

/// Force and torque measurement (wrench)
///
/// Represents 6-DOF force and torque measurements from force/torque sensors,
/// commonly used in manipulation and contact tasks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct WrenchStamped {
    /// Force vector [fx, fy, fz] in Newtons
    pub force: Vector3,
    /// Torque vector [tx, ty, tz] in Newton-meters
    pub torque: Vector3,
    /// Point of application (relative to sensor frame)
    pub point_of_application: Point3,
    /// Frame ID for the measurement
    pub frame_id: [u8; 32],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl WrenchStamped {
    /// Create a new wrench measurement
    pub fn new(force: Vector3, torque: Vector3) -> Self {
        Self {
            force,
            torque,
            point_of_application: Point3::origin(),
            frame_id: [0; 32],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create from force only
    pub fn force_only(force: Vector3) -> Self {
        Self::new(force, Vector3::zero())
    }

    /// Create from torque only
    pub fn torque_only(torque: Vector3) -> Self {
        Self::new(Vector3::zero(), torque)
    }

    /// Set the frame ID
    pub fn with_frame_id(mut self, frame_id: &str) -> Self {
        let frame_bytes = frame_id.as_bytes();
        let len = frame_bytes.len().min(31);
        self.frame_id[..len].copy_from_slice(&frame_bytes[..len]);
        self.frame_id[len] = 0;
        self
    }

    /// Get the magnitude of the force
    pub fn force_magnitude(&self) -> f64 {
        self.force.magnitude()
    }

    /// Get the magnitude of the torque
    pub fn torque_magnitude(&self) -> f64 {
        self.torque.magnitude()
    }

    /// Check if the wrench exceeds safety limits
    pub fn exceeds_limits(&self, max_force: f64, max_torque: f64) -> bool {
        self.force_magnitude() > max_force || self.torque_magnitude() > max_torque
    }

    /// Apply a simple low-pass filter to reduce noise
    pub fn filter(&mut self, prev_wrench: &WrenchStamped, alpha: f64) {
        let alpha = alpha.clamp(0.0, 1.0);

        // Filter force
        self.force.x = alpha * self.force.x + (1.0 - alpha) * prev_wrench.force.x;
        self.force.y = alpha * self.force.y + (1.0 - alpha) * prev_wrench.force.y;
        self.force.z = alpha * self.force.z + (1.0 - alpha) * prev_wrench.force.z;

        // Filter torque
        self.torque.x = alpha * self.torque.x + (1.0 - alpha) * prev_wrench.torque.x;
        self.torque.y = alpha * self.torque.y + (1.0 - alpha) * prev_wrench.torque.y;
        self.torque.z = alpha * self.torque.z + (1.0 - alpha) * prev_wrench.torque.z;
    }
}

/// Tactile sensor array data
///
/// Represents pressure/force measurements from multiple tactile sensors
/// arranged in an array (e.g., fingertip sensors, skin patches).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TactileArray {
    /// Array of pressure/force readings
    #[serde(with = "serde_arrays")]
    pub sensors: [f32; 64],
    /// Number of active sensors
    pub sensor_count: u8,
    /// Sensor arrangement (0=grid, 1=linear, 2=circular)
    pub arrangement: u8,
    /// Grid dimensions for grid arrangement
    pub grid_width: u8,
    pub grid_height: u8,
    /// Sensor spacing in millimeters
    pub sensor_spacing: f32,
    /// Sensor sensitivity (force per unit reading)
    pub sensitivity: f32,
    /// Frame ID for sensor location
    pub frame_id: [u8; 32],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl Default for TactileArray {
    fn default() -> Self {
        Self {
            sensors: [0.0; 64],
            sensor_count: 0,
            arrangement: 0,
            grid_width: 8,
            grid_height: 8,
            sensor_spacing: 2.0, // 2mm spacing
            sensitivity: 1.0,
            frame_id: [0; 32],
            timestamp: 0,
        }
    }
}

impl TactileArray {
    pub const ARRANGEMENT_GRID: u8 = 0;
    pub const ARRANGEMENT_LINEAR: u8 = 1;
    pub const ARRANGEMENT_CIRCULAR: u8 = 2;

    /// Create a new tactile array
    pub fn new(sensor_count: u8, arrangement: u8) -> Self {
        Self {
            sensor_count,
            arrangement,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            ..Default::default()
        }
    }

    /// Get active sensor readings
    pub fn get_active_sensors(&self) -> &[f32] {
        &self.sensors[..self.sensor_count as usize]
    }

    /// Set sensor reading
    pub fn set_sensor(&mut self, index: u8, value: f32) -> bool {
        if index < self.sensor_count && (index as usize) < self.sensors.len() {
            self.sensors[index as usize] = value;
            true
        } else {
            false
        }
    }

    /// Get sensor reading
    pub fn get_sensor(&self, index: u8) -> Option<f32> {
        if index < self.sensor_count && (index as usize) < self.sensors.len() {
            Some(self.sensors[index as usize])
        } else {
            None
        }
    }

    /// Calculate total force from all sensors
    pub fn total_force(&self) -> f32 {
        self.get_active_sensors().iter().sum::<f32>() * self.sensitivity
    }

    /// Calculate center of pressure for grid arrangement
    pub fn center_of_pressure(&self) -> Option<(f32, f32)> {
        if self.arrangement != Self::ARRANGEMENT_GRID || self.sensor_count == 0 {
            return None;
        }

        let mut total_force = 0.0;
        let mut weighted_x = 0.0;
        let mut weighted_y = 0.0;

        for i in 0..self.sensor_count {
            let sensor_force = self.sensors[i as usize];
            if sensor_force > 0.0 {
                let x = (i % self.grid_width) as f32;
                let y = (i / self.grid_width) as f32;

                weighted_x += x * sensor_force;
                weighted_y += y * sensor_force;
                total_force += sensor_force;
            }
        }

        if total_force > 0.0 {
            Some((weighted_x / total_force, weighted_y / total_force))
        } else {
            None
        }
    }

    /// Detect contact (any sensor above threshold)
    pub fn detect_contact(&self, threshold: f32) -> bool {
        self.get_active_sensors()
            .iter()
            .any(|&reading| reading > threshold)
    }

    /// Get contact pattern as boolean array
    pub fn contact_pattern(&self, threshold: f32) -> Vec<bool> {
        self.get_active_sensors()
            .iter()
            .map(|&reading| reading > threshold)
            .collect()
    }
}

/// Impedance control parameters
///
/// Defines the impedance behavior for force-controlled manipulation tasks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ImpedanceParameters {
    /// Stiffness matrix diagonal [Kx, Ky, Kz, Krx, Kry, Krz]
    pub stiffness: [f64; 6],
    /// Damping matrix diagonal [Dx, Dy, Dz, Drx, Dry, Drz]
    pub damping: [f64; 6],
    /// Inertia matrix diagonal [Mx, My, Mz, Mrx, Mry, Mrz]
    pub inertia: [f64; 6],
    /// Force limits [Fx_max, Fy_max, Fz_max, Tx_max, Ty_max, Tz_max]
    pub force_limits: [f64; 6],
    /// Whether impedance control is active
    pub enabled: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl ImpedanceParameters {
    /// Create new impedance parameters
    pub fn new() -> Self {
        Self {
            // Default: moderate stiffness and damping
            stiffness: [1000.0, 1000.0, 1000.0, 100.0, 100.0, 100.0],
            damping: [50.0, 50.0, 50.0, 5.0, 5.0, 5.0],
            inertia: [1.0, 1.0, 1.0, 0.1, 0.1, 0.1],
            force_limits: [50.0, 50.0, 50.0, 5.0, 5.0, 5.0],
            enabled: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create compliant impedance (low stiffness)
    pub fn compliant() -> Self {
        Self {
            stiffness: [100.0, 100.0, 100.0, 10.0, 10.0, 10.0],
            damping: [20.0, 20.0, 20.0, 2.0, 2.0, 2.0],
            ..Self::new()
        }
    }

    /// Create stiff impedance (high stiffness)
    pub fn stiff() -> Self {
        Self {
            stiffness: [5000.0, 5000.0, 5000.0, 500.0, 500.0, 500.0],
            damping: [100.0, 100.0, 100.0, 10.0, 10.0, 10.0],
            ..Self::new()
        }
    }

    /// Enable impedance control
    pub fn enable(&mut self) {
        self.enabled = true;
        self.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
    }

    /// Disable impedance control
    pub fn disable(&mut self) {
        self.enabled = false;
        self.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
    }
}

/// Force control command
///
/// Commands for force-controlled manipulation tasks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ForceCommand {
    /// Desired force vector [fx, fy, fz]
    pub target_force: Vector3,
    /// Desired torque vector [tx, ty, tz]
    pub target_torque: Vector3,
    /// Control mode selection [force_x, force_y, force_z, torque_x, torque_y, torque_z]
    /// true = force control, false = position control
    pub force_mode: [bool; 6],
    /// Position setpoint for position-controlled axes
    pub position_setpoint: Vector3,
    /// Orientation setpoint for orientation-controlled axes
    pub orientation_setpoint: Vector3, // Euler angles
    /// Maximum allowed deviation from setpoint
    pub max_deviation: Vector3,
    /// Control gains
    pub gains: [f64; 6], // [Kp_fx, Kp_fy, Kp_fz, Kp_tx, Kp_ty, Kp_tz]
    /// Timeout for command (0 = no timeout)
    pub timeout_seconds: f64,
    /// Frame ID for force/torque reference
    pub frame_id: [u8; 32],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl ForceCommand {
    /// Create a pure force command
    pub fn force_only(target_force: Vector3) -> Self {
        Self {
            target_force,
            target_torque: Vector3::zero(),
            force_mode: [true, true, true, false, false, false],
            position_setpoint: Vector3::zero(),
            orientation_setpoint: Vector3::zero(),
            max_deviation: Vector3::new(0.01, 0.01, 0.01), // 1cm max deviation
            gains: [1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            timeout_seconds: 0.0,
            frame_id: [0; 32],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a hybrid force/position command
    pub fn hybrid(force_axes: [bool; 6], target_force: Vector3, target_position: Vector3) -> Self {
        Self {
            target_force,
            target_torque: Vector3::zero(),
            force_mode: force_axes,
            position_setpoint: target_position,
            orientation_setpoint: Vector3::zero(),
            max_deviation: Vector3::new(0.01, 0.01, 0.01),
            gains: [1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            timeout_seconds: 0.0,
            frame_id: [0; 32],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a contact force command for surface following
    pub fn surface_contact(normal_force: f64, surface_normal: Vector3) -> Self {
        let mut normal = surface_normal;
        normal.normalize();
        let target_force = Vector3::new(
            normal.x * normal_force,
            normal.y * normal_force,
            normal.z * normal_force,
        );

        // Force control in normal direction, position control in tangential directions
        let force_mode = if normal.z.abs() > 0.8 {
            // Mostly vertical surface - control Z force
            [false, false, true, false, false, false]
        } else {
            // General case - control along surface normal (approximate)
            [true, true, true, false, false, false]
        };

        Self::hybrid(force_mode, target_force, Vector3::zero())
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_seconds: f64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }
}

/// Contact state detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ContactState {
    /// No contact detected
    NoContact = 0,
    /// Initial contact detected
    InitialContact = 1,
    /// Stable contact established
    StableContact = 2,
    /// Contact being broken
    ContactLoss = 3,
    /// Sliding contact
    Sliding = 4,
    /// Impact detected
    Impact = 5,
}

impl Default for ContactState {
    fn default() -> Self {
        Self::NoContact
    }
}

/// Contact detection and classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct ContactInfo {
    /// Current contact state
    pub state: ContactState,
    /// Contact force magnitude
    pub contact_force: f64,
    /// Contact normal vector (estimated)
    pub contact_normal: Vector3,
    /// Contact point (estimated)
    pub contact_point: Point3,
    /// Contact stiffness (estimated)
    pub stiffness: f64,
    /// Contact damping (estimated)
    pub damping: f64,
    /// Confidence in detection (0.0 to 1.0)
    pub confidence: f32,
    /// Time contact was first detected
    pub contact_start_time: u64,
    /// Frame ID for contact point reference
    pub frame_id: [u8; 32],
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl ContactInfo {
    /// Create new contact info
    pub fn new(state: ContactState, force_magnitude: f64) -> Self {
        Self {
            state,
            contact_force: force_magnitude,
            contact_normal: Vector3::new(0.0, 0.0, 1.0), // Default to Z-up
            contact_point: Point3::origin(),
            stiffness: 0.0,
            damping: 0.0,
            confidence: 0.5,
            contact_start_time: 0,
            frame_id: [0; 32],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Check if currently in contact
    pub fn is_in_contact(&self) -> bool {
        matches!(
            self.state,
            ContactState::InitialContact | ContactState::StableContact | ContactState::Sliding
        )
    }

    /// Get contact duration in seconds
    pub fn contact_duration_seconds(&self) -> f64 {
        if self.contact_start_time > 0 {
            (self.timestamp - self.contact_start_time) as f64 / 1e9
        } else {
            0.0
        }
    }
}

/// Haptic feedback command
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct HapticFeedback {
    /// Vibration intensity (0.0 to 1.0)
    pub vibration_intensity: f32,
    /// Vibration frequency in Hz
    pub vibration_frequency: f32,
    /// Duration of feedback in seconds
    pub duration_seconds: f32,
    /// Force feedback vector
    pub force_feedback: Vector3,
    /// Feedback pattern type (0=constant, 1=pulse, 2=ramp)
    pub pattern_type: u8,
    /// Enable/disable feedback
    pub enabled: bool,
    /// Timestamp in nanoseconds since epoch
    pub timestamp: u64,
}

impl HapticFeedback {
    pub const PATTERN_CONSTANT: u8 = 0;
    pub const PATTERN_PULSE: u8 = 1;
    pub const PATTERN_RAMP: u8 = 2;

    /// Create a vibration feedback
    pub fn vibration(intensity: f32, frequency: f32, duration: f32) -> Self {
        Self {
            vibration_intensity: intensity.clamp(0.0, 1.0),
            vibration_frequency: frequency,
            duration_seconds: duration,
            force_feedback: Vector3::zero(),
            pattern_type: Self::PATTERN_CONSTANT,
            enabled: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a force feedback
    pub fn force(force: Vector3, duration: f32) -> Self {
        Self {
            vibration_intensity: 0.0,
            vibration_frequency: 0.0,
            duration_seconds: duration,
            force_feedback: force,
            pattern_type: Self::PATTERN_CONSTANT,
            enabled: true,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        }
    }

    /// Create a pulse pattern
    pub fn pulse(intensity: f32, frequency: f32, duration: f32) -> Self {
        Self {
            pattern_type: Self::PATTERN_PULSE,
            ..Self::vibration(intensity, frequency, duration)
        }
    }
}

impl LogSummary for WrenchStamped {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for TactileArray {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for ImpedanceParameters {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for ForceCommand {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for ContactInfo {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for HapticFeedback {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

impl LogSummary for ContactState {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrench_creation() {
        let force = Vector3::new(10.0, 5.0, 0.0);
        let torque = Vector3::new(0.0, 0.0, 2.0);
        let wrench = WrenchStamped::new(force, torque);

        assert_eq!(wrench.force_magnitude(), force.magnitude());
        assert_eq!(wrench.torque_magnitude(), 2.0);
    }

    #[test]
    fn test_wrench_limits() {
        let force = Vector3::new(100.0, 0.0, 0.0);
        let wrench = WrenchStamped::force_only(force);

        assert!(wrench.exceeds_limits(50.0, 10.0));
        assert!(!wrench.exceeds_limits(150.0, 10.0));
    }

    #[test]
    fn test_tactile_array() {
        let mut tactile = TactileArray::new(16, TactileArray::ARRANGEMENT_GRID);
        tactile.grid_width = 4;
        tactile.grid_height = 4;

        // Set some sensor values
        tactile.set_sensor(0, 5.0);
        tactile.set_sensor(5, 10.0);
        tactile.set_sensor(10, 3.0);

        assert_eq!(tactile.get_sensor(0), Some(5.0));
        assert_eq!(tactile.total_force(), 18.0);
        assert!(tactile.detect_contact(2.0));
    }

    #[test]
    fn test_center_of_pressure() {
        let mut tactile = TactileArray::new(9, TactileArray::ARRANGEMENT_GRID);
        tactile.grid_width = 3;
        tactile.grid_height = 3;

        // Set corner sensors
        tactile.set_sensor(0, 10.0); // (0,0)
        tactile.set_sensor(2, 10.0); // (2,0)
        tactile.set_sensor(6, 10.0); // (0,2)
        tactile.set_sensor(8, 10.0); // (2,2)

        if let Some((cx, cy)) = tactile.center_of_pressure() {
            assert_eq!(cx, 1.0); // Center X
            assert_eq!(cy, 1.0); // Center Y
        } else {
            panic!("Should have calculated center of pressure");
        }
    }

    #[test]
    fn test_impedance_parameters() {
        let mut params = ImpedanceParameters::compliant();
        assert!(!params.enabled);
        assert_eq!(params.stiffness[0], 100.0);

        params.enable();
        assert!(params.enabled);
    }

    #[test]
    fn test_force_command() {
        let force = Vector3::new(0.0, 0.0, -10.0); // Downward force
        let cmd = ForceCommand::force_only(force);

        assert_eq!(cmd.target_force.z, -10.0);
        assert!(cmd.force_mode[2]); // Z-axis is force controlled
    }

    #[test]
    fn test_contact_info() {
        let contact = ContactInfo::new(ContactState::StableContact, 15.0);
        assert!(contact.is_in_contact());
        assert_eq!(contact.contact_force, 15.0);

        let no_contact = ContactInfo::new(ContactState::NoContact, 0.0);
        assert!(!no_contact.is_in_contact());
    }

    #[test]
    fn test_haptic_feedback() {
        let haptic = HapticFeedback::vibration(0.8, 200.0, 1.0);
        assert_eq!(haptic.vibration_intensity, 0.8);
        assert_eq!(haptic.pattern_type, HapticFeedback::PATTERN_CONSTANT);

        let pulse = HapticFeedback::pulse(0.5, 100.0, 2.0);
        assert_eq!(pulse.pattern_type, HapticFeedback::PATTERN_PULSE);
    }
}
