// Simple driver to test sim2d - sends velocity commands to move the robot
//
// Usage:
//   1. Start sim2d:  horus sim2d
//   2. Run driver:   cd tests/sim2d/sim2d_driver && horus run
//
// The robot should move forward and turn alternating left/right.

use horus::prelude::*;

struct Sim2DDriver {
    cmd_vel: Hub<CmdVel>,
    tick_count: u64,
}

impl Sim2DDriver {
    fn new() -> HorusResult<Self> {
        println!("sim2d_driver: Connecting to robot.cmd_vel...");
        Ok(Self {
            cmd_vel: Hub::new("robot.cmd_vel")?,
            tick_count: 0,
        })
    }
}

impl Node for Sim2DDriver {
    fn name(&self) -> &'static str {
        "sim2d_driver"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        self.tick_count += 1;

        // Alternate between forward+left and forward+right every 50 ticks
        let linear = 0.5;  // m/s forward
        let angular = if (self.tick_count / 50) % 2 == 0 { 0.3 } else { -0.3 }; // rad/s turn

        let cmd = CmdVel::new(linear, angular);
        self.cmd_vel.send(cmd, &mut ctx).ok();

        // Print status every 50 ticks
        if self.tick_count % 50 == 0 {
            println!("[tick {}] linear={:.2}, angular={:.2}", self.tick_count, linear, angular);
        }

        // Exit after 500 ticks (about 5 seconds at 100Hz)
        if self.tick_count >= 500 {
            println!("sim2d_driver: Done! Sent {} commands", self.tick_count);
            std::process::exit(0);
        }
    }
}

fn main() -> HorusResult<()> {
    println!("sim2d_driver: Starting...");
    println!("Make sure sim2d is running: horus sim2d");
    println!();

    let mut scheduler = Scheduler::new();

    scheduler.add(
        Box::new(Sim2DDriver::new()?),
        0,           // priority
        Some(true),  // enable logging
    );

    scheduler.run()
}
