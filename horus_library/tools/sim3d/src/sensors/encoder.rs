use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

/// Rotary encoder sensor for joint position/velocity measurement
#[derive(Component, Clone)]
pub struct Encoder {
    pub rate_hz: f32,
    pub last_update: f32,

    // Encoder resolution (ticks per revolution for rotary, ticks per meter for linear)
    pub resolution: u32,

    // Noise parameters
    pub position_noise_std: f32, // In radians or meters
    pub velocity_noise_std: f32,

    // Quantization (simulates discrete encoder ticks)
    pub enable_quantization: bool,

    // Encoder type
    pub encoder_type: EncoderType,

    // Internal state
    last_position: f32,
    last_velocity: f32,
    tick_count: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EncoderType {
    Rotary,  // For revolute joints (radians)
    Linear,  // For prismatic joints (meters)
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            rate_hz: 1000.0, // High rate for encoders
            last_update: 0.0,
            resolution: 4096, // 12-bit encoder
            position_noise_std: 0.0001, // Very low noise
            velocity_noise_std: 0.001,
            enable_quantization: true,
            encoder_type: EncoderType::Rotary,
            last_position: 0.0,
            last_velocity: 0.0,
            tick_count: 0,
        }
    }
}

impl Encoder {
    pub fn new(rate_hz: f32, resolution: u32, encoder_type: EncoderType) -> Self {
        Self {
            rate_hz,
            resolution,
            encoder_type,
            ..default()
        }
    }

    /// High-resolution encoder (optical, magnetic)
    pub fn high_resolution(encoder_type: EncoderType) -> Self {
        Self {
            resolution: 16384, // 14-bit
            position_noise_std: 0.00005,
            velocity_noise_std: 0.0005,
            encoder_type,
            ..default()
        }
    }

    /// Standard encoder
    pub fn standard(encoder_type: EncoderType) -> Self {
        Self {
            resolution: 4096, // 12-bit
            position_noise_std: 0.0001,
            velocity_noise_std: 0.001,
            encoder_type,
            ..default()
        }
    }

    /// Low-resolution encoder (potentiometer, hall effect)
    pub fn low_resolution(encoder_type: EncoderType) -> Self {
        Self {
            resolution: 1024, // 10-bit
            position_noise_std: 0.001,
            velocity_noise_std: 0.01,
            encoder_type,
            ..default()
        }
    }

    pub fn with_noise(mut self, position_noise: f32, velocity_noise: f32) -> Self {
        self.position_noise_std = position_noise;
        self.velocity_noise_std = velocity_noise;
        self
    }

    pub fn with_quantization(mut self, enable: bool) -> Self {
        self.enable_quantization = enable;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn update_time(&mut self, current_time: f32) {
        self.last_update = current_time;
    }

    /// Get encoder resolution in radians/tick or meters/tick
    pub fn resolution_per_tick(&self) -> f32 {
        match self.encoder_type {
            EncoderType::Rotary => (2.0 * PI) / self.resolution as f32,
            EncoderType::Linear => 0.001 / self.resolution as f32, // Assume 1mm per full cycle
        }
    }

    /// Quantize position to encoder ticks
    pub fn quantize(&self, position: f32) -> f32 {
        if !self.enable_quantization {
            return position;
        }

        let ticks = (position / self.resolution_per_tick()).round();
        ticks * self.resolution_per_tick()
    }

    /// Update tick count (for absolute encoders)
    pub fn update_ticks(&mut self, position: f32) {
        self.tick_count = (position / self.resolution_per_tick()).round() as i64;
    }

    /// Get current tick count
    pub fn ticks(&self) -> i64 {
        self.tick_count
    }
}

/// Encoder data output
#[derive(Component, Clone, Debug, Default)]
pub struct EncoderData {
    pub timestamp: f32,

    // Position (radians for rotary, meters for linear)
    pub position: f32,

    // Velocity (rad/s for rotary, m/s for linear)
    pub velocity: f32,

    // Encoder ticks (absolute or incremental)
    pub ticks: i64,

