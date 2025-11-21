//! Performance metrics tracking for algorithm evaluation
//!
//! Tracks path length, speed, collisions, goal achievement, and resource usage.
//! Provides real-time monitoring and post-run analysis capabilities.

use anyhow::Result;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Comprehensive performance metrics for robot navigation
#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    // Time tracking
    pub start_time: f64,
    pub current_time: f64,
    pub elapsed_time: f64,

    // Path metrics
    pub path_length: f32,
    pub path_smoothness: f32,
    pub avg_speed: f32,
    pub max_speed: f32,

    // Collisions and safety
    pub collision_count: u32,
    pub near_miss_count: u32,

    // Goal tracking
    pub goal_reached: bool,
    pub time_to_goal: Option<f64>,
    pub distance_to_goal: f32,
    pub goal_position: Option<Vec2>,

    // Energy consumption (simplified model)
    pub energy_consumed: f32,

    // Historical data for visualization
    pub speed_history: Vec<f32>,
    pub position_history: Vec<Vec2>,

    // Run metadata
    pub run_name: String,
    pub run_description: String,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            start_time: 0.0,
            current_time: 0.0,
            elapsed_time: 0.0,
            path_length: 0.0,
            path_smoothness: 1.0,
            avg_speed: 0.0,
            max_speed: 0.0,
            collision_count: 0,
            near_miss_count: 0,
            goal_reached: false,
            time_to_goal: None,
            distance_to_goal: f32::MAX,
            goal_position: None,
            energy_consumed: 0.0,
            speed_history: Vec::new(),
            position_history: Vec::new(),
            run_name: "Unnamed Run".to_string(),
            run_description: String::new(),
        }
    }
}

