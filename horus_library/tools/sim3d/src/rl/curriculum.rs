use bevy::prelude::*;
use std::collections::HashMap;

/// Curriculum learning stage
#[derive(Clone, Debug)]
pub struct CurriculumStage {
    pub name: String,
    pub difficulty: f32, // 0.0 to 1.0
    pub success_threshold: f32,
    pub min_episodes: usize,
    pub max_episodes: usize,
    pub parameters: HashMap<String, f32>,
}

impl CurriculumStage {
    pub fn new(name: String, difficulty: f32) -> Self {
        Self {
            name,
            difficulty,
            success_threshold: 0.8,
            min_episodes: 50,
            max_episodes: 1000,
            parameters: HashMap::new(),
        }
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.success_threshold = threshold;
        self
    }

    pub fn with_episodes(mut self, min: usize, max: usize) -> Self {
        self.min_episodes = min;
        self.max_episodes = max;
        self
    }

    pub fn with_parameter(mut self, key: String, value: f32) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

/// Curriculum learning manager
#[derive(Resource, Clone, Debug)]
pub struct CurriculumManager {
    pub stages: Vec<CurriculumStage>,
    pub current_stage_idx: usize,
    pub episode_count: usize,
    pub success_count: usize,
    pub enabled: bool,
}

impl Default for CurriculumManager {
    fn default() -> Self {
        Self {
            stages: Vec::new(),
            current_stage_idx: 0,
            episode_count: 0,
            success_count: 0,
            enabled: true,
        }
    }
}

impl CurriculumManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_stage(&mut self, stage: CurriculumStage) {
        self.stages.push(stage);
    }

    pub fn current_stage(&self) -> Option<&CurriculumStage> {
        self.stages.get(self.current_stage_idx)
    }

    pub fn record_episode(&mut self, success: bool) {
        self.episode_count += 1;
        if success {
            self.success_count += 1;
        }
    }

    pub fn should_advance(&self) -> bool {
        if !self.enabled || self.stages.is_empty() {
            return false;
        }

        let stage = match self.current_stage() {
            Some(s) => s,
            None => return false,
        };

        if self.episode_count < stage.min_episodes {
            return false;
        }

        if self.episode_count >= stage.max_episodes {
            return true;
        }

        let success_rate = self.success_count as f32 / self.episode_count as f32;
        success_rate >= stage.success_threshold
    }

    pub fn advance_stage(&mut self) -> bool {
        if self.current_stage_idx + 1 < self.stages.len() {
            self.current_stage_idx += 1;
            self.episode_count = 0;
            self.success_count = 0;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.current_stage_idx = 0;
        self.episode_count = 0;
        self.success_count = 0;
    }

    pub fn get_progress(&self) -> f32 {
        if self.stages.is_empty() {
            return 1.0;
        }
        (self.current_stage_idx as f32 + 1.0) / self.stages.len() as f32
    }

    pub fn is_complete(&self) -> bool {
        self.current_stage_idx >= self.stages.len().saturating_sub(1) && self.should_advance()
    }
}

/// Automatic curriculum builder
pub struct CurriculumBuilder;

impl CurriculumBuilder {
    /// Create a linear difficulty curriculum
    pub fn linear(num_stages: usize, task_name: &str) -> CurriculumManager {
        let mut manager = CurriculumManager::new();

        for i in 0..num_stages {
            let difficulty = (i + 1) as f32 / num_stages as f32;
            let stage = CurriculumStage::new(format!("{}_stage_{}", task_name, i + 1), difficulty)
                .with_threshold(0.75 + 0.05 * i as f32)
                .with_episodes(50 * (i + 1), 500 * (i + 1));

            manager.add_stage(stage);
        }

        manager
    }

    /// Create an exponential difficulty curriculum
    pub fn exponential(num_stages: usize, task_name: &str, base: f32) -> CurriculumManager {
        let mut manager = CurriculumManager::new();

        for i in 0..num_stages {
            let difficulty = base.powf(i as f32) / base.powf((num_stages - 1) as f32);
            let stage = CurriculumStage::new(format!("{}_stage_{}", task_name, i + 1), difficulty)
                .with_threshold(0.7 + 0.1 * difficulty)
                .with_episodes(100, 1000);

            manager.add_stage(stage);
        }

        manager
    }

    /// Create a custom curriculum with manual stages
    pub fn custom() -> CurriculumManager {
        CurriculumManager::new()
    }
}

/// Curriculum statistics
#[derive(Clone, Debug)]
pub struct CurriculumStats {
    pub current_stage: usize,
    pub total_stages: usize,
    pub episodes_in_stage: usize,
    pub success_rate: f32,
    pub progress: f32,
}

impl CurriculumStats {
    pub fn from_manager(manager: &CurriculumManager) -> Self {
        let success_rate = if manager.episode_count > 0 {
            manager.success_count as f32 / manager.episode_count as f32
        } else {
            0.0
        };

        Self {
            current_stage: manager.current_stage_idx,
            total_stages: manager.stages.len(),
            episodes_in_stage: manager.episode_count,
            success_rate,
            progress: manager.get_progress(),
        }
    }

