use serde::{Deserialize, Serialize};

/// Command velocity message for robot control
///
/// Standard message type used across the HORUS ecosystem for controlling
/// robot movement. Contains linear and angular velocity commands.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub struct CmdVel {
    pub stamp_nanos: u64,
    pub linear: f32,  // m/s forward velocity
    pub angular: f32, // rad/s turning velocity
}

impl CmdVel {
    /// Create a new CmdVel message with current timestamp
    pub fn new(linear: f32, angular: f32) -> Self {
        Self {
            stamp_nanos: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            linear,
            angular,
        }
    }

    /// Create a zero velocity command (stop)
    pub fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    /// Create a CmdVel with explicit timestamp
    pub fn with_timestamp(linear: f32, angular: f32, stamp_nanos: u64) -> Self {
        Self {
            stamp_nanos,
            linear,
            angular,
        }
    }
}

impl Default for CmdVel {
    fn default() -> Self {
        Self::zero()
    }
}

// Enable zero-copy serialization with bytemuck
unsafe impl bytemuck::Pod for CmdVel {}
unsafe impl bytemuck::Zeroable for CmdVel {}

// Enable iceoryx2 zero-copy IPC
#[cfg(feature = "iceoryx2")]
unsafe impl iceoryx2_bb_elementary_traits::zero_copy_send::ZeroCopySend for CmdVel {}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_cmd_vel_creation() {
        let cmd = CmdVel::new(1.5, 0.8);
        assert_relative_eq!(cmd.linear, 1.5);
        assert_relative_eq!(cmd.angular, 0.8);
        assert!(cmd.stamp_nanos > 0);
    }

    #[test]
    fn test_cmd_vel_zero() {
        let cmd = CmdVel::zero();
        assert_relative_eq!(cmd.linear, 0.0);
        assert_relative_eq!(cmd.angular, 0.0);
    }

    #[test]
    fn test_cmd_vel_with_timestamp() {
        let cmd = CmdVel::with_timestamp(2.0, 1.0, 123456789);
        assert_relative_eq!(cmd.linear, 2.0);
        assert_relative_eq!(cmd.angular, 1.0);
        assert_eq!(cmd.stamp_nanos, 123456789);
    }

    #[test]
    fn test_bytemuck_traits() {
        let cmd = CmdVel::new(1.0, 2.0);
        let _bytes: &[u8] = bytemuck::bytes_of(&cmd);
    }
}
