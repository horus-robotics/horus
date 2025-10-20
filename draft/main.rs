// Mobile robot controller

use horus::prelude::*;

struct Controller {
    cmd_vel: Hub<CmdVel>,
}

impl Controller {
    fn new() -> Result<Self> {
        Ok(Self {
            cmd_vel: Hub::new("motors/cmd_vel")?,
        })
    }
}

impl Node for Controller {
    fn name(&self) -> &'static str {
        "controller"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Your control logic here
        // ctx provides node state, timing info, and monitoring data
        let msg = CmdVel::new(1.0, 0.0);
        self.cmd_vel.send(msg, ctx).ok();
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();

    // Register the controller node with priority 10
    scheduler.register(
        Box::new(Controller::new()?),
        0,     // priority (0 = highest)
        Some(true)    // logging config
    );

    // Run the scheduler
    scheduler.tick_all()
}
