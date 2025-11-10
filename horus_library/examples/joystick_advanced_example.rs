// Advanced Joystick Input Node Example
//
// This example demonstrates all the new features:
// - Custom button and axis mapping
// - Deadzone configuration (global and per-axis)
// - Axis inversion
// - Button mapping profiles (Xbox, PlayStation)
// - Axis calibration

use horus_core::{Hub, Node, NodeInfo, NodeInfoExt, Scheduler};
use horus_library::nodes::joystick::{ButtonMapping, JoystickInputNode};
use horus_library::JoystickInput;

struct RobotController {
    joystick_sub: Hub<JoystickInput>,
    speed: f32,
}

impl RobotController {
    fn new() -> horus_core::error::HorusResult<Self> {
        Ok(Self {
            joystick_sub: Hub::new("joystick_input")?,
            speed: 1.0,
        })
    }
}

impl Node for RobotController {
    fn name(&self) -> &'static str {
        "RobotController"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        while let Some(input) = self.joystick_sub.recv(ctx.as_deref_mut()) {
            if input.is_button() {
                let button_name = input.get_element_name();

                if input.pressed {
                    match button_name.as_str() {
                        "A" | "Cross" => {
                            ctx.log_info("Action button pressed!");
                        }
                        "Start" | "Options" => {
                            ctx.log_info("Pausing robot...");
                        }
                        "LB" | "L1" => {
                            self.speed = (self.speed - 0.1).max(0.1);
                            ctx.log_info(&format!("Speed decreased to {:.1}", self.speed));
                        }
                        "RB" | "R1" => {
                            self.speed = (self.speed + 0.1).min(2.0);
                            ctx.log_info(&format!("Speed increased to {:.1}", self.speed));
                        }
                        _ => {}
                    }
                }
            } else if input.is_axis() {
                let axis_name = input.get_element_name();
                let value = input.value;

                match axis_name.as_str() {
                    "LeftStickX" => {
                        if value.abs() > 0.1 {
                            ctx.log_debug(&format!("Turning: {:.2}", value));
                        }
                    }
                    "LeftStickY" => {
                        if value.abs() > 0.1 {
                            ctx.log_debug(&format!("Moving: {:.2}", value * self.speed));
                        }
                    }
                    _ => {}
                }
            } else if input.is_connection_event() {
                if input.is_connected() {
                    ctx.log_info("Controller connected!");
                } else {
                    ctx.log_warning("Controller disconnected!");
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Advanced Joystick Input Example ===\n");

    // Create joystick input node
    let mut joystick = JoystickInputNode::new()?;

    // Example 1: Set button mapping profile
    println!("Configuring for Xbox controller...");
    joystick.set_button_mapping(ButtonMapping::Xbox360);

    // To use PlayStation controller instead:
    // joystick.set_button_mapping(ButtonMapping::PlayStation4);

    // Example 2: Configure global deadzone
    println!("Setting global deadzone to 15%...");
    joystick.set_deadzone(0.15);

    // Example 3: Configure per-axis deadzones (for worn controllers)
    println!("Setting higher deadzone for left stick due to drift...");
    joystick.set_axis_deadzone(0, 0.20); // LeftStickX - 20% deadzone
    joystick.set_axis_deadzone(1, 0.20); // LeftStickY - 20% deadzone

    // Example 4: Invert Y-axis for aircraft-style controls
    println!("Inverting Y-axis for aircraft controls...");
    joystick.set_axis_inversion(
        false, // Don't invert X
        true,  // Invert Y (push forward = down)
        false, // Don't invert right stick X
        false, // Don't invert right stick Y
    );

    // Example 5: Custom button names
    println!("Setting custom button names...");
    joystick.set_custom_button_name(0, "Jump".to_string());
    joystick.set_custom_button_name(1, "Fire".to_string());

    // Example 6: Custom axis names
    println!("Setting custom axis names...");
    joystick.set_custom_axis_name(0, "Steering".to_string());
    joystick.set_custom_axis_name(1, "Throttle".to_string());

    // Example 7: Axis calibration for precise control
    println!("Calibrating left stick...");
    // These values would typically come from a calibration routine
    // where the user moves the stick to all extremes
    joystick.calibrate_axis(
        0,     // LeftStickX
        0.02,  // Center offset (stick naturally rests at 0.02)
        -0.98, // Minimum value when pushed all the way left
        1.0,   // Maximum value when pushed all the way right
    );

    // Example 8: Check if controller is connected
    if joystick.is_connected() {
        println!("\nController connected!");
    } else {
        println!("\nNo controller detected. Please connect a gamepad.");
        println!("This example will still run in demo mode.\n");
    }

    // Example 9: Check battery level (if supported)
    if let Some(battery) = joystick.get_battery_level() {
        println!("Battery level: {:.0}%", battery * 100.0);
    } else {
        println!("Battery level not available (wired controller or not supported)");
    }

    println!("\n=== Configuration Complete ===");
    println!("Current settings:");
    println!("  - Mapping: {:?}", joystick.get_button_mapping());
    println!("  - Global deadzone: {:.2}", joystick.get_deadzone());
    println!("  - Left stick X deadzone: {:.2}", joystick.get_axis_deadzone(0));
    println!("  - Left stick Y deadzone: {:.2}", joystick.get_axis_deadzone(1));
    println!("\nPress buttons and move sticks to see filtered input...");
    println!("Press Ctrl+C to exit\n");

    // Create robot controller to process joystick input
    let controller = RobotController::new()?;

    // Setup scheduler and run
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(joystick), 0, Some(false)); // High priority, no logging
    scheduler.add(Box::new(controller), 1, Some(false)); // Normal priority, no logging

    scheduler.run()?;

    Ok(())
}
