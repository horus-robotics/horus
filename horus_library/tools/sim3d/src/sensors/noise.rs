use bevy::prelude::*;
use rand::Rng;
use rand::rngs::ThreadRng;
use rand_distr::{Distribution, Normal, Uniform};

/// Trait for noise models that can be applied to sensor data
/// Uses ThreadRng internally for simplicity
pub trait NoiseModel: Send + Sync {
    /// Apply noise to a single value
    fn apply(&self, value: f32, rng: &mut ThreadRng) -> f32;

    /// Apply noise to a vector
    fn apply_vec3(&self, value: Vec3, rng: &mut ThreadRng) -> Vec3 {
        Vec3::new(
            self.apply(value.x, rng),
            self.apply(value.y, rng),
            self.apply(value.z, rng),
        )
    }
}

/// Gaussian (normal) noise model
#[derive(Clone, Debug)]
pub struct GaussianNoise {
    pub mean: f32,
    pub std_dev: f32,
}

impl GaussianNoise {
    pub fn new(mean: f32, std_dev: f32) -> Self {
        Self { mean, std_dev }
    }

    pub fn zero_mean(std_dev: f32) -> Self {
        Self::new(0.0, std_dev)
    }
}

impl NoiseModel for GaussianNoise {
    fn apply(&self, value: f32, rng: &mut ThreadRng) -> f32 {
        if self.std_dev == 0.0 {
            return value;
        }

        let normal = Normal::new(self.mean, self.std_dev).unwrap();
        value + normal.sample(rng)
    }
}

/// Uniform noise model
#[derive(Clone, Debug)]
pub struct UniformNoise {
    pub min: f32,
    pub max: f32,
}

impl UniformNoise {
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    pub fn symmetric(range: f32) -> Self {
        Self::new(-range, range)
    }
}

impl NoiseModel for UniformNoise {
    fn apply(&self, value: f32, rng: &mut ThreadRng) -> f32 {
        let uniform = Uniform::new(self.min, self.max);
        value + uniform.sample(rng)
    }
}

/// Salt-and-pepper noise (random outliers)
#[derive(Clone, Debug)]
pub struct SaltPepperNoise {
    pub probability: f32,
    pub salt_value: f32,
    pub pepper_value: f32,
}

impl SaltPepperNoise {
    pub fn new(probability: f32, salt_value: f32, pepper_value: f32) -> Self {
        Self {
            probability,
            salt_value,
            pepper_value,
        }
    }
}

impl NoiseModel for SaltPepperNoise {
    fn apply(&self, value: f32, rng: &mut ThreadRng) -> f32 {
        let rand_val: f32 = rng.gen();

        if rand_val < self.probability / 2.0 {
            self.pepper_value
        } else if rand_val < self.probability {
            self.salt_value
        } else {
            value
        }
    }
}

/// Perlin-style drift noise (smooth, correlated noise)
#[derive(Clone, Debug)]
pub struct DriftNoise {
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
}

impl DriftNoise {
    pub fn new(amplitude: f32, frequency: f32) -> Self {
        Self {
            amplitude,
            frequency,
            phase: 0.0,
        }
    }

    /// Update internal state (call once per frame)
    pub fn update(&mut self, delta_time: f32, rng: &mut ThreadRng) {
        self.phase += delta_time * self.frequency;

        // Add random walk
        let random_drift: f32 = rng.gen_range(-0.01..0.01);
        self.phase += random_drift;
    }
}

impl NoiseModel for DriftNoise {
    fn apply(&self, value: f32, _rng: &mut ThreadRng) -> f32 {
        value + self.amplitude * self.phase.sin()
    }
}

/// Combined noise model (applies multiple noise sources)
pub struct CombinedNoise {
    pub models: Vec<Box<dyn NoiseModel>>,
}

impl CombinedNoise {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
        }
    }

    pub fn add_model(mut self, model: Box<dyn NoiseModel>) -> Self {
        self.models.push(model);
        self
    }
}

impl Default for CombinedNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseModel for CombinedNoise {
    fn apply(&self, value: f32, rng: &mut ThreadRng) -> f32 {
        let mut result = value;
        for model in &self.models {
            result = model.apply(result, rng);
        }
        result
    }
}

/// Helper functions for common sensor noise patterns
pub mod patterns {
    use super::*;

    /// IMU accelerometer noise (typical specs)
    pub fn imu_accel_noise(std_dev: f32) -> GaussianNoise {
        GaussianNoise::zero_mean(std_dev)
    }

