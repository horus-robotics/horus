// HORUS Rust Example: Temperature Sensor (Manual Implementation)
// This is the traditional way - more verbose but explicit

use horus::prelude::*;

struct TempSensor {
    temp_pub: Hub<f32>,
    counter: f32,
}

impl TempSensor {
    fn new() -> Result<Self> {
        Ok(Self {
            temp_pub: Hub::new("temperature")?,
            counter: 0.0,
        })
    }
}

impl Node for TempSensor {
    fn name(&self) -> &'static str {
        "TempSensor"
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        let temperature = 20.0 + (self.counter * 0.1).sin() * 5.0;
        self.temp_pub.send(temperature, ctx).ok();
        self.counter += 1.0;
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(TempSensor::new()?), 2, Some(true));
    scheduler.run()?;
    Ok(())
}

// Total: 26 lines (excluding comments/blanks)
