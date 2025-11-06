// Snake Game Controller - Backend Logic
// This handles keyboard/joystick input and publishes snake state
// Run snakesim_gui in another terminal to see the visualization

use horus::library::nodes::{JoystickInputNode, KeyboardInputNode};
use horus::prelude::*;

// Snake state message type
#[derive(Clone, Copy, Debug)]
pub struct SnakeState {
    pub direction: u32,
}

// Snake control node that converts input codes to SnakeState
struct SnakeControlNode {
    keyboard_subscriber: Hub<KeyboardInput>,
    joystick_subscriber: Hub<JoystickInput>,
    snake_publisher: Hub<SnakeState>,
}

impl SnakeControlNode {
    fn new() -> Result<Self> {
        Ok(Self {
            keyboard_subscriber: Hub::new("snakeinput")?,
            joystick_subscriber: Hub::new("snakeinput")?,
            snake_publisher: Hub::new("snakestate")?,
        })
    }
}

impl horus::core::LogSummary for SnakeState {
    fn log_summary(&self) -> String {
        format!("SnakeState(dir:{})", self.direction)
    }
}

impl Node for SnakeControlNode {
    fn name(&self) -> &'static str {
        "SnakeControlNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Handle keyboard input
        while let Some(input) = self.keyboard_subscriber.recv(ctx.as_deref_mut()) {
            if input.pressed {
                // Map keyboard codes to snake directions
                // Using standard arrow key codes (37-40) or WASD keys
                let direction = match input.code {
                    38 | 87 => 1,  // ArrowUp or W -> Up
                    40 | 83 => 2,  // ArrowDown or S -> Down
                    37 | 65 => 3,  // ArrowLeft or A -> Left
                    39 | 68 => 4,  // ArrowRight or D -> Right
                    _ => continue, // Ignore other keys
                };

                let snake_state = SnakeState { direction };
                let _ = self.snake_publisher.send(snake_state, ctx.as_deref_mut());
            }
        }

        // Handle joystick input
        while let Some(input) = self.joystick_subscriber.recv(ctx.as_deref_mut()) {
            if input.is_button() && input.pressed {
                let snake_state = SnakeState {
                    direction: input.element_id,
                };
                let _ = self.snake_publisher.send(snake_state, ctx.as_deref_mut());
            }
        }
    }
}

fn main() -> Result<()> {
    println!("=== Snake Game Controller ===");
    println!("Starting snake scheduler with keyboard input support...");
    println!("\nControls:");
    println!("  Arrow Keys or WASD - Control snake direction");
    println!("  ESC - Quit keyboard capture");
    println!("\nMake sure to run snakesim_gui in another terminal!");
    println!("===============================\n");

    let mut sched = Scheduler::new().name("SnakeScheduler");

    // Create keyboard input node - captures real keyboard input from terminal
    let keyboard_input_node = KeyboardInputNode::new_with_topic("snakeinput")?;

    // Create joystick input node
    let joystick_input_node = JoystickInputNode::new_with_topic("snakeinput")?;

    // Snake control node subscribes to snakeinput topic for both keyboard and joystick messages
    let snake_control_node = SnakeControlNode::new()?;

    sched.add(Box::new(keyboard_input_node), 0, Some(true));
    sched.add(Box::new(joystick_input_node), 1, None);
    sched.add(Box::new(snake_control_node), 2, Some(true));

    // Run the scheduler loop - continuously ticks all nodes
    sched.tick(&["KeyboardInputNode", "JoystickInputNode", "SnakeControlNode"])
}
