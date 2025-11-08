// Snake Game Controller - Backend Logic
// This handles keyboard/joystick input and publishes snake state
// Run snakesim_gui in another terminal to see the visualization

use horus::library::nodes::KeyboardInputNode;
use horus::prelude::*;


// Snake control node that converts input codes to SnakeState
struct SnakeControlNode {
    keyboard_subscriber: Hub<KeyboardInput>,
    snake_publisher: Hub<u32>,
}

impl SnakeControlNode {
    fn new() -> Result<Self> {
        Ok(Self {
            keyboard_subscriber: Hub::new("keyboard_input")?,
            snake_publisher: Hub::new("snakestate")?,
        })
    }
}

impl Node for SnakeControlNode {
    fn name(&self) -> &'static str {
        "SnakeControlNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Process latest keyboard input (one per tick for bounded execution)
        if let Some(input) = self.keyboard_subscriber.recv(None) {
            ctx.log_debug(&format!(
                "Received key code: {}, pressed: {}",
                input.code, input.pressed
            ));

            if input.pressed {
                // Map keyboard codes to snake directions
                // Using standard arrow key codes (37-40) or WASD keys
                let direction = match input.code {
                    38 | 87 => 1, // ArrowUp or W -> Up
                    40 | 83 => 2, // ArrowDown or S -> Down
                    37 | 65 => 3, // ArrowLeft or A -> Left
                    39 | 68 => 4, // ArrowRight or D -> Right
                    _ => return,  // Ignore other keys
                };

                ctx.log_debug(&format!("Publishing direction: {}", direction));
                let _ = self.snake_publisher.send(direction, ctx);
            }
        }
    }
}

fn main() -> Result<()> {
    eprintln!("=== Snake Game Controller ===");
    eprintln!("Starting snake scheduler with keyboard input support...");
    eprintln!("\nControls:");
    eprintln!("  Arrow Keys or WASD - Control snake direction");
    eprintln!("  ESC - Quit keyboard capture");
    eprintln!("\nMake sure to run snakesim_gui in another terminal!");
    eprintln!("===============================\n");

    let mut sched = Scheduler::new().name("SnakeScheduler");

    // Create keyboard input node - captures real keyboard input from terminal
    let keyboard_input_node = KeyboardInputNode::new_with_topic("keyboard_input")?;

    // Snake control node subscribes to keyboard_input topic
    let snake_control_node = SnakeControlNode::new()?;

    sched.add(Box::new(keyboard_input_node), 0, Some(true));
    sched.add(Box::new(snake_control_node), 1, Some(true));

    // Run the scheduler loop - continuously ticks all nodes
    sched.tick(&["KeyboardInputNode", "SnakeControlNode"])
}
