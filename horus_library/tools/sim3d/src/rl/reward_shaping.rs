use bevy::prelude::*;
use std::collections::HashMap;

/// Reward shaping function type
pub type RewardFn = fn(f32) -> f32;

/// Common reward shaping functions
pub struct RewardFunctions;

impl RewardFunctions {
    /// Linear reward: r' = r
    pub fn linear(reward: f32) -> f32 {
        reward
    }

    /// Exponential reward: r' = sign(r) * |r|^power
    pub fn exponential(reward: f32, power: f32) -> f32 {
        reward.signum() * reward.abs().powf(power)
    }

    /// Gaussian reward: r' = exp(-distance^2 / (2 * sigma^2))
    pub fn gaussian(distance: f32, sigma: f32) -> f32 {
        (-distance.powi(2) / (2.0 * sigma.powi(2))).exp()
    }

    /// Tanh reward: r' = tanh(r * scale)
    pub fn tanh(reward: f32, scale: f32) -> f32 {
        (reward * scale).tanh()
    }

    /// Clipped reward: r' = clip(r, min, max)
    pub fn clipped(reward: f32, min: f32, max: f32) -> f32 {
        reward.clamp(min, max)
    }

    /// Sparse reward: r' = 1 if distance < threshold else 0
    pub fn sparse(distance: f32, threshold: f32) -> f32 {
        if distance < threshold {
            1.0
        } else {
            0.0
        }
    }

    /// Tolerance-based reward: r' = 1 if |error| < tolerance else 0
    pub fn tolerance(error: f32, tolerance: f32) -> f32 {
        if error.abs() < tolerance {
            1.0
        } else {
            0.0
        }
    }

    /// Distance-based reward: r' = max(0, 1 - distance/max_distance)
    pub fn distance_based(distance: f32, max_distance: f32) -> f32 {
        (1.0 - distance / max_distance).max(0.0)
    }

    /// Velocity bonus: r' = base_reward + velocity_magnitude * scale
    pub fn velocity_bonus(base_reward: f32, velocity: Vec3, scale: f32) -> f32 {
        base_reward + velocity.length() * scale
    }

    /// Progress reward: r' = (current - previous) / dt
    pub fn progress(current: f32, previous: f32, dt: f32) -> f32 {
        (current - previous) / dt
    }
}

/// Composite reward combining multiple reward components
#[derive(Clone, Debug)]
pub struct CompositeReward {
    pub components: HashMap<String, RewardComponent>,
}

#[derive(Clone, Debug)]
pub struct RewardComponent {
    pub weight: f32,
    pub value: f32,
}

impl CompositeReward {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn add_component(&mut self, name: String, weight: f32, value: f32) {
        self.components
            .insert(name, RewardComponent { weight, value });
    }

    pub fn compute_total(&self) -> f32 {
        self.components
            .values()
            .map(|comp| comp.weight * comp.value)
            .sum()
    }

    pub fn get_component_contribution(&self, name: &str) -> f32 {
        self.components
            .get(name)
            .map(|comp| comp.weight * comp.value)
            .unwrap_or(0.0)
    }

    pub fn clear(&mut self) {
        self.components.clear();
    }

    pub fn normalize_weights(&mut self) {
        let total_weight: f32 = self.components.values().map(|c| c.weight).sum();
        if total_weight > 0.0 {
            for comp in self.components.values_mut() {
                comp.weight /= total_weight;
            }
        }
    }
}

/// Reward manager for tracking and analyzing rewards
#[derive(Resource, Clone, Debug)]
pub struct RewardManager {
    pub total_reward: f32,
    pub episode_reward: f32,
    pub step_rewards: Vec<f32>,
    pub reward_history: Vec<f32>,
    pub max_history_size: usize,
}

impl Default for RewardManager {
    fn default() -> Self {
        Self {
            total_reward: 0.0,
            episode_reward: 0.0,
            step_rewards: Vec::new(),
            reward_history: Vec::new(),
            max_history_size: 1000,
        }
    }
}

impl RewardManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_reward(&mut self, reward: f32) {
        self.total_reward += reward;
        self.episode_reward += reward;
        self.step_rewards.push(reward);
    }

    pub fn end_episode(&mut self) {
        self.reward_history.push(self.episode_reward);
        if self.reward_history.len() > self.max_history_size {
            self.reward_history.remove(0);
        }
        self.episode_reward = 0.0;
        self.step_rewards.clear();
    }

    pub fn get_average_episode_reward(&self) -> f32 {
        if self.reward_history.is_empty() {
            0.0
        } else {
            self.reward_history.iter().sum::<f32>() / self.reward_history.len() as f32
        }
    }

    pub fn get_recent_average(&self, n: usize) -> f32 {
        if self.reward_history.is_empty() {
            0.0
        } else {
            let start = self.reward_history.len().saturating_sub(n);
            let recent = &self.reward_history[start..];
            recent.iter().sum::<f32>() / recent.len() as f32
        }
    }

    pub fn reset(&mut self) {
        self.total_reward = 0.0;
        self.episode_reward = 0.0;
        self.step_rewards.clear();
        self.reward_history.clear();
    }
}

/// Reward statistics for analysis
#[derive(Clone, Debug)]
pub struct RewardStats {
    pub mean: f32,
    pub std: f32,
    pub min: f32,
    pub max: f32,
    pub total: f32,
}

