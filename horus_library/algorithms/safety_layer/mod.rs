//! Safety Layer for Robot Control
//!
//! Multi-layered safety monitoring and enforcement system.
//!
//! # Features
//!
//! - Velocity limiting
//! - Obstacle distance monitoring
//! - Battery level checking
//! - Temperature monitoring
//! - Configurable safety thresholds
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::safety_layer::SafetyLayer;
//!
//! let mut safety = SafetyLayer::new();
//!
//! // Configure safety limits
//! safety.set_max_velocity(2.0);
//! safety.set_min_obstacle_distance(0.5);
//!
//! // Check safety
//! if safety.check_velocity(2.5) {
//!     println!("Velocity safe!");
//! }
//! ```

/// Safety check result
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetyStatus {
    Safe,
    Warning,
    Critical,
}

/// Safety Layer
pub struct SafetyLayer {
    max_velocity: f64,
    min_obstacle_distance: f64,
    min_battery_percent: f64,
    max_temperature: f64,

    velocity_enabled: bool,
    obstacle_enabled: bool,
    battery_enabled: bool,
    temperature_enabled: bool,
}

impl SafetyLayer {
    /// Create new safety layer with default limits
    pub fn new() -> Self {
        Self {
            max_velocity: 2.0,
            min_obstacle_distance: 0.3,
            min_battery_percent: 10.0,
            max_temperature: 80.0,

            velocity_enabled: true,
            obstacle_enabled: true,
            battery_enabled: true,
            temperature_enabled: true,
        }
    }

    /// Set maximum safe velocity (m/s)
    pub fn set_max_velocity(&mut self, max: f64) {
        self.max_velocity = max;
    }

    /// Set minimum obstacle distance (m)
    pub fn set_min_obstacle_distance(&mut self, min: f64) {
        self.min_obstacle_distance = min;
    }

    /// Set minimum battery level (%)
    pub fn set_min_battery(&mut self, min: f64) {
        self.min_battery_percent = min;
    }

    /// Set maximum temperature (°C)
    pub fn set_max_temperature(&mut self, max: f64) {
        self.max_temperature = max;
    }

    /// Enable/disable velocity check
    pub fn enable_velocity_check(&mut self, enabled: bool) {
        self.velocity_enabled = enabled;
    }

    /// Enable/disable obstacle check
    pub fn enable_obstacle_check(&mut self, enabled: bool) {
        self.obstacle_enabled = enabled;
    }

    /// Enable/disable battery check
    pub fn enable_battery_check(&mut self, enabled: bool) {
        self.battery_enabled = enabled;
    }

    /// Enable/disable temperature check
    pub fn enable_temperature_check(&mut self, enabled: bool) {
        self.temperature_enabled = enabled;
    }

    /// Check if velocity is safe
    pub fn check_velocity(&self, velocity: f64) -> bool {
        !self.velocity_enabled || velocity.abs() <= self.max_velocity
    }

    /// Check if obstacle distance is safe
    pub fn check_obstacle_distance(&self, distance: f64) -> bool {
        !self.obstacle_enabled || distance >= self.min_obstacle_distance
    }

    /// Check if battery level is safe
    pub fn check_battery(&self, percent: f64) -> bool {
        !self.battery_enabled || percent >= self.min_battery_percent
    }

    /// Check if temperature is safe
    pub fn check_temperature(&self, temp: f64) -> bool {
        !self.temperature_enabled || temp <= self.max_temperature
    }

    /// Perform comprehensive safety check
    pub fn check_all(
        &self,
        velocity: f64,
        obstacle_dist: f64,
        battery: f64,
        temp: f64,
    ) -> SafetyStatus {
        let mut critical = false;
        let mut warning = false;

        // Velocity check
        if self.velocity_enabled {
            if velocity.abs() > self.max_velocity * 1.2 {
                critical = true;
            } else if velocity.abs() > self.max_velocity {
                warning = true;
            }
        }

        // Obstacle check
        if self.obstacle_enabled {
            if obstacle_dist < self.min_obstacle_distance * 0.5 {
                critical = true;
            } else if obstacle_dist < self.min_obstacle_distance {
                warning = true;
            }
        }

        // Battery check
        if self.battery_enabled {
            if battery < self.min_battery_percent * 0.5 {
                critical = true;
            } else if battery < self.min_battery_percent {
                warning = true;
            }
        }

        // Temperature check
        if self.temperature_enabled {
            if temp > self.max_temperature * 1.1 {
                critical = true;
            } else if temp > self.max_temperature {
                warning = true;
            }
        }

        if critical {
            SafetyStatus::Critical
        } else if warning {
            SafetyStatus::Warning
        } else {
            SafetyStatus::Safe
        }
    }

    /// Limit velocity to safe range
    pub fn limit_velocity(&self, velocity: f64) -> f64 {
        velocity.clamp(-self.max_velocity, self.max_velocity)
    }
}

impl Default for SafetyLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_check() {
        let safety = SafetyLayer::new();

        assert!(safety.check_velocity(1.5));
        assert!(safety.check_velocity(2.0));
        assert!(!safety.check_velocity(2.5));
    }

    #[test]
    fn test_obstacle_check() {
        let safety = SafetyLayer::new();

        assert!(safety.check_obstacle_distance(0.5));
        assert!(safety.check_obstacle_distance(0.3));
        assert!(!safety.check_obstacle_distance(0.2));
    }

    #[test]
    fn test_battery_check() {
        let safety = SafetyLayer::new();

        assert!(safety.check_battery(50.0));
        assert!(safety.check_battery(10.0));
        assert!(!safety.check_battery(5.0));
    }

    #[test]
    fn test_temperature_check() {
        let safety = SafetyLayer::new();

        assert!(safety.check_temperature(60.0));
        assert!(safety.check_temperature(80.0));
        assert!(!safety.check_temperature(90.0));
    }

    #[test]
    fn test_comprehensive_safe() {
        let safety = SafetyLayer::new();

        let status = safety.check_all(1.0, 1.0, 50.0, 60.0);
        assert_eq!(status, SafetyStatus::Safe);
    }

    #[test]
    fn test_comprehensive_warning() {
        let safety = SafetyLayer::new();

        let status = safety.check_all(2.1, 1.0, 50.0, 60.0);
        assert_eq!(status, SafetyStatus::Warning);
    }

    #[test]
    fn test_comprehensive_critical() {
        let safety = SafetyLayer::new();

        let status = safety.check_all(3.0, 1.0, 50.0, 60.0);
        assert_eq!(status, SafetyStatus::Critical);
    }

    #[test]
    fn test_disable_checks() {
        let mut safety = SafetyLayer::new();
        safety.enable_velocity_check(false);

        // Should pass even with excessive velocity
        assert!(safety.check_velocity(100.0));
    }

    #[test]
    fn test_limit_velocity() {
        let safety = SafetyLayer::new();

        assert_eq!(safety.limit_velocity(1.5), 1.5);
        assert_eq!(safety.limit_velocity(3.0), 2.0);
        assert_eq!(safety.limit_velocity(-3.0), -2.0);
    }

    #[test]
    fn test_custom_limits() {
        let mut safety = SafetyLayer::new();
        safety.set_max_velocity(5.0);
        safety.set_min_obstacle_distance(1.0);

        assert!(safety.check_velocity(4.5));
        assert!(!safety.check_obstacle_distance(0.8));
    }
}
