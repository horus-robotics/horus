use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Single trajectory point
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub timestamp: f64,
    pub position: Vec3,
    pub rotation: Quat,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
}

/// Joint state at a point in time
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JointState {
    pub timestamp: f64,
    pub positions: Vec<f32>,
    pub velocities: Vec<f32>,
    pub efforts: Vec<f32>,
}

/// Complete trajectory for an entity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trajectory {
    pub entity_name: String,
    pub points: Vec<TrajectoryPoint>,
    pub joint_states: Vec<JointState>,
    pub metadata: HashMap<String, String>,
}

impl Trajectory {
    pub fn new(entity_name: String) -> Self {
        Self {
            entity_name,
            points: Vec::new(),
            joint_states: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a trajectory point
    pub fn add_point(&mut self, point: TrajectoryPoint) {
        self.points.push(point);
    }

    /// Add joint state
    pub fn add_joint_state(&mut self, state: JointState) {
        self.joint_states.push(state);
    }

    /// Get duration in seconds
    pub fn duration(&self) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        let first = self.points.first().unwrap().timestamp;
        let last = self.points.last().unwrap().timestamp;
        last - first
    }

    /// Get point at specific time (interpolated)
    pub fn get_point_at(&self, time: f64) -> Option<TrajectoryPoint> {
        if self.points.is_empty() {
            return None;
        }

        // Find surrounding points
        let mut before_idx = 0;
        let mut after_idx = self.points.len() - 1;

        for (i, point) in self.points.iter().enumerate() {
            if point.timestamp <= time {
                before_idx = i;
            }
            if point.timestamp >= time {
                after_idx = i;
                break;
            }
        }

        if before_idx == after_idx {
            return Some(self.points[before_idx].clone());
        }

        // Interpolate
        let before = &self.points[before_idx];
        let after = &self.points[after_idx];
        let t = (time - before.timestamp) / (after.timestamp - before.timestamp);

        Some(TrajectoryPoint {
            timestamp: time,
            position: before.position.lerp(after.position, t as f32),
            rotation: before.rotation.slerp(after.rotation, t as f32),
            linear_velocity: before.linear_velocity.lerp(after.linear_velocity, t as f32),
            angular_velocity: before
                .angular_velocity
                .lerp(after.angular_velocity, t as f32),
        })
    }

    /// Sample trajectory at fixed intervals
    pub fn sample(&self, interval: f64) -> Vec<TrajectoryPoint> {
        let mut sampled = Vec::new();
        let duration = self.duration();

        if duration <= 0.0 {
            return sampled;
        }

        let mut time = self.points.first().unwrap().timestamp;
        let end_time = self.points.last().unwrap().timestamp;

        while time <= end_time {
            if let Some(point) = self.get_point_at(time) {
                sampled.push(point);
            }
            time += interval;
        }

        sampled
    }
}

/// Trajectory recorder component
#[derive(Component, Clone, Debug)]
pub struct TrajectoryRecorder {
    pub enabled: bool,
    pub recording_rate: f64, // Hz
    pub last_record_time: f64,
}

impl Default for TrajectoryRecorder {
    fn default() -> Self {
        Self {
            enabled: false,
            recording_rate: 30.0,
            last_record_time: -1.0,
        }
    }
}

impl TrajectoryRecorder {
    pub fn new(rate: f64) -> Self {
        Self {
            enabled: true,
            recording_rate: rate,
            last_record_time: -1.0,
        }
    }

    pub fn should_record(&mut self, current_time: f64) -> bool {
        if !self.enabled {
            return false;
        }

        let interval = 1.0 / self.recording_rate;
        if current_time - self.last_record_time >= interval {
            self.last_record_time = current_time;
            true
        } else {
            false
        }
    }
}

/// Trajectory recording session
#[derive(Resource, Clone, Debug)]
pub struct RecordingSession {
    pub active: bool,
    pub start_time: f64,
    pub trajectories: HashMap<String, Trajectory>,
    pub session_name: String,
}

impl Default for RecordingSession {
    fn default() -> Self {
        Self {
            active: false,
            start_time: 0.0,
            trajectories: HashMap::new(),
            session_name: String::from("session"),
        }
    }
}

impl RecordingSession {
    pub fn new(session_name: String) -> Self {
        Self {
            active: false,
            start_time: 0.0,
            trajectories: HashMap::new(),
            session_name,
        }
    }

