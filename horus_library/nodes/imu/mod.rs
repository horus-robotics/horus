use crate::Imu;
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(any(feature = "mpu6050-imu", feature = "bno055-imu"))]
use linux_embedded_hal::I2cdev;

#[cfg(feature = "mpu6050-imu")]
use mpu6050::Mpu6050;

#[cfg(feature = "bno055-imu")]
use bno055::{Bno055, BNO055OperationMode, BNO055PowerMode};

/// IMU backend type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImuBackend {
    Simulation,
    Mpu6050,
    Bno055,
    Icm20948,
}

/// IMU Node - Inertial Measurement Unit for orientation sensing
///
/// Reads accelerometer, gyroscope, and magnetometer data from IMU sensors
/// and publishes Imu messages with orientation and motion information.
///
/// Supports multiple hardware backends:
/// - MPU6050 (6-axis: accel + gyro)
/// - BNO055 (9-axis: accel + gyro + mag with sensor fusion)
/// - ICM20948 (9-axis: accel + gyro + mag)
/// - Simulation mode for testing
pub struct ImuNode {
    publisher: Hub<Imu>,

    // Configuration
    frame_id: String,
    sample_rate: f32,
    backend: ImuBackend,
    i2c_bus: String,
    i2c_address: u8,

    // State
    is_initialized: bool,
    sample_count: u64,
    last_sample_time: u64,

    // Hardware drivers
    #[cfg(feature = "mpu6050-imu")]
    mpu6050: Option<Mpu6050<I2cdev>>,

    #[cfg(feature = "bno055-imu")]
    bno055: Option<Bno055<I2cdev>>,

    // Simulation state for synthetic data
    sim_angle: f32,
}

impl ImuNode {
    /// Create a new IMU node with default topic "imu" in simulation mode
    pub fn new() -> Result<Self> {
        Self::new_with_backend("imu", ImuBackend::Simulation)
    }

    /// Create a new IMU node with custom topic
    pub fn new_with_topic(topic: &str) -> Result<Self> {
        Self::new_with_backend(topic, ImuBackend::Simulation)
    }

    /// Create a new IMU node with specific hardware backend
    pub fn new_with_backend(topic: &str, backend: ImuBackend) -> Result<Self> {
        Ok(Self {
            publisher: Hub::new(topic)?,
            frame_id: "imu_link".to_string(),
            sample_rate: 100.0, // 100 Hz default
            backend,
            i2c_bus: "/dev/i2c-1".to_string(), // Default for Raspberry Pi
            i2c_address: 0x68, // Default MPU6050 address
            is_initialized: false,
            sample_count: 0,
            last_sample_time: 0,
            #[cfg(feature = "mpu6050-imu")]
            mpu6050: None,
            #[cfg(feature = "bno055-imu")]
            bno055: None,
            sim_angle: 0.0,
        })
    }

    /// Set hardware backend
    pub fn set_backend(&mut self, backend: ImuBackend) {
        self.backend = backend;
        self.is_initialized = false; // Need to reinitialize
    }

    /// Set I2C bus and address for hardware IMU
    pub fn set_i2c_config(&mut self, bus: &str, address: u8) {
        self.i2c_bus = bus.to_string();
        self.i2c_address = address;
        self.is_initialized = false;
    }

    /// Set frame ID for coordinate system
    pub fn set_frame_id(&mut self, frame_id: &str) {
        self.frame_id = frame_id.to_string();
    }

