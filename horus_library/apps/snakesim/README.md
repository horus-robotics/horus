# Snakesim - Snake Game Controller

A simple snake game controller built with HORUS that demonstrates keyboard and joystick input handling.

## Architecture

This is a proper HORUS project with a single-file structure:
- `main.rs` - Backend controller (keyboard + joystick input + game logic)
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

### Keyboard
- Arrow Keys or WASD - Control snake direction
  - Up/W: Move up
  - Down/S: Move down
  - Left/A: Move left
  - Right/D: Move right
- ESC - Quit keyboard capture

### Joystick/Gamepad
- D-Pad - Control snake direction (Up/Down/Left/Right)
- Left Stick - Control snake direction (analog input with threshold)
  - Push up: Move up
  - Push down: Move down
  - Push left: Move left
  - Push right: Move right

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

- Uses HORUS Hub for pub/sub messaging (MPMC)
- **Input Topics:**
  - `keyboard_input` - KeyboardInput messages from KeyboardInputNode
  - `joystick_input` - JoystickInput messages from JoystickInputNode
- **Output Topic:**
  - `snakestate` - Snake direction (u32) from SnakeControlNode
- Priority-based scheduler:
  - Priority 0: KeyboardInputNode (captures keyboard events)
  - Priority 0: JoystickInputNode (captures gamepad events)
  - Priority 1: SnakeControlNode (converts inputs to directions)

## Message Flow

```
KeyboardInputNode → [keyboard_input] ↘
                                       → SnakeControlNode → [snakestate] → GUI
JoystickInputNode → [joystick_input] ↗
```

Since Hub is MPMC (Multi-Producer, Multi-Consumer), both input nodes can publish simultaneously, and SnakeControlNode subscribes to both topics.

## Message Types

### Input Messages
- `KeyboardInput` - Keyboard events (key code, pressed state)
- `JoystickInput` - Joystick events (buttons, axes, d-pad)

### Output Messages
- `direction: u32` - Snake direction (1=Up, 2=Down, 3=Left, 4=Right)
