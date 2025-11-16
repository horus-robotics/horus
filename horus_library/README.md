# HORUS Library

**Reusable components, messages, and complete applications for the HORUS robotics framework**

The HORUS Library provides a comprehensive collection of tested nodes, algorithms, message types, and complete applications that demonstrate HORUS framework capabilities and serve as building blocks for robotics applications.

## Table of Contents

- [Overview](#overview)
- [Components](#components)
  - [Messages](#messages)
  - [Nodes](#nodes)
  - [Algorithms](#algorithms)
  - [Example Apps (Applications)](#example-apps-applications)
- [Message Safety](#message-safety)
- [Usage Examples](#usage-examples)
- [Building](#building)

## Overview

The HORUS Standard Library is integrated into `horus_core` and provides essential components for robotics applications. All message types and components are available through the prelude:

```rust
use horus_core::prelude::*;

// All standard library types are available:
// - KeyboardInput, JoystickInput (input messages)
// - CmdVel, Direction (control messages)
// - Node, Hub, Scheduler (core types)
```

The library is organized into several key categories:

```
horus_library/
├── messages/         # Shared memory-safe message types
├── nodes/           # Reusable node implementations
├── algorithms/      # Common robotics algorithms (planned)
├── apps/            # Complete example applications
│   ├── snakesim/   # Multi-node snake game demo
│   └── tanksim/    # Tank simulation (in development)
├── tools/           # Development and debugging tools
│   └── sim2d/      # 2D physics simulator with visualization
└── traits/          # Common trait definitions
```

## Production-Ready Built-in Nodes

HORUS Library includes **32 production-ready nodes** with real hardware integration. These nodes are **NOT prototypes** - they're production-grade drivers ready for deployment in real robots.

### Complete Node Catalog

#### Safety & Monitoring (2 nodes)
- **EmergencyStopNode** - Hardware e-stop with watchdog timeout
- **SafetyMonitorNode** - Multi-layered safety (battery, CPU, memory, temperature)

#### Sensor Interface (9 nodes)
- **CameraNode** - OpenCV, V4L2 backends
- **DepthCameraNode** - Intel RealSense (D415, D435, D455, L515), ZED, Kinect
- **LidarNode** - Laser range finder (2D/3D)
- **IMUNode** - Accelerometer, gyroscope, magnetometer with calibration
- **EncoderNode** - Quadrature decoder for odometry
- **GPSNode** - NMEA serial (u-blox, MTK, etc.)
- **UltrasonicNode** - HC-SR04, US-100, Maxbotix (up to 16 sensors)
- **BatteryMonitorNode** - I2C fuel gauges (INA219, INA226, BQ27441)
- **ForceTorqueSensorNode** - 6-axis F/T sensors (ATI, Robotiq)

#### Control & Actuation (8 nodes)
- **DcMotorNode** - L298N, TB6612 motor controllers
- **BldcMotorNode** - BLDC ESCs (PWM, DShot, VESC, CAN)
- **StepperMotorNode** - A4988, DRV8825, TMC2208 drivers
- **ServoControllerNode** - Multi-servo control with limits
- **DynamixelNode** - Dynamixel smart servos (Protocol 1.0/2.0)
- **RoboclawNode** - BasicMicro Roboclaw dual-channel controllers
- **PidControllerNode** - Generic PID with anti-windup
- **DifferentialDriveNode** - Mobile robot base control

#### Navigation (4 nodes)
- **PathPlannerNode** - A*, RRT, Dijkstra algorithms
- **LocalizationNode** - Robot position estimation
- **OdometryNode** - Dead reckoning (differential, mecanum, ackermann)
- **CollisionDetectorNode** - Real-time collision detection

#### Industrial Integration (6 nodes)
- **CANBusNode** - Linux SocketCAN (CAN 2.0A/B, CAN-FD)
- **ModbusNode** - Modbus TCP/RTU for industrial PLCs
- **SerialNode** - UART/Serial communication
- **I2CBusNode** - I2C bus communication
- **SPIBusNode** - SPI communication
- **DigitalIONode** - GPIO control with debounce

#### Vision & Image Processing (1 node)
- **ImageProcessorNode** - Filtering, color conversion

#### Input Devices (2 nodes)
- **KeyboardInputNode** - Keyboard capture for teleoperation
- **JoystickInputNode** - Gamepad/joystick input

### Hardware Integration Status

| Node | Hardware Status | Tested On |
|------|----------------|-----------|
| DepthCameraNode | Full | Intel RealSense D435, D455, L515 |
| BatteryMonitorNode | Full | I2C INA219, INA226 on Raspberry Pi |
| BldcMotorNode | Full | Raspberry Pi GPIO PWM, ESC protocols |
| CANBusNode | Full | Linux SocketCAN (virtual and real) |
| RoboclawNode | Full | Roboclaw 2x7A through 2x160A models |
| GPSNode | Full | u-blox NEO/ZED, MTK3339 via NMEA |
| SafetyMonitorNode | Full | System monitoring on Linux/Pi |

All other nodes: Complete implementation with simulation fallback

### Quick Start: Real Robot

```rust
use horus::prelude::*;
use horus_library::nodes::*;

fn main() -> Result<()> {
    let mut scheduler = Scheduler::new();

    // PRODUCTION mobile robot

    // Navigation sensors
    let gps = GPSNode::new()?;  // Real GPS hardware
    let imu = IMUNode::new()?;  // Real IMU hardware
    let encoders = EncoderNode::new()?;  // Real encoders

    // Motor control
    let mut drive = DifferentialDriveNode::new()?;
    drive.set_wheelbase(0.5);  // Real robot dimensions

    // Power & safety
    let mut battery = BatteryMonitorNode::new()?;
    battery.configure_4s_lipo(5000.0);  // Real battery
    let safety = SafetyMonitorNode::new()?;

    // Add all to scheduler
    scheduler.add(Box::new(gps), 20, Some(true));
    scheduler.add(Box::new(imu), 10, Some(true));
    scheduler.add(Box::new(encoders), 5, Some(true));
    scheduler.add(Box::new(drive), 5, Some(true));
    scheduler.add(Box::new(battery), 50, Some(true));
    scheduler.add(Box::new(safety), 1, Some(true));

    scheduler.run()  // Production-ready!
}
```

### Extensibility: Wrapping Nodes

Built-in nodes are production-ready AS-IS. Wrap them only to add custom logic:

```rust
// Built-in node handles hardware perfectly
// Wrapper adds your custom algorithm

struct AIEnhancedCamera {
    camera: DepthCameraNode,  // Production-ready hardware driver
    detector: YOLOv8,         // Your custom AI model
}

impl Node for AIEnhancedCamera {
    fn name(&self) -> &'static str { "AIEnhancedCamera" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Built-in node captures from real RealSense
        self.camera.tick(ctx);

        // Your custom AI processing
        let image = get_latest_image();
        let objects = self.detector.detect(image);
        publish_detections(objects);
    }
}
```

**The camera node IS production-ready. You're just adding custom AI on top.**

### Documentation

Each node includes:
- Comprehensive README with hardware setup
- Configuration examples and presets
- Supported hardware list
- Error handling and troubleshooting
- API documentation

See individual node directories in `nodes/*/README.md` for details.

## Components

### Messages

Shared memory-safe message types for inter-node communication.

#### KeyboardInput
**Location**: `messages/keyboard_input_msg.rs`

Thread-safe keyboard input message with fixed-size arrays:

```rust
use horus_library::KeyboardInput;

// Message structure (shared memory safe)
pub struct KeyboardInput {
    pub key_name: [u8; 32],        // Fixed-size key name buffer
    pub code: u32,                 // Raw key code
    pub modifier_flags: u32,       // Bit flags for modifiers
    pub pressed: bool,             // Press/release state
    pub timestamp: u64,            // Unix timestamp in milliseconds
}

// Usage
let key_event = KeyboardInput::new(
    "a".to_string(),
    97,
    vec!["Ctrl".to_string()], // Converted to bit flags internally
    true
);

// Check modifiers
if key_event.is_ctrl() {
    eprintln!("Ctrl+{} pressed", key_event.get_key_name());
}

// Get all modifiers
let modifiers = key_event.get_modifiers(); // Vec<String>
```

#### JoystickInput
**Location**: `messages/joystick_msg.rs`

Gamepad and joystick input events:

```rust
use horus_library::JoystickInput;

// Fixed-size arrays for shared memory safety
pub struct JoystickInput {
    pub event_type: [u8; 32],      // "button", "axis", "hat"
    pub element_name: [u8; 32],    // Button/axis identifier
    pub value: f32,                // Event value
    pub timestamp: u64,            // Unix timestamp
}
```

#### SnakeGameState
**Location**: `messages/snake_state.rs`

Game state for the Snake simulation:

```rust
use horus_library::SnakeGameState;

// Demonstrates complex shared memory-safe structures
pub struct SnakeGameState {
    pub segments: [[i32; 2]; 100], // Fixed-size segment array
    pub segment_count: usize,      // Active segments
    pub direction: u32,            // Direction code (1=up, 2=down, 3=left, 4=right)
    pub food_position: [i32; 2],   // Food coordinates
    pub score: u32,                // Current score
    pub game_over: bool,           // Game state
}
```

### Nodes

Production-ready nodes for common robotics tasks.

#### KeyboardInputNode
**Location**: `nodes/keyboard_input/`

Real-time keyboard input capture with cross-platform support:

```rust
use horus_core::{Node, NodeInfo, Hub, Scheduler};
use horus_library::{KeyboardInput, nodes::KeyboardInputNode};

struct KeyHandler {
    subscriber: Hub<KeyboardInput>,
}

impl KeyHandler {
    pub fn new() -> Self {
        Self {
            subscriber: Hub::new("keyboard_input").expect("Failed to create subscriber"),
        }
    }
}

impl Node for KeyHandler {
    fn name(&self) -> &'static str { "KeyHandler" }
    
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Process one key event per tick (bounded execution)
        if let Some(key_event) = self.subscriber.recv(ctx) {
            if key_event.pressed { // Only handle key presses
                match key_event.get_key_name().as_str() {
                    "ArrowUp" => eprintln!("Move up"),
                    "ArrowDown" => eprintln!("Move down"),
                    "ArrowLeft" => eprintln!("Move left"),
                    "ArrowRight" => eprintln!("Move right"),
                    "q" if key_event.is_ctrl() => std::process::exit(0),
                    _ => {}
                }
            }
        }
    }
}

fn main() {
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(KeyboardInputNode::new()), 0, Some(true));
    scheduler.add(Box::new(KeyHandler::new()), 1, Some(true));
    scheduler.run().expect("Scheduler failed");
}
```

**Features:**
- Cross-platform keyboard capture (Linux, macOS, Windows)
- Raw terminal mode for full key event capture
- Built-in Ctrl+C handling for proper termination
- Customizable key mappings
- Both press and release event capture

#### JoystickInputNode
**Location**: `nodes/joystick/`

Gamepad and joystick input capture:

```rust
use horus_library::nodes::JoystickInputNode;

// Default gamepad mappings for common controllers
// D-pad: Up/Down/Left/Right  direction codes 1/2/3/4
// Face buttons: A/B/X/Y  direction codes 1/2/3/4

let joystick_node = JoystickInputNode::new();
```

### Algorithms

**Note**: The algorithms directory is currently empty. Common robotics algorithms (pathfinding, SLAM, etc.) are planned for future releases.

For now, implement custom algorithms as nodes using the Node trait.

### Example Apps (Applications)

Complete distributed applications demonstrating HORUS concepts.

#### SnakeSim
**Location**: `apps/snakesim/`

Multi-node snake game with dual input support:

```bash
# Run the complete snake game
cd horus_library/apps/snakesim
cargo run
```

**Architecture:**
- **KeyboardInputNode** (priority 0): Captures arrow keys  direction codes
- **JoystickInputNode** (priority 1): Captures gamepad input  direction codes  
- **SnakeControlNode** (priority 2): Processes direction codes  game logic

**Features:**
- Dual input support (keyboard + gamepad simultaneously)
- Built-in logging shows real-time message flow
- Priority-based execution ensures input responsiveness
- Demonstrates proper multi-node communication

**Expected Output:**
```
Registered node 'KeyboardInputNode' with priority 0 (logging: true)
Registered node 'JoystickInputNode' with priority 1 (logging: true)
Registered node 'SnakeControlNode' with priority 2 (logging: true)

[2025-08-10 11:30:00.123] [0ms] [TX] KeyboardInputNode  'keyboard_input' = ArrowUp
[2025-08-10 11:30:00.124] [1ms] [RX] SnakeControlNode  'keyboard_input' = ArrowUp
[2025-08-10 11:30:00.125] [2ms] [TX] SnakeControlNode  'direction_command' = 1
```

## Message Safety

All HORUS Library messages use fixed-size structures for shared memory safety:

### Safe Message Design

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SafeMessage {
    pub text: [u8; 32],        // Fixed-size array
    pub values: [f32; 10],     // Fixed-size array  
    pub timestamp: u64,        // Primitive type
}
```

### Unsafe Message Design

```rust
// DON'T DO THIS - will cause segmentation faults in shared memory!
pub struct UnsafeMessage {
    pub text: String,          // Heap pointer
    pub values: Vec<f32>,      // Heap pointer
    pub metadata: HashMap<String, String>, // Multiple heap pointers
}
```

### Conversion Utilities

All library messages provide conversion methods:

```rust
// Convert from dynamic types
let message = KeyboardInput::new(
    "Enter".to_string(),    // String  [u8; 32]
    13,
    vec!["Ctrl".to_string()], // Vec<String>  u32 bit flags
    true
);

// Convert back to dynamic types
let key_name: String = message.get_key_name();        // [u8; 32]  String
let modifiers: Vec<String> = message.get_modifiers(); // u32 flags  Vec<String>
```

## Usage Examples

### Creating a Sensor Node

```rust
use horus_core::{Node, NodeInfo, Hub, Scheduler};
use horus_library::SensorData; // Hypothetical message type

struct TemperatureSensor {
    publisher: Hub<SensorData>,
    reading_count: u32,
}

impl TemperatureSensor {
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            publisher: Hub::new("temperature_data")?,
            reading_count: 0,
        })
    }
}

impl Node for TemperatureSensor {
    fn name(&self) -> &'static str { "TemperatureSensor" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Simulate sensor reading
        let temperature = 20.0 + (self.reading_count as f32 * 0.1);
        let sensor_data = SensorData::new(temperature, "celsius");

        // Publish with automatic logging
        let _ = self.publisher.send(sensor_data, ctx);
        self.reading_count += 1;
    }
}
```

### Multi-Node Application

```rust
use horus_core::Scheduler;
use horus_core::error::HorusResult;

fn main() -> HorusResult<()> {
    let mut scheduler = Scheduler::new();

    // Input layer (highest priority)
    scheduler.add(Box::new(KeyboardInputNode::new()?), 0, Some(true));
    scheduler.add(Box::new(JoystickInputNode::new()?), 1, Some(true));

    // Processing layer (medium priority)
    scheduler.add(Box::new(ControllerNode::new()?), 5, Some(true));

    // Output layer (lowest priority)
    scheduler.add(Box::new(ActuatorNode::new()?), 10, Some(true));
    scheduler.add(Box::new(LoggerNode::new()?), 11, Some(false));

    // Run with built-in Ctrl+C handling
    scheduler.run()
}
```

## Building

### Build All Library Components

```bash
# From HORUS project root
cargo build --release -p horus_library

# Build specific components
cargo build --release -p keyboard_input_node
cargo build --release -p joystick_node
```

### Run Example Applications

```bash
# Run SnakeSim
cd horus_library/apps/snakesim
cargo run

# Monitor in another terminal
horus dashboard
```

### Testing Library Components

```bash
# Test message serialization/deserialization
cargo test -p horus_library

# Test individual nodes
cargo test -p keyboard_input_node
cargo test -p joystick_node

# Integration tests with actual hardware
cd horus_library/apps/snakesim
cargo run  # Use arrow keys and gamepad to test both input nodes
```

## Best Practices

### Node Design
1. **Use library messages** - All shared memory safe
2. **Follow priority patterns** - Input (0-4), Processing (5-9), Output (10+)  
3. **Enable logging during development** - `Some(true)` during registration
4. **Handle errors gracefully** - Don't panic in `tick()` methods

### Message Design
1. **Fixed-size only** - No String, Vec, or HashMap in messages
2. **Provide conversions** - Methods to convert to/from dynamic types
3. **Include timestamps** - For debugging and analysis
4. **Use bit flags** - More efficient than string arrays for flags

### Application Structure
1. **Start with examples** - Use SnakeSim as a template
2. **Layer by priority** - Input  Processing  Output
3. **Monitor everything** - Use `horus dashboard` during development
4. **Test incrementally** - Add one node at a time

## Contributing

### Adding New Messages
1. Use fixed-size arrays for all dynamic data
2. Implement `new()` constructor with type conversion
3. Provide getter methods for converting back to dynamic types
4. Add comprehensive tests

### Adding New Nodes
1. Follow the KeyboardInputNode pattern for input nodes
2. Implement all Node trait methods properly
3. Handle resources in `init()` and `shutdown()`
4. Use `send_raw()` in background tasks, `send()` in main thread

### Adding New Algorithms
1. Provide clean `process()` method interface
2. Include `reset()` for stateful algorithms
3. Add comprehensive unit tests
4. Document performance characteristics

## License

Part of the HORUS distributed robotics framework - Apache License 2.0.