    // Data validity
    pub valid: bool,
}

impl EncoderData {
    pub fn new() -> Self {
        Self::default()
    }
}

/// System to update encoder sensors from joint states
pub fn encoder_update_system(
    time: Res<Time>,
    mut query: Query<(&mut Encoder, &mut EncoderData, &crate::robot::state::JointState)>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    for (mut encoder, mut encoder_data, joint_state) in query.iter_mut() {
        if !encoder.should_update(current_time) {
            continue;
        }

        encoder.update_time(current_time);

        // Get true position and velocity from joint
        let true_position = joint_state.position;
        let true_velocity = joint_state.velocity;

        // Add noise
        let mut rng = rand::thread_rng();
        let position_noise = rng.gen_range(-encoder.position_noise_std..encoder.position_noise_std);
        let velocity_noise = rng.gen_range(-encoder.velocity_noise_std..encoder.velocity_noise_std);

        // Apply quantization
        let measured_position = encoder.quantize(true_position + position_noise);

        // Compute velocity (with noise and optional filtering)
        let measured_velocity = if dt > 0.0 {
            // Could use filtered derivative here
            (measured_position - encoder.last_position) / dt + velocity_noise
        } else {
            true_velocity + velocity_noise
        };

        // Update encoder data
        encoder_data.position = measured_position;
        encoder_data.velocity = measured_velocity;
        encoder_data.timestamp = current_time;
        encoder_data.valid = true;

        // Update tick count
        encoder.update_ticks(measured_position);
        encoder_data.ticks = encoder.ticks();

        // Store for next iteration
        encoder.last_position = measured_position;
        encoder.last_velocity = measured_velocity;
    }
}

/// Incremental encoder (relative position tracking)
#[derive(Component, Clone)]
pub struct IncrementalEncoder {
    pub encoder: Encoder,
    pub has_index: bool, // Z-channel for reference position
    pub index_position: f32,
    pub index_found: bool,
}

impl IncrementalEncoder {
    pub fn new(resolution: u32, encoder_type: EncoderType) -> Self {
        Self {
            encoder: Encoder::new(1000.0, resolution, encoder_type),
            has_index: true,
            index_position: 0.0,
            index_found: false,
        }
    }

    pub fn with_index(mut self, index_position: f32) -> Self {
        self.has_index = true;
        self.index_position = index_position;
        self
    }

    /// Check if passing through index position
    pub fn check_index(&mut self, current_position: f32, last_position: f32) -> bool {
        if !self.has_index || self.index_found {
            return false;
        }

        // Simple check for crossing index position
        let crossed = (last_position < self.index_position && current_position >= self.index_position)
            || (last_position > self.index_position && current_position <= self.index_position);

        if crossed {
            self.index_found = true;
            return true;
        }

        false
    }
}

/// Absolute encoder (maintains position through power cycles)
#[derive(Component, Clone)]
pub struct AbsoluteEncoder {
    pub encoder: Encoder,
    pub multi_turn: bool, // Track multiple rotations
    pub turns: i32,       // Number of complete rotations
}

impl AbsoluteEncoder {
    pub fn new(resolution: u32, encoder_type: EncoderType) -> Self {
        Self {
            encoder: Encoder::new(1000.0, resolution, encoder_type),
            multi_turn: true,
            turns: 0,
        }
    }

    pub fn single_turn() -> Self {
        Self {
            encoder: Encoder::high_resolution(EncoderType::Rotary),
            multi_turn: false,
            turns: 0,
        }
    }

    pub fn multi_turn() -> Self {
        Self {
            encoder: Encoder::high_resolution(EncoderType::Rotary),
            multi_turn: true,
            turns: 0,
        }
    }

    /// Update turn count
    pub fn update_turns(&mut self, current_position: f32, last_position: f32) {
        if !self.multi_turn {
            return;
        }

        // Calculate the angular change
        let delta = current_position - last_position;

        // Detect full rotation: if we moved more than π radians in either direction,
        // we must have wrapped around the circle
        // Use small epsilon to handle boundary cases
        const EPSILON: f32 = 1e-5;

        if delta >= PI - EPSILON {
            // Wrapped backward (e.g., from 0.1 to 2π-0.1)
            self.turns -= 1;
        } else if delta <= -PI + EPSILON {
            // Wrapped forward (e.g., from 2π-0.1 to 0.1)
            self.turns += 1;
        }
    }

