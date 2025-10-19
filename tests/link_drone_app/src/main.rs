//! Drone Flight Controller Application using Link (SPSC)
//!
//! This application demonstrates a complete drone flight control system using
//! HORUS Link for ultra-low latency point-to-point communication.
//!
//! Architecture:
//!   IMU Sensor -> State Estimator -> Flight Controller -> Motor Driver
//!
//! Run with:
//!   cargo run --manifest-path tests/link_drone_app/Cargo.toml

use horus_core::{Link, Node, NodeInfo, Scheduler};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================================
// Message Types
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct ImuData {
    accel_x: f32,
    accel_y: f32,
    accel_z: f32,
    gyro_x: f32,
    gyro_y: f32,
    gyro_z: f32,
}

#[derive(Debug, Clone, Copy)]
struct StateEstimate {
    position: [f32; 3],
    velocity: [f32; 3],
    attitude: [f32; 3], // roll, pitch, yaw (rad)
}

#[derive(Debug, Clone, Copy)]
struct MotorCommands {
    motor1: f32, // 0.0 to 1.0
    motor2: f32,
    motor3: f32,
    motor4: f32,
}

impl ImuData {
    fn new(ax: f32, ay: f32, az: f32, gx: f32, gy: f32, gz: f32) -> Self {
        Self {
            accel_x: ax,
            accel_y: ay,
            accel_z: az,
            gyro_x: gx,
            gyro_y: gy,
            gyro_z: gz,
        }
    }
}

impl StateEstimate {
    fn new() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            attitude: [0.0, 0.0, 0.0],
        }
    }
}

impl MotorCommands {
    fn new(m1: f32, m2: f32, m3: f32, m4: f32) -> Self {
        Self {
            motor1: m1.clamp(0.0, 1.0),
            motor2: m2.clamp(0.0, 1.0),
            motor3: m3.clamp(0.0, 1.0),
            motor4: m4.clamp(0.0, 1.0),
        }
    }

    fn avg(&self) -> f32 {
        (self.motor1 + self.motor2 + self.motor3 + self.motor4) / 4.0
    }
}

// ============================================================================
// Node 1: IMU Sensor
// ============================================================================

struct ImuSensorNode {
    imu_output: Link<ImuData>,
    tick_count: u64,
    start_time: Instant,
}

impl ImuSensorNode {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            imu_output: Link::producer("imu_raw")?,
            tick_count: 0,
            start_time: Instant::now(),
        })
    }

    fn read_imu(&self) -> ImuData {
        let t = self.tick_count as f32 * 0.001; // 1ms per tick
        ImuData::new(
            0.0 + (t * 0.5).sin() * 0.2,  // Simulated motion
            0.0 + (t * 0.3).cos() * 0.2,
            9.81 + (t * 0.1).sin() * 0.1, // Gravity + noise
            (t * 0.2).sin() * 0.1,         // Rotation rates
            (t * 0.15).cos() * 0.1,
            (t * 0.25).sin() * 0.05,
        )
    }
}

impl Node for ImuSensorNode {
    fn name(&self) -> &'static str {
        "ImuSensor"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.tick_count += 1;

        let imu_data = self.read_imu();

        if let Err(_) = self.imu_output.send(imu_data, ctx) {
            eprintln!("[{}] Warning: Buffer full!", self.name());
        }

        // Print status every 1000ms
        if self.tick_count % 1000 == 0 {
            let metrics = self.imu_output.get_metrics();
            println!("[{:>15}] Reading IMU: accel=({:>5.2}, {:>5.2}, {:>5.2}) gyro=({:>5.2}, {:>5.2}, {:>5.2}) | Sent: {} Failed: {}",
                self.name(),
                imu_data.accel_x, imu_data.accel_y, imu_data.accel_z,
                imu_data.gyro_x, imu_data.gyro_y, imu_data.gyro_z,
                metrics.messages_sent,
                metrics.send_failures
            );
        }

