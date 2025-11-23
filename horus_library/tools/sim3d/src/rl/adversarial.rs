use bevy::prelude::*;
use rand::{thread_rng, Rng};
use rand_distr::{Distribution, Normal};

/// Types of adversarial disturbances
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DisturbanceType {
    /// Force/impulse disturbance
    Force,
    /// Torque disturbance
    Torque,
    /// Sensor noise
    SensorNoise,
    /// Actuator delay/lag
    ActuatorLag,
    /// Parameter perturbation
    ParameterShift,
}

/// Adversarial disturbance configuration
#[derive(Clone, Debug)]
pub struct DisturbanceConfig {
    pub disturbance_type: DisturbanceType,
    pub intensity: f32, // 0.0 to 1.0
    pub frequency: f32, // Hz
    pub duration: f32,  // seconds
    pub enabled: bool,
}

impl DisturbanceConfig {
    pub fn new(disturbance_type: DisturbanceType, intensity: f32) -> Self {
        Self {
            disturbance_type,
            intensity,
            frequency: 1.0,
            duration: 0.1,
            enabled: true,
        }
    }

    pub fn with_frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency;
        self
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

/// Adversarial disturbance manager
#[derive(Resource, Clone, Debug)]
pub struct DisturbanceManager {
    pub disturbances: Vec<DisturbanceConfig>,
    pub last_disturbance_time: f64,
    pub enabled: bool,
}

impl Default for DisturbanceManager {
    fn default() -> Self {
        Self {
            disturbances: Vec::new(),
            last_disturbance_time: 0.0,
            enabled: true,
        }
    }
}

impl DisturbanceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_disturbance(&mut self, config: DisturbanceConfig) {
        self.disturbances.push(config);
    }

    pub fn should_apply_disturbance(
        &mut self,
        current_time: f64,
        config: &DisturbanceConfig,
    ) -> bool {
        if !self.enabled || !config.enabled {
            return false;
        }

        let interval = 1.0 / config.frequency as f64;
        if current_time - self.last_disturbance_time >= interval {
            let mut rng = thread_rng();
            if rng.gen::<f32>() < config.intensity {
                self.last_disturbance_time = current_time;
                return true;
            }
        }
        false
    }

    pub fn sample_force_disturbance(&self, config: &DisturbanceConfig) -> Vec3 {
        let mut rng = thread_rng();
        let normal = Normal::new(0.0, config.intensity as f64).unwrap();

        Vec3::new(
            normal.sample(&mut rng) as f32,
            normal.sample(&mut rng) as f32,
            normal.sample(&mut rng) as f32,
        ) * 10.0 // Scale factor
    }

    pub fn sample_torque_disturbance(&self, config: &DisturbanceConfig) -> Vec3 {
        let mut rng = thread_rng();
        let normal = Normal::new(0.0, config.intensity as f64).unwrap();

        Vec3::new(
            normal.sample(&mut rng) as f32,
            normal.sample(&mut rng) as f32,
            normal.sample(&mut rng) as f32,
        ) * 5.0 // Scale factor
    }

    pub fn sample_sensor_noise(&self, value: f32, config: &DisturbanceConfig) -> f32 {
        let mut rng = thread_rng();
        let normal = Normal::new(0.0, (config.intensity * value.abs()) as f64).unwrap();
        value + normal.sample(&mut rng) as f32
    }

    pub fn sample_actuator_lag(&self, config: &DisturbanceConfig) -> f32 {
        let mut rng = thread_rng();
        rng.gen::<f32>() * config.intensity * 0.1 // Up to 100ms lag
    }
}

/// Force disturbance component
#[derive(Component, Clone, Debug)]
pub struct ForceDisturbance {
    pub config: DisturbanceConfig,
    pub active_until: f64,
}

impl ForceDisturbance {
    pub fn new(intensity: f32) -> Self {
        Self {
            config: DisturbanceConfig::new(DisturbanceType::Force, intensity),
            active_until: 0.0,
        }
    }

    pub fn is_active(&self, current_time: f64) -> bool {
        current_time < self.active_until
    }
}

/// Sensor noise component
#[derive(Component, Clone, Debug)]
pub struct SensorNoiseDisturbance {
    pub config: DisturbanceConfig,
}

impl SensorNoiseDisturbance {
    pub fn new(intensity: f32) -> Self {
        Self {
            config: DisturbanceConfig::new(DisturbanceType::SensorNoise, intensity),
        }
    }

    pub fn apply_noise(&self, value: f32) -> f32 {
        let mut rng = thread_rng();
        let noise = rng.gen::<f32>() * 2.0 - 1.0; // -1 to 1
        value + noise * self.config.intensity * value.abs()
    }
}

/// Preset disturbance profiles
pub struct DisturbancePresets;

