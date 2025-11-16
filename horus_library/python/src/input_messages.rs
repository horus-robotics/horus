// Python wrappers for input messages
use horus_library::messages::{joystick_msg, keyboard_input_msg};
use pyo3::prelude::*;

/// Python wrapper for JoystickInput
#[pyclass(module = "horus.library._library", name = "JoystickInput")]
#[derive(Clone)]
pub struct PyJoystickInput {
    pub(crate) inner: joystick_msg::JoystickInput,
}

#[pymethods]
impl PyJoystickInput {
    #[new]
    fn new() -> Self {
        Self {
            inner: joystick_msg::JoystickInput::new_button(0, 0, "".to_string(), false),
        }
    }

    #[staticmethod]
    fn new_button(joystick_id: u32, button_id: u32, button_name: String, pressed: bool) -> Self {
        Self {
            inner: joystick_msg::JoystickInput::new_button(
                joystick_id,
                button_id,
                button_name,
                pressed,
            ),
        }
    }

    #[staticmethod]
    fn new_axis(joystick_id: u32, axis_id: u32, axis_name: String, value: f32) -> Self {
        Self {
            inner: joystick_msg::JoystickInput::new_axis(joystick_id, axis_id, axis_name, value),
        }
    }

    #[staticmethod]
    fn new_hat(joystick_id: u32, hat_id: u32, hat_name: String, value: f32) -> Self {
        Self {
            inner: joystick_msg::JoystickInput::new_hat(joystick_id, hat_id, hat_name, value),
        }
    }

    #[staticmethod]
    fn new_connection(joystick_id: u32, connected: bool) -> Self {
        Self {
            inner: joystick_msg::JoystickInput::new_connection(joystick_id, connected),
        }
    }

    #[getter]
    fn joystick_id(&self) -> u32 {
        self.inner.joystick_id
    }

    #[getter]
    fn element_id(&self) -> u32 {
        self.inner.element_id
    }

    #[getter]
    fn value(&self) -> f32 {
        self.inner.value
    }

    #[getter]
    fn pressed(&self) -> bool {
        self.inner.pressed
    }

    fn event_type(&self) -> String {
        self.inner.get_event_type()
    }

    fn element_name(&self) -> String {
        self.inner.get_element_name()
    }

    fn is_button(&self) -> bool {
        self.inner.is_button()
    }

    fn is_axis(&self) -> bool {
        self.inner.is_axis()
    }

    fn is_hat(&self) -> bool {
        self.inner.is_hat()
    }

    fn is_connection_event(&self) -> bool {
        self.inner.is_connection_event()
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    fn __repr__(&self) -> String {
        format!(
            "JoystickInput(id={}, type='{}', element='{}', value={:.2})",
            self.inner.joystick_id,
            self.inner.get_event_type(),
            self.inner.get_element_name(),
            self.inner.value
        )
    }
}

/// Python wrapper for KeyboardInput
#[pyclass(module = "horus.library._library", name = "KeyboardInput")]
#[derive(Clone)]
pub struct PyKeyboardInput {
    pub(crate) inner: keyboard_input_msg::KeyboardInput,
}

#[pymethods]
impl PyKeyboardInput {
    #[new]
    #[pyo3(signature = (key="", code=0, modifiers=vec![], pressed=true))]
    fn new(key: &str, code: u32, modifiers: Vec<String>, pressed: bool) -> Self {
        Self {
            inner: keyboard_input_msg::KeyboardInput::new(key.to_string(), code, modifiers, pressed),
        }
    }

    #[getter]
    fn code(&self) -> u32 {
        self.inner.code
    }

    #[getter]
    fn pressed(&self) -> bool {
        self.inner.pressed
    }

    fn key_name(&self) -> String {
        self.inner.get_key_name()
    }

    fn modifiers(&self) -> Vec<String> {
        self.inner.get_modifiers()
    }

    fn has_modifier(&self, modifier: &str) -> bool {
        self.inner.has_modifier(modifier)
    }

    fn is_ctrl(&self) -> bool {
        self.inner.is_ctrl()
    }

    fn is_shift(&self) -> bool {
        self.inner.is_shift()
    }

    fn is_alt(&self) -> bool {
        self.inner.is_alt()
    }

    fn __repr__(&self) -> String {
        let modifiers = self.inner.get_modifiers();
        let mod_str = if modifiers.is_empty() {
            String::new()
        } else {
            format!(" +{}", modifiers.join("+"))
        };
        format!(
            "KeyboardInput(key='{}'{}, {})",
            self.inner.get_key_name(),
            mod_str,
            if self.inner.pressed {
                "pressed"
            } else {
                "released"
            }
        )
    }
}