        thread::sleep(Duration::from_millis(1)); // 1kHz
    }
}

// ============================================================================
// Node 2: State Estimator (Sensor Fusion)
// ============================================================================

struct StateEstimatorNode {
    imu_input: Link<ImuData>,
    state_output: Link<StateEstimate>,
    state: StateEstimate,
    tick_count: u64,
}

impl StateEstimatorNode {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            imu_input: Link::consumer("imu_raw")?,
            state_output: Link::producer("state_estimate")?,
            state: StateEstimate::new(),
            tick_count: 0,
        })
    }

    fn update_state(&mut self, imu: ImuData) {
        let dt = 0.001; // 1ms

        // Simple integration for position/velocity
        self.state.velocity[0] += imu.accel_x * dt;
        self.state.velocity[1] += imu.accel_y * dt;
        self.state.velocity[2] += (imu.accel_z - 9.81) * dt;

        self.state.position[0] += self.state.velocity[0] * dt;
        self.state.position[1] += self.state.velocity[1] * dt;
        self.state.position[2] += self.state.velocity[2] * dt;

        // Simple attitude integration
        self.state.attitude[0] += imu.gyro_x * dt; // roll
        self.state.attitude[1] += imu.gyro_y * dt; // pitch
        self.state.attitude[2] += imu.gyro_z * dt; // yaw
    }
}

impl Node for StateEstimatorNode {
    fn name(&self) -> &'static str {
        "StateEstimator"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.tick_count += 1;

        if let Some(imu_data) = self.imu_input.recv(ctx) {
            self.update_state(imu_data);
        }

        if let Err(_) = self.state_output.send(self.state, None) {
            eprintln!("[{}] Warning: Buffer full!", self.name());
        }

        // Print status every 1000ms
        if self.tick_count % 1000 == 0 {
            println!("[{:>15}] State: pos=({:>6.2}, {:>6.2}, {:>6.2}) vel=({:>5.2}, {:>5.2}, {:>5.2})",
                self.name(),
                self.state.position[0], self.state.position[1], self.state.position[2],
                self.state.velocity[0], self.state.velocity[1], self.state.velocity[2]
            );
        }

        thread::sleep(Duration::from_millis(1)); // 1kHz
    }
}

// ============================================================================
// Node 3: Flight Controller
// ============================================================================

struct FlightControllerNode {
    state_input: Link<StateEstimate>,
    motor_output: Link<MotorCommands>,
    target_altitude: f32,
    tick_count: u64,
}

impl FlightControllerNode {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            state_input: Link::consumer("state_estimate")?,
            motor_output: Link::producer("motor_commands")?,
            target_altitude: 1.5, // Target 1.5m hover
            tick_count: 0,
        })
    }

    fn compute_commands(&self, state: StateEstimate) -> MotorCommands {
        // PD altitude controller
        let altitude_error = self.target_altitude - state.position[2];
        let altitude_rate = -state.velocity[2];

        let kp = 0.4;
        let kd = 0.25;
        let base = 0.55; // Hover throttle

        let throttle = base + kp * altitude_error + kd * altitude_rate;
        let throttle = throttle.clamp(0.0, 1.0);

        // Simple attitude stabilization (all motors same for now)
        MotorCommands::new(throttle, throttle, throttle, throttle)
    }
}

impl Node for FlightControllerNode {
    fn name(&self) -> &'static str {
        "FlightController"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.tick_count += 1;

        if let Some(state) = self.state_input.recv(ctx) {
            let commands = self.compute_commands(state);

            if let Err(_) = self.motor_output.send(commands, None) {
                eprintln!("[{}] Warning: Buffer full!", self.name());
            }

            // Print status every 1000ms
            if self.tick_count % 1000 == 0 {
                println!("[{:>15}] Target: {:>5.2}m | Actual: {:>5.2}m | Error: {:>6.3}m | Throttle: {:>5.1}%",
                    self.name(),
                    self.target_altitude,
                    state.position[2],
                    self.target_altitude - state.position[2],
                    commands.avg() * 100.0
                );
            }
        }

