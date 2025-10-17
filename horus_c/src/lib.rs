// HORUS C FFI - Handle-based safe API implementation
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::sync::{Mutex, atomic::{AtomicU32, Ordering}};
use std::time::{SystemTime, UNIX_EPOCH};
use horus_core::Hub;

// Handle management
static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

use std::any::Any;

lazy_static::lazy_static! {
    static ref PUBLISHERS: Mutex<HashMap<u32, Box<dyn Any + Send>>> = Mutex::new(HashMap::new());
    static ref SUBSCRIBERS: Mutex<HashMap<u32, Box<dyn Any + Send>>> = Mutex::new(HashMap::new());
    static ref NODE_NAME: Mutex<Option<String>> = Mutex::new(None);
}

// Generate unique handle
fn next_handle() -> u32 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

// Message type enum matching C
#[repr(C)]
#[derive(Clone, Copy)]
pub enum MessageType {
    Custom = 0,
    Twist = 1,
    Pose = 2,
    LaserScan = 3,
    Image = 4,
    IMU = 5,
    JointState = 6,
    PointCloud = 7,
}

// Core API
#[no_mangle]
pub extern "C" fn init(node_name: *const c_char) -> bool {
    let name = unsafe {
        if node_name.is_null() {
            "default_node"
        } else {
            CStr::from_ptr(node_name).to_str().unwrap_or("default_node")
        }
    };

    let mut node = NODE_NAME.lock().unwrap();
    *node = Some(name.to_string());
    true
}

#[no_mangle]
pub extern "C" fn shutdown() {
    let mut pubs = PUBLISHERS.lock().unwrap();
    let mut subs = SUBSCRIBERS.lock().unwrap();
    pubs.clear();
    subs.clear();

    let mut node = NODE_NAME.lock().unwrap();
    *node = None;
}

#[no_mangle]
pub extern "C" fn ok() -> bool {
    NODE_NAME.lock().unwrap().is_some()
}

// Publisher creation
#[no_mangle]
pub extern "C" fn publisher(topic: *const c_char, msg_type: MessageType) -> u32 {
    let topic_str = unsafe {
        if topic.is_null() { return 0; }
        CStr::from_ptr(topic).to_str().unwrap_or("")
    };

    let handle = next_handle();
    let mut pubs = PUBLISHERS.lock().unwrap();

    // Create appropriate publisher based on type
    match msg_type {
        MessageType::Twist => {
            if let Ok(p) = Hub::<Twist>::new(topic_str) {
                pubs.insert(handle, Box::new(p));
            } else {
                return 0;
            }
        }
        MessageType::Pose => {
            if let Ok(p) = Hub::<Pose>::new(topic_str) {
                pubs.insert(handle, Box::new(p));
            } else {
                return 0;
            }
        }
        // Add other types as needed
        _ => return 0,
    }

    handle
}

#[no_mangle]
pub extern "C" fn publisher_custom(topic: *const c_char, msg_size: usize) -> u32 {
    let topic_str = unsafe {
        if topic.is_null() { return 0; }
        CStr::from_ptr(topic).to_str().unwrap_or("")
    };

    let handle = next_handle();
    let mut pubs = PUBLISHERS.lock().unwrap();

    // For custom messages, store topic and size for runtime handling
    pubs.insert(handle, Box::new((topic_str.to_string(), msg_size)));

    handle
}

// Subscriber creation
#[no_mangle]
pub extern "C" fn subscriber(topic: *const c_char, msg_type: MessageType) -> u32 {
    let topic_str = unsafe {
        if topic.is_null() { return 0; }
        CStr::from_ptr(topic).to_str().unwrap_or("")
    };

    let handle = next_handle();
    let mut subs = SUBSCRIBERS.lock().unwrap();

    match msg_type {
        MessageType::Twist => {
            if let Ok(s) = Hub::<Twist>::new(topic_str) {
                subs.insert(handle, Box::new(s));
            } else {
                return 0;
            }
        }
        MessageType::Pose => {
            if let Ok(s) = Hub::<Pose>::new(topic_str) {
                subs.insert(handle, Box::new(s));
            } else {
                return 0;
            }
        }
        _ => return 0,
    }

    handle
}

