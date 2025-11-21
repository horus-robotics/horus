//! Pure Pursuit Path Following Algorithm
//!
//! Geometric path tracking controller for mobile robots.
//!
//! # Features
//!
//! - Simple and robust path following
//! - Look-ahead distance control
//! - Suitable for differential drive robots
//! - Smooth trajectory tracking
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::pure_pursuit::PurePursuit;
//!
//! let mut pursuit = PurePursuit::new(0.5);  // 0.5m look-ahead
//!
//! // Set path to follow
//! let path = vec![
//!     (0.0, 0.0),
//!     (1.0, 0.0),
//!     (2.0, 1.0),
//!     (3.0, 2.0),
//! ];
//! pursuit.set_path(path);
//!
//! // Compute control at current pose
//! let (linear_vel, angular_vel) = pursuit.compute_velocity(
//!     (0.5, 0.1, 0.0),  // current pose (x, y, theta)
//!     1.0,               // desired linear velocity
//! );
//! ```

/// Pure Pursuit Controller
pub struct PurePursuit {
    path: Vec<(f64, f64)>,
    look_ahead_distance: f64,
    min_look_ahead: f64,
    max_look_ahead: f64,
    current_segment: usize,
    goal_tolerance: f64,
}

impl PurePursuit {
    /// Create new Pure Pursuit controller
    pub fn new(look_ahead_distance: f64) -> Self {
        Self {
            path: Vec::new(),
            look_ahead_distance,
            min_look_ahead: 0.2,
            max_look_ahead: 2.0,
            current_segment: 0,
            goal_tolerance: 0.1,
        }
    }

    /// Set path to follow
    pub fn set_path(&mut self, path: Vec<(f64, f64)>) {
        self.path = path;
        self.current_segment = 0;
    }

    /// Set look-ahead distance
    pub fn set_look_ahead_distance(&mut self, distance: f64) {
        self.look_ahead_distance = distance.clamp(self.min_look_ahead, self.max_look_ahead);
    }

    /// Set look-ahead distance limits
    pub fn set_look_ahead_limits(&mut self, min: f64, max: f64) {
        self.min_look_ahead = min;
        self.max_look_ahead = max;
        self.look_ahead_distance = self.look_ahead_distance.clamp(min, max);
    }

    /// Set goal tolerance
    pub fn set_goal_tolerance(&mut self, tolerance: f64) {
        self.goal_tolerance = tolerance;
    }

    /// Check if goal reached
    pub fn is_goal_reached(&self, current_pose: (f64, f64, f64)) -> bool {
        if self.path.is_empty() {
            return true;
        }

        let goal = *self.path.last().unwrap();
        let dist = self.distance((current_pose.0, current_pose.1), goal);
        dist < self.goal_tolerance
    }

    /// Compute velocity commands
    ///
    /// Returns (linear_velocity, angular_velocity)
    pub fn compute_velocity(
        &mut self,
        current_pose: (f64, f64, f64), // (x, y, theta)
        desired_linear_velocity: f64,
    ) -> (f64, f64) {
        if self.path.is_empty() {
            return (0.0, 0.0);
        }

        // Check if goal reached
        if self.is_goal_reached(current_pose) {
            return (0.0, 0.0);
        }

        // Find look-ahead point
        let look_ahead_point = self.find_look_ahead_point(current_pose);

        // Compute curvature to look-ahead point
        let curvature = self.compute_curvature(current_pose, look_ahead_point);

        // Compute angular velocity
        let angular_velocity = desired_linear_velocity * curvature;

        (desired_linear_velocity, angular_velocity)
    }

    fn find_look_ahead_point(&mut self, current_pose: (f64, f64, f64)) -> (f64, f64) {
        let current_pos = (current_pose.0, current_pose.1);

        // Find closest point on path
        self.update_current_segment(current_pos);

        // Search for look-ahead point
        for i in self.current_segment..self.path.len() {
            let dist = self.distance(current_pos, self.path[i]);

            if dist >= self.look_ahead_distance {
                return self.path[i];
            }
        }

        // If no point found at look-ahead distance, return goal
        *self.path.last().unwrap()
    }

    fn update_current_segment(&mut self, current_pos: (f64, f64)) {
        // Find closest path segment
        let mut min_dist = f64::INFINITY;
        let mut closest_idx = self.current_segment;

        for i in self.current_segment..self.path.len() {
            let dist = self.distance(current_pos, self.path[i]);
            if dist < min_dist {
                min_dist = dist;
                closest_idx = i;
            }
        }

        self.current_segment = closest_idx;
    }

