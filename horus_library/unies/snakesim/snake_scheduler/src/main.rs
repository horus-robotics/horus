use horus::prelude::*;
use horus::library::nodes::{KeyboardInputNode, JoystickInputNode};
use snake_control_node::SnakeControlNode;

fn main() {
    println!("=== Snake Game Controller ===");
    println!("Starting snake scheduler with keyboard input support...");
    println!("\nControls:");
    println!("  Arrow Keys or WASD - Control snake direction");
    println!("  ESC - Quit keyboard capture");
    println!("\nMake sure to run snakesim_gui in another terminal!");
    println!("===============================\n");

    let mut sched = Scheduler::new().name("SnakeScheduler");

    // Create keyboard input node - this will capture real keyboard input from terminal
    let keyboard_input_node = KeyboardInputNode::new_with_topic("snakeinput");

    // Create joystick input node
    let joystick_input_node = JoystickInputNode::new_with_topic("snakeinput");

    // Snake control node subscribes to snakeinput topic for both keyboard and joystick messages
    let snake_control_node = SnakeControlNode::new_with_topics("snakeinput", "snakeinput", "snakestate");

    sched.register(Box::new(keyboard_input_node), 0, Some(true));
    sched.register(Box::new(joystick_input_node), 1, None);
    sched.register(Box::new(snake_control_node), 2, Some(true));

    // Run the scheduler loop - this will continuously tick all nodes
    let _ = sched.tick_node(&["KeyboardInputNode", "JoystickInputNode", "SnakeControlNode"]);
}

