//! Lock-step synchronization for deterministic multi-robot simulation

use super::RobotId;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

/// Synchronization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// No synchronization (real-time)
    None,
    /// Lock-step (all robots advance together)
    LockStep,
    /// Barrier synchronization (wait for all at barriers)
    Barrier,
}

/// Synchronization barrier
#[derive(Debug, Clone)]
pub struct SyncBarrier {
    /// Barrier ID
    pub id: String,
    /// Robots that must reach this barrier
    pub required_robots: HashSet<RobotId>,
    /// Robots that have reached this barrier
    pub reached_robots: HashSet<RobotId>,
}

impl SyncBarrier {
    pub fn new(id: impl Into<String>, robots: Vec<RobotId>) -> Self {
        Self {
            id: id.into(),
            required_robots: robots.into_iter().collect(),
            reached_robots: HashSet::new(),
        }
    }

    /// Mark a robot as having reached the barrier
    pub fn arrive(&mut self, robot_id: RobotId) -> bool {
        self.reached_robots.insert(robot_id)
    }

    /// Check if all robots have reached the barrier
    pub fn is_complete(&self) -> bool {
        self.required_robots == self.reached_robots
    }

    /// Reset the barrier
    pub fn reset(&mut self) {
        self.reached_robots.clear();
    }

    /// Get number of waiting robots
    pub fn waiting_count(&self) -> usize {
        self.required_robots.len() - self.reached_robots.len()
    }
}

/// Synchronization manager resource
#[derive(Resource)]
pub struct SynchronizationManager {
    /// Current synchronization mode
    pub mode: SyncMode,
    /// Current simulation step
    pub step: u64,
    /// Robots ready for next step
    ready_robots: HashSet<RobotId>,
    /// All registered robots
    registered_robots: HashSet<RobotId>,
    /// Active barriers
    barriers: HashMap<String, SyncBarrier>,
    /// Fixed time step for lock-step mode (seconds)
    pub fixed_timestep: f64,
    /// Accumulated time
    accumulated_time: f64,
}

impl Default for SynchronizationManager {
    fn default() -> Self {
        Self::new(SyncMode::None)
    }
}

impl SynchronizationManager {
    pub fn new(mode: SyncMode) -> Self {
        Self {
            mode,
            step: 0,
            ready_robots: HashSet::new(),
            registered_robots: HashSet::new(),
            barriers: HashMap::new(),
            fixed_timestep: 1.0 / 60.0, // 60 Hz default
            accumulated_time: 0.0,
        }
    }

    /// Register a robot for synchronization
    pub fn register_robot(&mut self, robot_id: RobotId) {
        self.registered_robots.insert(robot_id);
    }

    /// Unregister a robot
    pub fn unregister_robot(&mut self, robot_id: &RobotId) {
        self.registered_robots.remove(robot_id);
        self.ready_robots.remove(robot_id);
    }

    /// Mark a robot as ready for next step
    pub fn mark_ready(&mut self, robot_id: RobotId) {
        if self.registered_robots.contains(&robot_id) {
            self.ready_robots.insert(robot_id);
        }
    }

    /// Check if all robots are ready
    pub fn all_ready(&self) -> bool {
        !self.registered_robots.is_empty()
            && self.ready_robots.len() == self.registered_robots.len()
    }

    /// Advance to next step (only if all ready)
    pub fn try_step(&mut self) -> bool {
        if self.all_ready() {
            self.step += 1;
            self.ready_robots.clear();
            true
        } else {
            false
        }
    }

    /// Force step (regardless of ready state)
    pub fn force_step(&mut self) {
        self.step += 1;
        self.ready_robots.clear();
    }

    /// Get current step
    pub fn current_step(&self) -> u64 {
        self.step
    }

    /// Get number of robots waiting
    pub fn waiting_count(&self) -> usize {
        self.registered_robots.len() - self.ready_robots.len()
    }

    /// Create a synchronization barrier
    pub fn create_barrier(&mut self, id: impl Into<String>, robots: Vec<RobotId>) {
        let barrier = SyncBarrier::new(id, robots);
        self.barriers.insert(barrier.id.clone(), barrier);
    }