    fn compute_curvature(&self, current_pose: (f64, f64, f64), target: (f64, f64)) -> f64 {
        let (x, y, theta) = current_pose;

        // Transform target to robot frame
        let dx = target.0 - x;
        let dy = target.1 - y;

        let target_x = dx * theta.cos() + dy * theta.sin();
        let target_y = -dx * theta.sin() + dy * theta.cos();

        // Compute curvature
        let l_squared = target_x * target_x + target_y * target_y;

        if l_squared < 1e-6 {
            return 0.0;
        }

        2.0 * target_y / l_squared
    }

    fn distance(&self, p1: (f64, f64), p2: (f64, f64)) -> f64 {
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        (dx * dx + dy * dy).sqrt()
    }

    /// Reset controller state
    pub fn reset(&mut self) {
        self.current_segment = 0;
    }

    /// Get current path
    pub fn get_path(&self) -> &Vec<(f64, f64)> {
        &self.path
    }

    /// Get current segment index
    pub fn get_current_segment(&self) -> usize {
        self.current_segment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_straight_line_path() {
        let mut pursuit = PurePursuit::new(0.5);

        // Straight line path
        let path = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)];
        pursuit.set_path(path);

        // Robot at start, facing forward
        let (linear, angular) = pursuit.compute_velocity((0.0, 0.0, 0.0), 1.0);

        assert_eq!(linear, 1.0);
        assert!(angular.abs() < 0.1, "Should drive straight");
    }

    #[test]
    fn test_curved_path() {
        let mut pursuit = PurePursuit::new(0.5);

        // Curved path
        let path = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 1.0), (3.0, 2.0)];
        pursuit.set_path(path);

        // Robot at start
        let (linear, angular) = pursuit.compute_velocity((0.0, 0.0, 0.0), 1.0);

        assert_eq!(linear, 1.0);
        // Should have some turning
        assert!(angular != 0.0);
    }

    #[test]
    fn test_goal_reached() {
        let mut pursuit = PurePursuit::new(0.5);

        let path = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
        pursuit.set_path(path);

        // Not at goal
        assert!(!pursuit.is_goal_reached((0.0, 0.0, 0.0)));

        // At goal
        assert!(pursuit.is_goal_reached((2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_velocity_at_goal() {
        let mut pursuit = PurePursuit::new(0.5);

        let path = vec![(0.0, 0.0), (1.0, 0.0)];
        pursuit.set_path(path);

        // At goal, should stop
        let (linear, angular) = pursuit.compute_velocity((1.0, 0.0, 0.0), 1.0);

        assert_eq!(linear, 0.0);
        assert_eq!(angular, 0.0);
    }

    #[test]
    fn test_empty_path() {
        let mut pursuit = PurePursuit::new(0.5);

        let (linear, angular) = pursuit.compute_velocity((0.0, 0.0, 0.0), 1.0);

        assert_eq!(linear, 0.0);
        assert_eq!(angular, 0.0);
    }

    #[test]
    fn test_look_ahead_limits() {
        let mut pursuit = PurePursuit::new(0.5);

        pursuit.set_look_ahead_limits(0.3, 1.5);
        pursuit.set_look_ahead_distance(0.1); // Below min
        assert!(pursuit.look_ahead_distance >= 0.3);

        pursuit.set_look_ahead_distance(2.0); // Above max
        assert!(pursuit.look_ahead_distance <= 1.5);
    }

    #[test]
    fn test_segment_tracking() {
        let mut pursuit = PurePursuit::new(0.5);

        let path = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)];
        pursuit.set_path(path);

        // Initially at segment 0
        assert_eq!(pursuit.get_current_segment(), 0);

        // Move forward
        pursuit.compute_velocity((1.5, 0.0, 0.0), 1.0);
        assert!(pursuit.get_current_segment() > 0);
    }

    #[test]
    fn test_reset() {
        let mut pursuit = PurePursuit::new(0.5);

        let path = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
        pursuit.set_path(path);

        pursuit.compute_velocity((1.5, 0.0, 0.0), 1.0);
        assert!(pursuit.get_current_segment() > 0);

        pursuit.reset();
        assert_eq!(pursuit.get_current_segment(), 0);
    }
}