        thread::sleep(Duration::from_millis(1)); // 1kHz
    }
}

// ============================================================================
// Node 4: Motor Driver
// ============================================================================

struct MotorDriverNode {
    motor_input: Link<MotorCommands>,
    last_commands: Option<MotorCommands>,
    tick_count: u64,
}

impl MotorDriverNode {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            motor_input: Link::consumer("motor_commands")?,
            last_commands: None,
            tick_count: 0,
        })
    }

    fn apply_motors(&self, cmd: MotorCommands) {
        // In real hardware, this would set PWM outputs
        // For demo, we just track the commands
    }
}

impl Node for MotorDriverNode {
    fn name(&self) -> &'static str {
        "MotorDriver"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.tick_count += 1;

        if let Some(commands) = self.motor_input.recv(ctx) {
            self.last_commands = Some(commands);
            self.apply_motors(commands);

            // Print status every 1000ms
            if self.tick_count % 1000 == 0 {
                println!("[{:>15}] Motors: M1={:>4.1}% M2={:>4.1}% M3={:>4.1}% M4={:>4.1}%",
                    self.name(),
                    commands.motor1 * 100.0,
                    commands.motor2 * 100.0,
                    commands.motor3 * 100.0,
                    commands.motor4 * 100.0
                );
            }
        }

        thread::sleep(Duration::from_millis(1)); // 1kHz
    }
}

// ============================================================================
// Main Application
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║     HORUS Link (SPSC) Drone Flight Controller Application    ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();
    println!("This demonstrates ultra-low latency communication using Link.");
    println!();

    // Clean up any previous shared memory
    let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_imu_raw");
    let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_state_estimate");
    let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_motor_commands");

    println!("Creating nodes...");
    let imu = ImuSensorNode::new()?;
    let estimator = StateEstimatorNode::new()?;
    let controller = FlightControllerNode::new()?;
    let motors = MotorDriverNode::new()?;

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║ Link Topology                                                ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");
    println!("║                                                               ║");
    println!("║  ┌─────────────┐  Link: imu_raw   ┌──────────────────┐       ║");
    println!("║  │ IMU Sensor  ├─────────────────>│ State Estimator  │       ║");
    println!("║  └─────────────┘                  └────────┬─────────┘       ║");
    println!("║                                            │                  ║");
    println!("║                       Link: state_estimate │                  ║");
    println!("║                                            v                  ║");
    println!("║                                  ┌──────────────────┐         ║");
    println!("║                                  │ Flight Controller│         ║");
    println!("║                                  └────────┬─────────┘         ║");
    println!("║                                           │                   ║");
    println!("║                      Link: motor_commands │                   ║");
    println!("║                                           v                   ║");
    println!("║                                  ┌──────────────┐             ║");
    println!("║                                  │ Motor Driver │             ║");
    println!("║                                  └──────────────┘             ║");
    println!("║                                                               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    println!("Performance:");
    println!("  • Control loop rate: 1kHz (1ms cycle time)");
    println!("  • Link latency: ~389ns (vs Hub: ~606ns)");
    println!("  • Total pipeline: ~1.2µs (3 Links)");
    println!("  • Advantage: 1.56x faster than Hub");
    println!();

    println!("Target:");
    println!("  • Hover altitude: 1.5 meters");
    println!("  • Controller: PD (Kp=0.4, Kd=0.25)");
    println!();

    // Create scheduler
    let mut scheduler = Scheduler::new();

    // Register nodes with logging enabled
    scheduler.register(Box::new(imu), 0, Some(true));
    scheduler.register(Box::new(estimator), 1, Some(true));
    scheduler.register(Box::new(controller), 2, Some(true));
    scheduler.register(Box::new(motors), 3, Some(true));

    println!("═══════════════════════════════════════════════════════════════");
    println!("Starting flight controller... (Ctrl+C to stop)");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Run forever
    scheduler.tick_all()?;

    Ok(())
}