    pub fn print_summary(&self) {
        println!("Curriculum Statistics:");
        println!("  Stage: {}/{}", self.current_stage + 1, self.total_stages);
        println!("  Episodes: {}", self.episodes_in_stage);
        println!("  Success Rate: {:.2}%", self.success_rate * 100.0);
        println!("  Overall Progress: {:.1}%", self.progress * 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_creation() {
        let stage = CurriculumStage::new("stage1".to_string(), 0.5)
            .with_threshold(0.8)
            .with_episodes(100, 500);

        assert_eq!(stage.name, "stage1");
        assert_eq!(stage.difficulty, 0.5);
        assert_eq!(stage.success_threshold, 0.8);
        assert_eq!(stage.min_episodes, 100);
        assert_eq!(stage.max_episodes, 500);
    }

    #[test]
    fn test_manager_creation() {
        let manager = CurriculumManager::new();
        assert_eq!(manager.current_stage_idx, 0);
        assert_eq!(manager.episode_count, 0);
        assert!(manager.enabled);
    }

    #[test]
    fn test_add_stage() {
        let mut manager = CurriculumManager::new();
        manager.add_stage(CurriculumStage::new("stage1".to_string(), 0.5));
        manager.add_stage(CurriculumStage::new("stage2".to_string(), 1.0));

        assert_eq!(manager.stages.len(), 2);
        assert_eq!(manager.current_stage().unwrap().name, "stage1");
    }

    #[test]
    fn test_record_episode() {
        let mut manager = CurriculumManager::new();
        manager.add_stage(CurriculumStage::new("stage1".to_string(), 0.5));

        manager.record_episode(true);
        manager.record_episode(false);
        manager.record_episode(true);

        assert_eq!(manager.episode_count, 3);
        assert_eq!(manager.success_count, 2);
    }

    #[test]
    fn test_should_advance() {
        let mut manager = CurriculumManager::new();
        let stage = CurriculumStage::new("stage1".to_string(), 0.5)
            .with_threshold(0.8)
            .with_episodes(10, 100);
        manager.add_stage(stage);

        // Not enough episodes
        for _ in 0..5 {
            manager.record_episode(true);
        }
        assert!(!manager.should_advance());

        // Enough episodes, high success rate
        for _ in 0..10 {
            manager.record_episode(true);
        }
        assert!(manager.should_advance());
    }

    #[test]
    fn test_advance_stage() {
        let mut manager = CurriculumManager::new();
        manager.add_stage(CurriculumStage::new("stage1".to_string(), 0.5));
        manager.add_stage(CurriculumStage::new("stage2".to_string(), 1.0));

        assert_eq!(manager.current_stage_idx, 0);
        assert!(manager.advance_stage());
        assert_eq!(manager.current_stage_idx, 1);
        assert!(!manager.advance_stage()); // No more stages
    }

    #[test]
    fn test_linear_curriculum() {
        let manager = CurriculumBuilder::linear(5, "navigation");
        assert_eq!(manager.stages.len(), 5);

        // Check difficulty progression
        for (i, stage) in manager.stages.iter().enumerate() {
            let expected_difficulty = (i + 1) as f32 / 5.0;
            assert!((stage.difficulty - expected_difficulty).abs() < 0.01);
        }
    }

    #[test]
    fn test_exponential_curriculum() {
        let manager = CurriculumBuilder::exponential(4, "manipulation", 2.0);
        assert_eq!(manager.stages.len(), 4);

        // Difficulties should increase exponentially
        assert!(manager.stages[0].difficulty < manager.stages[1].difficulty);
        assert!(manager.stages[1].difficulty < manager.stages[2].difficulty);
    }

    #[test]
    fn test_get_progress() {
        let mut manager = CurriculumManager::new();
        manager.add_stage(CurriculumStage::new("stage1".to_string(), 0.5));
        manager.add_stage(CurriculumStage::new("stage2".to_string(), 1.0));

        assert_eq!(manager.get_progress(), 0.5); // Stage 1/2

        manager.advance_stage();
        assert_eq!(manager.get_progress(), 1.0); // Stage 2/2
    }

    #[test]
    fn test_is_complete() {
        let mut manager = CurriculumManager::new();
        let stage = CurriculumStage::new("stage1".to_string(), 1.0)
            .with_threshold(0.8)
            .with_episodes(10, 100);
        manager.add_stage(stage);

        assert!(!manager.is_complete());

        // Complete the stage
        for _ in 0..15 {
            manager.record_episode(true);
        }

        assert!(manager.is_complete());
    }

    #[test]
    fn test_stats() {
        let mut manager = CurriculumManager::new();
        manager.add_stage(CurriculumStage::new("stage1".to_string(), 0.5));
        manager.add_stage(CurriculumStage::new("stage2".to_string(), 1.0));

        for _ in 0..10 {
            manager.record_episode(true);
        }
        for _ in 0..5 {
            manager.record_episode(false);
        }

        let stats = CurriculumStats::from_manager(&manager);
        assert_eq!(stats.current_stage, 0);
        assert_eq!(stats.total_stages, 2);
        assert_eq!(stats.episodes_in_stage, 15);
        assert!((stats.success_rate - 10.0 / 15.0).abs() < 0.01);
    }
}
