use crate::{DigitalIO, EmergencyStop, LaserScan, Odometry};
use horus_core::{Hub, Node, NodeInfo};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// Collision Detector Node - Safety system for obstacle avoidance and collision prevention
///
/// Monitors lidar and other sensors to detect potential collisions and trigger
/// emergency stops or evasive actions to ensure robot and operator safety.
pub struct CollisionDetectorNode {
    emergency_publisher: Hub<EmergencyStop>,
    lidar_subscriber: Hub<LaserScan>,
    odometry_subscriber: Hub<Odometry>,
    digital_io_subscriber: Hub<DigitalIO>, // For safety sensors

    // Safety zones (distances in meters)
    critical_zone: f64,   // Immediate stop zone
    warning_zone: f64,    // Slow down zone
    monitoring_zone: f64, // Track obstacles zone

    // Robot geometry
    robot_width: f64,
    robot_length: f64,
    safety_margin: f64,

    // Current state
    current_velocity: (f64, f64, f64), // (vx, vy, omega)
    current_pose: (f64, f64, f64),     // (x, y, theta)

    // Collision detection state
    collision_imminent: bool,
    warning_active: bool,
    obstacles_detected: Vec<(f64, f64)>, // Obstacle positions
    safety_sensors_active: bool,

    // Dynamic safety parameters
    velocity_dependent_zones: bool,
    min_stopping_distance: f64,
    max_deceleration: f64, // m/s²

    // Collision history for filtering
    collision_history: VecDeque<bool>,
    history_length: usize,

    // Safety sensor configuration
    safety_sensor_pins: Vec<u8>, // Digital input pins for safety sensors

    // Timing
    last_lidar_time: u64,
    emergency_cooldown: u64,
    last_emergency_time: u64,
}

impl CollisionDetectorNode {
    /// Create a new collision detector node with default topics
    pub fn new() -> Self {
        Self::new_with_topics("emergency_stop", "lidar_scan", "odom", "digital_input")
    }

    /// Create a new collision detector node with custom topics
    pub fn new_with_topics(
        emergency_topic: &str,
        lidar_topic: &str,
        odom_topic: &str,
        io_topic: &str,
    ) -> Self {
        Self {
            emergency_publisher: Hub::new(emergency_topic)
                .expect("Failed to create emergency stop publisher"),
            lidar_subscriber: Hub::new(lidar_topic).expect("Failed to subscribe to lidar"),
            odometry_subscriber: Hub::new(odom_topic).expect("Failed to subscribe to odometry"),
            digital_io_subscriber: Hub::new(io_topic).expect("Failed to subscribe to digital I/O"),

            // Default safety zones
            critical_zone: 0.3,   // 30cm immediate stop
            warning_zone: 0.8,    // 80cm slow down
            monitoring_zone: 2.0, // 2m obstacle tracking

            // Default robot geometry
            robot_width: 0.6,   // 60cm wide robot
            robot_length: 0.8,  // 80cm long robot
            safety_margin: 0.1, // 10cm additional margin

            current_velocity: (0.0, 0.0, 0.0),
            current_pose: (0.0, 0.0, 0.0),

            collision_imminent: false,
            warning_active: false,
            obstacles_detected: Vec::new(),
            safety_sensors_active: false,

            velocity_dependent_zones: true,
            min_stopping_distance: 0.2, // 20cm minimum
            max_deceleration: 2.0,      // 2 m/s² max braking

            collision_history: VecDeque::new(),
            history_length: 5, // Track last 5 readings

            safety_sensor_pins: vec![0, 1, 2, 3], // Default safety sensor pins

            last_lidar_time: 0,
            emergency_cooldown: 500, // 500ms cooldown between emergency stops
            last_emergency_time: 0,
        }
    }

    /// Configure safety zones
    pub fn set_safety_zones(&mut self, critical: f64, warning: f64, monitoring: f64) {
        self.critical_zone = critical;
        self.warning_zone = warning;
        self.monitoring_zone = monitoring;
    }

    /// Configure robot geometry for collision detection
    pub fn set_robot_geometry(&mut self, width: f64, length: f64, margin: f64) {
        self.robot_width = width;
        self.robot_length = length;
        self.safety_margin = margin;
    }

    /// Configure safety sensor input pins
    pub fn set_safety_sensor_pins(&mut self, pins: Vec<u8>) {
        self.safety_sensor_pins = pins;
    }

