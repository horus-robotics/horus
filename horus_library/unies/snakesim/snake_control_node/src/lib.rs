use horus::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct SnakeState {
    pub direction: u32,
}

/// Generic snake control node that converts input codes to SnakeState
pub struct SnakeControlNode {
    keyboard_subscriber: Hub<KeyboardInput>,
    joystick_subscriber: Hub<JoystickInput>,
    snake_publisher: Hub<SnakeState>,
}
impl Default for SnakeControlNode {
    fn default() -> Self {
        Self::new()
    }
}

impl SnakeControlNode {
    /// Create with default topics
    pub fn new() -> Self {
        Self {
            keyboard_subscriber: Hub::new("keyboard_input")
                .expect("Failed to create keyboard subscriber"),
            joystick_subscriber: Hub::new("joystick_input")
                .expect("Failed to create joystick subscriber"),
            snake_publisher: Hub::new("snakestate").expect("Failed to create snake publisher"),
        }
    }

    /// Create with custom topics - both keyboard and joystick can use the same topic
    pub fn new_with_topics(keyboard_topic: &str, joystick_topic: &str, snake_topic: &str) -> Self {
        Self {
            keyboard_subscriber: Hub::new(keyboard_topic)
                .expect("Failed to create keyboard subscriber"),
            joystick_subscriber: Hub::new(joystick_topic)
                .expect("Failed to create joystick subscriber"),
            snake_publisher: Hub::new(snake_topic).expect("Failed to create snake publisher"),
        }
    }
}

impl Node for SnakeControlNode {
    fn name(&self) -> &'static str {
        "SnakeControlNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Handle keyboard input
        while let Some(input) = self.keyboard_subscriber.recv(ctx.as_deref_mut()) {
            if input.pressed {
                // Map keyboard codes to snake directions
                // Using standard arrow key codes (37-40) or WASD keys
                let direction = match input.code {
                    38 | 87 => 1,  // ArrowUp or W -> Up
                    40 | 83 => 2,  // ArrowDown or S -> Down
                    37 | 65 => 3,  // ArrowLeft or A -> Left
                    39 | 68 => 4,  // ArrowRight or D -> Right
                    _ => continue, // Ignore other keys
                };

                let snake_state = SnakeState { direction };
                let _ = self.snake_publisher.send(snake_state, ctx.as_deref_mut());
            }
        }

        // Handle joystick input
        while let Some(input) = self.joystick_subscriber.recv(ctx.as_deref_mut()) {
            if input.is_button() && input.pressed {
                let snake_state = SnakeState {
                    direction: input.element_id,
                };
                let _ = self.snake_publisher.send(snake_state, ctx.as_deref_mut());
            }
        }
    }
}
