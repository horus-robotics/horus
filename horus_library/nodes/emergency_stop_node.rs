use crate::{EmergencyStop, SafetyStatus};
use horus_core::{Hub, Node, NodeInfo};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Emergency Stop Node - Hardware emergency stop handler for industrial safety
///
/// Monitors emergency stop buttons, software triggers, and system conditions.
/// Publishes EmergencyStop messages when triggered and maintains safety state.
pub struct EmergencyStopNode {
    publisher: Hub<EmergencyStop>,
    safety_publisher: Hub<SafetyStatus>,
    is_stopped: Arc<AtomicBool>,
    stop_reason: String,
    gpio_pin: Option<u8>,
    last_gpio_state: bool,
    auto_reset: bool,
    stop_timeout_ms: u64,
    last_stop_time: u64,
}

impl EmergencyStopNode {
    /// Create a new emergency stop node with default topic "emergency_stop"
    pub fn new() -> Self {
        Self::new_with_topic("emergency_stop")
    }

    /// Create a new emergency stop node with custom topic
    pub fn new_with_topic(topic: &str) -> Self {
        let safety_topic = format!("{}_safety", topic);
        Self {
            publisher: Hub::new(topic).expect("Failed to create emergency stop hub"),
            safety_publisher: Hub::new(&safety_topic).expect("Failed to create safety hub"),
            is_stopped: Arc::new(AtomicBool::new(false)),
            stop_reason: String::new(),
            gpio_pin: None,
            last_gpio_state: true, // Assume normal state (not pressed)
            auto_reset: false,
            stop_timeout_ms: 5000, // 5 second timeout for auto-reset
            last_stop_time: 0,
        }
    }

    /// Set GPIO pin for hardware emergency stop button (Raspberry Pi)
    pub fn set_gpio_pin(&mut self, pin: u8) {
        self.gpio_pin = Some(pin);
    }

    /// Enable or disable automatic reset after timeout
    pub fn set_auto_reset(&mut self, enabled: bool) {
        self.auto_reset = enabled;
    }

    /// Set timeout for automatic reset in milliseconds
    pub fn set_reset_timeout(&mut self, timeout_ms: u64) {
        self.stop_timeout_ms = timeout_ms;
    }

    /// Trigger emergency stop with reason
    pub fn trigger_stop(&mut self, reason: &str) {
        self.is_stopped.store(true, Ordering::Relaxed);
        self.stop_reason = reason.to_string();
        self.last_stop_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.publish_emergency_stop(true, reason);
        self.publish_safety_status();
    }

    /// Reset emergency stop (manual reset)
    pub fn reset(&mut self) {
        if self.is_stopped.load(Ordering::Relaxed) {
            self.is_stopped.store(false, Ordering::Relaxed);
            self.stop_reason.clear();
            self.publish_emergency_stop(false, "Reset");
            self.publish_safety_status();
        }
    }

    /// Check if emergency stop is active
    pub fn is_emergency_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }

    /// Get current stop reason
    pub fn get_stop_reason(&self) -> String {
        self.stop_reason.clone()
    }

    #[cfg(feature = "raspberry-pi")]
    fn check_gpio_pin(&mut self) -> bool {
        use rppal::gpio::{Gpio, Level};

        if let Some(pin_num) = self.gpio_pin {
            if let Some(gpio) = Gpio::new() {
                if let Some(pin) = gpio.get(pin_num) {
                    let input = pin.into_input_pullup();
                    let current_state = input.read() == Level::High;

                    // Emergency stop button pressed (assuming active low)
                    if self.last_gpio_state && !current_state {
                        self.last_gpio_state = current_state;
                        return true; // Emergency stop triggered
                    }

                    self.last_gpio_state = current_state;
                }
            }
        }
        false
    }

    #[cfg(not(feature = "raspberry-pi"))]
    fn check_gpio_pin(&mut self) -> bool {
        false // No GPIO support in simulation
    }

    fn check_auto_reset(&mut self) {
        if self.auto_reset && self.is_stopped.load(Ordering::Relaxed) {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            if current_time - self.last_stop_time > self.stop_timeout_ms {
                self.reset();
            }
        }
    }

    fn publish_emergency_stop(&self, is_emergency: bool, reason: &str) {
        let emergency_stop = if is_emergency {
            EmergencyStop::engage(reason)
        } else {
            EmergencyStop::release()
        };
        let _ = self.publisher.send(emergency_stop, None);
    }

    fn publish_safety_status(&self) {
        let status = if self.is_stopped.load(Ordering::Relaxed) {
            {
                let mut status = SafetyStatus::new();
                status.estop_engaged = true;
                status.mode = SafetyStatus::MODE_SAFE_STOP;
                status.set_fault(1); // Emergency stop fault code
                status
            }
        } else {
            SafetyStatus::new()
        };
        let _ = self.safety_publisher.send(status, None);
    }
}

impl Node for EmergencyStopNode {
    fn name(&self) -> &'static str {
        "EmergencyStopNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Check GPIO pin for hardware button
        if self.check_gpio_pin() {
            self.trigger_stop("Hardware emergency stop button pressed");
        }

        // Check for auto-reset timeout
        self.check_auto_reset();

        // Periodically publish safety status
        self.publish_safety_status();
    }
}

impl Default for EmergencyStopNode {
    fn default() -> Self {
        Self::new()
    }
}
