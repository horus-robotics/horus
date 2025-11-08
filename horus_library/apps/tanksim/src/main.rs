//! TankSim - Simple tank simulation using HORUS
//!
//! A simple tank simulation demonstrating:
//! - Direct keyboard control
//! - Topic-based communication
//! - Integration with sim2d visualizer
//!
//! Usage:
//!   1. Run this scheduler: cargo run -p tanksim
//!   2. In another terminal, run sim2d:
//!      cd horus_library/tools/sim2d
//!      cargo run -- --topic /tank/tank_1/cmd_vel
//!
//! Controls:
//!   WASD/Arrows - Control tank
//!   ESC - Quit

//! Tank-specific modules (local to tanksim package)
mod tank_controller_node;

use horus::prelude::*;
use horus_core::{HorusError, Scheduler};
use horus_library::nodes::KeyboardInputNode;
use tank_controller_node::TankControllerNode;

fn main() -> AnyResult<()> {
    println!("\n");
    println!("        HORUS TankSim - Tank Demo         ");
    println!("\n");

    println!("[>] Controls:");
    println!("   WASD / Arrow Keys - Move tank");
    println!("   ESC               - Stop keyboard capture\n");

    println!(" HORUS Topics:");
    println!("   keyboard_input       - Keyboard events");
    println!("   /tank/tank_1/cmd_vel - Tank control\n");

    println!(" Starting scheduler...\n");

    // Create scheduler
    let mut scheduler = Scheduler::new().name("TankSimScheduler");

    // 1. Keyboard Input Node (Priority 0 - highest, captures input first)
    println!(" Adding KeyboardInputNode...");
    let keyboard_node = KeyboardInputNode::new_with_topic("keyboard_input")?;
    scheduler.add(Box::new(keyboard_node), 0, Some(true));

    // 2. Tank Controller Node (Priority 1 - converts keyboard to tank commands)
    println!("[>] Adding TankControllerNode...");
    let controller_node = TankControllerNode::new()
        .map_err(|e| HorusError::Config(format!("Failed to create controller node: {}", e)))?;
    scheduler.add(Box::new(controller_node), 1, Some(true));

    println!("\n All nodes added successfully!\n");
    println!("\n");
    println!(" TIP: Run sim2d in another terminal to visualize:");
    println!("   cd horus_library/tools/sim2d");
    println!("   cargo run -- --topic /tank/tank_1/cmd_vel\n");
    println!("\n");
    println!(" Starting simulation... (Press Ctrl+C to stop)\n");

    // Run the scheduler
    let _ = scheduler.run();

    println!("\n🛑 TankSim shutdown complete.\n");

    Ok(())
}