    /// IMU gyroscope noise (typical specs)
    pub fn imu_gyro_noise(std_dev: f32) -> GaussianNoise {
        GaussianNoise::zero_mean(std_dev)
    }

    /// GPS horizontal position noise (typical ~2-5m)
    pub fn gps_horizontal_noise(std_dev: f32) -> GaussianNoise {
        GaussianNoise::zero_mean(std_dev)
    }

    /// GPS vertical position noise (typically worse than horizontal)
    pub fn gps_vertical_noise(std_dev: f32) -> GaussianNoise {
        GaussianNoise::zero_mean(std_dev)
    }

    /// Lidar measurement noise (typical ~2cm std dev)
    pub fn lidar_measurement_noise(std_dev: f32) -> GaussianNoise {
        GaussianNoise::zero_mean(std_dev)
    }

    /// Camera depth noise (increases with distance)
    pub fn depth_camera_noise(base_std_dev: f32, distance: f32) -> GaussianNoise {
        // Noise increases quadratically with distance
        let std_dev = base_std_dev * (1.0 + 0.01 * distance.powi(2));
        GaussianNoise::zero_mean(std_dev)
    }

    /// Encoder noise (quantization + measurement noise)
    pub fn encoder_noise(resolution: u32) -> CombinedNoise {
        let quantization = 1.0 / resolution as f32;
        CombinedNoise::new()
            .add_model(Box::new(UniformNoise::symmetric(quantization / 2.0)))
            .add_model(Box::new(GaussianNoise::zero_mean(quantization * 0.1)))
    }
}

/// Component to add noise to sensor readings
#[derive(Component)]
pub struct SensorNoise {
    pub model: Box<dyn NoiseModel>,
    pub enabled: bool,
}

impl SensorNoise {
    pub fn new(model: Box<dyn NoiseModel>) -> Self {
        Self {
            model,
            enabled: true,
        }
    }

    pub fn apply(&self, value: f32, rng: &mut ThreadRng) -> f32 {
        if self.enabled {
            self.model.apply(value, rng)
        } else {
            value
        }
    }

    pub fn apply_vec3(&self, value: Vec3, rng: &mut ThreadRng) -> Vec3 {
        if self.enabled {
            self.model.apply_vec3(value, rng)
        } else {
            value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn test_gaussian_noise() {
        let mut rng = thread_rng();
        let noise = GaussianNoise::zero_mean(0.1);

        let value = 5.0;
        let noisy = noise.apply(value, &mut rng);

        // Should be close but not identical
        assert!((noisy - value).abs() < 1.0);
    }

    #[test]
    fn test_uniform_noise() {
        let mut rng = thread_rng();
        let noise = UniformNoise::symmetric(0.5);

        let value = 5.0;
        let noisy = noise.apply(value, &mut rng);

        assert!(noisy >= 4.5 && noisy <= 5.5);
    }

    #[test]
    fn test_combined_noise() {
        let mut rng = thread_rng();
        let noise = CombinedNoise::new()
            .add_model(Box::new(GaussianNoise::zero_mean(0.1)))
            .add_model(Box::new(UniformNoise::symmetric(0.05)));

        let value = 5.0;
        let noisy = noise.apply(value, &mut rng);

        assert!((noisy - value).abs() < 1.0);
    }

    #[test]
    fn test_drift_noise() {
        let mut rng = thread_rng();
        let mut noise = DriftNoise::new(0.5, 1.0);

        let value = 5.0;
        let noisy1 = noise.apply(value, &mut rng);

        noise.update(0.1, &mut rng);
        let noisy2 = noise.apply(value, &mut rng);

        // Values should be different after update
        assert_ne!(noisy1, noisy2);
    }

    #[test]
    fn test_sensor_noise_component() {
        let mut rng = thread_rng();
        let noise = SensorNoise::new(Box::new(GaussianNoise::zero_mean(0.1)));

        let value = Vec3::new(1.0, 2.0, 3.0);
        let noisy = noise.apply_vec3(value, &mut rng);

        assert!((noisy - value).length() < 1.0);
    }

    #[test]
    fn test_patterns() {
        let _imu_noise = patterns::imu_accel_noise(0.01);
        let _gps_noise = patterns::gps_horizontal_noise(2.0);
        let _lidar_noise = patterns::lidar_measurement_noise(0.02);
        let _encoder_noise = patterns::encoder_noise(1024);

        // Just ensure they can be created without panic
    }
}
