// HORUS Rust Example: Robot Controller (Manual Implementation)
// Multi-node system with sensors, controller, and safety

use horus::prelude::*;

// ============================================================================
// IMU Driver Node
// ============================================================================

struct ImuDriver {
    imu_pub: Hub<Imu>,
}

impl ImuDriver {
    fn new() -> Result<Self> {
        Ok(Self {
            imu_pub: Hub::new("imu")?,
        })
    }
}

impl Node for ImuDriver {
    fn name(&self) -> &'static str {
        "ImuDriver"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        let imu_data = read_imu_hardware();
        self.imu_pub.send(imu_data, ctx).ok();
    }
}

// ============================================================================
// Controller Node
// ============================================================================

struct Controller {
    imu_sub: Hub<Imu>,
    scan_sub: Hub<LaserScan>,
    cmd_pub: Hub<Twist>,
    commands_sent: u32,
}

impl Controller {
    fn new() -> Result<Self> {
        Ok(Self {
            imu_sub: Hub::new("imu")?,
            scan_sub: Hub::new("scan")?,
            cmd_pub: Hub::new("cmd_vel")?,
            commands_sent: 0,
        })
    }
}

impl Node for Controller {
    fn name(&self) -> &'static str {
        "Controller"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        if let (Some(imu), Some(scan)) = (
            self.imu_sub.recv(ctx.as_deref_mut()),
            self.scan_sub.recv(ctx.as_deref_mut()),
        ) {
            let cmd = compute_control(imu, scan);
            self.cmd_pub.send(cmd, ctx.as_deref_mut()).ok();
            self.commands_sent += 1;
        }
    }

    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        ctx.log_info(&format!("Total commands sent: {}", self.commands_sent));
        self.cmd_pub.send(Twist::stop(), None)?;
        Ok(())
    }
}

// ============================================================================
// Safety Monitor Node
// ============================================================================

struct SafetyMonitor {
    cmd_sub: Hub<Twist>,
    estop_pub: Hub<EmergencyStop>,
    violations: u32,
}

impl SafetyMonitor {
    fn new() -> Result<Self> {
        Ok(Self {
            cmd_sub: Hub::new("cmd_vel")?,
            estop_pub: Hub::new("estop")?,
            violations: 0,
        })
    }
}

impl Node for SafetyMonitor {
    fn name(&self) -> &'static str {
        "SafetyMonitor"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        if let Some(cmd) = self.cmd_sub.recv(ctx.as_deref_mut()) {
            if is_unsafe(cmd) {
                if let Some(ctx) = ctx {
                    ctx.log_warn("UNSAFE COMMAND DETECTED!");
                }
                self.estop_pub.send(
                    EmergencyStop::engage("Velocity limit exceeded"),
                    ctx.as_deref_mut()
                ).ok();
                self.violations += 1;
            }
        }
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();

    // Add nodes with priorities (0=Critical, 1=High, 2=Normal)
    scheduler.add(Box::new(SafetyMonitor::new()?), 0, Some(true)); // Critical
    scheduler.add(Box::new(Controller::new()?), 1, Some(true));    // High
    scheduler.add(Box::new(ImuDriver::new()?), 2, Some(true));     // Normal

    scheduler.run()?;
    Ok(())
}

// Helper functions
fn read_imu_hardware() -> Imu {
    Imu {
        accel_x: 0.0,
        accel_y: 0.0,
        accel_z: 9.81,
        gyro_x: 0.0,
        gyro_y: 0.0,
        gyro_z: 0.0,
    }
}

fn compute_control(_imu: Imu, _scan: LaserScan) -> Twist {
    Twist::new_2d(1.0, 0.0)
}

fn is_unsafe(cmd: Twist) -> bool {
    cmd.linear[0].abs() > 2.0 || cmd.angular[2].abs() > 1.0
}

// Total: 120 lines (excluding comments/blanks)