    pub fn start(&mut self, time: f64) {
        self.active = true;
        self.start_time = time;
        self.trajectories.clear();
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn add_trajectory_point(&mut self, entity_name: String, point: TrajectoryPoint) {
        self.trajectories
            .entry(entity_name.clone())
            .or_insert_with(|| Trajectory::new(entity_name))
            .add_point(point);
    }

    /// Save to file
    pub fn save_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let data = serde_json::to_string_pretty(&self.trajectories)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Load from file
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<HashMap<String, Trajectory>> {
        let data = std::fs::read_to_string(path)?;
        let trajectories = serde_json::from_str(&data)?;
        Ok(trajectories)
    }
}

/// System to record trajectories
pub fn record_trajectories_system(
    time: Res<Time>,
    mut session: ResMut<RecordingSession>,
    mut query: Query<(
        &Name,
        &Transform,
        &mut TrajectoryRecorder,
        Option<&Velocity>,
    )>,
) {
    if !session.active {
        return;
    }

    let current_time = time.elapsed_secs_f64();

    for (name, transform, mut recorder, velocity) in query.iter_mut() {
        if recorder.should_record(current_time) {
            let (linear_vel, angular_vel) = if let Some(vel) = velocity {
                (vel.linvel, vel.angvel)
            } else {
                (Vec3::ZERO, Vec3::ZERO)
            };

            let point = TrajectoryPoint {
                timestamp: current_time - session.start_time,
                position: transform.translation,
                rotation: transform.rotation,
                linear_velocity: linear_vel,
                angular_velocity: angular_vel,
            };

            session.add_trajectory_point(name.to_string(), point);
        }
    }
}

/// Velocity component (from rapier)
#[derive(Component, Clone, Debug)]
pub struct Velocity {
    pub linvel: Vec3,
    pub angvel: Vec3,
}

/// Trajectory playback component
#[derive(Component, Clone, Debug)]
pub struct TrajectoryPlayback {
    pub trajectory: Trajectory,
    pub current_time: f64,
    pub playing: bool,
    pub loop_playback: bool,
    pub playback_speed: f64,
}

impl TrajectoryPlayback {
    pub fn new(trajectory: Trajectory) -> Self {
        Self {
            trajectory,
            current_time: 0.0,
            playing: false,
            loop_playback: false,
            playback_speed: 1.0,
        }
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn reset(&mut self) {
        self.current_time = 0.0;
    }

    pub fn is_finished(&self) -> bool {
        self.current_time >= self.trajectory.duration()
    }
}

/// System to playback trajectories
pub fn playback_trajectories_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut TrajectoryPlayback)>,
) {
    let dt = time.delta_secs_f64();

    for (mut transform, mut playback) in query.iter_mut() {
        if !playback.playing {
            continue;
        }

        playback.current_time += dt * playback.playback_speed;

        if playback.is_finished() {
            if playback.loop_playback {
                playback.reset();
            } else {
                playback.pause();
                continue;
            }
        }

        if let Some(point) = playback.trajectory.get_point_at(playback.current_time) {
            transform.translation = point.position;
            transform.rotation = point.rotation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_creation() {
        let traj = Trajectory::new("robot".to_string());
        assert_eq!(traj.entity_name, "robot");
        assert_eq!(traj.points.len(), 0);
    }

    #[test]
    fn test_trajectory_add_point() {
        let mut traj = Trajectory::new("robot".to_string());
        traj.add_point(TrajectoryPoint {
            timestamp: 0.0,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });
        assert_eq!(traj.points.len(), 1);
    }

    #[test]
    fn test_trajectory_duration() {
        let mut traj = Trajectory::new("robot".to_string());
        traj.add_point(TrajectoryPoint {
            timestamp: 0.0,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });
        traj.add_point(TrajectoryPoint {
            timestamp: 5.0,
            position: Vec3::ONE,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });
        assert_eq!(traj.duration(), 5.0);
    }

    #[test]
    fn test_trajectory_interpolation() {
        let mut traj = Trajectory::new("robot".to_string());
        traj.add_point(TrajectoryPoint {
            timestamp: 0.0,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });
        traj.add_point(TrajectoryPoint {
            timestamp: 2.0,
            position: Vec3::new(2.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });

        let point = traj.get_point_at(1.0).unwrap();
        assert!((point.position.x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_trajectory_sampling() {
        let mut traj = Trajectory::new("robot".to_string());
        traj.add_point(TrajectoryPoint {
            timestamp: 0.0,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });
        traj.add_point(TrajectoryPoint {
            timestamp: 1.0,
            position: Vec3::ONE,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });

        let sampled = traj.sample(0.25);
        assert!(sampled.len() >= 4); // At least 0.0, 0.25, 0.5, 0.75, 1.0
    }

    #[test]
    fn test_recorder_should_record() {
        let mut recorder = TrajectoryRecorder::new(10.0); // 10 Hz
        assert!(recorder.should_record(0.0));
        assert!(!recorder.should_record(0.05)); // Too soon
        assert!(recorder.should_record(0.11)); // Enough time passed (>0.1)
    }

    #[test]
    fn test_recording_session() {
        let mut session = RecordingSession::new("test".to_string());
        assert!(!session.active);

        session.start(0.0);
        assert!(session.active);
        assert_eq!(session.start_time, 0.0);

        session.add_trajectory_point(
            "robot".to_string(),
            TrajectoryPoint {
                timestamp: 0.0,
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                linear_velocity: Vec3::ZERO,
                angular_velocity: Vec3::ZERO,
            },
        );

        assert_eq!(session.trajectories.len(), 1);
        session.stop();
        assert!(!session.active);
    }

    #[test]
    fn test_trajectory_playback() {
        let mut traj = Trajectory::new("robot".to_string());
        traj.add_point(TrajectoryPoint {
            timestamp: 0.0,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });
        traj.add_point(TrajectoryPoint {
            timestamp: 1.0,
            position: Vec3::ONE,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
        });

        let mut playback = TrajectoryPlayback::new(traj);
        assert!(!playback.playing);

        playback.play();
        assert!(playback.playing);

        playback.current_time = 1.5;
        assert!(playback.is_finished());
    }
}
