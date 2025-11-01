# User Acceptance Test: Node Lifecycle and Scheduler

## Feature
Node trait with init/tick/shutdown lifecycle managed by priority-based scheduler.

## User Story
As a robotics developer, I want my nodes to have clear initialization, execution, and cleanup phases, managed automatically by a scheduler that respects priorities.

## Node Lifecycle Tests

### Scenario 1: Complete Node Lifecycle
**Given:** Node implements init, tick, and shutdown
**When:** Scheduler runs the node
**Then:**
- [ ] init() is called exactly once at startup
- [ ] tick() is called repeatedly
- [ ] shutdown() is called exactly once at end
- [ ] Order is always: init  tick(multiple)  shutdown

**Acceptance Criteria:**
```rust
struct TestNode {
    init_called: Arc<AtomicBool>,
    tick_count: Arc<AtomicU32>,
    shutdown_called: Arc<AtomicBool>,
}

impl Node for TestNode {
    fn name(&self) -> &'static str { "TestNode" }

    fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        self.init_called.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        self.tick_count.fetch_add(1, Ordering::SeqCst);
    }

    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        self.shutdown_called.store(true, Ordering::SeqCst);
        Ok(())
    }
}

// After running:
assert!(init_called);
assert!(tick_count > 0);
assert!(shutdown_called);
```

### Scenario 2: init() Failure Prevents Execution
**Given:** Node's init() returns Err
**When:** Scheduler tries to start node
**Then:**
- [ ] init() is called
- [ ] Error is logged
- [ ] tick() is NOT called
- [ ] shutdown() is NOT called
- [ ] Other nodes continue running

**Acceptance Criteria:**
```rust
fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
    Err(HorusError::config("Initialization failed"))
}

// Scheduler should:
// - Log error
// - Not execute tick()
// - Continue with other nodes
```

### Scenario 3: tick() Never Returns Error
**Given:** tick() has signature `fn tick(&mut self, Option<&mut NodeInfo>)`
**When:** tick() executes
**Then:**
- [ ] No Result type to return
- [ ] Errors must be handled internally
- [ ] Node cannot stop scheduler from tick()
- [ ] Panic would crash the scheduler (user responsibility)

