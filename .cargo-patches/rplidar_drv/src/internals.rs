use std::time::Duration;
use super::answers::*;

/// Default timeout when communicating with RPLIDAR
pub const RPLIDAR_DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

/// Default cache depth of scan points
pub const RPLIDAR_DEFAULT_CACHE_DEPTH: usize = 8192;

/// Default motor PWM
pub const RPLIDAR_DEFAULT_MOTOR_PWM: u16 = 600;

#[derive(Debug, Clone, PartialEq)]
pub enum CachedPrevCapsule {
    None,
    Capsuled(RplidarResponseCapsuleMeasurementNodes),
    UltraCapsuled(RplidarResponseUltraCapsuleMeasurementNodes),
}
