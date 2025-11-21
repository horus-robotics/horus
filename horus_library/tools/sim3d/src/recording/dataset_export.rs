use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Dataset format for export
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatasetFormat {
    /// HDF5 format (binary, compressed)
    HDF5,
    /// NumPy compressed archive (.npz)
    NPZ,
    /// JSON format (human-readable, for debugging)
    JSON,
}

/// RL experience sample
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Experience {
    pub observation: Vec<f32>,
    pub action: Vec<f32>,
    pub reward: f32,
    pub done: bool,
    pub next_observation: Vec<f32>,
    pub info: HashMap<String, f32>,
}

impl Experience {
    pub fn new(
        observation: Vec<f32>,
        action: Vec<f32>,
        reward: f32,
        done: bool,
        next_observation: Vec<f32>,
    ) -> Self {
        Self {
            observation,
            action,
            reward,
            done,
            next_observation,
            info: HashMap::new(),
        }
    }

    pub fn with_info(mut self, key: String, value: f32) -> Self {
        self.info.insert(key, value);
        self
    }
}

/// Episode data for RL
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Episode {
    pub episode_id: u32,
    pub experiences: Vec<Experience>,
    pub total_reward: f32,
    pub length: usize,
    pub metadata: HashMap<String, String>,
}

impl Episode {
    pub fn new(episode_id: u32) -> Self {
        Self {
            episode_id,
            experiences: Vec::new(),
            total_reward: 0.0,
            length: 0,
            metadata: HashMap::new(),
        }
    }

    pub fn add_experience(&mut self, experience: Experience) {
        self.total_reward += experience.reward;
        self.length += 1;
        self.experiences.push(experience);
    }

    pub fn is_empty(&self) -> bool {
        self.experiences.is_empty()
    }

    pub fn get_returns(&self) -> Vec<f32> {
        let mut returns = Vec::with_capacity(self.experiences.len());
        let mut cumulative_return = 0.0;

        for exp in self.experiences.iter().rev() {
            cumulative_return = exp.reward + cumulative_return;
            returns.push(cumulative_return);
        }

        returns.reverse();
        returns
    }

    pub fn get_advantages(&self, gamma: f32, lambda: f32, value_estimates: &[f32]) -> Vec<f32> {
        let mut advantages = Vec::with_capacity(self.experiences.len());
        let mut last_advantage = 0.0;

        for i in (0..self.experiences.len()).rev() {
            let reward = self.experiences[i].reward;
            let value = value_estimates.get(i).copied().unwrap_or(0.0);
            let next_value = value_estimates.get(i + 1).copied().unwrap_or(0.0);

            let delta = reward
                + gamma * next_value * (1.0 - self.experiences[i].done as u32 as f32)
                - value;
            last_advantage = delta
                + gamma * lambda * (1.0 - self.experiences[i].done as u32 as f32) * last_advantage;
            advantages.push(last_advantage);
        }

        advantages.reverse();
        advantages
    }
}

/// Dataset for RL training
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RLDataset {
    pub episodes: Vec<Episode>,
    pub observation_shape: Vec<usize>,
    pub action_shape: Vec<usize>,
    pub metadata: DatasetMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub name: String,
    pub environment: String,
    pub agent: String,
    pub total_episodes: usize,
    pub total_timesteps: usize,
    pub avg_episode_reward: f32,
    pub avg_episode_length: f32,
    pub creation_time: String,
    pub additional_info: HashMap<String, String>,
}

impl RLDataset {
    pub fn new(name: String, observation_shape: Vec<usize>, action_shape: Vec<usize>) -> Self {
        Self {
            episodes: Vec::new(),
            observation_shape,
            action_shape,
            metadata: DatasetMetadata {
                name,
                environment: String::new(),
                agent: String::new(),
                total_episodes: 0,
                total_timesteps: 0,
                avg_episode_reward: 0.0,
                avg_episode_length: 0.0,
                creation_time: chrono::Utc::now().to_rfc3339(),
                additional_info: HashMap::new(),
            },
        }
    }

