# Scheduler API Design: Unified Backend with Optional Convenience

## Philosophy

**HORUS uses a unified backend with optional convenience constructors.**

The core principle: **Don't add APIs when configuration works.**

## The Problem We Solved

Previously, we had multiple constructors that duplicated logic:

```rust
// OLD (redundant)
pub fn new_realtime() -> Result<Self> {
    let mut nodes = Vec::new();
    nodes.reserve(128);

    let mut sched = Self {
        nodes,
        // ... 20 lines of initialization ...
        learning_complete: true,
        safety_monitor: Some(SafetyMonitor::new(3)),
    };

    sched.set_config(SchedulerConfig::hard_realtime());
    Ok(sched)
}

pub fn new_deterministic() -> Self {
    let mut sched = Self::new();
    sched.learning_complete = true;
    sched.classifier = None;
    sched
}
```

**Problem**: These constructors just set configuration and flags. They're not fundamentally different from `new()`.

## The Solution: Builder Pattern

### Unified API (Recommended for Advanced Users)

```rust
// Flexible composition - mix any features
let mut scheduler = Scheduler::new()
    .with_config(SchedulerConfig::hard_realtime())
    .with_capacity(128)
    .enable_determinism()
    .with_safety_monitor(3)
    .with_name("MyRTScheduler");

// Add OS integration
scheduler.set_realtime_priority(99)?;
scheduler.pin_to_cpu(7)?;
scheduler.lock_memory()?;
```

### Convenience Constructors (For Discoverability)

```rust
// Thin wrapper - equivalent to above
let mut scheduler = Scheduler::new_realtime()?;
scheduler.set_realtime_priority(99)?;
scheduler.pin_to_cpu(7)?;
scheduler.lock_memory()?;
```

**Key insight**: `new_realtime()` is just:
```rust
pub fn new_realtime() -> Result<Self> {
    Ok(Self::new()
        .with_config(SchedulerConfig::hard_realtime())
        .with_capacity(128)
        .enable_determinism()
        .with_safety_monitor(3)
        .with_name("RealtimeScheduler"))
}
```

## Builder Methods (Unified API)

| Method | Purpose | Type |
|--------|---------|------|
| `new()` | Base constructor | Core |
| `with_config(config)` | Apply preset (hard_realtime, standard, etc.) | Builder |
| `with_capacity(n)` | Pre-allocate nodes (determinism) | Builder |
| `enable_determinism()` | Enable reproducible execution | Builder |
| `with_safety_monitor(max_misses)` | Enable deadline monitoring | Builder |
| `with_name(name)` | Set scheduler name | Builder |

## OS Integration Methods (Low-Level)

These are **genuinely different** from configuration - they make Linux syscalls:

| Method | Syscall | Purpose |
|--------|---------|---------|
| `set_realtime_priority(99)` | `sched_setscheduler(SCHED_FIFO)` | RT scheduling |
| `pin_to_cpu(7)` | `sched_setaffinity()` | CPU isolation |
| `lock_memory()` | `mlockall()` | Prevent page faults |
| `prefault_stack(8MB)` | Touch pages | Pre-fault memory |

**These cannot be replaced by configuration** - they require OS permissions and make system calls.

## Comparison: Unified vs Multiple APIs

### Pros of Unified API (What We Chose)

✅ **Single entry point**: Just `Scheduler::new()`
✅ **Flexible composition**: Mix any options
✅ **Less code**: No duplicate constructors
✅ **Clear separation**: Builder = config, Methods = OS integration
✅ **Extensible**: Add new options without new constructors

### Cons of Multiple Constructors (What We Avoided)

❌ **API bloat**: 3-4 constructors for same backend
❌ **False distinction**: They all call `new()` internally
❌ **Rigid**: Can't mix features (RT + learning?)
❌ **Maintenance**: Test all constructor combinations
❌ **Confusing**: Which constructor do I use?

## Configuration Presets

All complex setup is moved to configuration:

```rust
// config.rs
impl SchedulerConfig {
    pub fn hard_realtime() -> Self {
        let mut config = Self::standard();
        config.preset = RobotPreset::HardRealTime;
        config.execution = ExecutionMode::JITOptimized;
        config.timing.global_rate_hz = 1000.0;
        config.timing.max_jitter_us = 5;
        config.timing.deadline_miss_policy = DeadlineMissPolicy::Panic;
        // ... 20 more settings ...
        config
    }

    pub fn industrial_robot() -> Self { /* ... */ }
    pub fn drone() -> Self { /* ... */ }
    pub fn surgical_robot() -> Self { /* ... */ }
}
```

Users can then:
```rust
// Use preset
.with_config(SchedulerConfig::hard_realtime())

// Or customize
let mut config = SchedulerConfig::hard_realtime();
config.timing.global_rate_hz = 2000.0; // Override to 2kHz
scheduler.with_config(config)
```

## Real-World Usage Patterns

### Pattern 1: Hard Real-Time Control (Surgical Robot)

```rust
let mut scheduler = Scheduler::new()
    .with_config(SchedulerConfig::hard_realtime())
    .with_capacity(64)  // 64 control nodes
    .enable_determinism()
    .with_safety_monitor(3);  // Max 3 misses before e-stop

// OS setup (requires root/CAP_SYS_NICE)
scheduler.set_realtime_priority(99)?;
scheduler.pin_to_cpu(7)?;  // Isolated core
scheduler.lock_memory()?;
scheduler.prefault_stack(8 * 1024 * 1024)?;

// Add nodes
scheduler.add_rt(motor_controller, 0, Duration::from_micros(50), Duration::from_millis(1));
scheduler.add_rt(force_sensor, 1, Duration::from_micros(20), Duration::from_millis(1));
scheduler.run()?;
```

### Pattern 2: Deterministic Simulation (Testing)

```rust
let mut scheduler = Scheduler::new()
    .enable_determinism()  // Predictable, reproducible
    .with_name("SimScheduler");

// No OS integration needed - runs in userspace
scheduler.add(physics_sim, 0, None);
scheduler.add(renderer, 1, None);
scheduler.run()?;
```

### Pattern 3: Mixed Workload (Research Robot)

```rust
let mut scheduler = Scheduler::new()
    .with_config(SchedulerConfig::standard())
    .with_capacity(128);

// Learning enabled (default) - scheduler adapts to workload
scheduler.add(camera, 0, Some(true));
scheduler.add(ml_inference, 1, Some(false));  // No logging
scheduler.add(motor_control, 2, Some(true));
scheduler.run()?;
```

## Design Lessons

1. **Configuration over constructors**: If it's just setting variables, use builder pattern
2. **Keep OS integration separate**: Syscalls are genuinely different, deserve dedicated methods
3. **Thin convenience wrappers**: `new_realtime()` helps discoverability without duplicating logic
4. **Document equivalence**: Show users the builder pattern equivalent
5. **Single source of truth**: All paths lead to `Scheduler::new()` + configuration

## Future Extensions

Adding new features is now trivial:

```rust
// New builder method
pub fn with_power_management(mut self, enabled: bool) -> Self {
    if let Some(config) = &mut self.config {
        config.resources.power_management = enabled;
    }
    self
}

// New preset
pub fn battery_optimized() -> Self { /* ... */ }
```

No new constructors needed!

## Summary

| Approach | When to Use |
|----------|-------------|
| **Builder Pattern** | Default - maximum flexibility |
| **Convenience Constructors** | Quick start, common patterns |
| **OS Integration Methods** | Always explicit - require permissions |
| **Configuration Presets** | Robot type, workload type |

**The key principle**: Keep API surface small, but provide convenient shortcuts that don't duplicate backend logic.
