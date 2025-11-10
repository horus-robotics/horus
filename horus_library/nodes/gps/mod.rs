use crate::NavSatFix;
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo, NodeInfoExt};
use std::time::{SystemTime, UNIX_EPOCH};

/// GPS/GNSS Position Node
///
/// Provides GPS/GNSS position data from satellite navigation receivers.
/// Supports various GPS modules (u-blox, NMEA parsers, etc.).
/// Publishes latitude, longitude, altitude, and accuracy information.
pub struct GpsNode {
    publisher: Hub<NavSatFix>,

    // Configuration
    update_rate_hz: f32,
    min_satellites: u16,
    max_hdop: f32,
    frame_id: String,

    // State
    last_fix: NavSatFix,
    fix_count: u64,
    last_update_time: u64,

    // Simulation state
    sim_latitude: f64,
    sim_longitude: f64,
    sim_altitude: f64,
    sim_enabled: bool,
}

impl GpsNode {
    /// Create a new GPS node with default topic "gps/fix"
    pub fn new() -> Result<Self> {
        Self::new_with_topic("gps/fix")
    }

    /// Create a new GPS node with custom topic
    pub fn new_with_topic(topic: &str) -> Result<Self> {
        Ok(Self {
            publisher: Hub::new(topic)?,
            update_rate_hz: 1.0, // 1 Hz default (typical for GPS)
            min_satellites: 4,    // Minimum for 3D fix
            max_hdop: 20.0,       // Maximum acceptable HDOP
            frame_id: "gps".to_string(),
            last_fix: NavSatFix::default(),
            fix_count: 0,
            last_update_time: 0,
            sim_latitude: 37.7749,  // San Francisco (default)
            sim_longitude: -122.4194,
            sim_altitude: 10.0,
            sim_enabled: false,
        })
    }

    /// Set GPS update rate in Hz (typically 1-10 Hz)
    pub fn set_update_rate(&mut self, rate_hz: f32) {
        self.update_rate_hz = rate_hz.clamp(0.1, 20.0);
    }

    /// Set minimum number of satellites required for valid fix
    pub fn set_min_satellites(&mut self, count: u16) {
        self.min_satellites = count;
    }

    /// Set maximum acceptable HDOP
    pub fn set_max_hdop(&mut self, hdop: f32) {
        self.max_hdop = hdop;
    }

    /// Set coordinate frame ID
    pub fn set_frame_id(&mut self, frame_id: &str) {
        self.frame_id = frame_id.to_string();
    }

    /// Get last GPS fix
    pub fn get_last_fix(&self) -> &NavSatFix {
        &self.last_fix
    }

    /// Get number of fixes received
    pub fn get_fix_count(&self) -> u64 {
        self.fix_count
    }

    /// Check if we have a valid GPS fix
    pub fn has_valid_fix(&self) -> bool {
        self.last_fix.has_fix()
            && self.last_fix.satellites_visible >= self.min_satellites
            && self.last_fix.hdop <= self.max_hdop
    }

    /// Enable simulation mode with custom coordinates
    pub fn enable_simulation(&mut self, lat: f64, lon: f64, alt: f64) {
        self.sim_latitude = lat;
        self.sim_longitude = lon;
        self.sim_altitude = alt;
        self.sim_enabled = true;
    }

    /// Disable simulation mode
    pub fn disable_simulation(&mut self) {
        self.sim_enabled = false;
    }

    /// Read GPS data from receiver
    fn read_gps(&mut self, mut ctx: Option<&mut NodeInfo>) -> Option<NavSatFix> {
        // Check if enough time has passed for next update
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let update_interval_ms = (1000.0 / self.update_rate_hz) as u64;
        if current_time - self.last_update_time < update_interval_ms {
            return None;
        }

        self.last_update_time = current_time;

        // In real implementation, this would read from actual GPS hardware
        // For now, generate simulated data if enabled
        if self.sim_enabled {
            let mut fix = NavSatFix::from_coordinates(
                self.sim_latitude,
                self.sim_longitude,
                self.sim_altitude,
            );

            // Add realistic GPS characteristics
            fix.satellites_visible = 8;
            fix.hdop = 1.2;
            fix.vdop = 1.8;
            fix.speed = 0.0;
            fix.heading = 0.0;
            fix.position_covariance_type = NavSatFix::COVARIANCE_TYPE_APPROXIMATED;

            // Set covariance (rough GPS accuracy ~3m)
            fix.position_covariance[0] = 9.0; // lat variance
            fix.position_covariance[4] = 9.0; // lon variance
            fix.position_covariance[8] = 16.0; // alt variance

            ctx.log_debug(&format!(
                "GPS: {:.6}, {:.6}, alt={:.1}m, sats={}",
                fix.latitude, fix.longitude, fix.altitude, fix.satellites_visible
            ));

            Some(fix)
        } else {
            // No GPS data available (would read from hardware here)
            None
        }
    }

    /// Validate GPS fix quality
    fn validate_fix(&self, fix: &NavSatFix, mut ctx: Option<&mut NodeInfo>) -> bool {
        // Check if coordinates are valid
        if !fix.is_valid() {
            ctx.log_warning("Invalid GPS coordinates");
            return false;
        }

        // Check satellite count
        if fix.satellites_visible < self.min_satellites {
            ctx.log_warning(&format!(
                "Insufficient satellites: {} < {}",
                fix.satellites_visible, self.min_satellites
            ));
            return false;
        }

        // Check HDOP
        if fix.hdop > self.max_hdop {
            ctx.log_warning(&format!(
                "Poor GPS accuracy: HDOP {:.1} > {:.1}",
                fix.hdop, self.max_hdop
            ));
            return false;
        }

        true
    }
}

impl Node for GpsNode {
    fn name(&self) -> &'static str {
        "GpsNode"
    }

    fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        ctx.log_info("GPS node initialized");
        if self.sim_enabled {
            ctx.log_info("GPS simulation mode enabled");
        }
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Read GPS data
        if let Some(fix) = self.read_gps(ctx.as_deref_mut()) {
            // Validate fix quality
            if self.validate_fix(&fix, ctx.as_deref_mut()) {
                self.last_fix = fix;
                self.fix_count += 1;

                // Publish GPS fix
                let _ = self.publisher.send(fix, None);
            }
        }
    }
}
