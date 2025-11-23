//! sim2d binary entry point

use anyhow::Result;
use clap::Parser;
use sim2d::*;
use tracing::info;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    let headless = args.headless;

    if headless {
        info!("Starting sim2d - Headless Mode");
        info!("Physics only, no visualization");
    } else {
        info!("Starting sim2d - Simple 2D Robotics Simulator");
        info!("One command, physics + visualization!");
    }

    info!("[>] Control the robot from another terminal:");
    info!("   cargo run -p simple_driver");
    info!("   (publishes to: {})", args.topic);

    // Build simulation using library API
    let sim = Sim2DBuilder::new()
        .robot_name(args.name.clone())
        .topic_prefix(args.topic.trim_end_matches("/cmd_vel").to_string())
        .headless(headless)
        .build()?;

    // Run blocking
    sim.run_blocking();

    Ok(())
}
