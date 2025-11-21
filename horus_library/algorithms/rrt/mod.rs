//! RRT (Rapidly-exploring Random Tree) Motion Planning
//!
//! Sampling-based path planning for complex and high-dimensional spaces.
//!
//! # Features
//!
//! - Probabilistically complete path planning
//! - Handles complex obstacle environments
//! - No grid discretization required
//! - Configurable sampling and growth parameters
//! - Goal biasing for faster convergence
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::rrt::RRT;
//!
//! let mut rrt = RRT::new(
//!     (0.0, 0.0),     // start
//!     (10.0, 10.0),   // goal
//!     (-5.0, -5.0),   // bounds min
//!     (15.0, 15.0),   // bounds max
//! );
//!
//! // Add circular obstacle
//! rrt.add_obstacle((5.0, 5.0), 1.0);
//!
//! // Configure and plan
//! rrt.set_max_iterations(2000);
//! rrt.set_step_size(0.5);
//!
//! if let Some(path) = rrt.plan() {
//!     println!("Found path with {} waypoints", path.len());
//! }
//! ```

use rand::Rng;

/// RRT Tree Node
#[derive(Debug, Clone)]
struct Node {
    position: (f64, f64),
    parent: Option<usize>,
}

/// Circular obstacle
#[derive(Debug, Clone)]
pub struct Obstacle {
    pub center: (f64, f64),
    pub radius: f64,
}

/// RRT Motion Planner
pub struct RRT {
    start: (f64, f64),
    goal: (f64, f64),
    bounds_min: (f64, f64),
    bounds_max: (f64, f64),

    tree: Vec<Node>,
    obstacles: Vec<Obstacle>,

    max_iterations: usize,
    step_size: f64,
    goal_tolerance: f64,
    goal_bias: f64,

    goal_node_index: Option<usize>,
}

impl RRT {
    /// Create new RRT planner
    pub fn new(
        start: (f64, f64),
        goal: (f64, f64),
        bounds_min: (f64, f64),
        bounds_max: (f64, f64),
    ) -> Self {
        let start_node = Node {
            position: start,
            parent: None,
        };

        Self {
            start,
            goal,
            bounds_min,
            bounds_max,
            tree: vec![start_node],
            obstacles: Vec::new(),
            max_iterations: 1000,
            step_size: 0.5,
            goal_tolerance: 0.3,
            goal_bias: 0.1,
            goal_node_index: None,
        }
    }

    /// Set maximum iterations
    pub fn set_max_iterations(&mut self, max_iterations: usize) {
        self.max_iterations = max_iterations;
    }

    /// Set step size for tree extension
    pub fn set_step_size(&mut self, step_size: f64) {
        self.step_size = step_size;
    }

    /// Set goal tolerance
    pub fn set_goal_tolerance(&mut self, tolerance: f64) {
        self.goal_tolerance = tolerance;
    }

    /// Set goal bias probability (0.0 to 1.0)
    pub fn set_goal_bias(&mut self, bias: f64) {
        self.goal_bias = bias.clamp(0.0, 1.0);
    }

    /// Add circular obstacle
    pub fn add_obstacle(&mut self, center: (f64, f64), radius: f64) {
        self.obstacles.push(Obstacle { center, radius });
    }

    /// Clear all obstacles
    pub fn clear_obstacles(&mut self) {
        self.obstacles.clear();
    }

    /// Reset planner (clear tree, keep configuration)
    pub fn reset(&mut self) {
        let start_node = Node {
            position: self.start,
            parent: None,
        };
        self.tree = vec![start_node];
        self.goal_node_index = None;
    }

    /// Get tree size
    pub fn tree_size(&self) -> usize {
        self.tree.len()
    }

    /// Plan path from start to goal
    pub fn plan(&mut self) -> Option<Vec<(f64, f64)>> {
        let mut rng = rand::thread_rng();

        for _ in 0..self.max_iterations {
            // Sample random point (with goal bias)
            let sample = if rng.gen::<f64>() < self.goal_bias {
                self.goal
            } else {
                self.sample_random_point(&mut rng)
            };

            // Find nearest node in tree
            let nearest_idx = self.nearest_node(sample);

            // Extend tree toward sample
            let new_pos = self.steer(self.tree[nearest_idx].position, sample);

            // Check collision
            if self.is_collision_free(self.tree[nearest_idx].position, new_pos) {
                let new_node = Node {
                    position: new_pos,
                    parent: Some(nearest_idx),
                };

                let new_idx = self.tree.len();
                self.tree.push(new_node);

                // Check if goal reached
                if self.distance(new_pos, self.goal) < self.goal_tolerance {
                    self.goal_node_index = Some(new_idx);
                    return Some(self.extract_path(new_idx));
                }
            }
        }

        None // No path found within max iterations
    }

    /// Calculate path cost
    pub fn path_cost(path: &[(f64, f64)]) -> f64 {
        path.windows(2)
            .map(|w| {
                let dx = w[1].0 - w[0].0;
                let dy = w[1].1 - w[0].1;
                (dx * dx + dy * dy).sqrt()
            })
            .sum()
    }

    fn sample_random_point(&self, rng: &mut impl Rng) -> (f64, f64) {
        let x = rng.gen_range(self.bounds_min.0..self.bounds_max.0);
        let y = rng.gen_range(self.bounds_min.1..self.bounds_max.1);
        (x, y)
    }