impl PerformanceMetrics {
    /// Create new metrics with a name and description
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            run_name: name.into(),
            run_description: description.into(),
            ..Default::default()
        }
    }

    /// Reset all metrics for a new run
    pub fn reset(&mut self) {
        let name = self.run_name.clone();
        let desc = self.run_description.clone();
        *self = Self::default();
        self.run_name = name;
        self.run_description = desc;
    }

    /// Update metrics with current robot state
    pub fn update(&mut self, position: Vec2, velocity: Vec2, dt: f32) {
        self.elapsed_time += dt as f64;
        self.current_time = self.elapsed_time;

        let speed = velocity.length();

        // Update path length
        if let Some(last_pos) = self.position_history.last() {
            let distance = (*last_pos - position).length();
            self.path_length += distance;
        }
        self.position_history.push(position);

        // Update speed metrics
        self.speed_history.push(speed);

        // Calculate average speed
        if !self.speed_history.is_empty() {
            self.avg_speed =
                self.speed_history.iter().sum::<f32>() / self.speed_history.len() as f32;
        }

        // Update max speed
        self.max_speed = self.max_speed.max(speed);

        // Calculate path smoothness (inverse of speed variance)
        if self.speed_history.len() > 2 {
            let variance = self.calculate_variance(&self.speed_history);
            self.path_smoothness = 1.0 / (1.0 + variance);
        }

        // Goal tracking
        if let Some(goal_pos) = self.goal_position {
            self.distance_to_goal = (position - goal_pos).length();

            // Check if goal reached (within 0.5m threshold)
            if self.distance_to_goal < 0.5 && !self.goal_reached {
                self.goal_reached = true;
                self.time_to_goal = Some(self.elapsed_time);
            }
        }

        // Energy consumption (simplified: proportional to speed and time)
        self.energy_consumed += speed.abs() * dt;
    }

    /// Set the goal position for tracking
    pub fn set_goal(&mut self, goal: Vec2) {
        self.goal_position = Some(goal);
    }

    /// Record a collision event
    pub fn record_collision(&mut self) {
        self.collision_count += 1;
    }

    /// Record a near miss event (close approach to obstacle)
    pub fn record_near_miss(&mut self, distance: f32) {
        if distance < 0.5 {
            // Within 0.5m threshold
            self.near_miss_count += 1;
        }
    }

    /// Calculate safety score (0.0 to 1.0)
    pub fn safety_score(&self) -> f32 {
        let collision_penalty = self.collision_count as f32 * 0.1;
        let near_miss_penalty = self.near_miss_count as f32 * 0.02;
        (1.0 - (collision_penalty + near_miss_penalty)).max(0.0)
    }

    /// Calculate efficiency score (0.0 to 1.0)
    pub fn efficiency_score(&self) -> f32 {
        if self.path_length == 0.0 {
            return 0.0;
        }

        // Perfect score if goal reached optimally
        if let (Some(goal_pos), Some(start_pos)) =
            (self.goal_position, self.position_history.first())
        {
            let optimal_distance = (*start_pos - goal_pos).length();
            if optimal_distance > 0.0 {
                return (optimal_distance / self.path_length).min(1.0);
            }
        }

        // Fallback: smoothness as efficiency proxy
        self.path_smoothness
    }

    /// Calculate overall score combining multiple factors
    pub fn overall_score(&self) -> f32 {
        let safety = self.safety_score();
        let efficiency = self.efficiency_score();
        let goal_bonus = if self.goal_reached { 1.0 } else { 0.5 };

        (safety * 0.4 + efficiency * 0.4 + goal_bonus * 0.2).clamp(0.0, 1.0)
    }

    /// Export metrics to CSV format
    pub fn export_to_csv(&self, path: &Path) -> Result<()> {
        let mut wtr = csv::WriterBuilder::new()
            .flexible(true) // Allow variable number of fields
            .from_path(path)?;

        // Write summary section
        wtr.write_record(&["Metric", "Value"])?;
        wtr.write_record(&["Run Name", &self.run_name])?;
        wtr.write_record(&["Run Description", &self.run_description])?;
        wtr.write_record(&["Elapsed Time (s)", &self.elapsed_time.to_string()])?;
        wtr.write_record(&["Path Length (m)", &self.path_length.to_string()])?;
        wtr.write_record(&["Path Smoothness", &self.path_smoothness.to_string()])?;
        wtr.write_record(&["Average Speed (m/s)", &self.avg_speed.to_string()])?;
        wtr.write_record(&["Max Speed (m/s)", &self.max_speed.to_string()])?;
        wtr.write_record(&["Collision Count", &self.collision_count.to_string()])?;
        wtr.write_record(&["Near Miss Count", &self.near_miss_count.to_string()])?;
        wtr.write_record(&["Goal Reached", &self.goal_reached.to_string()])?;
        wtr.write_record(&["Time to Goal (s)", &format!("{:?}", self.time_to_goal)])?;
        wtr.write_record(&["Distance to Goal (m)", &self.distance_to_goal.to_string()])?;
        wtr.write_record(&["Energy Consumed (J)", &self.energy_consumed.to_string()])?;
        wtr.write_record(&["Safety Score", &self.safety_score().to_string()])?;
        wtr.write_record(&["Efficiency Score", &self.efficiency_score().to_string()])?;
        wtr.write_record(&["Overall Score", &self.overall_score().to_string()])?;

        wtr.flush()?;

        // Write time series data in a new CSV section
        wtr.write_record(&[
            "Time (s)",
            "Speed (m/s)",
            "Position X (m)",
            "Position Y (m)",
        ])?;

        let dt = if !self.speed_history.is_empty() {
            self.elapsed_time / self.speed_history.len() as f64
        } else {
            0.016 // Default 60 FPS
        };

        for (i, (&speed, &pos)) in self
            .speed_history
            .iter()
            .zip(self.position_history.iter())
            .enumerate()
        {
            wtr.write_record(&[
                format!("{:.3}", i as f64 * dt),
                speed.to_string(),
                pos.x.to_string(),
                pos.y.to_string(),
            ])?;
        }

        wtr.flush()?;
        Ok(())
    }

    /// Export metrics to JSON format
    pub fn export_to_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load metrics from JSON file
    pub fn load_from_json(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let metrics: Self = serde_json::from_str(&json)?;
        Ok(metrics)
    }

    /// Calculate variance of a dataset
    fn calculate_variance(&self, data: &[f32]) -> f32 {
        if data.is_empty() {
            return 0.0;
        }

        let mean = data.iter().sum::<f32>() / data.len() as f32;
        let variance = data.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / data.len() as f32;
        variance
    }

    /// Get recent speed data (last N samples)
    pub fn recent_speed(&self, samples: usize) -> &[f32] {
        let start = self.speed_history.len().saturating_sub(samples);
        &self.speed_history[start..]
    }

    /// Get recent position data (last N samples)
    pub fn recent_positions(&self, samples: usize) -> &[Vec2] {
        let start = self.position_history.len().saturating_sub(samples);
        &self.position_history[start..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = PerformanceMetrics::new("Test Run", "Testing metrics");
        assert_eq!(metrics.run_name, "Test Run");
        assert_eq!(metrics.run_description, "Testing metrics");
        assert_eq!(metrics.elapsed_time, 0.0);
        assert_eq!(metrics.path_length, 0.0);
    }

    #[test]
    fn test_metrics_update() {
        let mut metrics = PerformanceMetrics::default();

        // Update with some movement
        metrics.update(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), 0.1);
        metrics.update(Vec2::new(0.1, 0.0), Vec2::new(1.0, 0.0), 0.1);

        assert!(metrics.elapsed_time > 0.0);
        assert!(metrics.path_length > 0.0);
        assert_eq!(metrics.speed_history.len(), 2);
        assert_eq!(metrics.position_history.len(), 2);
    }

    #[test]
    fn test_collision_tracking() {
        let mut metrics = PerformanceMetrics::default();

        assert_eq!(metrics.collision_count, 0);
        metrics.record_collision();
        assert_eq!(metrics.collision_count, 1);
        metrics.record_collision();
        assert_eq!(metrics.collision_count, 2);
    }

    #[test]
    fn test_near_miss_tracking() {
        let mut metrics = PerformanceMetrics::default();

        assert_eq!(metrics.near_miss_count, 0);
        metrics.record_near_miss(0.3); // Within threshold
        assert_eq!(metrics.near_miss_count, 1);
        metrics.record_near_miss(0.8); // Outside threshold
        assert_eq!(metrics.near_miss_count, 1); // Should not increment
    }

    #[test]
    fn test_goal_tracking() {
        let mut metrics = PerformanceMetrics::default();

        // Set goal
        metrics.set_goal(Vec2::new(10.0, 10.0));
        assert_eq!(metrics.goal_position, Some(Vec2::new(10.0, 10.0)));

        // Move toward goal
        metrics.update(Vec2::new(0.0, 0.0), Vec2::ZERO, 0.1);
        assert!(!metrics.goal_reached);

        // Reach goal
        metrics.update(Vec2::new(9.9, 9.9), Vec2::ZERO, 0.1);
        assert!(metrics.goal_reached);
        assert!(metrics.time_to_goal.is_some());
    }

    #[test]
    fn test_safety_score() {
        let mut metrics = PerformanceMetrics::default();

        // Perfect safety
        assert_eq!(metrics.safety_score(), 1.0);

        // With collisions
        metrics.record_collision();
        assert!(metrics.safety_score() < 1.0);

        // With many collisions
        for _ in 0..10 {
            metrics.record_collision();
        }
        assert_eq!(metrics.safety_score(), 0.0); // Clamped to 0
    }

    #[test]
    fn test_metrics_reset() {
        let mut metrics = PerformanceMetrics::new("Test", "Description");
        metrics.update(Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.0), 0.1);
        metrics.record_collision();

        // Reset
        metrics.reset();

        assert_eq!(metrics.run_name, "Test"); // Name preserved
        assert_eq!(metrics.run_description, "Description"); // Description preserved
        assert_eq!(metrics.elapsed_time, 0.0);
        assert_eq!(metrics.path_length, 0.0);
        assert_eq!(metrics.collision_count, 0);
        assert!(metrics.speed_history.is_empty());
    }

    #[test]
    fn test_csv_export() {
        let mut metrics = PerformanceMetrics::new("CSV Test", "Testing CSV export");
        metrics.update(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), 0.1);
        metrics.update(Vec2::new(0.1, 0.0), Vec2::new(1.0, 0.0), 0.1);

        let temp_path = std::env::temp_dir().join("test_metrics.csv");
        let result = metrics.export_to_csv(&temp_path);
        if let Err(ref e) = result {
            panic!("CSV export failed: {}", e);
        }
        assert!(result.is_ok());
        assert!(temp_path.exists());

        // Cleanup
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_json_export_import() {
        let mut metrics = PerformanceMetrics::new("JSON Test", "Testing JSON export");
        metrics.update(Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), 0.1);
        metrics.update(Vec2::new(0.1, 0.0), Vec2::new(1.0, 0.0), 0.1);
        metrics.record_collision();

        let temp_path = std::env::temp_dir().join("test_metrics.json");

        // Export
        let result = metrics.export_to_json(&temp_path);
        assert!(result.is_ok());
        assert!(temp_path.exists());

        // Import
        let loaded = PerformanceMetrics::load_from_json(&temp_path);
        assert!(loaded.is_ok());

        let loaded_metrics = loaded.unwrap();
        assert_eq!(loaded_metrics.run_name, metrics.run_name);
        assert_eq!(loaded_metrics.collision_count, metrics.collision_count);
        assert_eq!(loaded_metrics.path_length, metrics.path_length);

        // Cleanup
        std::fs::remove_file(temp_path).ok();
    }
}
