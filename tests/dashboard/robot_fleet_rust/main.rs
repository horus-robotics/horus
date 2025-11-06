// Complete Fleet Management System - Rust
// This demonstrates a full robotics application with:
// - Multiple sensor nodes (Camera, LIDAR, IMU, GPS)
// - Control nodes (Navigation, Obstacle Avoidance)
// - Actuator nodes (Motor Controller, Arm Controller)
// - Monitoring nodes (Battery Monitor, System Health)

use horus::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// CUSTOM MESSAGES
// ============================================================================

message!(GpsCoordinates = (f64, f64, f64)); // lat, lon, altitude
message!(BatteryStatus = (f32, bool)); // voltage, is_charging
message!(SystemHealth = (u8, u32)); // health_percent, error_count
message!(ObstacleAlert = (f32, f32)); // distance, angle

// ============================================================================
// SENSOR NODES
// ============================================================================

struct CameraNode {
    publisher: Hub<Image>,
    frame_count: u64,
}

impl CameraNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("sensors/camera")?,
            frame_count: 0,
        })
    }
}

impl Node for CameraNode {
    fn name(&self) -> &'static str { "CameraNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Simulate camera frame capture
        self.frame_count += 1;

        let img = Image {
            width: 640,
            height: 480,
            encoding: *b"rgb8\0\0\0\0\0\0\0\0\0\0\0\0",
            data_size: 640 * 480 * 3,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        };

        self.publisher.send(img, ctx).ok();
    }
}

struct LidarNode {
    publisher: Hub<LaserScan>,
    scan_count: u64,
}

impl LidarNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("sensors/lidar")?,
            scan_count: 0,
        })
    }
}

impl Node for LidarNode {
    fn name(&self) -> &'static str { "LidarNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.scan_count += 1;

        // Simulate LIDAR scan (360 degree, 1 degree resolution)
        let mut ranges = [0.0f32; 360];
        for i in 0..360 {
            // Simulate varying distances (2-10 meters)
            ranges[i] = 2.0 + (i as f32 * 0.02).sin() * 8.0;
        }

        let scan = LaserScan {
            angle_min: -std::f32::consts::PI,
            angle_max: std::f32::consts::PI,
            angle_increment: std::f32::consts::PI / 180.0,
            range_min: 0.1,
            range_max: 30.0,
            ranges,
            intensities: [0.0; 360],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        };

        self.publisher.send(scan, ctx).ok();
    }
}

struct ImuNode {
    publisher: Hub<Imu>,
    sample_count: u64,
}

impl ImuNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("sensors/imu")?,
            sample_count: 0,
        })
    }
}

impl Node for ImuNode {
    fn name(&self) -> &'static str { "ImuNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.sample_count += 1;
        let t = self.sample_count as f64 * 0.01;

        let imu = Imu {
            orientation: [
                (t * 0.5).cos() as f32,
                (t * 0.3).sin() as f32,
                (t * 0.7).cos() as f32,
                (t * 0.4).sin() as f32,
            ],
            angular_velocity: [0.01, -0.02, 0.05],
            linear_acceleration: [0.0, 0.0, 9.81],
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        };

        self.publisher.send(imu, ctx).ok();
    }
}

struct GpsNode {
    publisher: Hub<GpsCoordinates>,
    position: (f64, f64, f64),
}

impl GpsNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("sensors/gps")?,
            position: (37.7749, -122.4194, 10.0), // San Francisco
        })
    }
}

impl Node for GpsNode {
    fn name(&self) -> &'static str { "GpsNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Simulate slight GPS drift
        self.position.0 += 0.00001 * (rand::random::<f64>() - 0.5);
        self.position.1 += 0.00001 * (rand::random::<f64>() - 0.5);

        let coords = GpsCoordinates(
            self.position.0,
            self.position.1,
            self.position.2
        );

        self.publisher.send(coords, ctx).ok();
    }
}

// ============================================================================
// CONTROL NODES
// ============================================================================

struct NavigationNode {
    lidar_sub: Hub<LaserScan>,
    gps_sub: Hub<GpsCoordinates>,
    cmd_pub: Hub<CmdVel>,
    waypoint: (f64, f64),
}

impl NavigationNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            lidar_sub: Hub::new("sensors/lidar")?,
            gps_sub: Hub::new("sensors/gps")?,
            cmd_pub: Hub::new("control/cmd_vel")?,
            waypoint: (37.7750, -122.4195),
        })
    }
}

impl Node for NavigationNode {
    fn name(&self) -> &'static str { "NavigationNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Read GPS position
        if let Some(gps) = self.gps_sub.recv(ctx) {
            // Simple navigation: move toward waypoint
            let dx = self.waypoint.0 - gps.0;
            let dy = self.waypoint.1 - gps.1;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance > 0.0001 {
                let linear = (distance * 1000.0).min(1.5) as f32;
                let angular = (dy.atan2(dx) * 0.5) as f32;

                let cmd = CmdVel::new(linear, angular);
                self.cmd_pub.send(cmd, ctx).ok();
            }
        }
    }
}

struct ObstacleAvoidanceNode {
    lidar_sub: Hub<LaserScan>,
    alert_pub: Hub<ObstacleAlert>,
    override_pub: Hub<CmdVel>,
}

