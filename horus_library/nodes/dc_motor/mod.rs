use crate::PwmCommand;
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo, NodeInfoExt};
use std::time::{SystemTime, UNIX_EPOCH};

/// DC Motor Controller Node - PWM-based DC motor control
///
/// Controls DC motors using PWM signals for speed and direction control.
/// Compatible with common motor drivers (L298N, TB6612, DRV8833, etc.).
/// Supports multiple motor channels with independent control.
pub struct DcMotorNode {
    subscriber: Hub<PwmCommand>,
    publisher: Hub<PwmCommand>, // Echo commands for monitoring

    // Configuration
    num_channels: u8,
    max_duty_cycle: f32,  // Limit maximum speed (0.0-1.0)
    min_duty_cycle: f32,  // Dead zone compensation
    pwm_frequency: u32,   // PWM frequency in Hz
    invert_channels: u8,  // Bitfield for channel inversion
    enable_feedback: bool, // Publish motor feedback

    // State tracking per channel (up to 8 channels)
    current_duty_cycles: [f32; 8],
    current_enabled: [bool; 8],
    last_command_time: [u64; 8],
    command_timeout_ms: u64, // Auto-stop after timeout
}

impl DcMotorNode {
    /// Create a new DC motor node with default topic "motor_cmd"
    pub fn new() -> Result<Self> {
        Self::new_with_topic("motor_cmd")
    }

    /// Create a new DC motor node with custom topic
    pub fn new_with_topic(topic: &str) -> Result<Self> {
        Ok(Self {
            subscriber: Hub::new(topic)?,
            publisher: Hub::new(&format!("{}_feedback", topic))?,
            num_channels: 2, // Default to 2 motors (typical robot)
            max_duty_cycle: 1.0,
            min_duty_cycle: 0.0,
            pwm_frequency: 10000, // 10kHz default
            invert_channels: 0,
            enable_feedback: true,
            current_duty_cycles: [0.0; 8],
            current_enabled: [false; 8],
            last_command_time: [0; 8],
            command_timeout_ms: 1000, // 1 second timeout
        })
    }

    /// Set the number of motor channels (1-8)
    pub fn set_num_channels(&mut self, channels: u8) {
        self.num_channels = channels.clamp(1, 8);
    }

    /// Set duty cycle limits (for safety or motor protection)
    pub fn set_duty_cycle_limits(&mut self, min: f32, max: f32) {
        self.min_duty_cycle = min.clamp(0.0, 1.0);
        self.max_duty_cycle = max.clamp(0.0, 1.0);
    }

    /// Set PWM frequency in Hz (typical range: 1kHz-20kHz)
    pub fn set_pwm_frequency(&mut self, frequency: u32) {
        self.pwm_frequency = frequency;
    }

    /// Invert a specific channel (swap forward/reverse)
    pub fn set_channel_inverted(&mut self, channel: u8, inverted: bool) {
        if channel < 8 {
            if inverted {
                self.invert_channels |= 1 << channel;
            } else {
                self.invert_channels &= !(1 << channel);
            }
        }
    }

    /// Set command timeout in milliseconds (0 = disable)
    pub fn set_command_timeout(&mut self, timeout_ms: u64) {
        self.command_timeout_ms = timeout_ms;
    }

    /// Enable/disable motor feedback publishing
    pub fn set_feedback_enabled(&mut self, enabled: bool) {
        self.enable_feedback = enabled;
    }

    /// Get current duty cycle for a channel
    pub fn get_duty_cycle(&self, channel: u8) -> Option<f32> {
        if channel < 8 {
            Some(self.current_duty_cycles[channel as usize])
        } else {
            None
        }
    }

    /// Check if a channel is enabled
    pub fn is_channel_enabled(&self, channel: u8) -> bool {
        if channel < 8 {
            self.current_enabled[channel as usize]
        } else {
            false
        }
    }

    /// Process a motor command
    fn process_command(&mut self, mut cmd: PwmCommand, mut ctx: Option<&mut NodeInfo>) {
        let channel = cmd.channel_id;

        // Validate channel
        if channel >= self.num_channels {
            ctx.log_warning(&format!("Invalid channel ID: {}", channel));
            return;
        }

        // Apply channel inversion
        if (self.invert_channels & (1 << channel)) != 0 {
            cmd.duty_cycle = -cmd.duty_cycle;
        }

        // Apply duty cycle limits
        let duty = cmd.duty_cycle.abs();
        let limited_duty = if duty < self.min_duty_cycle {
            0.0 // Below minimum, treat as stopped
        } else {
            duty.clamp(self.min_duty_cycle, self.max_duty_cycle)
        };

        cmd.duty_cycle = if cmd.duty_cycle >= 0.0 {
            limited_duty
        } else {
            -limited_duty
        };

        // Update state
        let idx = channel as usize;
        self.current_duty_cycles[idx] = cmd.duty_cycle;
        self.current_enabled[idx] = cmd.enable;
        self.last_command_time[idx] = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // In real implementation, this would set actual PWM hardware
        // For now, just log the command
        ctx.log_debug(&format!(
            "Motor {}: duty={:.1}%, enable={}, freq={}Hz",
            channel,
            cmd.duty_cycle * 100.0,
            cmd.enable,
            cmd.frequency
        ));

        // Publish feedback if enabled
        if self.enable_feedback {
            let _ = self.publisher.send(cmd, None);
        }
    }

    /// Check for command timeouts and stop motors if needed
    fn check_timeouts(&mut self, mut ctx: Option<&mut NodeInfo>) {
        if self.command_timeout_ms == 0 {
            return; // Timeout disabled
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        for channel in 0..self.num_channels {
            let idx = channel as usize;
            if self.current_enabled[idx] {
                let elapsed = current_time - self.last_command_time[idx];
                if elapsed > self.command_timeout_ms {
                    // Timeout - stop motor
                    self.current_duty_cycles[idx] = 0.0;
                    self.current_enabled[idx] = false;

                    ctx.log_warning(&format!(
                        "Motor {} stopped due to command timeout ({}ms)",
                        channel, elapsed
                    ));

                    // Publish stop command
                    if self.enable_feedback {
                        let stop_cmd = PwmCommand::coast(channel);
                        let _ = self.publisher.send(stop_cmd, None);
                    }
                }
            }
        }
    }

    /// Emergency stop all motors
    pub fn emergency_stop(&mut self) {
        for channel in 0..self.num_channels {
            let idx = channel as usize;
            self.current_duty_cycles[idx] = 0.0;
            self.current_enabled[idx] = false;

            // Publish stop command
            if self.enable_feedback {
                let stop_cmd = PwmCommand::brake(channel);
                let _ = self.publisher.send(stop_cmd, None);
            }
        }
    }
}

impl Node for DcMotorNode {
    fn name(&self) -> &'static str {
        "DcMotorNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Process all pending commands
        while let Some(cmd) = self.subscriber.recv(None) {
            self.process_command(cmd, ctx.as_deref_mut());
        }

        // Check for command timeouts
        self.check_timeouts(ctx.as_deref_mut());
    }
}
