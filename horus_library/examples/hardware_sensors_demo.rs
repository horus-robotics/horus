/// Hardware Sensors Demo
///
/// This example demonstrates how to use real hardware sensors with HORUS.
///
/// Sensors shown:
/// - MPU6050 IMU via I2C
/// - NMEA GPS via Serial
/// - RPLidar A2 via Serial
///
/// Build with hardware support:
/// ```bash
/// cargo run --example hardware_sensors_demo --features full-hardware
/// ```
///
/// Build for simulation (no hardware):
/// ```bash
/// cargo run --example hardware_sensors_demo
/// ```
use horus_core::{Node, Runtime};
use horus_library::nodes::gps::{GpsBackend, GpsNode};
use horus_library::nodes::imu::{ImuBackend, ImuNode};
use horus_library::nodes::lidar::{LidarBackend, LidarNode};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HORUS Hardware Sensors Demo ===\n");

    // Check command line arguments for mode
    let args: Vec<String> = env::args().collect();
    let use_hardware = args.iter().any(|arg| arg == "--hardware");

    if use_hardware {
        println!("Running with REAL HARDWARE");
        println!("Make sure sensors are connected!\n");
    } else {
        println!("Running in SIMULATION MODE");
        println!("Use --hardware flag to connect to real sensors\n");
    }

    let mut runtime = Runtime::new()?;

    // === IMU Setup ===
    println!("Setting up IMU...");
    let mut imu = if use_hardware {
        println!("  → MPU6050 on I2C bus /dev/i2c-1 @ 0x68");
        let mut node = ImuNode::new_with_backend("imu.data", ImuBackend::Mpu6050)?;
        node.set_i2c_config("/dev/i2c-1", 0x68);
        node.set_sample_rate(100.0); // 100 Hz
        node
    } else {
        println!("  → Simulation mode (synthetic data)");
        ImuNode::new_with_topic("imu.data")?
    };
    runtime.add_node(imu);

    // === GPS Setup ===
    println!("Setting up GPS...");
    let mut gps = if use_hardware {
        println!("  → NMEA GPS on /dev/ttyUSB0 @ 9600 baud");
        let mut node = GpsNode::new_with_backend("gps.fix", GpsBackend::NmeaSerial)?;
        node.set_serial_config("/dev/ttyUSB0", 9600);
        node.set_update_rate(1.0); // 1 Hz
        node.set_min_satellites(4);
        node
    } else {
        println!("  → Simulation mode (San Francisco: 37.7749°N, 122.4194°W)");
        let mut node = GpsNode::new_with_topic("gps.fix")?;
        node.set_simulation_position(37.7749, -122.4194, 10.0);
        node
    };
    runtime.add_node(gps);

    // === LiDAR Setup ===
    println!("Setting up LiDAR...");
    let mut lidar = if use_hardware {
        println!("  → RPLidar A2 on /dev/ttyUSB1");
        println!("  WARNING: Motor will start spinning!");
        let mut node = LidarNode::new_with_backend("scan", LidarBackend::RplidarA2)?;
        node.set_serial_port("/dev/ttyUSB1");
        node.set_scan_frequency(10.0); // 10 Hz
        node
    } else {
        println!("  → Simulation mode (synthetic obstacles)");
        LidarNode::new_with_topic("scan")?
    };
    runtime.add_node(lidar);

    println!("\nAll sensors configured!");
    println!("\nPublishing to topics:");
    println!("  - imu.data   : IMU orientation and motion");
    println!("  - gps.fix    : GPS position and accuracy");
    println!("  - scan       : LiDAR point cloud");
    println!("\nStarting runtime...\n");

    // Run the system
    runtime.spin()?;

    Ok(())
}