impl ObstacleAvoidanceNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            lidar_sub: Hub::new("sensors/lidar")?,
            alert_pub: Hub::new("safety/obstacle_alert")?,
            override_pub: Hub::new("control/cmd_vel_override")?,
        })
    }
}

impl Node for ObstacleAvoidanceNode {
    fn name(&self) -> &'static str { "ObstacleAvoidanceNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        if let Some(scan) = self.lidar_sub.recv(ctx) {
            // Check front 60 degrees for obstacles
            let start_idx = 150; // -30 degrees
            let end_idx = 210;   // +30 degrees

            let mut min_dist = f32::MAX;
            let mut min_angle = 0.0f32;

            for i in start_idx..end_idx {
                if scan.ranges[i] < min_dist {
                    min_dist = scan.ranges[i];
                    min_angle = scan.angle_min + (i as f32 * scan.angle_increment);
                }
            }

            // If obstacle within 1.5m, send alert and emergency stop
            if min_dist < 1.5 {
                let alert = ObstacleAlert(min_dist, min_angle);
                self.alert_pub.send(alert, ctx).ok();

                // Emergency stop
                let stop = CmdVel::zero();
                self.override_pub.send(stop, ctx).ok();
            }
        }
    }
}

// ============================================================================
// ACTUATOR NODES
// ============================================================================

struct MotorControllerNode {
    cmd_sub: Hub<CmdVel>,
    override_sub: Hub<CmdVel>,
    odometry_pub: Hub<Odometry>,
    current_velocity: (f32, f32),
}

impl MotorControllerNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            cmd_sub: Hub::new("control/cmd_vel")?,
            override_sub: Hub::new("control/cmd_vel_override")?,
            odometry_pub: Hub::new("sensors/odometry")?,
            current_velocity: (0.0, 0.0),
        })
    }
}

impl Node for MotorControllerNode {
    fn name(&self) -> &'static str { "MotorControllerNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Check for emergency override first
        if let Some(override_cmd) = self.override_sub.recv(ctx) {
            self.current_velocity = (override_cmd.linear, override_cmd.angular);
        } else if let Some(cmd) = self.cmd_sub.recv(ctx) {
            self.current_velocity = (cmd.linear, cmd.angular);
        }

        // Publish odometry feedback
        let mut odom = Odometry::new();
        odom.twist.linear[0] = self.current_velocity.0;
        odom.twist.angular[2] = self.current_velocity.1;
        odom.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        self.odometry_pub.send(odom, ctx).ok();
    }
}

// ============================================================================
// MONITORING NODES
// ============================================================================

struct BatteryMonitorNode {
    publisher: Hub<BatteryStatus>,
    voltage: f32,
}

impl BatteryMonitorNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("system/battery")?,
            voltage: 12.6, // Full charge
        })
    }
}

impl Node for BatteryMonitorNode {
    fn name(&self) -> &'static str { "BatteryMonitorNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Simulate battery drain
        self.voltage -= 0.0001;
        if self.voltage < 10.5 {
            self.voltage = 12.6; // Simulate recharge
        }

        let is_charging = self.voltage > 12.4;
        let status = BatteryStatus(self.voltage, is_charging);

        self.publisher.send(status, ctx).ok();
    }
}

struct SystemHealthNode {
    publisher: Hub<SystemHealth>,
    error_count: u32,
}

impl SystemHealthNode {
    fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("system/health")?,
            error_count: 0,
        })
    }
}

impl Node for SystemHealthNode {
    fn name(&self) -> &'static str { "SystemHealthNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Random error simulation
        if rand::random::<f32>() > 0.95 {
            self.error_count += 1;
        }

        let health_percent = ((1000 - self.error_count) * 100 / 1000).min(100) as u8;
        let health = SystemHealth(health_percent, self.error_count);

        self.publisher.send(health, ctx).ok();
    }
}

// ============================================================================
// MAIN
// ============================================================================

fn main() -> HorusResult<()> {
    println!("🤖 Starting Robot Fleet Management System (Rust)");
    println!("📊 Dashboard available at: http://localhost:8080");
    println!("🔧 Run 'horus dashboard' in another terminal to monitor\n");

    let mut scheduler = Scheduler::new();

    // Sensor Layer (Priority 0-9: Highest)
    scheduler.add(Box::new(CameraNode::new()?), 0, Some(true));
    scheduler.add(Box::new(LidarNode::new()?), 1, Some(true));
    scheduler.add(Box::new(ImuNode::new()?), 2, Some(true));
    scheduler.add(Box::new(GpsNode::new()?), 3, Some(true));

    // Control Layer (Priority 10-19: High)
    scheduler.add(Box::new(ObstacleAvoidanceNode::new()?), 10, Some(true));
    scheduler.add(Box::new(NavigationNode::new()?), 11, Some(true));

    // Actuation Layer (Priority 20-29: Medium)
    scheduler.add(Box::new(MotorControllerNode::new()?), 20, Some(true));

    // Monitoring Layer (Priority 30-39: Low)
    scheduler.add(Box::new(BatteryMonitorNode::new()?), 30, Some(true));
    scheduler.add(Box::new(SystemHealthNode::new()?), 31, Some(true));

    println!("✅ All nodes registered");
    println!("🚀 Starting scheduler...\n");

    scheduler.run()
}
