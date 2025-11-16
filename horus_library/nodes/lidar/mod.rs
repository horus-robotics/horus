use crate::LaserScan;
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "rplidar")]
use rplidar_drv::{RplidarDevice, RplidarHostProtocol, ScanOptions};

/// LiDAR backend type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LidarBackend {
    Simulation,
    RplidarA1,
    RplidarA2,
    RplidarA3,
}

/// LiDAR Node - Generic LiDAR interface for obstacle detection and mapping
///
/// Captures laser scan data from various LiDAR sensors and publishes LaserScan messages.
/// Supports multiple hardware backends:
/// - RPLidar A1/A2/A3 series
/// - YDLIDAR (future)
/// - Simulation mode for testing
pub struct LidarNode {
    publisher: Hub<LaserScan>,

    // Configuration
    frame_id: String,
    scan_frequency: f32,
    min_range: f32,
    max_range: f32,
    angle_increment: f32,
    backend: LidarBackend,
    serial_port: String,

    // State
    is_initialized: bool,
    scan_count: u64,
    last_scan_time: u64,

    // Hardware drivers
    #[cfg(feature = "rplidar")]
    rplidar: Option<RplidarDevice<RplidarHostProtocol>>,
}

impl LidarNode {
    /// Create a new LiDAR node with default topic "scan" in simulation mode
    pub fn new() -> Result<Self> {
        Self::new_with_backend("scan", LidarBackend::Simulation)
    }

    /// Create a new LiDAR node with custom topic in simulation mode
    pub fn new_with_topic(topic: &str) -> Result<Self> {
        Self::new_with_backend(topic, LidarBackend::Simulation)
    }

    /// Create a new LiDAR node with specific backend
    pub fn new_with_backend(topic: &str, backend: LidarBackend) -> Result<Self> {
        Ok(Self {
            publisher: Hub::new(topic)?,
            frame_id: "laser_frame".to_string(),
            scan_frequency: 10.0,
            min_range: 0.1,
            max_range: match backend {
                LidarBackend::RplidarA1 => 12.0,  // A1: 12m max range
                LidarBackend::RplidarA2 => 16.0,  // A2: 16m max range
                LidarBackend::RplidarA3 => 25.0,  // A3: 25m max range
                _ => 30.0,
            },
            angle_increment: std::f32::consts::PI / 180.0, // 1 degree
            backend,
            serial_port: "/dev/ttyUSB0".to_string(),
            is_initialized: false,
            scan_count: 0,
            last_scan_time: 0,
            #[cfg(feature = "rplidar")]
            rplidar: None,
        })
    }

    /// Set LiDAR backend
    pub fn set_backend(&mut self, backend: LidarBackend) {
        self.backend = backend;
        self.is_initialized = false;
    }

    /// Set serial port for LiDAR
    pub fn set_serial_port(&mut self, port: &str) {
        self.serial_port = port.to_string();
        self.is_initialized = false;
    }

    /// Set frame ID for coordinate system
    pub fn set_frame_id(&mut self, frame_id: &str) {
        self.frame_id = frame_id.to_string();
    }

    /// Set scan frequency (Hz)
    pub fn set_scan_frequency(&mut self, frequency: f32) {
        self.scan_frequency = frequency.clamp(0.1, 100.0);
    }

    /// Set range limits (meters)
    pub fn set_range_limits(&mut self, min_range: f32, max_range: f32) {
        self.min_range = min_range.max(0.0);
        self.max_range = max_range.max(self.min_range + 0.1);
    }

    /// Set angular resolution (radians)
    pub fn set_angle_increment(&mut self, increment: f32) {
        self.angle_increment = increment.clamp(0.001, 0.1);
    }