    /// Mark robot as having reached a barrier
    pub fn arrive_barrier(&mut self, barrier_id: &str, robot_id: RobotId) -> bool {
        if let Some(barrier) = self.barriers.get_mut(barrier_id) {
            barrier.arrive(robot_id)
        } else {
            false
        }
    }

    /// Check if barrier is complete
    pub fn is_barrier_complete(&self, barrier_id: &str) -> bool {
        self.barriers
            .get(barrier_id)
            .is_some_and(|b| b.is_complete())
    }

    /// Reset a barrier
    pub fn reset_barrier(&mut self, barrier_id: &str) {
        if let Some(barrier) = self.barriers.get_mut(barrier_id) {
            barrier.reset();
        }
    }

    /// Remove a barrier
    pub fn remove_barrier(&mut self, barrier_id: &str) {
        self.barriers.remove(barrier_id);
    }

    /// Update with real time (for lock-step mode)
    pub fn update(&mut self, delta_time: f64) -> bool {
        match self.mode {
            SyncMode::LockStep => {
                self.accumulated_time += delta_time;
                if self.accumulated_time >= self.fixed_timestep {
                    self.accumulated_time -= self.fixed_timestep;
                    self.force_step();
                    true
                } else {
                    false
                }
            }
            _ => {
                // No time stepping for other modes
                false
            }
        }
    }

    /// Get time remaining until next step
    pub fn time_until_next_step(&self) -> f64 {
        if self.mode == SyncMode::LockStep {
            self.fixed_timestep - self.accumulated_time
        } else {
            0.0
        }
    }

    /// Set synchronization mode
    pub fn set_mode(&mut self, mode: SyncMode) {
        self.mode = mode;
        self.accumulated_time = 0.0;
    }

    /// Get registered robot count
    pub fn robot_count(&self) -> usize {
        self.registered_robots.len()
    }

    /// Get all registered robots
    pub fn registered_robots(&self) -> Vec<RobotId> {
        self.registered_robots.iter().cloned().collect()
    }
}

/// Component to mark robots that should be synchronized
#[derive(Component, Default)]
pub struct Synchronized {
    /// Whether this robot is currently waiting
    pub waiting: bool,
}

