// HORUS Rust Example: Temperature Sensor (Macro Implementation)
// This is the modern way - concise and declarative

use horus::prelude::*;
use horus_macros::node;

node! {
    TempSensor {
        pub {
            temp_pub: f32 -> "temperature",
        }

        data {
            counter: f32 = 0.0,
        }

        tick {
            let temperature = 20.0 + (self.counter * 0.1).sin() * 5.0;
            self.temp_pub.send(temperature, None).ok();
            self.counter += 1.0;
        }
    }
}

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(TempSensor::new()), 2, Some(true));
    scheduler.run()?;
    Ok(())
}

// Total: 15 lines (excluding comments/blanks)
// 42% reduction from manual implementation!
