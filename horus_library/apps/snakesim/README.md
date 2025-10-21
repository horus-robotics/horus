# SnakeSim - Uni Snake Game Demo

**Demonstration of HORUS uni (multi-node) architecture** using a classic snake game.

## What is SnakeSim?

SnakeSim is a **uni (multi-node application)** that demonstrates HORUS's pub/sub messaging and priority-based scheduling using a playable snake game.

**Architecture:**
```
┌─────────────────────┐
│  Keyboard Input     │ Priority 0 (highest)
│  Reads arrow keys   │
└──────┬──────────────┘
       │ direction
       ▼
┌─────────────────────┐
│  Snake Control      │ Priority 2
│  Game logic         │
└──────┬──────────────┘
       │ state
       ▼
┌─────────────────────┐
│  GUI Renderer       │ Priority 3 (lowest)
│  Displays game      │
└─────────────────────┘
```

## Quick Start

### Run with Scheduler (Recommended)

```bash
cd horus_library/apps/snakesim/snake_scheduler
cargo run --release
```

**Controls:**
- ⬆️ Arrow keys to move snake
- Press Ctrl+C to quit

### Run with HORUS CLI

```bash
# From HORUS root
horus run horus_library/apps/snakesim/snake_scheduler/src/main.rs

# Or with release mode
horus run -r horus_library/apps/snakesim/snake_scheduler/src/main.rs
```

## Project Structure

```
snakesim/
├── README.md              # This file
├── snake_control_node/    # Game logic node
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs         # Snake control implementation
├── snake_scheduler/       # Scheduler that runs all nodes
│   ├── Cargo.toml
│   └── src/
│       └── main.rs        # Main entry point
└── snakesim_gui/          # GUI renderer node
    ├── Cargo.toml
    └── src/
        └── lib.rs         # Terminal UI rendering
```

## How It Works

### Node Communication

**Topics:**
- `keyboard/direction` - KeyboardInput → SnakeControl
- `snake/state` - SnakeControl → GUI

**Message Types:**
- `Direction` - Arrow key input (Up, Down, Left, Right)
- `SnakeState` - Game state (snake position, food, score)

### Priority-Based Execution

```rust
// Priority 0: Read input first
scheduler.register(keyboard_node, 0, Some(true));

// Priority 2: Process game logic
scheduler.register(snake_control_node, 2, Some(true));

// Priority 3: Render last
scheduler.register(gui_node, 3, Some(true));
```

Each `tick()` executes nodes in priority order:
1. Read keyboard input
2. Update snake position
3. Render new state

## Learning Objectives

SnakeSim demonstrates:

 **Uni (multi-node) architecture** - Multiple nodes working together
 **Pub/sub messaging** - Nodes communicate via topics
 **Priority scheduling** - Deterministic execution order
 **Built-in logging** - Automatic message tracking
 **Clean separation** - Input/Logic/Display separated

## Development

### Build Individual Nodes

```bash
# Build snake control logic
cd snake_control_node
cargo build --release

# Build GUI renderer
cd ../snakesim_gui
cargo build --release

# Build scheduler (main binary)
cd ../snake_scheduler
cargo build --release
```

### Run with Logging

```bash
cd snake_scheduler
RUST_LOG=debug cargo run --release
```

## Game Rules

- 🟢 Green snake starts in center
- 🔴 Red food appears randomly
- Eat food to grow (+1 segment)
- Don't hit walls or yourself
- Score increases with each food

## Code Examples

### Snake Control Node (Simplified)

```rust
pub struct SnakeControlNode {
    direction_sub: Hub<Direction>,
    state_pub: Hub<SnakeState>,
    snake: Vec<Position>,
    food: Position,
}

impl Node for SnakeControlNode {
    fn tick(&mut self, ctx: Option<&mut NodeInfo>) {
        // Read input
        if let Some(dir) = self.direction_sub.recv(ctx) {
            self.current_direction = dir;
        }

        // Update snake
        self.move_snake();
        self.check_collisions();

        // Publish state
        self.state_pub.send(self.get_state(), ctx).ok();
    }
}
```

### Scheduler (Simplified)

```rust
fn main() {
    let mut scheduler = Scheduler::new();

    // Register nodes with priorities
    scheduler.register(Box::new(KeyboardInputNode::new()?), 0, Some(true));
    scheduler.register(Box::new(SnakeControlNode::new()?), 2, Some(true));
    scheduler.register(Box::new(GUINode::new()?), 3, Some(true));

    // Run forever (or until Ctrl+C)
    scheduler.tick_all();
}
```

## Next Steps

After trying SnakeSim:

1. **Modify game logic** - Change snake speed, add obstacles
2. **Add new nodes** - Add scoreboard node, AI player node
3. **Create your own uni** - Use as template for multi-node apps
4. **Deploy to robot** - Apply same patterns to real robotics

## Troubleshooting

**"Failed to create Hub":**
- Make sure no other instance is running
- Topics are automatically created on first use

**Snake doesn't move:**
- Check that keyboard input is working
- Verify message flow with logging enabled

**Build fails:**
- Run `cargo clean` in each node directory
- Ensure you're in the correct directory

## Tips

- Start simple: Read `snake_scheduler/src/main.rs` first
- Enable logging to see message flow
- Try modifying snake speed in `SnakeControlNode`
- Experiment with different priorities

## License

Same as HORUS framework - MIT/Apache-2.0

---

**Perfect introduction to HORUS unis (multi-node applications)!** 🐍