    /// Get absolute position including turns
    pub fn absolute_position(&self, position: f32) -> f32 {
        if self.multi_turn {
            position + (self.turns as f32 * 2.0 * PI)
        } else {
            position % (2.0 * PI)
        }
    }
}

/// Quadrature encoder (A/B channels)
#[derive(Component, Clone)]
pub struct QuadratureEncoder {
    pub encoder: Encoder,
    pub last_phase_a: bool,
    pub last_phase_b: bool,
}

impl QuadratureEncoder {
    pub fn new(resolution: u32, encoder_type: EncoderType) -> Self {
        Self {
            encoder: Encoder::new(1000.0, resolution, encoder_type),
            last_phase_a: false,
            last_phase_b: false,
        }
    }

    /// Decode quadrature signals to determine direction
    pub fn decode(&mut self, phase_a: bool, phase_b: bool) -> i32 {
        let direction = match (self.last_phase_a, self.last_phase_b, phase_a, phase_b) {
            (false, false, false, true) => 1,
            (false, true, true, true) => 1,
            (true, true, true, false) => 1,
            (true, false, false, false) => 1,
            (false, true, false, false) => -1,
            (true, true, false, true) => -1,
            (true, false, true, true) => -1,
            (false, false, true, false) => -1,
            _ => 0,
        };

        self.last_phase_a = phase_a;
        self.last_phase_b = phase_b;

        direction
    }

    /// Generate quadrature signals from position
    pub fn generate_signals(&self, position: f32) -> (bool, bool) {
        let angle = position % (2.0 * PI);
        let ticks_per_rev = self.encoder.resolution as f32 * 4.0; // x4 encoding
        let tick_angle = (2.0 * PI) / ticks_per_rev;

        let phase = (angle / tick_angle) as i32;
        let phase_a = (phase % 4) < 2;
        let phase_b = ((phase + 1) % 4) < 2;

        (phase_a, phase_b)
    }
}

/// Encoder calibration data
#[derive(Component, Clone)]
pub struct EncoderCalibration {
    pub offset: f32,
    pub scale: f32,
    pub lookup_table: Option<Vec<f32>>, // For non-linear encoders
}

impl Default for EncoderCalibration {
    fn default() -> Self {
        Self {
            offset: 0.0,
            scale: 1.0,
            lookup_table: None,
        }
    }
}

impl EncoderCalibration {
    pub fn new(offset: f32, scale: f32) -> Self {
        Self {
            offset,
            scale,
            lookup_table: None,
        }
    }

    pub fn apply(&self, raw_position: f32) -> f32 {
        let calibrated = (raw_position - self.offset) * self.scale;

        if let Some(lut) = &self.lookup_table {
            // Apply lookup table correction
            self.interpolate_lut(calibrated, lut)
        } else {
            calibrated
        }
    }

    fn interpolate_lut(&self, position: f32, lut: &[f32]) -> f32 {
        if lut.is_empty() {
            return position;
        }

        let index = position.abs() as usize % lut.len();
        lut[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_quantization() {
        let encoder = Encoder::standard(EncoderType::Rotary);

        let position = 1.5;
        let quantized = encoder.quantize(position);

        // Should be quantized to nearest tick
        let ticks = (quantized / encoder.resolution_per_tick()).round();
        assert_eq!(ticks * encoder.resolution_per_tick(), quantized);
    }

    #[test]
    fn test_encoder_resolution() {
        let encoder = Encoder::new(1000.0, 4096, EncoderType::Rotary);

        let res = encoder.resolution_per_tick();
        assert!((res - (2.0 * PI / 4096.0)).abs() < 1e-6);
    }

    #[test]
    fn test_incremental_encoder_index() {
        let mut encoder = IncrementalEncoder::new(4096, EncoderType::Rotary)
            .with_index(0.0);

        assert!(!encoder.index_found);

        let crossed = encoder.check_index(0.1, -0.1);
        assert!(crossed);
        assert!(encoder.index_found);
    }

    #[test]
    fn test_absolute_encoder_turns() {
        let mut encoder = AbsoluteEncoder::multi_turn();

        // Test forward wrap: from high angle (PI+0.1) to low angle (0.1)
        // This means we wrapped around 0, incrementing the turn count
        encoder.update_turns(0.1, PI + 0.1);
        assert_eq!(encoder.turns, 1);

        // Test another forward wrap to increment again
        encoder.update_turns(0.2, PI + 0.2);
        assert_eq!(encoder.turns, 2);

        // Test backward wrap: from low angle (0.1) to high angle (PI+0.1)
        // This means we unwrapped, decrementing the turn count
        encoder.update_turns(PI + 0.1, 0.1);
        assert_eq!(encoder.turns, 1);
    }
}