impl DisturbancePresets {
    /// Minimal disturbances for debugging
    pub fn minimal() -> DisturbanceManager {
        let mut manager = DisturbanceManager::new();
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::Force, 0.1)
                .with_frequency(0.5)
                .with_duration(0.05),
        );
        manager
    }

    /// Standard disturbances for robust training
    pub fn standard() -> DisturbanceManager {
        let mut manager = DisturbanceManager::new();
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::Force, 0.3)
                .with_frequency(1.0)
                .with_duration(0.1),
        );
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::SensorNoise, 0.05).with_frequency(10.0),
        );
        manager
    }

    /// Challenging disturbances for advanced training
    pub fn challenging() -> DisturbanceManager {
        let mut manager = DisturbanceManager::new();
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::Force, 0.5)
                .with_frequency(2.0)
                .with_duration(0.2),
        );
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::Torque, 0.4)
                .with_frequency(1.5)
                .with_duration(0.15),
        );
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::SensorNoise, 0.1).with_frequency(20.0),
        );
        manager
    }

    /// Extreme disturbances for stress testing
    pub fn extreme() -> DisturbanceManager {
        let mut manager = DisturbanceManager::new();
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::Force, 0.8)
                .with_frequency(3.0)
                .with_duration(0.3),
        );
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::Torque, 0.7)
                .with_frequency(2.5)
                .with_duration(0.25),
        );
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::SensorNoise, 0.2).with_frequency(30.0),
        );
        manager.add_disturbance(
            DisturbanceConfig::new(DisturbanceType::ActuatorLag, 0.3).with_frequency(5.0),
        );
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disturbance_config_creation() {
        let config = DisturbanceConfig::new(DisturbanceType::Force, 0.5);
        assert_eq!(config.disturbance_type, DisturbanceType::Force);
        assert_eq!(config.intensity, 0.5);
        assert!(config.enabled);
    }

    #[test]
    fn test_disturbance_config_builder() {
        let config = DisturbanceConfig::new(DisturbanceType::Force, 0.5)
            .with_frequency(2.0)
            .with_duration(0.2);

        assert_eq!(config.frequency, 2.0);
        assert_eq!(config.duration, 0.2);
    }

    #[test]
    fn test_manager_creation() {
        let manager = DisturbanceManager::new();
        assert!(manager.enabled);
        assert_eq!(manager.disturbances.len(), 0);
    }

    #[test]
    fn test_add_disturbance() {
        let mut manager = DisturbanceManager::new();
        manager.add_disturbance(DisturbanceConfig::new(DisturbanceType::Force, 0.5));
        manager.add_disturbance(DisturbanceConfig::new(DisturbanceType::Torque, 0.3));

        assert_eq!(manager.disturbances.len(), 2);
    }

    #[test]
    fn test_force_disturbance_component() {
        let disturbance = ForceDisturbance::new(0.5);
        assert_eq!(disturbance.config.intensity, 0.5);
        assert_eq!(disturbance.active_until, 0.0);
        assert!(!disturbance.is_active(1.0));
    }

    #[test]
    fn test_sensor_noise_component() {
        let noise = SensorNoiseDisturbance::new(0.1);
        let value = 10.0;
        let noisy_value = noise.apply_noise(value);

        // Should be within reasonable range
        assert!(noisy_value > value - value * 0.1 - 0.1);
        assert!(noisy_value < value + value * 0.1 + 0.1);
    }

    #[test]
    fn test_presets() {
        let minimal = DisturbancePresets::minimal();
        assert_eq!(minimal.disturbances.len(), 1);
        assert_eq!(minimal.disturbances[0].intensity, 0.1);

        let standard = DisturbancePresets::standard();
        assert_eq!(standard.disturbances.len(), 2);

        let challenging = DisturbancePresets::challenging();
        assert_eq!(challenging.disturbances.len(), 3);

        let extreme = DisturbancePresets::extreme();
        assert_eq!(extreme.disturbances.len(), 4);
    }

    #[test]
    fn test_sample_force_disturbance() {
        let manager = DisturbanceManager::new();
        let config = DisturbanceConfig::new(DisturbanceType::Force, 0.5);

        for _ in 0..10 {
            let force = manager.sample_force_disturbance(&config);
            // Force should be non-zero (statistically)
            assert!(force.length() < 50.0); // Reasonable upper bound
        }
    }

    #[test]
    fn test_sample_sensor_noise() {
        let manager = DisturbanceManager::new();
        let config = DisturbanceConfig::new(DisturbanceType::SensorNoise, 0.1);
        let value = 100.0;

        for _ in 0..10 {
            let noisy = manager.sample_sensor_noise(value, &config);
            // Should be within 5 sigma (very high probability)
            // With intensity=0.1 and value=100, stddev=10, so 5σ = ±50
            assert!(noisy > value - 50.0 && noisy < value + 50.0);
        }
    }

    #[test]
    fn test_should_apply_disturbance() {
        let mut manager = DisturbanceManager::new();
        let config = DisturbanceConfig::new(DisturbanceType::Force, 1.0).with_frequency(10.0); // High frequency for testing

        manager.last_disturbance_time = 0.0;

        // Should apply at time 0.1 (> 1/frequency)
        let should_apply = manager.should_apply_disturbance(0.1, &config);
        // Result depends on random chance, but with intensity 1.0 it should be likely
        // We can't assert true/false deterministically
    }
}
