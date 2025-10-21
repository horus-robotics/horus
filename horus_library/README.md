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
├── core/              # Standard library components
│   ├── messages/     # Shared memory-safe message types
│   ├── nodes/        # Reusable node implementations
│   └── algorithms/   # Common robotics algorithms
├── examples/         # Complete example applications
│   ├── snakesim/    # Multi-node snake game demo
│   └── tanksim/     # Tank simulation (in development)
├── tools/           # Development and debugging tools
│   └── sim2d/      # 2D physics simulator with visualization
└── models/          # Pre-trained models (future)
```

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
    println!("Ctrl+{} pressed", key_event.get_key_name());
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
**Location**: `nodes/keyboard_input_node/`

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
        while let Some(key_event) = self.subscriber.recv(ctx) {
            if key_event.pressed { // Only handle key presses
                match key_event.get_key_name().as_str() {
                    "ArrowUp" => println!("Move up"),
                    "ArrowDown" => println!("Move down"), 
                    "ArrowLeft" => println!("Move left"),
                    "ArrowRight" => println!("Move right"),
                    "q" if key_event.is_ctrl() => std::process::exit(0),
                    _ => {}
                }
            }
        }
    }
}

fn main() {
    let mut scheduler = Scheduler::new();
    scheduler.register(Box::new(KeyboardInputNode::new()), 0, Some(true));
    scheduler.register(Box::new(KeyHandler::new()), 1, Some(true));
    scheduler.tick_all();
}
```

**Features:**
- Cross-platform keyboard capture (Linux, macOS, Windows)
- Raw terminal mode for full key event capture
- Built-in Ctrl+C handling for proper termination
- Customizable key mappings
- Both press and release event capture

#### JoystickInputNode
**Location**: `nodes/joystick_node/`

Gamepad and joystick input capture:

```rust
use horus_library::nodes::JoystickInputNode;

// Default gamepad mappings for common controllers
// D-pad: Up/Down/Left/Right → direction codes 1/2/3/4
// Face buttons: A/B/X/Y → direction codes 1/2/3/4

let joystick_node = JoystickInputNode::new();
```

### Algorithms

#### Pathfinding
**Location**: `algorithms/pathfinding/`

Basic pathfinding algorithm template:

```rust
use horus_library::algorithms::Pathfinding;

let mut pathfinder = Pathfinding::new();
let path = pathfinder.process(&input_data);
pathfinder.reset(); // Reset algorithm state
```

### Example Apps (Applications)

Complete distributed applications demonstrating HORUS concepts.

#### SnakeSim
**Location**: `examples/snakesim/`

Multi-node snake game with dual input support:

```bash
# Run the complete snake game
cd horus_library/examples/snakesim/snake_scheduler
cargo run
```

**Architecture:**
- **KeyboardInputNode** (priority 0): Captures arrow keys → direction codes
- **JoystickInputNode** (priority 1): Captures gamepad input → direction codes  
- **SnakeControlNode** (priority 2): Processes direction codes → game logic

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

[2025-08-10 11:30:00.123] [0ms] 📤 KeyboardInputNode → 'keyboard_input' = ArrowUp
[2025-08-10 11:30:00.124] [1ms] 📥 SnakeControlNode ← 'keyboard_input' = ArrowUp
[2025-08-10 11:30:00.125] [2ms] 📤 SnakeControlNode → 'direction_command' = 1
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
    "Enter".to_string(),    // String → [u8; 32]
    13,
    vec!["Ctrl".to_string()], // Vec<String> → u32 bit flags
    true
);

// Convert back to dynamic types
let key_name: String = message.get_key_name();        // [u8; 32] → String
let modifiers: Vec<String> = message.get_modifiers(); // u32 flags → Vec<String>
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
    pub fn new() -> Self {
        Self {
            publisher: Hub::new("temperature_data").expect("Failed to create publisher"),
            reading_count: 0,
        }
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
        
        // Sleep for realistic sensor timing
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
```

### Multi-Node Application

```rust
use horus_core::Scheduler;

fn main() {
    let mut scheduler = Scheduler::new();
    
    // Input layer (highest priority)
    scheduler.register(Box::new(KeyboardInputNode::new()), 0, Some(true));
    scheduler.register(Box::new(JoystickInputNode::new()), 1, Some(true));
    
    // Processing layer (medium priority)
    scheduler.register(Box::new(ControllerNode::new()), 5, Some(true));
    
    // Output layer (lowest priority)
    scheduler.register(Box::new(ActuatorNode::new()), 10, Some(true));
    scheduler.register(Box::new(LoggerNode::new()), 11, Some(false));
    
    // Run with built-in Ctrl+C handling
    scheduler.tick_all();
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
cd horus_library/apps/snakesim/snake_scheduler
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
cd horus_library/apps/snakesim/snake_scheduler
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
2. **Layer by priority** - Input → Processing → Output
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

Part of the HORUS distributed robotics framework - MIT License.