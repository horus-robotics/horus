//! Tank Controller Node - Converts keyboard input to tank commands
//!
//! Subscribes to keyboard input and publishes CmdVel for a single tank.

use horus_core::{Hub, Node, NodeInfo};
use horus_library::messages::{CmdVel, KeyboardInput};

pub struct TankControllerNode {
    keyboard_sub: Hub<KeyboardInput>,
    cmd_vel_pub: Hub<CmdVel>,
    tank_id: String,

    // Tank control parameters
    max_linear_speed: f32,
    max_angular_speed: f32,
    acceleration: f32,

    // Current movement state
    current_linear: f32,
    current_angular: f32,
}

impl TankControllerNode {
    /// Create new tank controller with default topics
    pub fn new() -> Result<Self, String> {
        Self::new_with_topics("keyboard_input", "/tank/tank_1/cmd_vel", "tank_1")
    }

    /// Create with custom topic names
    pub fn new_with_topics(
        keyboard_topic: &str,
        cmd_vel_topic: &str,
        tank_id: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            keyboard_sub: Hub::new(keyboard_topic)
                .map_err(|e| format!("Failed to create keyboard subscriber: {}", e))?,
            cmd_vel_pub: Hub::new(cmd_vel_topic)
                .map_err(|e| format!("Failed to create cmd_vel publisher: {}", e))?,
            tank_id: tank_id.to_string(),
            max_linear_speed: 3.0,  // 3 m/s max
            max_angular_speed: 2.0, // 2 rad/s max
            acceleration: 0.1,      // Smooth acceleration
            current_linear: 0.0,
            current_angular: 0.0,
        })
    }

    /// Set tank control parameters
    pub fn set_parameters(&mut self, max_linear: f32, max_angular: f32, accel: f32) {
        self.max_linear_speed = max_linear;
        self.max_angular_speed = max_angular;
        self.acceleration = accel;
    }

    /// Process keyboard input and update tank commands
    fn process_keyboard(&mut self, key: &KeyboardInput) {
        // Target velocities based on keyboard
        let mut target_linear = 0.0;
        let mut target_angular = 0.0;

        // WASD or Arrow Keys
        let key_name = key.get_key_name();
        match key_name.as_str() {
            "w" | "W" | "ArrowUp" => target_linear = self.max_linear_speed,
            "s" | "S" | "ArrowDown" => target_linear = -self.max_linear_speed * 0.7, // Slower reverse
            "a" | "A" | "ArrowLeft" => target_angular = self.max_angular_speed,
            "d" | "D" | "ArrowRight" => target_angular = -self.max_angular_speed,
            _ => {}
        }

        // Smooth acceleration
        if (target_linear - self.current_linear).abs() > 0.01 {
            if target_linear > self.current_linear {
                self.current_linear += self.acceleration;
            } else {
                self.current_linear -= self.acceleration;
            }
            self.current_linear = self
                .current_linear
                .clamp(-self.max_linear_speed, self.max_linear_speed);
        }

        if (target_angular - self.current_angular).abs() > 0.01 {
            if target_angular > self.current_angular {
                self.current_angular += self.acceleration;
            } else {
                self.current_angular -= self.acceleration;
            }
            self.current_angular = self
                .current_angular
                .clamp(-self.max_angular_speed, self.max_angular_speed);
        }

        // If no input, gradually stop
        if target_linear == 0.0 {
            self.current_linear *= 0.9; // Damping
            if self.current_linear.abs() < 0.01 {
                self.current_linear = 0.0;
            }
        }

        if target_angular == 0.0 {
            self.current_angular *= 0.9;
            if self.current_angular.abs() < 0.01 {
                self.current_angular = 0.0;
            }
        }
    }
}

impl Node for TankControllerNode {
    fn name(&self) -> &'static str {
        "TankControllerNode"
    }

    fn init(&mut self, ctx: &mut NodeInfo) -> Result<(), String> {
        ctx.log_info(&format!(
            "TankControllerNode initialized - controlling {}",
            self.tank_id
        ));
        Ok(())
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Process keyboard input
        while let Some(key) = self.keyboard_sub.recv(ctx.as_deref_mut()) {
            if !key.pressed {
                continue; // Only handle key press, not release
            }

            self.process_keyboard(&key);
        }

        // Publish command
        let cmd = CmdVel {
            stamp_nanos: 0, // Will be set by Hub
            linear: self.current_linear,
            angular: self.current_angular,
        };

        let _ = self.cmd_vel_pub.send(cmd, ctx);
    }

    fn shutdown(&mut self, ctx: &mut NodeInfo) -> Result<(), String> {
        ctx.log_info("TankControllerNode shutting down");
        Ok(())
    }
}