    /// Enable/disable velocity-dependent safety zones
    pub fn set_velocity_dependent_zones(&mut self, enabled: bool) {
        self.velocity_dependent_zones = enabled;
    }

    /// Get current collision status
    pub fn is_collision_imminent(&self) -> bool {
        self.collision_imminent
    }

    /// Get warning status
    pub fn is_warning_active(&self) -> bool {
        self.warning_active
    }

    /// Get detected obstacles
    pub fn get_obstacles(&self) -> &Vec<(f64, f64)> {
        &self.obstacles_detected
    }

    fn calculate_dynamic_zones(&self) -> (f64, f64, f64) {
        if !self.velocity_dependent_zones {
            return (self.critical_zone, self.warning_zone, self.monitoring_zone);
        }

        let speed = (self.current_velocity.0.powi(2) + self.current_velocity.1.powi(2)).sqrt();

        // Calculate stopping distance: v²/(2*a) + safety margin
        let stopping_distance =
            (speed * speed) / (2.0 * self.max_deceleration) + self.min_stopping_distance;

        let dynamic_critical = stopping_distance.max(self.critical_zone);
        let dynamic_warning = (stopping_distance * 2.0).max(self.warning_zone);
        let dynamic_monitoring = (stopping_distance * 3.0).max(self.monitoring_zone);

        (dynamic_critical, dynamic_warning, dynamic_monitoring)
    }

    fn detect_lidar_obstacles(&mut self, lidar: &LaserScan) -> (bool, bool) {
        self.obstacles_detected.clear();

        let (critical_zone, warning_zone, _monitoring_zone) = self.calculate_dynamic_zones();

        let mut critical_collision = false;
        let mut warning_collision = false;

        let robot_x = self.current_pose.0;
        let robot_y = self.current_pose.1;
        let robot_theta = self.current_pose.2;

        // Analyze lidar scan for obstacles in robot's path
        for (i, &range) in lidar.ranges.iter().enumerate() {
            if range > 0.1 && range < lidar.range_max {
                let beam_angle = lidar.angle_min as f64 + i as f64 * lidar.angle_increment as f64;
                let absolute_angle = robot_theta + beam_angle;

                // Convert to world coordinates
                let obstacle_x = robot_x + range as f64 * absolute_angle.cos();
                let obstacle_y = robot_y + range as f64 * absolute_angle.sin();

                // Check if obstacle is in robot's collision envelope
                if self.is_in_collision_path(obstacle_x, obstacle_y, range as f64, beam_angle) {
                    self.obstacles_detected.push((obstacle_x, obstacle_y));

                    // Determine collision severity
                    if (range as f64) < critical_zone {
                        critical_collision = true;
                    } else if (range as f64) < warning_zone {
                        warning_collision = true;
                    }
                }
            }
        }

        (critical_collision, warning_collision)
    }

    fn is_in_collision_path(
        &self,
        obstacle_x: f64,
        obstacle_y: f64,
        range: f64,
        beam_angle: f64,
    ) -> bool {
        // Simple rectangular collision envelope around robot
        let robot_half_width = (self.robot_width + self.safety_margin) / 2.0;
        let robot_half_length = (self.robot_length + self.safety_margin) / 2.0;

        // Transform obstacle to robot coordinate frame
        let robot_x = self.current_pose.0;
        let robot_y = self.current_pose.1;
        let robot_theta = self.current_pose.2;

        let dx = obstacle_x - robot_x;
        let dy = obstacle_y - robot_y;

        // Rotate to robot frame
        let local_x = dx * robot_theta.cos() + dy * robot_theta.sin();
        let local_y = -dx * robot_theta.sin() + dy * robot_theta.cos();

        // Check if obstacle is within robot's bounding box
        local_x.abs() <= robot_half_length && local_y.abs() <= robot_half_width &&
        local_x > -robot_half_length // Only check forward direction

        // Additional check for angular obstacles
        || (range < 1.0 && beam_angle.abs() < std::f64::consts::PI / 3.0) // 60° forward cone
    }

    fn check_safety_sensors(&mut self, digital_io: &DigitalIO) -> bool {
        if digital_io.pin_count == 0 {
            return false;
        }

        let mut safety_triggered = false;

        // Check configured safety sensor pins
        for &pin in &self.safety_sensor_pins {
            if pin < digital_io.pin_count {
                let pin_state = if (pin as usize) < digital_io.pins.len() {
                    digital_io.pins[pin as usize]
                } else {
                    false
                };

                // Assume safety sensors are normally open (true = triggered)
                if pin_state {
                    safety_triggered = true;
                    break;
                }
            }
        }

        self.safety_sensors_active = safety_triggered;
        safety_triggered
    }