#[no_mangle]
pub extern "C" fn subscriber_custom(topic: *const c_char, msg_size: usize) -> u32 {
    let topic_str = unsafe {
        if topic.is_null() { return 0; }
        CStr::from_ptr(topic).to_str().unwrap_or("")
    };

    let handle = next_handle();
    let mut subs = SUBSCRIBERS.lock().unwrap();

    subs.insert(handle, Box::new((topic_str.to_string(), msg_size)));

    handle
}

// Send message
#[no_mangle]
pub extern "C" fn send(pub_handle: u32, data: *const c_void) -> bool {
    if data.is_null() { return false; }

    let pubs = PUBLISHERS.lock().unwrap();
    if let Some(publisher) = pubs.get(&pub_handle) {
        // Type-erased send - would need proper type handling in real implementation
        // For now, return true to indicate message was "sent"
        true
    } else {
        false
    }
}

// Receive message (blocking)
#[no_mangle]
pub extern "C" fn recv(sub_handle: u32, data: *mut c_void) -> bool {
    if data.is_null() { return false; }

    let subs = SUBSCRIBERS.lock().unwrap();
    if let Some(_subscriber) = subs.get(&sub_handle) {
        // Type-erased receive - would need proper type handling
        true
    } else {
        false
    }
}

// Try receive message (non-blocking)
#[no_mangle]
pub extern "C" fn try_recv(sub_handle: u32, data: *mut c_void) -> bool {
    if data.is_null() { return false; }

    let subs = SUBSCRIBERS.lock().unwrap();
    if let Some(_subscriber) = subs.get(&sub_handle) {
        // Type-erased try_recv
        false // No message available
    } else {
        false
    }
}

// Timing utilities
#[no_mangle]
pub extern "C" fn sleep_ms(ms: u32) {
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
}

#[no_mangle]
pub extern "C" fn time_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[no_mangle]
pub extern "C" fn spin_once() {
    // Process one round of callbacks
    sleep_ms(1);
}

#[no_mangle]
pub extern "C" fn spin() {
    // Process callbacks until shutdown
    while ok() {
        spin_once();
    }
}

// Logging
#[no_mangle]
pub extern "C" fn log_info(msg: *const c_char) {
    let msg_str = unsafe {
        if msg.is_null() { return; }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };
    println!("[INFO] {}", msg_str);
}

#[no_mangle]
pub extern "C" fn log_warn(msg: *const c_char) {
    let msg_str = unsafe {
        if msg.is_null() { return; }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };
    println!("[WARN] {}", msg_str);
}

#[no_mangle]
pub extern "C" fn log_error(msg: *const c_char) {
    let msg_str = unsafe {
        if msg.is_null() { return; }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };
    eprintln!("[ERROR] {}", msg_str);
}

#[no_mangle]
pub extern "C" fn log_debug(msg: *const c_char) {
    let msg_str = unsafe {
        if msg.is_null() { return; }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };
    println!("[DEBUG] {}", msg_str);
}

// Message type definitions matching C structs
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Twist {
    pub linear: Vector3,
    pub angular: Vector3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub position: Vector3,
    pub orientation: Quaternion,
}

#[repr(C)]
pub struct IMU {
    pub linear_acceleration: Vector3,
    pub angular_velocity: Vector3,
    pub orientation: Quaternion,
    pub covariance: [f32; 9],
}

#[repr(C)]
pub struct LaserScan {
    pub ranges: *mut f32,
    pub intensities: *mut f32,
    pub count: u32,
    pub angle_min: f32,
    pub angle_max: f32,
    pub angle_increment: f32,
    pub range_min: f32,
    pub range_max: f32,
    pub scan_time: f32,
}