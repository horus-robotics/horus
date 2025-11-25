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

    // Show articulated robot info if using presets
    if let Some(preset) = &args.preset {
        info!("[>] Using articulated robot preset: {}", preset);
        info!("   Available presets: arm_2dof, arm_6dof, humanoid");
    }

    if let Some(articulated_file) = &args.articulated {
        info!("[>] Loading articulated robot from: {}", articulated_file);
    }

    if args.gravity {
        info!("[>] Gravity enabled (side-view simulation)");
    }

    info!("[>] Control the robot from another terminal:");
    info!("   cargo run -p simple_driver");
    info!("   (publishes to: {})", args.topic);

    // Run the simulation using main_impl directly for full CLI support
    run_simulation(args)
}
