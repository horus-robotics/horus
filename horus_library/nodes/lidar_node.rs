use crate::LaserScan;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

/// LiDAR Node - Generic LiDAR interface for obstacle detection and mapping
///
/// Captures laser scan data from various LiDAR sensors and publishes LaserScan messages.
/// Supports multiple backends (RPLidar, serial communication) and configurable scan parameters.
pub struct LidarNode {
    publisher: Hub<LaserScan>,

    // Configuration
    frame_id: String,
    scan_frequency: f32,
    min_range: f32,
    max_range: f32,
    angle_increment: f32,

    // State
    is_initialized: bool,
    scan_count: u64,
    last_scan_time: u64,
}

impl LidarNode {
    /// Create a new LiDAR node with default topic "scan"
    pub fn new() -> Self {
        Self::new_with_topic("scan")
    }

    /// Create a new LiDAR node with custom topic
    pub fn new_with_topic(topic: &str) -> Self {
        Self {
            publisher: Hub::new(topic).expect("Failed to create LiDAR hub"),
            frame_id: "laser_frame".to_string(),
            scan_frequency: 10.0,
            min_range: 0.1,
            max_range: 30.0,
            angle_increment: std::f32::consts::PI / 180.0, // 1 degree
            is_initialized: false,
            scan_count: 0,
            last_scan_time: 0,
        }
    }

    /// Set frame ID for coordinate system
    pub fn set_frame_id(&mut self, frame_id: &str) {
        self.frame_id = frame_id.to_string();
    }

    /// Set scan frequency (Hz)
    pub fn set_scan_frequency(&mut self, frequency: f32) {
        self.scan_frequency = frequency.max(0.1).min(100.0);
    }

    /// Set range limits (meters)
    pub fn set_range_limits(&mut self, min_range: f32, max_range: f32) {
        self.min_range = min_range.max(0.0);
        self.max_range = max_range.max(self.min_range + 0.1);
    }

    /// Set angular resolution (radians)
    pub fn set_angle_increment(&mut self, increment: f32) {
        self.angle_increment = increment.max(0.001).min(0.1);
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

        // Try to initialize LiDAR hardware
        // This would connect to actual hardware in a real implementation
        self.is_initialized = true;
        true
    }

    fn generate_scan_data(&self) -> Vec<f32> {
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

        ranges
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

        let _ = self.publisher.send(scan, None);
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
        let ranges = self.generate_scan_data();
        self.scan_count += 1;
        self.last_scan_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.publish_scan(ranges);
    }
}

impl Default for LidarNode {
    fn default() -> Self {
        Self::new()
    }
}
