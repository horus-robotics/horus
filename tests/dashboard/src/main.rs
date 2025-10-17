use horus::prelude::*;
use horus::core::node::TopicMetadata;
use std::time::Duration;

// ============= MESSAGE TYPES =============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    pub distance: f64,
    pub angle: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleData {
    pub detected: bool,
    pub distance: f64,
    pub urgency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCommand {
    pub target_linear: f64,
    pub target_angular: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorCommand {
    pub left_speed: f64,
    pub right_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub healthy: bool,
    pub cpu_usage: f64,
    pub message_count: u64,
}

// ============= SENSOR NODE (Priority 0) =============
// Simulates a rotating LIDAR sensor

pub struct SensorNode {
    publisher: Hub<SensorReading>,
    angle: f64,
    counter: u64,
}

impl SensorNode {
    pub fn new() -> Result<Self> {
        Ok(Self {
            publisher: Hub::new("sensors/lidar").map_err(|e| HorusError::Communication(e.to_string()))?,
            angle: 0.0,
            counter: 0,
        })
    }
}

impl Node for SensorNode {
    fn name(&self) -> &'static str {
        "SensorNode"
    }

    fn get_publishers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "sensors/lidar".to_string(),
                type_name: "SensorReading".to_string(),
            }
        ]
    }

    fn init(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("🔵 SensorNode initialized - simulating LIDAR sensor");
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Simulate rotating LIDAR with varying distances
        let distance = 5.0 + (self.angle.sin() * 2.0).abs(); // 3-7 meters

        let reading = SensorReading {
            distance,
            angle: self.angle,
            timestamp: self.counter,
        };

        let _ = self.publisher.send(reading, ctx.as_deref_mut());

        self.angle += 15.0; // Rotate 15 degrees per tick
        if self.angle >= 360.0 {
            self.angle = 0.0;
        }

        self.counter += 1;
        std::thread::sleep(Duration::from_millis(50)); // 20 Hz
    }

    fn shutdown(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("🔵 SensorNode shutdown - {} readings published", self.counter);
        Ok(())
    }
}

// ============= OBSTACLE DETECTOR (Priority 5) =============
// Processes sensor data to detect obstacles

pub struct ObstacleDetector {
    sensor_sub: Hub<SensorReading>,
    obstacle_pub: Hub<ObstacleData>,
    detection_count: u64,
}

impl ObstacleDetector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            sensor_sub: Hub::new("sensors/lidar").map_err(|e| HorusError::Communication(e.to_string()))?,
            obstacle_pub: Hub::new("perception/obstacles").map_err(|e| HorusError::Communication(e.to_string()))?,
            detection_count: 0,
        })
    }
}

impl Node for ObstacleDetector {
    fn name(&self) -> &'static str {
        "ObstacleDetector"
    }

    fn get_publishers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "perception/obstacles".to_string(),
                type_name: "ObstacleData".to_string(),
            }
        ]
    }

    fn get_subscribers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "sensors/lidar".to_string(),
                type_name: "SensorReading".to_string(),
            }
        ]
    }

    fn init(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("🟡 ObstacleDetector initialized - processing sensor data");
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        let mut processed = 0;

        // Process all available sensor readings
        while let Some(reading) = self.sensor_sub.recv(ctx.as_deref_mut()) {
            processed += 1;

            // Detect obstacles closer than 4 meters
            let detected = reading.distance < 4.0;
            let urgency = if detected {
                (4.0 - reading.distance) / 4.0 // 0.0 to 1.0
            } else {
                0.0
            };

            let obstacle = ObstacleData {
                detected,
                distance: reading.distance,
                urgency,
            };

            if detected {
                self.detection_count += 1;
            }

            let _ = self.obstacle_pub.send(obstacle, ctx.as_deref_mut());
        }

        if processed > 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn shutdown(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("🟡 ObstacleDetector shutdown - {} obstacles detected", self.detection_count);
        Ok(())
    }
}

// ============= PATH PLANNER (Priority 6) =============
// Plans navigation path based on obstacle data

pub struct PathPlanner {
    obstacle_sub: Hub<ObstacleData>,
    path_pub: Hub<PathCommand>,
    adjustments: u64,
}

impl PathPlanner {
    pub fn new() -> Result<Self> {
        Ok(Self {
            obstacle_sub: Hub::new("perception/obstacles").map_err(|e| HorusError::Communication(e.to_string()))?,
            path_pub: Hub::new("planning/path").map_err(|e| HorusError::Communication(e.to_string()))?,
            adjustments: 0,
        })
    }
}

