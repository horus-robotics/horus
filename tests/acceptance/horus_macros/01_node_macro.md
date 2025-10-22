# User Acceptance Test: node! Macro

## Feature
Procedural macro for eliminating boilerplate in Node implementations.

## User Story
As a Rust developer, I want to define nodes with minimal code using a clean, section-based syntax so that I can focus on logic instead of boilerplate.

## Basic Macro Tests

### Scenario 1: Minimal Node
**Given:** User wants simplest possible node
**When:** Using node! macro with only tick
**Then:**
- [ ] Compiles successfully
- [ ] Generates struct with Default impl
- [ ] Generates Node trait impl
- [ ] tick() method works

**Acceptance Criteria:**
```rust
use horus_macros::node;

node! {
    MinimalNode {
        tick {
            println!("Ticking!");
        }
    }
}

// Should compile and work
let node = MinimalNode::default();
```

### Scenario 2: Node with Publishers
**Given:** User wants to publish data
**When:** Defining pub section
**Then:**
- [ ] Hub fields generated in struct
- [ ] new() method creates Hubs
- [ ] Publishers are accessible in tick

**Acceptance Criteria:**
```rust
node! {
    SensorNode {
        pub {
            temperature: f32 -> "sensors/temp",
            pressure: f32 -> "sensors/pressure",
        }

        tick(ctx) {
            self.temperature.send(25.0, ctx).ok();
            self.pressure.send(101.3, ctx).ok();
        }
    }
}

let node = SensorNode::new().unwrap();
```

### Scenario 3: Node with Subscribers
**Given:** User wants to receive data
**When:** Defining sub section
**Then:**
- [ ] Hub fields generated for subscriptions
- [ ] Can receive messages in tick

**Acceptance Criteria:**
```rust
node! {
    ControlNode {
        sub {
            cmd_vel: CmdVel <- "robot/cmd",
        }

        tick(ctx) {
            if let Some(cmd) = self.cmd_vel.recv(ctx) {
                // Process command
            }
        }
    }
}
```

### Scenario 4: Node with Data Fields
**Given:** User needs internal state
**When:** Defining data section
**Then:**
- [ ] Fields added to struct
- [ ] Default values used in new()
- [ ] Fields accessible in methods

**Acceptance Criteria:**
```rust
node! {
    CounterNode {
        data {
            counter: u32 = 0,
            name: String = String::from("default"),
        }

        tick {
            self.counter += 1;
            println!("{}: {}", self.name, self.counter);
        }
    }
}
```

### Scenario 5: Node with init and shutdown
**Given:** User needs lifecycle methods
**When:** Defining init and shutdown sections
**Then:**
- [ ] Methods generated in Node trait impl
- [ ] init() called at startup
- [ ] shutdown() called at exit

**Acceptance Criteria:**
```rust
node! {
    LifecycleNode {
        init(ctx) {
            ctx.log_info("Node starting");
            Ok(())
        }

        tick(ctx) {
            // Main loop
        }

        shutdown(ctx) {
            ctx.log_info("Node stopping");
            Ok(())
        }
    }
}
```

### Scenario 6: Node with Custom Methods
**Given:** User wants helper methods
**When:** Defining impl section
**Then:**
- [ ] Methods added to struct impl
- [ ] Accessible from tick() and other methods

**Acceptance Criteria:**
```rust
node! {
    ProcessorNode {
        sub {
            input: f32 <- "raw_data",
        }

        pub {
            output: f32 -> "processed_data",
        }

        tick(ctx) {
            if let Some(data) = self.input.recv(ctx) {
                let result = self.process(data);
                self.output.send(result, ctx).ok();
            }
        }

        impl {
            fn process(&self, data: f32) -> f32 {
                data * 2.0 + 1.0
            }

            fn another_helper(&self) -> String {
                String::from("helper")
            }
        }
    }
}

// Custom methods should be usable:
let node = ProcessorNode::new().unwrap();
assert_eq!(node.process(5.0), 11.0);
```