    /// Get actual scan rate (scans per second)
    pub fn get_actual_scan_rate(&self) -> f32 {
        if self.scan_count < 2 {
            return 0.0;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let time_diff = current_time - self.last_scan_time;
        if time_diff > 0 {
            1000.0 / time_diff as f32
        } else {
            0.0
        }
    }

    fn initialize_lidar(&mut self) -> bool {
        if self.is_initialized {
            return true;
        }

        match self.backend {
            LidarBackend::Simulation => {
                self.is_initialized = true;
                true
            }
            #[cfg(feature = "rplidar")]
            LidarBackend::RplidarA1 | LidarBackend::RplidarA2 | LidarBackend::RplidarA3 => {
                match RplidarDevice::open_port(&self.serial_port) {
                    Ok(mut device) => {
                        // Get device info
                        match device.get_device_info() {
                            Ok(info) => {
                                eprintln!("RPLidar connected: model={}, firmware={}.{}, hardware={}",
                                    info.model, info.firmware_version.0, info.firmware_version.1,
                                    info.hardware_version);
                            }
                            Err(e) => {
                                eprintln!("Failed to get RPLidar info: {:?}", e);
                            }
                        }

                        // Check health
                        match device.check_health() {
                            Ok(health) => {
                                if !health.is_healthy() {
                                    eprintln!("RPLidar health warning: {:?}", health);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to check RPLidar health: {:?}", e);
                            }
                        }

                        // Start motor
                        if let Err(e) = device.start_motor() {
                            eprintln!("Failed to start RPLidar motor: {:?}", e);
                            return false;
                        }

                        self.rplidar = Some(device);
                        self.is_initialized = true;
                        true
                    }
                    Err(e) => {
                        eprintln!("Failed to open RPLidar on {}: {:?}", self.serial_port, e);
                        false
                    }
                }
            }
            _ => {
                eprintln!("Unsupported LiDAR backend: {:?}", self.backend);
                false
            }
        }
    }

    fn generate_scan_data(&mut self) -> Option<Vec<f32>> {
        match self.backend {
            LidarBackend::Simulation => {
                // Generate synthetic scan data for testing
                let num_points = (2.0 * std::f32::consts::PI / self.angle_increment) as usize;
                let mut ranges = Vec::with_capacity(num_points);

                for i in 0..num_points {
                    let angle = i as f32 * self.angle_increment;

                    // Create some obstacles at different distances
                    let range = if angle.cos() > 0.8 {
                        2.0 + 0.5 * angle.sin() // Wall-like obstacle
                    } else if (angle - std::f32::consts::PI / 2.0).abs() < 0.5 {
                        1.0 // Closer obstacle
                    } else {
                        self.max_range // No obstacle detected
                    };

                    ranges.push(range.min(self.max_range).max(self.min_range));
                }

                Some(ranges)
            }
            #[cfg(feature = "rplidar")]
            LidarBackend::RplidarA1 | LidarBackend::RplidarA2 | LidarBackend::RplidarA3 => {
                if let Some(ref mut device) = self.rplidar {
                    // Start scan
                    match device.start_scan() {
                        Ok(mut scan) => {
                            let mut ranges = vec![0.0; 360];

                            // Collect one full rotation of scan data
                            let mut got_full_scan = false;
                            for _ in 0..400 {
                                // Read up to 400 points (more than 360 degrees)
                                match scan.next() {
                                    Ok(Some(measurement)) => {
                                        let angle_deg = measurement.angle();
                                        let distance_m = measurement.distance() / 1000.0; // mm to meters
                                        let quality = measurement.quality();

                                        // Only use high-quality measurements
                                        if quality > 10 && distance_m >= self.min_range
                                            && distance_m <= self.max_range
                                        {
                                            let idx = angle_deg as usize % 360;
                                            ranges[idx] = distance_m;
                                        }

                                        // Check if we completed a rotation
                                        if measurement.is_sync() && got_full_scan {
                                            break;
                                        }
                                        if !got_full_scan && angle_deg > 180.0 {
                                            got_full_scan = true;
                                        }
                                    }
                                    Ok(None) => break,
                                    Err(e) => {
                                        eprintln!("RPLidar scan error: {:?}", e);
                                        break;
                                    }
                                }
                            }

                            Some(ranges)
                        }
                        Err(e) => {
                            eprintln!("Failed to start RPLidar scan: {:?}", e);
                            None
                        }
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn publish_scan(&self, ranges: Vec<f32>) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut scan = LaserScan::new();
        scan.angle_min = 0.0;
        scan.angle_max = 2.0 * std::f32::consts::PI;
        scan.angle_increment = self.angle_increment;
        scan.time_increment = 1.0 / self.scan_frequency;
        scan.scan_time = 0.1;
        scan.range_min = self.min_range;
        scan.range_max = self.max_range;

        // Copy ranges to fixed array
        for (i, &range) in ranges.iter().take(360).enumerate() {
            scan.ranges[i] = range;
        }

        scan.timestamp = current_time;

        let _ = self.publisher.send(scan, &mut None);
    }
}

impl Node for LidarNode {
    fn name(&self) -> &'static str {
        "LidarNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Initialize LiDAR on first tick
        if !self.is_initialized && !self.initialize_lidar() {
            return; // Skip if initialization failed
        }

        // Generate and publish scan data
        if let Some(ranges) = self.generate_scan_data() {
            self.scan_count += 1;
            self.last_scan_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            self.publish_scan(ranges);
        }
    }
}

impl Drop for LidarNode {
    fn drop(&mut self) {
        // Stop motor when node is dropped
        #[cfg(feature = "rplidar")]
        if let Some(ref mut device) = self.rplidar {
            let _ = device.stop_motor();
        }
    }
}

// Default impl removed - use LidarNode::new() instead which returns HorusResult