impl Node for PathPlanner {
    fn name(&self) -> &'static str {
        "PathPlanner"
    }

    fn get_publishers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "planning/path".to_string(),
                type_name: "PathCommand".to_string(),
            }
        ]
    }

    fn get_subscribers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "perception/obstacles".to_string(),
                type_name: "ObstacleData".to_string(),
            }
        ]
    }

    fn init(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("🟢 PathPlanner initialized - computing navigation paths");
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        let mut max_urgency = 0.0;
        let mut processed = 0;

        // Find the most urgent obstacle
        while let Some(obstacle) = self.obstacle_sub.recv(ctx.as_deref_mut()) {
            processed += 1;
            if obstacle.urgency > max_urgency {
                max_urgency = obstacle.urgency;
            }
        }

        if processed > 0 {
            // Generate path command based on urgency
            let command = if max_urgency > 0.5 {
                // High urgency - slow down and turn
                self.adjustments += 1;
                PathCommand {
                    target_linear: 0.3 * (1.0 - max_urgency),
                    target_angular: 0.8 * max_urgency, // Turn away
                }
            } else {
                // Low urgency - normal speed
                PathCommand {
                    target_linear: 1.0,
                    target_angular: 0.1,
                }
            };

            let _ = self.path_pub.send(command, ctx.as_deref_mut());
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn shutdown(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("🟢 PathPlanner shutdown - {} path adjustments made", self.adjustments);
        Ok(())
    }
}

// ============= MOTOR CONTROLLER (Priority 10) =============
// Converts path commands to motor speeds

pub struct MotorController {
    path_sub: Hub<PathCommand>,
    motor_pub: Hub<MotorCommand>,
    commands_sent: u64,
}

impl MotorController {
    pub fn new() -> Result<Self> {
        Ok(Self {
            path_sub: Hub::new("planning/path").map_err(|e| HorusError::Communication(e.to_string()))?,
            motor_pub: Hub::new("actuators/motors").map_err(|e| HorusError::Communication(e.to_string()))?,
            commands_sent: 0,
        })
    }
}

impl Node for MotorController {
    fn name(&self) -> &'static str {
        "MotorController"
    }

    fn get_publishers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "actuators/motors".to_string(),
                type_name: "MotorCommand".to_string(),
            }
        ]
    }

    fn get_subscribers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "planning/path".to_string(),
                type_name: "PathCommand".to_string(),
            }
        ]
    }

    fn init(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("🟣 MotorController initialized - controlling differential drive");
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        while let Some(path_cmd) = self.path_sub.recv(ctx.as_deref_mut()) {
            // Differential drive kinematics
            let left_speed = path_cmd.target_linear - path_cmd.target_angular;
            let right_speed = path_cmd.target_linear + path_cmd.target_angular;

            let motor_cmd = MotorCommand {
                left_speed,
                right_speed,
            };

            let _ = self.motor_pub.send(motor_cmd, ctx.as_deref_mut());
            self.commands_sent += 1;
        }
    }

    fn shutdown(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("🟣 MotorController shutdown - {} motor commands sent", self.commands_sent);
        Ok(())
    }
}

// ============= STATUS MONITOR (Priority 15) =============
// Monitors system health and message flow

pub struct StatusMonitor {
    motor_sub: Hub<MotorCommand>,
    status_pub: Hub<SystemStatus>,
    message_count: u64,
}

impl StatusMonitor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            motor_sub: Hub::new("actuators/motors").map_err(|e| HorusError::Communication(e.to_string()))?,
            status_pub: Hub::new("system/status").map_err(|e| HorusError::Communication(e.to_string()))?,
            message_count: 0,
        })
    }
}

impl Node for StatusMonitor {
    fn name(&self) -> &'static str {
        "StatusMonitor"
    }

    fn get_publishers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "system/status".to_string(),
                type_name: "SystemStatus".to_string(),
            }
        ]
    }

    fn get_subscribers(&self) -> Vec<TopicMetadata> {
        vec![
            TopicMetadata {
                topic_name: "actuators/motors".to_string(),
                type_name: "MotorCommand".to_string(),
            }
        ]
    }

    fn init(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("⚪ StatusMonitor initialized - tracking system health");
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        while let Some(_motor_cmd) = self.motor_sub.recv(ctx.as_deref_mut()) {
            self.message_count += 1;
        }

        // Publish status every few ticks
        if self.message_count % 10 == 0 {
            let status = SystemStatus {
                healthy: true,
                cpu_usage: 25.0 + (self.message_count as f64 * 0.1) % 50.0,
                message_count: self.message_count,
            };

            let _ = self.status_pub.send(status, ctx.as_deref_mut());
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    fn shutdown(&mut self, _ctx: &mut NodeInfo) -> std::result::Result<(), String> {
        println!("⚪ StatusMonitor shutdown - {} total messages monitored", self.message_count);
        Ok(())
    }
}

// ============= MAIN =============

fn main() -> AnyResult<()> {
    println!("\n=== HORUS Robotics Test Application ===");
    println!("Testing Dashboard Monitoring Features\n");
    println!("🎯 System Architecture:");
    println!("  [SensorNode] → sensors/lidar → [ObstacleDetector]");
    println!("  [ObstacleDetector] → perception/obstacles → [PathPlanner]");
    println!("  [PathPlanner] → planning/path → [MotorController]");
    println!("  [MotorController] → actuators/motors → [StatusMonitor]");
    println!("  [StatusMonitor] → system/status\n");
    println!("💡 Run 'horus dashboard' in another terminal to monitor!\n");

    let mut scheduler = Scheduler::new();

    // Register nodes in priority order
    println!("📋 Registering nodes...");
    scheduler.register(
        Box::new(SensorNode::new()?),
        0,  // Highest priority - input layer
        Some(true)  // Enable logging
    );

    scheduler.register(
        Box::new(ObstacleDetector::new()?),
        5,  // Processing layer
        Some(true)
    );

    scheduler.register(
        Box::new(PathPlanner::new()?),
        6,  // Planning layer
        Some(true)
    );

    scheduler.register(
        Box::new(MotorController::new()?),
        10,  // Actuation layer
        Some(true)
    );

    scheduler.register(
        Box::new(StatusMonitor::new()?),
        15,  // Monitoring layer (lowest priority)
        Some(true)
    );

    println!("✅ All nodes registered\n");
    println!("🚀 Starting scheduler... (Press Ctrl+C to stop)\n");

    // Run the scheduler
    scheduler.tick_all();

    Ok(())
}