### Scenario 4: shutdown() Failure
**Given:** Node's shutdown() returns Err
**When:** Scheduler stops
**Then:**
- [ ] shutdown() is called
- [ ] Error is logged
- [ ] Scheduler continues shutting down other nodes
- [ ] Process exits normally (error doesn't block shutdown)

### Scenario 5: Optional init/shutdown
**Given:** Node only implements tick() (not init/shutdown)
**When:** Scheduler runs node
**Then:**
- [ ] Default init() does nothing (returns Ok)
- [ ] tick() executes normally
- [ ] Default shutdown() does nothing (returns Ok)
- [ ] No errors occur

**Acceptance Criteria:**
```rust
impl Node for MinimalNode {
    fn name(&self) -> &'static str { "MinimalNode" }

    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Only required method
    }

    // init and shutdown are optional (default implementations)
}
```

## Scheduler Tests

### Scenario 6: Priority-Based Execution
**Given:** Three nodes with priorities 0, 5, 10
**When:** Scheduler executes tick_all()
**Then:**
- [ ] Priority 0 executes first
- [ ] Priority 5 executes second
- [ ] Priority 10 executes last
- [ ] Order is consistent every tick

**Acceptance Criteria:**
```rust
let mut scheduler = Scheduler::new();
scheduler.register(Box::new(HighPriority), 0, Some(true));
scheduler.register(Box::new(MediumPriority), 5, Some(true));
scheduler.register(Box::new(LowPriority), 10, Some(true));

// Each tick: HighPriority  MediumPriority  LowPriority
```

### Scenario 7: Same Priority (Order Not Guaranteed)
**Given:** Two nodes both with priority 5
**When:** Scheduler executes
**Then:**
- [ ] Both nodes execute
- [ ] Order between them is undefined
- [ ] Both complete each tick

### Scenario 8: Scheduler Loop
**Given:** Scheduler with registered nodes
**When:** tick_all() is called
**Then:**
- [ ] Runs continuously until stopped
- [ ] Executes all nodes each iteration
- [ ] Sleeps ~16ms between ticks (60 FPS)
- [ ] Ctrl+C stops gracefully

**Acceptance Criteria:**
```rust
let mut scheduler = Scheduler::new();
scheduler.register(Box::new(MyNode), 0, Some(true));

// This blocks until Ctrl+C:
scheduler.tick_all()?;
```

### Scenario 9: Ctrl+C Graceful Shutdown
**Given:** Scheduler is running
**When:** User presses Ctrl+C
**Then:**
- [ ] Signal is caught
- [ ] All nodes' shutdown() methods called
- [ ] Shared memory cleaned up
- [ ] Process exits with code 0
- [ ] No zombie processes

### Scenario 10: Selective Node Execution
**Given:** Scheduler has multiple nodes
**When:** User calls `tick_node(&["NodeA", "NodeB"])`
**Then:**
- [ ] Only NodeA and NodeB tick
- [ ] Other nodes do not execute
- [ ] Useful for testing individual nodes

**Acceptance Criteria:**
```rust
scheduler.register(Box::new(NodeA), 0, Some(true));
scheduler.register(Box::new(NodeB), 1, Some(true));
scheduler.register(Box::new(NodeC), 2, Some(true));

// Only ticks NodeA and NodeB:
scheduler.tick_node(&["NodeA", "NodeB"])?;
```

### Scenario 11: Logging Enabled/Disabled
**Given:** Nodes registered with different logging settings
**When:** Scheduler executes
**Then:**
- [ ] Logging-enabled nodes show messages
- [ ] Logging-disabled nodes are silent
- [ ] Performance difference is measurable

**Acceptance Criteria:**
```rust
scheduler.register(Box::new(Node1), 0, Some(true));  // Logs
scheduler.register(Box::new(Node2), 1, Some(false)); // Silent
scheduler.register(Box::new(Node3), 2, None);        // Default (enabled)
```

### Scenario 12: Scheduler Heartbeat
**Given:** Scheduler is running
**When:** Monitoring heartbeat file
**Then:**
- [ ] Heartbeat file created in /dev/shm/horus/heartbeats/
- [ ] Updated every tick
- [ ] Contains timestamp
- [ ] Used by monitoring tools

**Acceptance Criteria:**
```bash
$ watch -n 1 cat /dev/shm/horus/heartbeats/scheduler
# Should update every tick
```

### Scenario 13: Multiple Schedulers (Warning)
**Given:** User creates two Scheduler instances
**When:** Both try to run
**Then:**
- [ ] Warning or error about multiple schedulers
- [ ] Or both run independently (if design allows)
- [ ] Document expected behavior

## Context (NodeInfo) Tests

### Scenario 14: NodeInfo in tick()
**Given:** tick() receives `Option<&mut NodeInfo>`
**When:** Node checks `ctx`
**Then:**
- [ ] `Some(ctx)` when logging enabled
- [ ] `None` when logging disabled
- [ ] Node must handle both cases

**Acceptance Criteria:**
```rust
fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
    if let Some(ctx) = ctx {
        ctx.log_info("Logging enabled");
    }
    // Continue regardless of ctx
}
```

### Scenario 15: Logging Methods
**Given:** NodeInfo is available
**When:** Node logs messages
**Then:**
- [ ] log_info() works
- [ ] log_warning() works
- [ ] log_error() works
- [ ] log_pub() auto-called by Hub.send()
- [ ] log_sub() auto-called by Hub.recv()

**Acceptance Criteria:**
```rust
fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
    ctx.log_info("Node initialized");
    ctx.log_warning("This is a warning");
    ctx.log_error("This is an error");
    Ok(())
}
```

### Scenario 16: Metrics Tracking
**Given:** Node has been running
**When:** Accessing ctx.metrics()
**Then:**
- [ ] total_ticks available
- [ ] avg_tick_duration_ms calculated
- [ ] max_tick_duration_ms recorded
- [ ] messages_sent counted
- [ ] messages_received counted
- [ ] uptime_seconds available

**Acceptance Criteria:**
```rust
fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
    if let Some(ctx) = ctx {
        let m = ctx.metrics();
        println!("Tick #{}, Avg: {}ms", m.total_ticks, m.avg_tick_duration_ms);
    }
}
```

## Error Handling

### Scenario 17: Node Panic in tick()
**Given:** Node panics during tick()
**When:** Scheduler executes
**Then:**
- [ ] Entire scheduler crashes (Rust panic behavior)
- [ ] OR panic is caught and logged (if using catch_unwind)
- [ ] Document expected behavior
- [ ] User responsibility to avoid panics

### Scenario 18: Slow tick() Execution
**Given:** Node's tick() takes 500ms
**When:** Scheduler runs at 60 FPS
**Then:**
- [ ] Scheduler is blocked for 500ms
- [ ] Other nodes wait
- [ ] Warning logged about slow node
- [ ] Frame rate drops below target

### Scenario 19: Resource Leaks
**Given:** Node allocates resources in init()
**When:** Node runs for extended period
**Then:**
- [ ] Memory usage is stable
- [ ] No resource leaks
- [ ] shutdown() properly cleans up

## Performance Tests

### Scenario 20: Tick Rate Accuracy
**Given:** Scheduler runs with no nodes
**When:** Measuring tick rate
**Then:**
- [ ] Achieves ~60 FPS (16ms sleep)
- [ ] Consistent timing
- [ ] Minimal jitter

### Scenario 21: Many Nodes Performance
**Given:** 50 nodes registered
**When:** Scheduler executes
**Then:**
- [ ] All nodes execute each tick
- [ ] Overhead is minimal (<1ms)
- [ ] Tick rate maintained

### Scenario 22: Low Latency Priority
**Given:** Critical node at priority 0
**When:** Scheduler executes
**Then:**
- [ ] Priority 0 executes immediately
- [ ] No waiting for lower priority nodes
- [ ] Deterministic execution order

## Non-Functional Requirements

- [ ] Scheduler starts in < 100ms
- [ ] Shutdown completes in < 1 second
- [ ] Priority sorting is efficient (O(n log n))
- [ ] Ctrl+C response time < 100ms
- [ ] Logging has minimal overhead (< 10μs per log)
- [ ] Works on Linux and macOS
- [ ] No unsafe code in user-facing API
- [ ] Thread-safe where documented