impl RewardStats {
    pub fn from_history(history: &[f32]) -> Self {
        if history.is_empty() {
            return Self {
                mean: 0.0,
                std: 0.0,
                min: 0.0,
                max: 0.0,
                total: 0.0,
            };
        }

        let total: f32 = history.iter().sum();
        let mean = total / history.len() as f32;
        let variance =
            history.iter().map(|r| (r - mean).powi(2)).sum::<f32>() / history.len() as f32;
        let std = variance.sqrt();
        let min = history.iter().copied().fold(f32::INFINITY, f32::min);
        let max = history.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        Self {
            mean,
            std,
            min,
            max,
            total,
        }
    }

    pub fn print_summary(&self) {
        println!("Reward Statistics:");
        println!("  Mean: {:.4}", self.mean);
        println!("  Std: {:.4}", self.std);
        println!("  Min: {:.4}", self.min);
        println!("  Max: {:.4}", self.max);
        println!("  Total: {:.4}", self.total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_reward() {
        assert_eq!(RewardFunctions::linear(5.0), 5.0);
        assert_eq!(RewardFunctions::linear(-3.0), -3.0);
    }

    #[test]
    fn test_exponential_reward() {
        let reward = RewardFunctions::exponential(2.0, 2.0);
        assert_eq!(reward, 4.0);

        let negative_reward = RewardFunctions::exponential(-2.0, 2.0);
        assert_eq!(negative_reward, -4.0);
    }

    #[test]
    fn test_gaussian_reward() {
        let reward = RewardFunctions::gaussian(0.0, 1.0);
        assert_eq!(reward, 1.0);

        let reward_1 = RewardFunctions::gaussian(1.0, 1.0);
        assert!(reward_1 < 1.0 && reward_1 > 0.0);
    }

    #[test]
    fn test_tanh_reward() {
        let reward = RewardFunctions::tanh(0.0, 1.0);
        assert_eq!(reward, 0.0);

        let reward_pos = RewardFunctions::tanh(1.0, 1.0);
        assert!(reward_pos > 0.0 && reward_pos < 1.0);
    }

    #[test]
    fn test_clipped_reward() {
        assert_eq!(RewardFunctions::clipped(5.0, -1.0, 1.0), 1.0);
        assert_eq!(RewardFunctions::clipped(-5.0, -1.0, 1.0), -1.0);
        assert_eq!(RewardFunctions::clipped(0.5, -1.0, 1.0), 0.5);
    }

    #[test]
    fn test_sparse_reward() {
        assert_eq!(RewardFunctions::sparse(0.5, 1.0), 1.0);
        assert_eq!(RewardFunctions::sparse(1.5, 1.0), 0.0);
    }

    #[test]
    fn test_tolerance_reward() {
        assert_eq!(RewardFunctions::tolerance(0.5, 1.0), 1.0);
        assert_eq!(RewardFunctions::tolerance(1.5, 1.0), 0.0);
    }

    #[test]
    fn test_distance_based_reward() {
        assert_eq!(RewardFunctions::distance_based(0.0, 10.0), 1.0);
        assert_eq!(RewardFunctions::distance_based(5.0, 10.0), 0.5);
        assert_eq!(RewardFunctions::distance_based(15.0, 10.0), 0.0);
    }

    #[test]
    fn test_velocity_bonus() {
        let velocity = Vec3::new(1.0, 0.0, 0.0);
        let reward = RewardFunctions::velocity_bonus(10.0, velocity, 2.0);
        assert_eq!(reward, 12.0);
    }

    #[test]
    fn test_progress_reward() {
        let reward = RewardFunctions::progress(5.0, 3.0, 0.1);
        assert_eq!(reward, 20.0);
    }

    #[test]
    fn test_composite_reward() {
        let mut composite = CompositeReward::new();
        composite.add_component("distance".to_string(), 1.0, 10.0);
        composite.add_component("velocity".to_string(), 0.5, 5.0);

        let total = composite.compute_total();
        assert_eq!(total, 12.5);

        let distance_contribution = composite.get_component_contribution("distance");
        assert_eq!(distance_contribution, 10.0);
    }

    #[test]
    fn test_normalize_weights() {
        let mut composite = CompositeReward::new();
        composite.add_component("a".to_string(), 2.0, 10.0);
        composite.add_component("b".to_string(), 3.0, 5.0);

        composite.normalize_weights();

        let total_weight: f32 = composite.components.values().map(|c| c.weight).sum();
        assert!((total_weight - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_reward_manager() {
        let mut manager = RewardManager::new();

        manager.record_reward(10.0);
        manager.record_reward(5.0);
        assert_eq!(manager.episode_reward, 15.0);

        manager.end_episode();
        assert_eq!(manager.episode_reward, 0.0);
        assert_eq!(manager.reward_history.len(), 1);
        assert_eq!(manager.reward_history[0], 15.0);
    }

    #[test]
    fn test_average_episode_reward() {
        let mut manager = RewardManager::new();

        manager.record_reward(10.0);
        manager.end_episode();

        manager.record_reward(20.0);
        manager.end_episode();

        let avg = manager.get_average_episode_reward();
        assert_eq!(avg, 15.0);
    }

    #[test]
    fn test_recent_average() {
        let mut manager = RewardManager::new();

        for i in 0..10 {
            manager.record_reward(i as f32);
            manager.end_episode();
        }

        let recent_avg = manager.get_recent_average(3);
        assert_eq!(recent_avg, 8.0); // Average of 7, 8, 9
    }

    #[test]
    fn test_reward_stats() {
        let history = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = RewardStats::from_history(&history);

        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.total, 15.0);
        assert!(stats.std > 0.0);
    }
}