    pub fn add_episode(&mut self, episode: Episode) {
        self.metadata.total_episodes += 1;
        self.metadata.total_timesteps += episode.length;
        self.episodes.push(episode);
        self.update_statistics();
    }

    pub fn update_statistics(&mut self) {
        if self.episodes.is_empty() {
            return;
        }

        let total_reward: f32 = self.episodes.iter().map(|e| e.total_reward).sum();
        let total_length: usize = self.episodes.iter().map(|e| e.length).sum();

        self.metadata.avg_episode_reward = total_reward / self.episodes.len() as f32;
        self.metadata.avg_episode_length = total_length as f32 / self.episodes.len() as f32;
        self.metadata.total_episodes = self.episodes.len();
        self.metadata.total_timesteps = total_length;
    }

    pub fn get_all_observations(&self) -> Vec<Vec<f32>> {
        self.episodes
            .iter()
            .flat_map(|ep| ep.experiences.iter().map(|exp| exp.observation.clone()))
            .collect()
    }

    pub fn get_all_actions(&self) -> Vec<Vec<f32>> {
        self.episodes
            .iter()
            .flat_map(|ep| ep.experiences.iter().map(|exp| exp.action.clone()))
            .collect()
    }

    pub fn get_all_rewards(&self) -> Vec<f32> {
        self.episodes
            .iter()
            .flat_map(|ep| ep.experiences.iter().map(|exp| exp.reward))
            .collect()
    }

    pub fn get_all_dones(&self) -> Vec<bool> {
        self.episodes
            .iter()
            .flat_map(|ep| ep.experiences.iter().map(|exp| exp.done))
            .collect()
    }