## Generated Code Tests

### Scenario 7: Struct Generation
**Given:** node! macro with name MyNode
**When:** Macro expands
**Then:**
- [ ] Struct named MyNode created
- [ ] All pub, sub, and data fields present
- [ ] Correct visibility (pub struct)

**Acceptance Criteria:**
```rust
// Input:
node! { MyNode { ... } }

// Generated (conceptually):
pub struct MyNode {
    pub_hub: Hub<T>,
    sub_hub: Hub<T>,
    data_field: Type,
}
```

### Scenario 8: new() Generation
**Given:** Node has Hubs
**When:** new() method is called
**Then:**
- [ ] All Hubs created with Hub::new()
- [ ] Returns HorusResult<Self>
- [ ] Error propagation with ?

**Acceptance Criteria:**
```rust
node! {
    TestNode {
        pub { output: i32 -> "test" }
    }
}

// Generated new():
impl TestNode {
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            output: Hub::new("test")?,
        })
    }
}
```

### Scenario 9: Node Trait Implementation
**Given:** macro defines node
**When:** Node trait is implemented
**Then:**
- [ ] name() returns snake_case version of struct name
- [ ] tick() calls user-defined tick
- [ ] init() and shutdown() included if defined

**Acceptance Criteria:**
```rust
node! { MyRobotNode { tick {} } }

// Generated:
impl Node for MyRobotNode {
    fn name(&self) -> &'static str {
        "my_robot_node"  // CamelCase -> snake_case
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // User's tick code
    }
}
```

### Scenario 10: Default Trait Implementation
**Given:** Node is generated
**When:** Default trait is used
**Then:**
- [ ] Calls new() and expects success
- [ ] Panics with clear message on failure

**Acceptance Criteria:**
```rust
node! { TestNode { tick {} } }

// Generated:
impl Default for TestNode {
    fn default() -> Self {
        Self::new().expect("Failed to create TestNode")
    }
}

let node = TestNode::default();
```

## Context Parameter Tests

### Scenario 11: tick with ctx
**Given:** tick(ctx) syntax used
**When:** Macro expands
**Then:**
- [ ] ctx parameter available in tick body
- [ ] Type is Option<&mut NodeInfo>

**Acceptance Criteria:**
```rust
node! {
    LoggingNode {
        tick(ctx) {
            if let Some(ctx) = ctx {
                ctx.log_info("Tick!");
            }
        }
    }
}
```

### Scenario 12: tick without ctx
**Given:** tick {} (no parameter)
**When:** Macro expands
**Then:**
- [ ] No ctx available in tick body
- [ ] Generated signature still has ctx parameter
- [ ] Compiles successfully

**Acceptance Criteria:**
```rust
node! {
    SimpleNode {
        tick {
            // No ctx available here
            println!("Tick");
        }
    }
}

// Generated signature still includes ctx but doesn't expose it to user
```

### Scenario 13: init/shutdown with ctx
**Given:** init(ctx) and shutdown(ctx) defined
**When:** Methods are generated
**Then:**
- [ ] ctx is &mut NodeInfo (not Option)
- [ ] Available in method body

**Acceptance Criteria:**
```rust
node! {
    MyNode {
        init(ctx) {
            ctx.log_info("Init");  // ctx is &mut NodeInfo
            Ok(())
        }

        tick {}

        shutdown(ctx) {
            ctx.log_info("Shutdown");
            Ok(())
        }
    }
}
```

## Type Safety Tests

### Scenario 14: Type Checking
**Given:** User specifies message types
**When:** Compiling
**Then:**
- [ ] Type mismatches caught at compile time
- [ ] No runtime type errors possible
- [ ] Hub<T> is properly typed

