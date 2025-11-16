// HORUS C++ FFI - Handle-based safe API implementation
use horus_core::{HorusError, HorusResult, Hub, Node, NodeInfo, Scheduler};
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::{SystemTime, UNIX_EPOCH};

// Handle management
static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

// Trait for type-erased pub/sub operations with logging support
trait PubHandle: Send {
    fn send(&self, data: *const c_void, node_info: Option<&mut NodeInfo>) -> bool;
}

trait SubHandle: Send {
    fn recv(&self, data: *mut c_void, node_info: Option<&mut NodeInfo>) -> bool;
}

// Implement for Twist
impl PubHandle for Hub<Twist> {
    fn send(&self, data: *const c_void, mut node_info: Option<&mut NodeInfo>) -> bool {
        if data.is_null() {
            return false;
        }
        unsafe {
            let twist = *(data as *const Twist);
            Hub::send(self, twist, &mut node_info).is_ok()
        }
    }
}

impl SubHandle for Hub<Twist> {
    fn recv(&self, data: *mut c_void, mut node_info: Option<&mut NodeInfo>) -> bool {
        if data.is_null() {
            return false;
        }
        if let Some(twist) = Hub::recv(self, &mut node_info) {
            unsafe {
                *(data as *mut Twist) = twist;
            }
            true
        } else {
            false
        }
    }
}

// Implement for Pose
impl PubHandle for Hub<Pose> {
    fn send(&self, data: *const c_void, mut node_info: Option<&mut NodeInfo>) -> bool {
        if data.is_null() {
            return false;
        }
        unsafe {
            let pose = *(data as *const Pose);
            Hub::send(self, pose, &mut node_info).is_ok()
        }
    }
}

impl SubHandle for Hub<Pose> {
    fn recv(&self, data: *mut c_void, mut node_info: Option<&mut NodeInfo>) -> bool {
        if data.is_null() {
            return false;
        }
        if let Some(pose) = Hub::recv(self, &mut node_info) {
            unsafe {
                *(data as *mut Pose) = pose;
            }
            true
        } else {
            false
        }
    }
}