    /// Export to JSON (for debugging/analysis)
    pub fn export_to_json(&self, path: &PathBuf) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Export to compressed JSON
    pub fn export_to_compressed_json(&self, path: &PathBuf) -> anyhow::Result<()> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let json = serde_json::to_string(&self)?;
        let file = std::fs::File::create(path)?;
        let mut encoder = GzEncoder::new(file, Compression::best());
        encoder.write_all(json.as_bytes())?;
        encoder.finish()?;
        Ok(())
    }

    /// Load from JSON
    pub fn load_from_json(path: &PathBuf) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let dataset = serde_json::from_str(&json)?;
        Ok(dataset)
    }

    /// Load from compressed JSON
    pub fn load_from_compressed_json(path: &PathBuf) -> anyhow::Result<Self> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let file = std::fs::File::open(path)?;
        let mut decoder = GzDecoder::new(file);
        let mut json = String::new();
        decoder.read_to_string(&mut json)?;
        let dataset = serde_json::from_str(&json)?;
        Ok(dataset)
    }

    /// Get statistics summary
    pub fn get_statistics(&self) -> DatasetStatistics {
        let rewards: Vec<f32> = self.get_all_rewards();
        let episode_lengths: Vec<usize> = self.episodes.iter().map(|e| e.length).collect();
        let episode_rewards: Vec<f32> = self.episodes.iter().map(|e| e.total_reward).collect();

        let mean_reward = if !rewards.is_empty() {
            rewards.iter().sum::<f32>() / rewards.len() as f32
        } else {
            0.0
        };

        let std_reward = if rewards.len() > 1 {
            let variance = rewards
                .iter()
                .map(|r| (r - mean_reward).powi(2))
                .sum::<f32>()
                / (rewards.len() - 1) as f32;
            variance.sqrt()
        } else {
            0.0
        };

        let min_reward = rewards.iter().copied().fold(f32::INFINITY, f32::min);
        let max_reward = rewards.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        let mean_episode_reward = if !episode_rewards.is_empty() {
            episode_rewards.iter().sum::<f32>() / episode_rewards.len() as f32
        } else {
            0.0
        };

        let mean_episode_length = if !episode_lengths.is_empty() {
            episode_lengths.iter().sum::<usize>() as f32 / episode_lengths.len() as f32
        } else {
            0.0
        };

        DatasetStatistics {
            total_episodes: self.episodes.len(),
            total_timesteps: self.metadata.total_timesteps,
            mean_reward,
            std_reward,
            min_reward,
            max_reward,
            mean_episode_reward,
            mean_episode_length,
            observation_shape: self.observation_shape.clone(),
            action_shape: self.action_shape.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DatasetStatistics {
    pub total_episodes: usize,
    pub total_timesteps: usize,
    pub mean_reward: f32,
    pub std_reward: f32,
    pub min_reward: f32,
    pub max_reward: f32,
    pub mean_episode_reward: f32,
    pub mean_episode_length: f32,
    pub observation_shape: Vec<usize>,
    pub action_shape: Vec<usize>,
}

impl DatasetStatistics {
    pub fn print_summary(&self) {
        println!("Dataset Statistics:");
        println!("  Total Episodes: {}", self.total_episodes);
        println!("  Total Timesteps: {}", self.total_timesteps);
        println!("  Mean Reward: {:.4}", self.mean_reward);
        println!("  Std Reward: {:.4}", self.std_reward);
        println!("  Min Reward: {:.4}", self.min_reward);
        println!("  Max Reward: {:.4}", self.max_reward);
        println!("  Mean Episode Reward: {:.4}", self.mean_episode_reward);
        println!("  Mean Episode Length: {:.2}", self.mean_episode_length);
        println!("  Observation Shape: {:?}", self.observation_shape);
        println!("  Action Shape: {:?}", self.action_shape);
    }
}

/// Dataset recorder resource
#[derive(Resource)]
pub struct DatasetRecorder {
    pub active: bool,
    pub dataset: RLDataset,
    pub current_episode: Option<Episode>,
    pub episode_counter: u32,
}

impl DatasetRecorder {
    pub fn new(
        dataset_name: String,
        observation_shape: Vec<usize>,
        action_shape: Vec<usize>,
    ) -> Self {
        Self {
            active: false,
            dataset: RLDataset::new(dataset_name, observation_shape, action_shape),
            current_episode: None,
            episode_counter: 0,
        }
    }

    pub fn start_recording(&mut self) {
        self.active = true;
    }

    pub fn stop_recording(&mut self) {
        self.active = false;
        self.finish_episode();
    }

    pub fn start_episode(&mut self) {
        if self.active {
            self.finish_episode();
            self.current_episode = Some(Episode::new(self.episode_counter));
            self.episode_counter += 1;
        }
    }

    pub fn finish_episode(&mut self) {
        if let Some(episode) = self.current_episode.take() {
            if !episode.is_empty() {
                self.dataset.add_episode(episode);
            }
        }
    }

    pub fn add_experience(&mut self, experience: Experience) {
        if self.active {
            if let Some(episode) = &mut self.current_episode {
                episode.add_experience(experience);
            }
        }
    }

    pub fn export(&mut self, path: &PathBuf, format: DatasetFormat) -> anyhow::Result<()> {
        self.dataset.update_statistics();

        match format {
            DatasetFormat::HDF5 => {
                // HDF5 export would require hdf5-rust crate
                // For now, export as compressed JSON with .h5 extension
                self.dataset.export_to_compressed_json(path)?;
            }
            DatasetFormat::NPZ => {
                // NPZ export would require numpy/ndarray serialization
                // For now, export as compressed JSON with .npz extension
                self.dataset.export_to_compressed_json(path)?;
            }
            DatasetFormat::JSON => {
                self.dataset.export_to_json(path)?;
            }
        }

        Ok(())
    }
}

/// Batch sampler for RL training
pub struct BatchSampler {
    dataset: RLDataset,
    indices: Vec<usize>,
    current_idx: usize,
}

impl BatchSampler {
    pub fn new(dataset: RLDataset, shuffle: bool) -> Self {
        let total_samples: usize = dataset.episodes.iter().map(|e| e.length).sum();
        let mut indices: Vec<usize> = (0..total_samples).collect();

        if shuffle {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            indices.shuffle(&mut rng);
        }

        Self {
            dataset,
            indices,
            current_idx: 0,
        }
    }

    pub fn get_batch(&mut self, batch_size: usize) -> Option<Batch> {
        if self.current_idx >= self.indices.len() {
            return None;
        }

        let end_idx = (self.current_idx + batch_size).min(self.indices.len());
        let batch_indices = &self.indices[self.current_idx..end_idx];

        let mut observations = Vec::new();
        let mut actions = Vec::new();
        let mut rewards = Vec::new();
        let mut dones = Vec::new();
        let mut next_observations = Vec::new();

        for &idx in batch_indices {
            let (episode_idx, exp_idx) = self.get_experience_location(idx);
            if let Some(episode) = self.dataset.episodes.get(episode_idx) {
                if let Some(experience) = episode.experiences.get(exp_idx) {
                    observations.push(experience.observation.clone());
                    actions.push(experience.action.clone());
                    rewards.push(experience.reward);
                    dones.push(experience.done);
                    next_observations.push(experience.next_observation.clone());
                }
            }
        }

        self.current_idx = end_idx;

        Some(Batch {
            observations,
            actions,
            rewards,
            dones,
            next_observations,
        })
    }

    fn get_experience_location(&self, global_idx: usize) -> (usize, usize) {
        let mut cumulative = 0;
        for (ep_idx, episode) in self.dataset.episodes.iter().enumerate() {
            if global_idx < cumulative + episode.length {
                return (ep_idx, global_idx - cumulative);
            }
            cumulative += episode.length;
        }
        (self.dataset.episodes.len() - 1, 0)
    }

    pub fn reset(&mut self) {
        self.current_idx = 0;
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        self.indices.shuffle(&mut rng);
        self.current_idx = 0;
    }
}

#[derive(Clone, Debug)]
pub struct Batch {
    pub observations: Vec<Vec<f32>>,
    pub actions: Vec<Vec<f32>>,
    pub rewards: Vec<f32>,
    pub dones: Vec<bool>,
    pub next_observations: Vec<Vec<f32>>,
}

impl Batch {
    pub fn size(&self) -> usize {
        self.observations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experience_creation() {
        let exp = Experience::new(
            vec![1.0, 2.0, 3.0],
            vec![0.5],
            1.0,
            false,
            vec![1.1, 2.1, 3.1],
        );
        assert_eq!(exp.observation.len(), 3);
        assert_eq!(exp.action.len(), 1);
        assert_eq!(exp.reward, 1.0);
        assert!(!exp.done);
    }

    #[test]
    fn test_episode_creation() {
        let mut episode = Episode::new(0);
        assert_eq!(episode.episode_id, 0);
        assert!(episode.is_empty());
        assert_eq!(episode.total_reward, 0.0);

        let exp = Experience::new(vec![1.0], vec![0.5], 1.0, false, vec![1.1]);
        episode.add_experience(exp);

        assert!(!episode.is_empty());
        assert_eq!(episode.length, 1);
        assert_eq!(episode.total_reward, 1.0);
    }

    #[test]
    fn test_episode_returns() {
        let mut episode = Episode::new(0);

        episode.add_experience(Experience::new(vec![1.0], vec![0.5], 1.0, false, vec![1.1]));
        episode.add_experience(Experience::new(vec![1.1], vec![0.5], 2.0, false, vec![1.2]));
        episode.add_experience(Experience::new(vec![1.2], vec![0.5], 3.0, true, vec![1.3]));

        let returns = episode.get_returns();
        assert_eq!(returns.len(), 3);
        assert_eq!(returns[0], 6.0); // 1 + 2 + 3
        assert_eq!(returns[1], 5.0); // 2 + 3
        assert_eq!(returns[2], 3.0); // 3
    }

    #[test]
    fn test_dataset_creation() {
        let dataset = RLDataset::new("test".to_string(), vec![3], vec![1]);
        assert_eq!(dataset.observation_shape, vec![3]);
        assert_eq!(dataset.action_shape, vec![1]);
        assert_eq!(dataset.episodes.len(), 0);
    }

    #[test]
    fn test_dataset_add_episode() {
        let mut dataset = RLDataset::new("test".to_string(), vec![3], vec![1]);
        let mut episode = Episode::new(0);

        episode.add_experience(Experience::new(
            vec![1.0, 2.0, 3.0],
            vec![0.5],
            1.0,
            false,
            vec![1.1, 2.1, 3.1],
        ));
        episode.add_experience(Experience::new(
            vec![1.1, 2.1, 3.1],
            vec![0.6],
            2.0,
            true,
            vec![1.2, 2.2, 3.2],
        ));

        dataset.add_episode(episode);

        assert_eq!(dataset.episodes.len(), 1);
        assert_eq!(dataset.metadata.total_episodes, 1);
        assert_eq!(dataset.metadata.total_timesteps, 2);
    }

    #[test]
    fn test_dataset_statistics() {
        let mut dataset = RLDataset::new("test".to_string(), vec![3], vec![1]);

        let mut episode1 = Episode::new(0);
        episode1.add_experience(Experience::new(
            vec![1.0, 2.0, 3.0],
            vec![0.5],
            1.0,
            false,
            vec![1.1, 2.1, 3.1],
        ));
        episode1.add_experience(Experience::new(
            vec![1.1, 2.1, 3.1],
            vec![0.6],
            2.0,
            true,
            vec![1.2, 2.2, 3.2],
        ));

        let mut episode2 = Episode::new(1);
        episode2.add_experience(Experience::new(
            vec![2.0, 3.0, 4.0],
            vec![0.7],
            3.0,
            false,
            vec![2.1, 3.1, 4.1],
        ));
        episode2.add_experience(Experience::new(
            vec![2.1, 3.1, 4.1],
            vec![0.8],
            4.0,
            true,
            vec![2.2, 3.2, 4.2],
        ));

        dataset.add_episode(episode1);
        dataset.add_episode(episode2);

        let stats = dataset.get_statistics();
        assert_eq!(stats.total_episodes, 2);
        assert_eq!(stats.total_timesteps, 4);
        assert_eq!(stats.mean_reward, 2.5); // (1+2+3+4)/4
    }

    #[test]
    fn test_dataset_recorder() {
        let mut recorder = DatasetRecorder::new("test".to_string(), vec![3], vec![1]);
        assert!(!recorder.active);

        recorder.start_recording();
        assert!(recorder.active);

        recorder.start_episode();
        assert!(recorder.current_episode.is_some());

        recorder.add_experience(Experience::new(
            vec![1.0, 2.0, 3.0],
            vec![0.5],
            1.0,
            false,
            vec![1.1, 2.1, 3.1],
        ));

        recorder.finish_episode();
        assert!(recorder.current_episode.is_none());
        assert_eq!(recorder.dataset.episodes.len(), 1);
    }

    #[test]
    fn test_batch_sampler() {
        let mut dataset = RLDataset::new("test".to_string(), vec![3], vec![1]);

        let mut episode = Episode::new(0);
        for i in 0..10 {
            episode.add_experience(Experience::new(
                vec![i as f32; 3],
                vec![0.5],
                i as f32,
                false,
                vec![(i + 1) as f32; 3],
            ));
        }

        dataset.add_episode(episode);

        let mut sampler = BatchSampler::new(dataset, false);
        let batch = sampler.get_batch(5).unwrap();

        assert_eq!(batch.size(), 5);
        assert_eq!(batch.observations.len(), 5);
        assert_eq!(batch.rewards.len(), 5);
    }

    #[test]
    fn test_batch_sampler_exhaustion() {
        let mut dataset = RLDataset::new("test".to_string(), vec![3], vec![1]);

        let mut episode = Episode::new(0);
        for i in 0..5 {
            episode.add_experience(Experience::new(
                vec![i as f32; 3],
                vec![0.5],
                i as f32,
                false,
                vec![(i + 1) as f32; 3],
            ));
        }

        dataset.add_episode(episode);

        let mut sampler = BatchSampler::new(dataset, false);

        let batch1 = sampler.get_batch(3);
        assert!(batch1.is_some());
        assert_eq!(batch1.unwrap().size(), 3);

        let batch2 = sampler.get_batch(3);
        assert!(batch2.is_some());
        assert_eq!(batch2.unwrap().size(), 2); // Only 2 left

        let batch3 = sampler.get_batch(3);
        assert!(batch3.is_none()); // Exhausted
    }
}
