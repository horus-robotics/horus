# Snakesim - Snake Game Controller

A simple snake game controller built with HORUS that demonstrates keyboard and joystick input handling.

## Architecture

This is a proper HORUS project with a single-file structure:
- `main.rs` - Backend controller (keyboard input + game logic)
- `snakesim_gui/` - Separate GUI application (optional visualization)

## Running

### Using horus run (recommended)

```bash
cd snakesim
horus run
```

### Using cargo

```bash
cd snakesim
cargo run
```

## Controls

- Arrow Keys or WASD - Control snake direction
  - Up/W: Move up
  - Down/S: Move down
  - Left/A: Move left
  - Right/D: Move right
- ESC - Quit keyboard capture

## Running with GUI

The GUI is a separate application that subscribes to the snake state messages:

```bash
# Terminal 1: Run the controller
cd snakesim
horus run

# Terminal 2: Run the GUI
cd snakesim_gui
cargo run
```

## Technical Details

- Uses HORUS Hub for pub/sub messaging
- SnakeState messages published to "snakestate" topic
- Keyboard and joystick input on "snakeinput" topic
- Priority-based scheduler:
  - Priority 0: KeyboardInputNode
  - Priority 1: JoystickInputNode
  - Priority 2: SnakeControlNode

## Message Types

### SnakeState
- `direction: u32` - Snake direction (1=Up, 2=Down, 3=Left, 4=Right)