    fn nearest_node(&self, point: (f64, f64)) -> usize {
        self.tree
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let dist_a = self.distance(a.position, point);
                let dist_b = self.distance(b.position, point);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .map(|(idx, _)| idx)
            .unwrap()
    }

    fn steer(&self, from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
        let dist = self.distance(from, to);

        if dist < self.step_size {
            return to;
        }

        let theta = (to.1 - from.1).atan2(to.0 - from.0);
        let new_x = from.0 + self.step_size * theta.cos();
        let new_y = from.1 + self.step_size * theta.sin();

        (new_x, new_y)
    }

    fn distance(&self, p1: (f64, f64), p2: (f64, f64)) -> f64 {
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        (dx * dx + dy * dy).sqrt()
    }

    fn is_collision_free(&self, from: (f64, f64), to: (f64, f64)) -> bool {
        // Check bounds
        if !self.is_within_bounds(to) {
            return false;
        }

        // Check obstacles
        for obstacle in &self.obstacles {
            if self.line_circle_collision(from, to, obstacle.center, obstacle.radius) {
                return false;
            }
        }

        true
    }

    fn is_within_bounds(&self, point: (f64, f64)) -> bool {
        point.0 >= self.bounds_min.0
            && point.0 <= self.bounds_max.0
            && point.1 >= self.bounds_min.1
            && point.1 <= self.bounds_max.1
    }

    fn line_circle_collision(
        &self,
        p1: (f64, f64),
        p2: (f64, f64),
        center: (f64, f64),
        radius: f64,
    ) -> bool {
        let dx = p2.0 - p1.0;
        let dy = p2.1 - p1.1;
        let fx = p1.0 - center.0;
        let fy = p1.1 - center.1;

        let a = dx * dx + dy * dy;
        let b = 2.0 * (fx * dx + fy * dy);
        let c = fx * fx + fy * fy - radius * radius;

        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return false;
        }

        let t1 = (-b - discriminant.sqrt()) / (2.0 * a);
        let t2 = (-b + discriminant.sqrt()) / (2.0 * a);

        (0.0..=1.0).contains(&t1) || (0.0..=1.0).contains(&t2) || (t1 < 0.0 && t2 > 1.0)
    }

    fn extract_path(&self, goal_idx: usize) -> Vec<(f64, f64)> {
        let mut path = Vec::new();
        let mut current = Some(goal_idx);

        while let Some(idx) = current {
            path.push(self.tree[idx].position);
            current = self.tree[idx].parent;
        }

        path.reverse();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_planning() {
        let mut rrt = RRT::new((0.0, 0.0), (10.0, 10.0), (-5.0, -5.0), (15.0, 15.0));

        rrt.set_max_iterations(2000);
        let result = rrt.plan();

        assert!(result.is_some(), "Should find a path in open space");

        let path = result.unwrap();
        assert!(path.len() >= 2, "Path should have at least start and goal");
        assert_eq!(path[0], (0.0, 0.0), "Path should start at origin");

        // Goal should be close to target
        let last = path.last().unwrap();
        let dist_to_goal = ((last.0 - 10.0).powi(2) + (last.1 - 10.0).powi(2)).sqrt();
        assert!(dist_to_goal < 0.5, "Path should reach near goal");
    }

    #[test]
    fn test_with_obstacle() {
        let mut rrt = RRT::new((0.0, 0.0), (10.0, 10.0), (-5.0, -5.0), (15.0, 15.0));

        rrt.set_max_iterations(3000);
        rrt.add_obstacle((5.0, 5.0), 2.0);

        let result = rrt.plan();
        assert!(result.is_some(), "Should find path around obstacle");

        let path = result.unwrap();

        // Verify no path point is inside obstacle
        for point in &path {
            let dx = point.0 - 5.0;
            let dy = point.1 - 5.0;
            let dist = (dx * dx + dy * dy).sqrt();
            assert!(dist >= 2.0, "Path should not pass through obstacle");
        }
    }

    #[test]
    fn test_goal_bias() {
        let mut rrt = RRT::new((0.0, 0.0), (10.0, 10.0), (-5.0, -5.0), (15.0, 15.0));

        // High goal bias should find path faster
        rrt.set_max_iterations(1000);
        rrt.set_goal_bias(0.3);

        let result = rrt.plan();
        assert!(
            result.is_some(),
            "High goal bias should help find path quickly"
        );
    }

    #[test]
    fn test_tree_growth() {
        let mut rrt = RRT::new((0.0, 0.0), (10.0, 10.0), (-5.0, -5.0), (15.0, 15.0));

        rrt.set_max_iterations(100);
        rrt.plan();

        let tree_size = rrt.tree_size();
        assert!(tree_size > 50, "Tree should grow with iterations");
    }

    #[test]
    fn test_reset() {
        let mut rrt = RRT::new((0.0, 0.0), (10.0, 10.0), (-5.0, -5.0), (15.0, 15.0));

        rrt.set_max_iterations(500);
        rrt.plan();

        let size_before = rrt.tree_size();
        assert!(size_before > 1);

        rrt.reset();

        let size_after = rrt.tree_size();
        assert_eq!(
            size_after, 1,
            "Tree should only have start node after reset"
        );
    }

    #[test]
    fn test_path_cost() {
        let path = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (2.0, 1.0)];

        let cost = RRT::path_cost(&path);
        assert!((cost - 3.0).abs() < 0.01);
    }
}
