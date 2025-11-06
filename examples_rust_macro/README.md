# HORUS Rust Macro Examples

This directory contains side-by-side comparisons of HORUS nodes written with **manual implementation** vs **`node!` macro**.

## Files

### Simple Examples
- **`simple_sensor_manual.rs`** - Manual implementation (26 lines)
- **`simple_sensor_macro.rs`** - Macro implementation (15 lines) → **42% reduction**

### Complex Examples
- **`robot_controller_manual.rs`** - Manual implementation (120 lines)
- **`robot_controller_macro.rs`** - Macro implementation (65 lines) → **46% reduction**

## Running Examples

```bash
# Manual implementation
horus run examples_rust_macro/simple_sensor_manual.rs

# Macro implementation
horus run examples_rust_macro/simple_sensor_macro.rs
```

## Key Differences

### Manual Implementation
```rust
// Explicit struct definition
struct TempSensor {
    temp_pub: Hub<f32>,
    counter: f32,
}

// Explicit constructor with error handling
impl TempSensor {
    fn new() -> Result<Self> {
        Ok(Self {
            temp_pub: Hub::new("temperature")?,
            counter: 0.0,
        })
    }
}

// Explicit Node trait implementation
impl Node for TempSensor {
    fn name(&self) -> &'static str { "TempSensor" }
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // ...
    }
}
```

**Pros:**
- Explicit control over all code
- Easy to debug (no macro expansion)
- Can implement custom traits beyond Node

**Cons:**
- Verbose boilerplate
- Easy to forget Hub::new() error handling
- Repetitive struct + impl pattern

### Macro Implementation
```rust
node! {
    TempSensor {
        pub { temp_pub: f32 -> "temperature" }
        data { counter: f32 = 0.0 }

        tick {
            // Same logic, less boilerplate!
        }
    }
}
```

**Pros:**
- **42-46% less code**
- Declarative data flow (pub/sub sections)
- Auto-generated Hub::new() with error handling
- Auto-generated Node trait impl
- Auto snake_case naming

**Cons:**
- Macro expansion can be opaque (use `cargo expand`)
- Less control over generated code

## Recommendations

### Use `node!` Macro When:
✅ Building standard pub/sub nodes
✅ Learning HORUS (clearer intent)
✅ Rapid prototyping
✅ Standard lifecycle patterns

### Use Manual When:
⚠️ Complex custom initialization
⚠️ Dynamic number of publishers/subscribers
⚠️ Custom trait implementations
⚠️ Performance debugging

**Default: Start with macro, move to manual only when needed.**

## Performance

Both implementations generate **identical runtime code** after macro expansion:
- Same memory layout
- Same function calls
- Same zero-cost abstractions

**Performance difference: 0%**

## Generated Code Inspection

To see what the macro generates:

```bash
cargo install cargo-expand
cargo expand --example simple_sensor_macro
```

You'll see the macro expands to the same code as the manual implementation!

## Further Reading

See `../RUST_BOILERPLATE_REDUCTION.md` for detailed analysis and recommendations.