    fn filter_collision_detection(&mut self, collision_detected: bool) -> bool {
        // Add current detection to history
        self.collision_history.push_back(collision_detected);

        // Maintain history length
        while self.collision_history.len() > self.history_length {
            self.collision_history.pop_front();
        }

        // Apply filtering - require majority consensus for collision
        let collision_count = self.collision_history.iter().filter(|&&x| x).count();
        collision_count > self.history_length / 2
    }

    fn trigger_emergency_stop(&mut self, reason: &str) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Check emergency cooldown to prevent spam
        if current_time - self.last_emergency_time < self.emergency_cooldown {
            return;
        }

        let emergency_msg = EmergencyStop::engage(reason);
        let _ = self.emergency_publisher.send(emergency_msg, None);

        self.last_emergency_time = current_time;
    }

    fn update_collision_state(&mut self, critical: bool, warning: bool, safety_sensors: bool) {
        let previous_collision = self.collision_imminent;
        let previous_warning = self.warning_active;

        // Apply filtering to critical collisions
        self.collision_imminent = self.filter_collision_detection(critical || safety_sensors);
        self.warning_active = warning && !self.collision_imminent;

        // Trigger emergency stop on collision state change
        if self.collision_imminent && !previous_collision {
            if safety_sensors {
                self.trigger_emergency_stop("Safety sensor triggered");
            } else {
                self.trigger_emergency_stop("Collision imminent - obstacle detected");
            }
        }

        // Log state changes for debugging
        if self.collision_imminent != previous_collision || self.warning_active != previous_warning
        {
            // In a real implementation, this would log to a proper logging system
        }
    }

    /// Manual emergency stop trigger
    pub fn trigger_manual_emergency(&mut self) {
        self.trigger_emergency_stop("Manual emergency stop");
        self.collision_imminent = true;
    }

    /// Reset collision state (after manual intervention)
    pub fn reset_collision_state(&mut self) {
        self.collision_imminent = false;
        self.warning_active = false;
        self.obstacles_detected.clear();
        self.collision_history.clear();
        self.safety_sensors_active = false;
    }

    /// Get collision detection statistics
    pub fn get_detection_stats(&self) -> (usize, f64) {
        let total_obstacles = self.obstacles_detected.len();
        let min_distance = self
            .obstacles_detected
            .iter()
            .map(|&(x, y)| {
                let dx = x - self.current_pose.0;
                let dy = y - self.current_pose.1;
                (dx * dx + dy * dy).sqrt()
            })
            .fold(f64::INFINITY, f64::min);

        (
            total_obstacles,
            if min_distance.is_finite() {
                min_distance
            } else {
                0.0
            },
        )
    }
}

impl Node for CollisionDetectorNode {
    fn name(&self) -> &'static str {
        "CollisionDetectorNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Update current pose and velocity
        if let Some(odom) = self.odometry_subscriber.recv(None) {
            self.current_pose = (odom.pose.x, odom.pose.y, odom.pose.theta);

            self.current_velocity = (
                odom.twist.linear[0],
                odom.twist.linear[1],
                odom.twist.angular[2],
            );
        }

        let mut critical_collision = false;
        let mut warning_collision = false;
        let mut safety_sensors_triggered = false;

        // Check lidar for obstacles
        if let Some(lidar) = self.lidar_subscriber.recv(None) {
            if lidar.timestamp > self.last_lidar_time {
                let (critical, warning) = self.detect_lidar_obstacles(&lidar);
                critical_collision = critical;
                warning_collision = warning;
                self.last_lidar_time = lidar.timestamp;
            }
        }

        // Check safety sensors
        if let Some(digital_io) = self.digital_io_subscriber.recv(None) {
            safety_sensors_triggered = self.check_safety_sensors(&digital_io);
        }

        // Update collision state and trigger emergency stops if necessary
        self.update_collision_state(
            critical_collision,
            warning_collision,
            safety_sensors_triggered,
        );
    }
}

impl Default for CollisionDetectorNode {
    fn default() -> Self {
        let mut node = Self::new();
        node.set_safety_zones(0.3, 0.8, 2.0); // 30cm critical, 80cm warning, 2m monitoring
        node.set_robot_geometry(0.6, 0.8, 0.1); // 60x80cm robot with 10cm margin
        node
    }
}
