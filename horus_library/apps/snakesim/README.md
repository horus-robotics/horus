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

The snake game has a **beautiful graphical interface** that shows the snake moving on a grid!

The GUI is included as a pre-built binary. Run both components together:

```bash
# Terminal 1: Run the backend (keyboard input + game logic)
horus run

# Terminal 2: Run the GUI (visual display)
./snakesim_gui
```

Use **Arrow Keys or WASD** in Terminal 1 to control the snake, and watch it move in the GUI window!

### GUI Features
- 20x20 grid with dark background
- Bright green snake with animated eyes
- Eyes point in the direction of movement
- Smooth updates (200ms tick rate)
- Real-time IPC communication via HORUS Hub

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