**Acceptance Criteria:**
```rust
node! {
    TypedNode {
        pub {
            velocity: CmdVel -> "cmd_vel",
        }

        tick {
            let cmd = CmdVel::new(1.0, 0.0);
            self.velocity.send(cmd, None).ok();
            // self.velocity.send(42, None);  // Compile error!
        }
    }
}
```

### Scenario 15: Generic Message Types
**Given:** User uses custom structs
**When:** Defining pub/sub
**Then:**
- [ ] Supports any type implementing required traits
- [ ] Serialization requirements enforced

**Acceptance Criteria:**
```rust
#[derive(Clone, Serialize, Deserialize)]
struct CustomMsg {
    data: f64,
}

node! {
    CustomNode {
        pub {
            custom: CustomMsg -> "custom_topic",
        }

        tick {}
    }
}
```

## Error Handling Tests

### Scenario 16: Missing Required Section
**Given:** Node definition without tick
**When:** Compiling
**Then:**
- [ ] Compile error
- [ ] Error message: "tick section is required"

**Acceptance Criteria:**
```rust
node! {
    BadNode {
        // No tick!
    }
}
// Should fail to compile
```

### Scenario 17: Invalid Syntax
**Given:** Malformed node definition
**When:** Macro expansion attempted
**Then:**
- [ ] Clear compile error
- [ ] Points to problematic syntax
- [ ] Suggests correction if possible

### Scenario 18: Hub Creation Failure
**Given:** Hub::new() fails (e.g., permissions)
**When:** new() is called
**Then:**
- [ ] Returns Err(HorusError)
- [ ] Error propagates with ?
- [ ] No partial initialization

**Acceptance Criteria:**
```rust
node! {
    FailNode {
        pub { out: i32 -> "test" }
        tick {}
    }
}

match FailNode::new() {
    Ok(node) => { /* use node */ },
    Err(e) => println!("Failed: {}", e),
}
```

## Naming Tests

### Scenario 19: CamelCase to snake_case
**Given:** Struct named MyRobotNode
**When:** name() method is called
**Then:**
- [ ] Returns "my_robot_node"
- [ ] Conversion is automatic

**Acceptance Criteria:**
```rust
node! { SensorDriverV2 { tick {} } }

let node = SensorDriverV2::default();
assert_eq!(node.name(), "sensor_driver_v2");
```

### Scenario 20: Single Word Name
**Given:** Struct named Robot
**When:** name() method is called
**Then:**
- [ ] Returns "robot"

### Scenario 21: Acronyms in Name
**Given:** Struct named IMUNode
**When:** name() method is called
**Then:**
- [ ] Returns "imu_node"

## Integration Tests

### Scenario 22: Macro Node in Scheduler
**Given:** Node defined with macro
**When:** Registered with Scheduler
**Then:**
- [ ] Works identically to manual impl
- [ ] Lifecycle methods execute
- [ ] Communication works

**Acceptance Criteria:**
```rust
node! {
    TestNode {
        pub { out: i32 -> "test" }

        tick(ctx) {
            self.out.send(42, ctx).ok();
        }
    }
}

let mut scheduler = Scheduler::new();
scheduler.register(Box::new(TestNode::new()?), 0, Some(true));
// Works!
```

### Scenario 23: Multiple Nodes with Macro
**Given:** Multiple nodes defined
**When:** All use node! macro
**Then:**
- [ ] No naming conflicts
- [ ] Each generates independently
- [ ] All work in same scheduler

### Scenario 24: Macro and Manual Nodes Together
**Given:** Some nodes use macro, some don't
**When:** Running together
**Then:**
- [ ] Both types work
- [ ] Can communicate via Hubs
- [ ] No compatibility issues

## Non-Functional Requirements

- [ ] Macro expansion is fast (< 100ms)
- [ ] Generated code is readable (for debugging)
- [ ] No unsafe code generated
- [ ] Error messages are helpful
- [ ] Works with rust-analyzer (IDE support)
- [ ] Documentation comments preserved
- [ ] Attributes on struct preserved (if applicable)
