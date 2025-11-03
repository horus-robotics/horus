use crate::JoystickInput;
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

/// Joystick Input Node - Gamepad/joystick input capture (placeholder implementation)
///
/// This is a simplified placeholder for the joystick input functionality.
/// The full implementation would capture gamepad events and publish them.
pub struct JoystickInputNode {
    publisher: Hub<JoystickInput>,
    last_input_time: u64,
}

impl JoystickInputNode {
    /// Create a new joystick input node with default topic "joystick_input"
    pub fn new() -> Result<Self> {
        Self::new_with_topic("joystick_input")
    }

    /// Create a new joystick input node with custom topic
    pub fn new_with_topic(topic: &str) -> Result<Self> {
        Ok(Self {
            publisher: Hub::new(topic)?,
            last_input_time: 0,
        })
    }
}

impl Node for JoystickInputNode {
    fn name(&self) -> &'static str {
        "JoystickInputNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Placeholder implementation - would capture actual joystick input
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Only send periodic test messages
        if current_time - self.last_input_time > 3000 {
            // Every 3 seconds
            let joystick_input = JoystickInput::new_button(
                1, // joystick_id
                0, // button_id
                "ButtonA".to_string(),
                true, // pressed
            );
            let _ = self.publisher.send(joystick_input, None);
            self.last_input_time = current_time;
        }
    }
}

// Default impl removed - use Node::new() instead which returns HorusResult
