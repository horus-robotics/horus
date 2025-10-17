use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cameras: Vec<CameraDevice>,
    pub usb_devices: Vec<UsbDevice>,
    pub gpio_available: bool,
    pub i2c_buses: Vec<I2cBus>,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDevice {
    pub device_path: String,
    pub name: String,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDevice {
    pub vendor_id: String,
    pub product_id: String,
    pub name: String,
    pub device_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I2cBus {
    pub bus_number: u32,
    pub device_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub arch: String,
    pub os: String,
    pub cpu_count: usize,
    pub total_memory_mb: u64,
}

/// Detect all available hardware on the system
pub fn detect_hardware() -> HardwareInfo {
    HardwareInfo {
        cameras: detect_cameras(),
        usb_devices: detect_usb_devices(),
        gpio_available: check_gpio(),
        i2c_buses: detect_i2c(),
        system_info: get_system_info(),
    }
}

fn detect_cameras() -> Vec<CameraDevice> {
    let mut cameras = Vec::new();

    // Check /dev/video* devices
    if let Ok(entries) = fs::read_dir("/dev") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("video") {
                    if let Ok(index) = name.trim_start_matches("video").parse::<u32>() {
                        cameras.push(CameraDevice {
                            device_path: format!("/dev/{}", name),
                            name: format!("Camera {}", index),
                            index,
                        });
                    }
                }
            }
        }
    }

    cameras.sort_by_key(|c| c.index);
    cameras
}

fn detect_usb_devices() -> Vec<UsbDevice> {
    let mut devices = Vec::new();

    // Try to read from /sys/bus/usb/devices
    if let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let path = entry.path();

            // Read vendor and product IDs
            let vendor_path = path.join("idVendor");
            let product_path = path.join("idProduct");
            let product_name_path = path.join("product");

            if vendor_path.exists() && product_path.exists() {
                let vendor_id = fs::read_to_string(&vendor_path)
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let product_id = fs::read_to_string(&product_path)
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let name = fs::read_to_string(&product_name_path)
                    .unwrap_or_else(|_| format!("USB Device {}:{}", vendor_id, product_id))
                    .trim()
                    .to_string();

                devices.push(UsbDevice {
                    vendor_id,
                    product_id,
                    name,
                    device_path: path.display().to_string(),
                });
            }
        }
    }

    devices
}

fn check_gpio() -> bool {
    // Check for common GPIO interfaces
    PathBuf::from("/sys/class/gpio").exists() ||
    PathBuf::from("/dev/gpiochip0").exists()
}

fn detect_i2c() -> Vec<I2cBus> {
    let mut buses = Vec::new();

    // Check /dev/i2c-* devices
    if let Ok(entries) = fs::read_dir("/dev") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("i2c-") {
                    if let Ok(bus_num) = name.trim_start_matches("i2c-").parse::<u32>() {
                        buses.push(I2cBus {
                            bus_number: bus_num,
                            device_path: format!("/dev/{}", name),
                        });
                    }
                }
            }
        }
    }

    buses.sort_by_key(|b| b.bus_number);
    buses
}

fn get_system_info() -> SystemInfo {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();

    SystemInfo {
        hostname: hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string()),
        arch: std::env::consts::ARCH.to_string(),
        os: std::env::consts::OS.to_string(),
        cpu_count: sys.cpus().len(),
        total_memory_mb: sys.total_memory() / 1024 / 1024,
    }
}