/// System to handle lock-step synchronization
pub fn lock_step_sync_system(mut sync_manager: ResMut<SynchronizationManager>, time: Res<Time>) {
    if sync_manager.mode == SyncMode::LockStep {
        sync_manager.update(time.delta_secs_f64());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_barrier() {
        let mut barrier =
            SyncBarrier::new("test", vec![RobotId::new("robot1"), RobotId::new("robot2")]);

        assert!(!barrier.is_complete());
        assert_eq!(barrier.waiting_count(), 2);

        barrier.arrive(RobotId::new("robot1"));
        assert!(!barrier.is_complete());
        assert_eq!(barrier.waiting_count(), 1);

        barrier.arrive(RobotId::new("robot2"));
        assert!(barrier.is_complete());
        assert_eq!(barrier.waiting_count(), 0);
    }

    #[test]
    fn test_sync_barrier_reset() {
        let mut barrier = SyncBarrier::new("test", vec![RobotId::new("robot1")]);

        barrier.arrive(RobotId::new("robot1"));
        assert!(barrier.is_complete());

        barrier.reset();
        assert!(!barrier.is_complete());
        assert_eq!(barrier.waiting_count(), 1);
    }

    #[test]
    fn test_sync_manager() {
        let mut manager = SynchronizationManager::new(SyncMode::LockStep);

        manager.register_robot(RobotId::new("robot1"));
        manager.register_robot(RobotId::new("robot2"));

        assert_eq!(manager.current_step(), 0);
        assert!(!manager.all_ready());
    }

    #[test]
    fn test_sync_ready() {
        let mut manager = SynchronizationManager::new(SyncMode::LockStep);

        manager.register_robot(RobotId::new("robot1"));
        manager.register_robot(RobotId::new("robot2"));

        manager.mark_ready(RobotId::new("robot1"));
        assert!(!manager.all_ready());
        assert_eq!(manager.waiting_count(), 1);

        manager.mark_ready(RobotId::new("robot2"));
        assert!(manager.all_ready());
        assert_eq!(manager.waiting_count(), 0);
    }

    #[test]
    fn test_sync_step() {
        let mut manager = SynchronizationManager::new(SyncMode::LockStep);

        manager.register_robot(RobotId::new("robot1"));
        manager.register_robot(RobotId::new("robot2"));

        manager.mark_ready(RobotId::new("robot1"));
        assert!(!manager.try_step());

        manager.mark_ready(RobotId::new("robot2"));
        assert!(manager.try_step());
        assert_eq!(manager.current_step(), 1);

        // Ready flags should be cleared
        assert!(!manager.all_ready());
    }

    #[test]
    fn test_sync_force_step() {
        let mut manager = SynchronizationManager::new(SyncMode::LockStep);

        manager.register_robot(RobotId::new("robot1"));
        assert_eq!(manager.current_step(), 0);

        manager.force_step();
        assert_eq!(manager.current_step(), 1);
    }

    #[test]
    fn test_sync_barriers() {
        let mut manager = SynchronizationManager::new(SyncMode::Barrier);

        manager.create_barrier(
            "barrier1",
            vec![RobotId::new("robot1"), RobotId::new("robot2")],
        );

        assert!(!manager.is_barrier_complete("barrier1"));

        manager.arrive_barrier("barrier1", RobotId::new("robot1"));
        assert!(!manager.is_barrier_complete("barrier1"));

        manager.arrive_barrier("barrier1", RobotId::new("robot2"));
        assert!(manager.is_barrier_complete("barrier1"));
    }

    #[test]
    fn test_sync_barrier_reset_manager() {
        let mut manager = SynchronizationManager::new(SyncMode::Barrier);

        manager.create_barrier("barrier1", vec![RobotId::new("robot1")]);
        manager.arrive_barrier("barrier1", RobotId::new("robot1"));
        assert!(manager.is_barrier_complete("barrier1"));

        manager.reset_barrier("barrier1");
        assert!(!manager.is_barrier_complete("barrier1"));
    }

    #[test]
    fn test_lock_step_timing() {
        let mut manager = SynchronizationManager::new(SyncMode::LockStep);
        manager.fixed_timestep = 0.1;

        assert_eq!(manager.current_step(), 0);

        // Accumulate less than timestep
        assert!(!manager.update(0.05));
        assert_eq!(manager.current_step(), 0);

        // Accumulate to reach timestep
        assert!(manager.update(0.06));
        assert_eq!(manager.current_step(), 1);
    }

    #[test]
    fn test_time_until_next_step() {
        let mut manager = SynchronizationManager::new(SyncMode::LockStep);
        manager.fixed_timestep = 0.1;

        assert_eq!(manager.time_until_next_step(), 0.1);

        manager.update(0.03);
        assert!((manager.time_until_next_step() - 0.07).abs() < 0.001);
    }

    #[test]
    fn test_unregister_robot() {
        let mut manager = SynchronizationManager::new(SyncMode::LockStep);

        let robot1 = RobotId::new("robot1");
        let robot2 = RobotId::new("robot2");

        manager.register_robot(robot1.clone());
        manager.register_robot(robot2.clone());
        assert_eq!(manager.robot_count(), 2);

        manager.unregister_robot(&robot1);
        assert_eq!(manager.robot_count(), 1);

        manager.mark_ready(robot2);
        assert!(manager.all_ready());
    }

    #[test]
    fn test_mode_switching() {
        let mut manager = SynchronizationManager::new(SyncMode::None);
        assert_eq!(manager.mode, SyncMode::None);

        manager.set_mode(SyncMode::LockStep);
        assert_eq!(manager.mode, SyncMode::LockStep);
        assert_eq!(manager.accumulated_time, 0.0);
    }
}
