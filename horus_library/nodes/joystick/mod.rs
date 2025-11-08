use crate::JoystickInput;
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo, NodeInfoExt};

#[cfg(feature = "gilrs")]
use gilrs::{Gilrs, Event, EventType, Button, Axis};

#[cfg(not(feature = "gilrs"))]
use std::time::{SystemTime, UNIX_EPOCH};

/// Joystick Input Node - Real gamepad/joystick input capture
///
/// Captures real joystick/gamepad input using the gilrs library.
/// Publishes button presses and axis movements to the Hub.
pub struct JoystickInputNode {
    publisher: Hub<JoystickInput>,
    #[cfg(feature = "gilrs")]
    gilrs: Gilrs,
    #[cfg(not(feature = "gilrs"))]
    last_input_time: u64,
}

impl JoystickInputNode {
    /// Create a new joystick input node with default topic "joystick_input"
    pub fn new() -> Result<Self> {
        Self::new_with_topic("joystick_input")
    }

    /// Create a new joystick input node with custom topic
    pub fn new_with_topic(topic: &str) -> Result<Self> {
        #[cfg(feature = "gilrs")]
        {
            let gilrs = Gilrs::new().map_err(|e| {
                horus_core::error::HorusError::InitializationFailed(
                    format!("Failed to initialize gilrs: {}", e)
                )
            })?;

            Ok(Self {
                publisher: Hub::new(topic)?,
                gilrs,
            })
        }

        #[cfg(not(feature = "gilrs"))]
        {
            Ok(Self {
                publisher: Hub::new(topic)?,
                last_input_time: 0,
            })
        }
    }
}

impl Node for JoystickInputNode {
    fn name(&self) -> &'static str {
        "JoystickInputNode"
    }

    fn init(&mut self, ctx: &mut NodeInfo) -> Result<()> {
        #[cfg(feature = "gilrs")]
        {
            let connected = self.gilrs.gamepads().count();
            ctx.log_info(&format!("Joystick input node initialized - {} gamepad(s) connected", connected));
        }

        #[cfg(not(feature = "gilrs"))]
        {
            ctx.log_warn("Joystick input node in placeholder mode - build with 'gilrs' feature for real gamepad support");
        }

        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        #[cfg(feature = "gilrs")]
        {
            // Poll for gamepad events
            while let Some(Event { id, event, time: _ }) = self.gilrs.next_event() {
                let gamepad_id: u32 = usize::from(id) as u32;

                match event {
                    EventType::ButtonPressed(button, _) => {
                        let button_name = format!("{:?}", button);
                        let button_id = button_to_id(button);

                        let joystick_input = JoystickInput::new_button(
                            gamepad_id,
                            button_id,
                            button_name.clone(),
                            true,
                        );

                        self.publisher.send(joystick_input, ctx.as_deref_mut()).ok();
                        ctx.log_debug(&format!("Button pressed: {} (gamepad {})", button_name, gamepad_id));
                    }
                    EventType::ButtonReleased(button, _) => {
                        let button_name = format!("{:?}", button);
                        let button_id = button_to_id(button);

                        let joystick_input = JoystickInput::new_button(
                            gamepad_id,
                            button_id,
                            button_name,
                            false,
                        );

                        self.publisher.send(joystick_input, ctx.as_deref_mut()).ok();
                    }
                    EventType::AxisChanged(axis, value, _) => {
                        let axis_name = format!("{:?}", axis);
                        let axis_id = axis_to_id(axis);

                        let joystick_input = JoystickInput::new_axis(
                            gamepad_id,
                            axis_id,
                            axis_name.clone(),
                            value,
                        );

                        self.publisher.send(joystick_input, ctx.as_deref_mut()).ok();

                        // Only log significant axis movements to avoid spam
                        if value.abs() > 0.5 {
                            ctx.log_debug(&format!("Axis {}: {:.2} (gamepad {})", axis_name, value, gamepad_id));
                        }
                    }
                    EventType::Connected => {
                        ctx.log_info(&format!("Gamepad {} connected", gamepad_id));
                    }
                    EventType::Disconnected => {
                        ctx.log_info(&format!("Gamepad {} disconnected", gamepad_id));
                    }
                    _ => {}
                }
            }
        }

        #[cfg(not(feature = "gilrs"))]
        {
            // Placeholder implementation when gilrs feature is not enabled
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            if current_time - self.last_input_time > 3000 {
                let joystick_input = JoystickInput::new_button(
                    1,
                    0,
                    "ButtonA (placeholder)".to_string(),
                    true,
                );
                self.publisher.send(joystick_input, ctx.as_deref_mut()).ok();
                ctx.log_debug("Published placeholder joystick input");
                self.last_input_time = current_time;
            }
        }
    }
}

#[cfg(feature = "gilrs")]
fn button_to_id(button: Button) -> u32 {
    match button {
        Button::South => 0,
        Button::East => 1,
        Button::North => 2,
        Button::West => 3,
        Button::LeftTrigger => 4,
        Button::LeftTrigger2 => 5,
        Button::RightTrigger => 6,
        Button::RightTrigger2 => 7,
        Button::Select => 8,
        Button::Start => 9,
        Button::Mode => 10,
        Button::LeftThumb => 11,
        Button::RightThumb => 12,
        Button::DPadUp => 13,
        Button::DPadDown => 14,
        Button::DPadLeft => 15,
        Button::DPadRight => 16,
        _ => 255,
    }
}

#[cfg(feature = "gilrs")]
fn axis_to_id(axis: Axis) -> u32 {
    match axis {
        Axis::LeftStickX => 0,
        Axis::LeftStickY => 1,
        Axis::LeftZ => 2,
        Axis::RightStickX => 3,
        Axis::RightStickY => 4,
        Axis::RightZ => 5,
        Axis::DPadX => 6,
        Axis::DPadY => 7,
        _ => 255,
    }
}