    /// Set IMU sample rate (Hz)
    pub fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate.clamp(1.0, 1000.0);
    }

    /// Get actual sample rate (samples per second)
    pub fn get_actual_sample_rate(&self) -> f32 {
        if self.sample_count < 2 {
            return 0.0;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let time_diff = current_time - self.last_sample_time;
        if time_diff > 0 {
            1000.0 / time_diff as f32
        } else {
            0.0
        }
    }

    fn initialize_imu(&mut self) -> bool {
        if self.is_initialized {
            return true;
        }

        match self.backend {
            ImuBackend::Simulation => {
                // Simulation mode requires no hardware initialization
                self.is_initialized = true;
                true
            }
            #[cfg(feature = "mpu6050-imu")]
            ImuBackend::Mpu6050 => {
                use std::thread;
                use std::time::Duration;

                match I2cdev::new(&self.i2c_bus) {
                    Ok(i2c) => {
                        match Mpu6050::new(i2c) {
                            Ok(mut mpu) => {
                                // Initialize the MPU6050
                                if mpu.init().is_ok() {
                                    // Small delay for sensor stabilization
                                    thread::sleep(Duration::from_millis(100));
                                    self.mpu6050 = Some(mpu);
                                    self.is_initialized = true;
                                    true
                                } else {
                                    eprintln!("Failed to initialize MPU6050");
                                    false
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to create MPU6050: {:?}", e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to open I2C bus {}: {:?}", self.i2c_bus, e);
                        false
                    }
                }
            }
            #[cfg(feature = "bno055-imu")]
            ImuBackend::Bno055 => {
                use std::thread;
                use std::time::Duration;

                match I2cdev::new(&self.i2c_bus) {
                    Ok(i2c) => {
                        match Bno055::new(i2c) {
                            Ok(mut bno) => {
                                // Initialize BNO055 in NDOF mode (full sensor fusion)
                                if bno.init().is_ok()
                                    && bno.set_mode(BNO055OperationMode::NDOF).is_ok()
                                {
                                    thread::sleep(Duration::from_millis(100));
                                    self.bno055 = Some(bno);
                                    self.is_initialized = true;
                                    true
                                } else {
                                    eprintln!("Failed to initialize BNO055");
                                    false
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to create BNO055: {:?}", e);
                                false
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to open I2C bus {}: {:?}", self.i2c_bus, e);
                        false
                    }
                }
            }
            _ => {
                eprintln!("Unsupported IMU backend: {:?}", self.backend);
                false
            }
        }
    }

    fn read_imu_data(&mut self) -> Option<Imu> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        match self.backend {
            ImuBackend::Simulation => {
                // Generate synthetic IMU data for testing
                self.sim_angle += 0.01; // Slow rotation

                let mut imu = Imu::new();
                imu.orientation = [
                    0.0,
                    0.0,
                    self.sim_angle.cos() as f64,
                    self.sim_angle.sin() as f64,
                ];
                imu.angular_velocity = [0.01, 0.0, 0.0];
                imu.linear_acceleration = [0.0, 0.0, -9.81];
                imu.timestamp = current_time;
                Some(imu)
            }
            #[cfg(feature = "mpu6050-imu")]
            ImuBackend::Mpu6050 => {
                if let Some(ref mut mpu) = self.mpu6050 {
                    // Read accelerometer and gyroscope data
                    match (mpu.get_acc(), mpu.get_gyro()) {
                        (Ok(acc), Ok(gyro)) => {
                            let mut imu = Imu::new();

                            // MPU6050 provides raw accel/gyro, no orientation
                            // In m/s^2 (MPU returns g-force, convert to m/s^2)
                            imu.linear_acceleration = [
                                acc.x as f64 * 9.81,
                                acc.y as f64 * 9.81,
                                acc.z as f64 * 9.81,
                            ];

                            // In rad/s (MPU returns deg/s, convert to rad/s)
                            imu.angular_velocity = [
                                gyro.x as f64 * 0.017453292519943295,
                                gyro.y as f64 * 0.017453292519943295,
                                gyro.z as f64 * 0.017453292519943295,
                            ];

                            // MPU6050 doesn't provide orientation - would need complementary filter
                            imu.orientation = [0.0, 0.0, 0.0, 1.0]; // Identity quaternion
                            imu.timestamp = current_time;
                            Some(imu)
                        }
                        _ => {
                            eprintln!("Failed to read MPU6050 data");
                            None
                        }
                    }
                } else {
                    None
                }
            }
            #[cfg(feature = "bno055-imu")]
            ImuBackend::Bno055 => {
                if let Some(ref mut bno) = self.bno055 {
                    // BNO055 provides fused orientation as quaternion
                    let mut imu = Imu::new();

                    if let Ok(quat) = bno.quaternion() {
                        imu.orientation = [quat.x, quat.y, quat.z, quat.w];
                    }

                    if let Ok(gyro) = bno.gyro() {
                        imu.angular_velocity = [gyro.x, gyro.y, gyro.z];
                    }

                    if let Ok(accel) = bno.accel() {
                        imu.linear_acceleration = [accel.x, accel.y, accel.z];
                    }

                    imu.timestamp = current_time;
                    Some(imu)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Node for ImuNode {
    fn name(&self) -> &'static str {
        "ImuNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Initialize IMU on first tick
        if !self.is_initialized && !self.initialize_imu() {
            return;
        }

        // Read and publish IMU data
        if let Some(imu_data) = self.read_imu_data() {
            self.sample_count += 1;
            self.last_sample_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            let _ = self.publisher.send(imu_data, None);
        }
    }
}

// Default impl removed - use Node::new() instead which returns HorusResult
