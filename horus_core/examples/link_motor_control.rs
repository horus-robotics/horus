//! Real-World Motor Control Example with Link
//!
//! This example demonstrates how Link (SPSC) is ideal for tight control loops
//! in robotics applications. It shows a motor controller receiving encoder
//! feedback and sending velocity commands with minimal latency.
//!
//! **Why Link instead of Hub?**
//! - 1.56x faster than Hub (~389ns vs ~606ns)
//! - Point-to-point communication (no broadcast overhead)
//! - Predictable latency for real-time control
//! - Perfect for high-frequency control loops (>1kHz)
//!
//! Run this example:
//! ```bash
//! cargo run --example link_motor_control
//! ```

use horus_core::{Link, Node, NodeInfo, Scheduler};
use std::thread;
use std::time::{Duration, Instant};

/// Motor encoder reading (sent from motor driver to controller)
#[derive(Debug, Clone, Copy)]
struct EncoderReading {
    timestamp_us: u64,
    position: f64, // radians
    velocity: f64, // rad/s
    current: f32,  // amps
}

/// Motor command (sent from controller to motor driver)
#[derive(Debug, Clone, Copy)]
struct MotorCommand {
    timestamp_us: u64,
    voltage: f32, // volts (-24.0 to +24.0)
    enable: bool,
}

impl EncoderReading {
    fn new(position: f64, velocity: f64, current: f32) -> Self {
        Self {
            timestamp_us: Instant::now().elapsed().as_micros() as u64,
            position,
            velocity,
            current,
        }
    }
}

impl MotorCommand {
    fn new(voltage: f32, enable: bool) -> Self {
        Self {
            timestamp_us: Instant::now().elapsed().as_micros() as u64,
            voltage: voltage.clamp(-24.0, 24.0),
            enable,
        }
    }
}

/// Motor driver node - simulates physical motor hardware
/// Receives commands and sends encoder feedback
struct MotorDriverNode {
    // Receives commands from controller
    cmd_link: Link<MotorCommand>,
    // Sends encoder readings to controller
    encoder_link: Link<EncoderReading>,

    // Simulated motor state
    position: f64,
    velocity: f64,
    voltage: f32,
    enabled: bool,
}

impl MotorDriverNode {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            cmd_link: Link::consumer("motor_cmd")?,
            encoder_link: Link::producer("encoder_feedback")?,
            position: 0.0,
            velocity: 0.0,
            voltage: 0.0,
            enabled: false,
        })
    }

    fn simulate_motor(&mut self, dt: f64) {
        if self.enabled {
            // Simple motor dynamics: voltage -> acceleration
            let torque = self.voltage as f64 * 0.1;
            let friction = -0.5 * self.velocity;
            let acceleration = torque + friction;

            self.velocity += acceleration * dt;
            self.position += self.velocity * dt;

            // Simulate some motor dynamics noise
            self.position += (self.position * 0.0001).sin() * 0.001;
        } else {
            // Apply brake
            self.velocity *= 0.9;
        }
    }
}

impl Node for MotorDriverNode {
    fn name(&self) -> &'static str {
        "MotorDriver"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Check for new commands (non-blocking)
        if let Some(cmd) = self.cmd_link.recv(None) {
            self.voltage = cmd.voltage;
            self.enabled = cmd.enable;
        }

        // Simulate motor physics (1ms timestep)
        self.simulate_motor(0.001);

        // Send encoder reading
        let reading = EncoderReading::new(
            self.position,
            self.velocity,
            (self.voltage * 0.5) as f32, // Simulated current
        );

        if let Err(_) = self.encoder_link.send(reading, None) {
            eprintln!("[{}] Warning: Encoder buffer full!", self.name());
        }

        thread::sleep(Duration::from_micros(1000)); // 1kHz loop
    }
}

/// PID Controller node - implements position control
struct MotorControllerNode {
    // Receives encoder readings
    encoder_link: Link<EncoderReading>,
    // Sends motor commands
    cmd_link: Link<MotorCommand>,

    // Control parameters
    target_position: f64,
    kp: f64,
    kd: f64,

    // State
    last_error: f64,
    iteration: u64,
}

impl MotorControllerNode {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            encoder_link: Link::consumer("encoder_feedback")?,
            cmd_link: Link::producer("motor_cmd")?,
            target_position: 0.0,
            kp: 5.0,
            kd: 0.5,
            last_error: 0.0,
            iteration: 0,
        })
    }

    fn update_target(&mut self) {
        // Sinusoidal target trajectory
        let t = self.iteration as f64 * 0.001;
        self.target_position = 2.0 * (t * 0.5).sin();
    }
}

impl Node for MotorControllerNode {
    fn name(&self) -> &'static str {
        "MotorController"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.iteration += 1;
        self.update_target();

        // Wait for encoder reading (blocking in real system, non-blocking here)
        if let Some(reading) = self.encoder_link.recv(None) {
            // PD control law
            let error = self.target_position - reading.position;
            let derivative = error - self.last_error;
            let voltage = (self.kp * error + self.kd * derivative) as f32;

            self.last_error = error;

            // Send motor command
            let cmd = MotorCommand::new(voltage, true);
            if let Err(_) = self.cmd_link.send(cmd, None) {
                eprintln!("[{}] Warning: Command buffer full!", self.name());
            }

            // Print status every 100ms
            if self.iteration % 100 == 0 {
                println!(
                    "[{:>15}] Target: {:>6.3} | Position: {:>6.3} | Error: {:>6.3} | Voltage: {:>5.2}V",
                    self.name(),
                    self.target_position,
                    reading.position,
                    error,
                    voltage
                );
            }
        }

        thread::sleep(Duration::from_micros(1000)); // 1kHz loop
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HORUS Link Motor Control Example ===\n");
    println!("This demonstrates Link (SPSC) for ultra-low latency control loops.\n");

    // Clean up any previous shared memory
    let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_motor_cmd");
    let _ = std::fs::remove_file("/dev/shm/horus/topics/horus_links_encoder_feedback");

    println!("Creating nodes...");
    let motor_driver = MotorDriverNode::new()?;
    let controller = MotorControllerNode::new()?;

    println!("Link topology:");
    println!("  MotorController --[Link: motor_cmd]--> MotorDriver");
    println!("  MotorDriver --[Link: encoder_feedback]--> MotorController");
    println!("\nControl parameters:");
    println!("  - Loop rate: 1kHz (1ms cycle time)");
    println!("  - Controller: PD with Kp=5.0, Kd=0.5");
    println!("  - Target: Sinusoidal trajectory (2.0 * sin(0.5t))");
    println!("\nPerformance:");
    println!("  - Link latency: ~389ns (vs Hub: ~606ns)");
    println!("  - 1.56x faster than Hub for this control loop\n");

    // Create scheduler
    let mut scheduler = Scheduler::new();

    // Register nodes with priorities (higher priority = runs first)
    scheduler.register(Box::new(motor_driver), 0, Some(false)); // High priority
    scheduler.register(Box::new(controller), 1, Some(false)); // Normal priority

    println!("Starting control loop... (Ctrl+C to stop)\n");

    // Run forever (Ctrl+C to stop)
    scheduler.tick_all()?;

    Ok(())
}