lazy_static::lazy_static! {
    static ref PUBLISHERS: Mutex<HashMap<u32, Box<dyn PubHandle>>> = Mutex::new(HashMap::new());
    static ref SUBSCRIBERS: Mutex<HashMap<u32, Box<dyn SubHandle>>> = Mutex::new(HashMap::new());
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
        if topic.is_null() {
            return 0;
        }
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

// Subscriber creation
#[no_mangle]
pub extern "C" fn subscriber(topic: *const c_char, msg_type: MessageType) -> u32 {
    let topic_str = unsafe {
        if topic.is_null() {
            return 0;
        }
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

// Send message (without logging - for backwards compatibility)
#[no_mangle]
pub extern "C" fn send(pub_handle: u32, data: *const c_void) -> bool {
    if data.is_null() {
        return false;
    }

    let pubs = PUBLISHERS.lock().unwrap();
    if let Some(publisher) = pubs.get(&pub_handle) {
        publisher.send(data, None) // No logging
    } else {
        false
    }
}

// Receive message (non-blocking, without logging)
#[no_mangle]
pub extern "C" fn recv(sub_handle: u32, data: *mut c_void) -> bool {
    if data.is_null() {
        return false;
    }

    let subs = SUBSCRIBERS.lock().unwrap();
    if let Some(subscriber) = subs.get(&sub_handle) {
        subscriber.recv(data, None) // No logging
    } else {
        false
    }
}

// Context-aware send (with logging when in node context)
#[no_mangle]
pub extern "C" fn node_send(ctx: *mut NodeContext, pub_handle: u32, data: *const c_void) -> bool {
    if data.is_null() || ctx.is_null() {
        return false;
    }

    let node_info_ptr = unsafe { (*ctx).node_info };
    let node_info_opt = if node_info_ptr.is_null() {
        None
    } else {
        Some(unsafe { &mut *node_info_ptr })
    };

    let pubs = PUBLISHERS.lock().unwrap();
    if let Some(publisher) = pubs.get(&pub_handle) {
        publisher.send(data, node_info_opt)
    } else {
        false
    }
}

// Context-aware recv (with logging when in node context)
#[no_mangle]
pub extern "C" fn node_recv(ctx: *mut NodeContext, sub_handle: u32, data: *mut c_void) -> bool {
    if data.is_null() || ctx.is_null() {
        return false;
    }

    let node_info_ptr = unsafe { (*ctx).node_info };
    let node_info_opt = if node_info_ptr.is_null() {
        None
    } else {
        Some(unsafe { &mut *node_info_ptr })
    };

    let subs = SUBSCRIBERS.lock().unwrap();
    if let Some(subscriber) = subs.get(&sub_handle) {
        subscriber.recv(data, node_info_opt)
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

// Logging - with dashboard integration
#[no_mangle]
pub extern "C" fn log_info(msg: *const c_char) {
    use chrono::Local;
    use horus_core::core::log_buffer::{publish_log, LogEntry, LogType};

    let msg_str = unsafe {
        if msg.is_null() {
            return;
        }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };

    println!("[INFO] {}", msg_str);

    publish_log(LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        node_name: "cpp_global".to_string(),
        log_type: LogType::Info,
        topic: None,
        message: msg_str.to_string(),
        tick_us: 0,
        ipc_ns: 0,
    });
}

#[no_mangle]
pub extern "C" fn log_warn(msg: *const c_char) {
    use chrono::Local;
    use horus_core::core::log_buffer::{publish_log, LogEntry, LogType};

    let msg_str = unsafe {
        if msg.is_null() {
            return;
        }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };

    println!("[WARN] {}", msg_str);

    publish_log(LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        node_name: "cpp_global".to_string(),
        log_type: LogType::Warning,
        topic: None,
        message: msg_str.to_string(),
        tick_us: 0,
        ipc_ns: 0,
    });
}

#[no_mangle]
pub extern "C" fn log_error(msg: *const c_char) {
    use chrono::Local;
    use horus_core::core::log_buffer::{publish_log, LogEntry, LogType};

    let msg_str = unsafe {
        if msg.is_null() {
            return;
        }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };

    eprintln!("[ERROR] {}", msg_str);

    publish_log(LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        node_name: "cpp_global".to_string(),
        log_type: LogType::Error,
        topic: None,
        message: msg_str.to_string(),
        tick_us: 0,
        ipc_ns: 0,
    });
}

#[no_mangle]
pub extern "C" fn log_debug(msg: *const c_char) {
    use chrono::Local;
    use horus_core::core::log_buffer::{publish_log, LogEntry, LogType};

    let msg_str = unsafe {
        if msg.is_null() {
            return;
        }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };

    println!("[DEBUG] {}", msg_str);

    publish_log(LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        node_name: "cpp_global".to_string(),
        log_type: LogType::Debug,
        topic: None,
        message: msg_str.to_string(),
        tick_us: 0,
        ipc_ns: 0,
    });
}

// Message type definitions matching C structs
#[repr(C)]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Twist {
    pub linear: Vector3,
    pub angular: Vector3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pose {
    pub position: Vector3,
    pub orientation: Quaternion,
}

impl horus_core::core::LogSummary for Twist {
    fn log_summary(&self) -> String {
        format!(
            "lin:({:.2},{:.2},{:.2}) ang:({:.2},{:.2},{:.2})",
            self.linear.x,
            self.linear.y,
            self.linear.z,
            self.angular.x,
            self.angular.y,
            self.angular.z
        )
    }
}

impl horus_core::core::LogSummary for Pose {
    fn log_summary(&self) -> String {
        format!(
            "pos:({:.2},{:.2},{:.2}) ori:({:.2},{:.2},{:.2},{:.2})",
            self.position.x,
            self.position.y,
            self.position.z,
            self.orientation.x,
            self.orientation.y,
            self.orientation.z,
            self.orientation.w
        )
    }
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

// ============================================================================
// Framework API - Node/Scheduler integration
// ============================================================================

// Priority is now just a plain u32 (0=Critical, 1=High, 2=Normal, 3=Low, 4=Background)
// C++ passes it directly as uint32_t, no enum needed

// C callback function types
type NodeInitCallback = extern "C" fn(*mut NodeContext, *mut c_void) -> bool;
type NodeTickCallback = extern "C" fn(*mut NodeContext, *mut c_void);
type NodeShutdownCallback = extern "C" fn(*mut NodeContext, *mut c_void);

// Node context passed to C++ callbacks
#[repr(C)]
pub struct NodeContext {
    node_name: *const c_char,
    node_info: *mut NodeInfo, // Pointer to NodeInfo for creating logged pub/sub
}

lazy_static::lazy_static! {
    static ref NODES: Mutex<HashMap<u32, Box<CNodeWrapper>>> = Mutex::new(HashMap::new());
    static ref SCHEDULERS: Mutex<HashMap<u32, Arc<Mutex<Scheduler>>>> = Mutex::new(HashMap::new());
}

// C++ Node wrapper that implements Rust Node trait
pub struct CNodeWrapper {
    name: String,
    name_cstr: CString,
    init_fn: NodeInitCallback,
    tick_fn: NodeTickCallback,
    shutdown_fn: Option<NodeShutdownCallback>,
    user_data: *mut c_void,
    context: NodeContext,
}

unsafe impl Send for CNodeWrapper {}

impl CNodeWrapper {
    fn new(
        name: String,
        init_fn: NodeInitCallback,
        tick_fn: NodeTickCallback,
        shutdown_fn: Option<NodeShutdownCallback>,
        user_data: *mut c_void,
    ) -> Self {
        let name_cstr = CString::new(name.clone()).unwrap();
        let context = NodeContext {
            node_name: name_cstr.as_ptr(),
            node_info: std::ptr::null_mut(), // Will be set during callbacks
        };

        CNodeWrapper {
            name,
            name_cstr,
            init_fn,
            tick_fn,
            shutdown_fn,
            user_data,
            context,
        }
    }
}

impl Node for CNodeWrapper {
    fn name(&self) -> &'static str {
        // SAFETY: We leak the string to ensure it lives for 'static
        // This is acceptable for node names which live for the program duration
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn init(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
        // Set NodeInfo pointer for callback duration
        self.context.node_info = ctx as *mut NodeInfo;
        let success = (self.init_fn)(&mut self.context, self.user_data);
        self.context.node_info = std::ptr::null_mut();

        if success {
            Ok(())
        } else {
            Err(HorusError::node(
                &self.name,
                "C++ node init callback returned false",
            ))
        }
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        if let Some(ctx) = ctx {
            self.context.node_info = ctx as *mut NodeInfo;
        }
        (self.tick_fn)(&mut self.context, self.user_data);
        self.context.node_info = std::ptr::null_mut();
    }

    fn shutdown(&mut self, ctx: &mut NodeInfo) -> HorusResult<()> {
        if let Some(shutdown_fn) = self.shutdown_fn {
            self.context.node_info = ctx as *mut NodeInfo;
            (shutdown_fn)(&mut self.context, self.user_data);
            self.context.node_info = std::ptr::null_mut();
        }
        Ok(())
    }
}

// FFI Functions for Framework API

#[no_mangle]
pub extern "C" fn node_create(
    name: *const c_char,
    init_fn: NodeInitCallback,
    tick_fn: NodeTickCallback,
    shutdown_fn: Option<NodeShutdownCallback>,
    user_data: *mut c_void,
) -> u32 {
    let name_str = unsafe {
        if name.is_null() {
            return 0;
        }
        CStr::from_ptr(name).to_str().unwrap_or("unnamed_node")
    };

    let node = CNodeWrapper::new(
        name_str.to_string(),
        init_fn,
        tick_fn,
        shutdown_fn,
        user_data,
    );

    let handle = next_handle();
    let mut nodes = NODES.lock().unwrap();
    nodes.insert(handle, Box::new(node));

    handle
}

#[no_mangle]
pub extern "C" fn node_destroy(node_handle: u32) {
    let mut nodes = NODES.lock().unwrap();
    nodes.remove(&node_handle);
}

#[no_mangle]
pub extern "C" fn scheduler_create(name: *const c_char) -> u32 {
    let name_str = unsafe {
        if name.is_null() {
            "scheduler"
        } else {
            CStr::from_ptr(name).to_str().unwrap_or("scheduler")
        }
    };

    let scheduler = Scheduler::new().name(name_str);
    let handle = next_handle();

    let mut schedulers = SCHEDULERS.lock().unwrap();
    schedulers.insert(handle, Arc::new(Mutex::new(scheduler)));

    handle
}

#[no_mangle]
pub extern "C" fn scheduler_add(
    sched_handle: u32,
    node_handle: u32,
    priority: u32,
    enable_logging: bool,
) -> bool {
    let mut nodes = NODES.lock().unwrap();
    let schedulers = SCHEDULERS.lock().unwrap();

    if let (Some(node), Some(sched_arc)) =
        (nodes.remove(&node_handle), schedulers.get(&sched_handle))
    {
        let mut sched = sched_arc.lock().unwrap();

        // Priority is already u32 (0 = Critical, 4 = Background)
        // CNodeWrapper implements Node trait, so we can pass it as Box<dyn Node>
        // Use the enable_logging parameter from C++
        sched.add(node as Box<dyn Node>, priority, Some(enable_logging));
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn scheduler_run(sched_handle: u32) {
    let schedulers = SCHEDULERS.lock().unwrap();

    if let Some(sched_arc) = schedulers.get(&sched_handle).cloned() {
        drop(schedulers); // Release lock before running

        let mut sched = sched_arc.lock().unwrap();
        if let Err(e) = sched.run() {
            eprintln!("[HORUS] Scheduler error: {:?}", e);
        }
    }
}

#[no_mangle]
pub extern "C" fn scheduler_tick(
    sched_handle: u32,
    node_names: *const *const c_char,
    count: usize,
) {
    let schedulers = SCHEDULERS.lock().unwrap();

    if let Some(sched_arc) = schedulers.get(&sched_handle).cloned() {
        drop(schedulers); // Release lock before running

        // Convert C string array to Rust Vec<&str>
        let mut names: Vec<&str> = Vec::new();
        if !node_names.is_null() {
            for i in 0..count {
                unsafe {
                    let name_ptr = *node_names.add(i);
                    if !name_ptr.is_null() {
                        if let Ok(name) = std::ffi::CStr::from_ptr(name_ptr).to_str() {
                            names.push(name);
                        }
                    }
                }
            }
        }

        let mut sched = sched_arc.lock().unwrap();
        if let Err(e) = sched.tick(&names) {
            eprintln!("[HORUS] Scheduler error: {:?}", e);
        }
    }
}

#[no_mangle]
pub extern "C" fn scheduler_stop(sched_handle: u32) {
    let schedulers = SCHEDULERS.lock().unwrap();

    if let Some(sched_arc) = schedulers.get(&sched_handle) {
        let sched = sched_arc.lock().unwrap();
        sched.stop();
    }
}

#[no_mangle]
pub extern "C" fn scheduler_destroy(sched_handle: u32) {
    let mut schedulers = SCHEDULERS.lock().unwrap();
    schedulers.remove(&sched_handle);
}

// Context API functions for use in C++ callbacks

#[no_mangle]
pub extern "C" fn node_create_publisher(
    _ctx: *mut NodeContext,
    topic: *const c_char,
    msg_type: MessageType,
) -> u32 {
    publisher(topic, msg_type)
}

#[no_mangle]
pub extern "C" fn node_create_subscriber(
    _ctx: *mut NodeContext,
    topic: *const c_char,
    msg_type: MessageType,
) -> u32 {
    subscriber(topic, msg_type)
}

#[no_mangle]
pub extern "C" fn node_log_info(ctx: *mut NodeContext, msg: *const c_char) {
    use chrono::Local;
    use horus_core::core::log_buffer::{publish_log, LogEntry, LogType};

    let msg_str = unsafe {
        if msg.is_null() {
            return;
        }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };

    let node_name = unsafe {
        if ctx.is_null() {
            "cpp_unknown".to_string()
        } else {
            CStr::from_ptr((*ctx).node_name)
                .to_str()
                .unwrap_or("cpp_unknown")
                .to_string()
        }
    };

    println!("[INFO] [{}] {}", node_name, msg_str);

    publish_log(LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        node_name,
        log_type: LogType::Info,
        topic: None,
        message: msg_str.to_string(),
        tick_us: 0,
        ipc_ns: 0,
    });
}

#[no_mangle]
pub extern "C" fn node_log_warn(ctx: *mut NodeContext, msg: *const c_char) {
    use chrono::Local;
    use horus_core::core::log_buffer::{publish_log, LogEntry, LogType};

    let msg_str = unsafe {
        if msg.is_null() {
            return;
        }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };

    let node_name = unsafe {
        if ctx.is_null() {
            "cpp_unknown".to_string()
        } else {
            CStr::from_ptr((*ctx).node_name)
                .to_str()
                .unwrap_or("cpp_unknown")
                .to_string()
        }
    };

    println!("[WARN] [{}] {}", node_name, msg_str);

    publish_log(LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        node_name,
        log_type: LogType::Warning,
        topic: None,
        message: msg_str.to_string(),
        tick_us: 0,
        ipc_ns: 0,
    });
}

#[no_mangle]
pub extern "C" fn node_log_error(ctx: *mut NodeContext, msg: *const c_char) {
    use chrono::Local;
    use horus_core::core::log_buffer::{publish_log, LogEntry, LogType};

    let msg_str = unsafe {
        if msg.is_null() {
            return;
        }
        CStr::from_ptr(msg).to_str().unwrap_or("")
    };

    let node_name = unsafe {
        if ctx.is_null() {
            "cpp_unknown".to_string()
        } else {
            CStr::from_ptr((*ctx).node_name)
                .to_str()
                .unwrap_or("cpp_unknown")
                .to_string()
        }
    };

    eprintln!("[ERROR] [{}] {}", node_name, msg_str);

    publish_log(LogEntry {
        timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
        node_name,
        log_type: LogType::Error,
        topic: None,
        message: msg_str.to_string(),
        tick_us: 0,
        ipc_ns: 0,
    });
}
