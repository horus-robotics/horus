//! A* (A-Star) Pathfinding Algorithm
//!
//! Grid-based optimal pathfinding using heuristic search.
//!
//! # Features
//!
//! - Optimal pathfinding with admissible heuristics
//! - 8-directional movement (diagonal allowed)
//! - Configurable heuristic weight for speed/optimality tradeoff
//! - Euclidean and Manhattan distance heuristics
//! - Obstacle-aware grid navigation
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::astar::AStar;
//!
//! let mut astar = AStar::new(100, 100);
//!
//! // Set start and goal
//! astar.set_start(10, 10);
//! astar.set_goal(90, 90);
//!
//! // Add obstacles
//! astar.set_obstacle(50, 50);
//!
//! // Find path
//! if let Some(path) = astar.plan() {
//!     println!("Found path with {} waypoints", path.len());
//!     for (x, y) in path {
//!         println!("  ({}, {})", x, y);
//!     }
//! }
//! ```

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// A* Node in the search tree
#[derive(Clone, Debug)]
struct Node {
    x: i32,
    y: i32,
    g_cost: f64, // Cost from start
    h_cost: f64, // Heuristic cost to goal
    f_cost: f64, // Total cost (g + h)
    parent: Option<(i32, i32)>,
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Heuristic function types
#[derive(Debug, Clone, Copy)]
pub enum Heuristic {
    /// Euclidean distance (best for 8-directional movement)
    Euclidean,
    /// Manhattan distance (best for 4-directional movement)
    Manhattan,
    /// Diagonal distance (Chebyshev distance)
    Diagonal,
}

/// A* Pathfinding Algorithm
pub struct AStar {
    width: usize,
    height: usize,
    grid: Vec<Vec<bool>>, // true = obstacle, false = free
    start: (i32, i32),
    goal: (i32, i32),
    heuristic: Heuristic,
    heuristic_weight: f64,
    allow_diagonal: bool,
}

impl AStar {
    /// Create new A* planner with grid dimensions
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            grid: vec![vec![false; width]; height],
            start: (0, 0),
            goal: (0, 0),
            heuristic: Heuristic::Euclidean,
            heuristic_weight: 1.0,
            allow_diagonal: true,
        }
    }

    /// Set start position
    pub fn set_start(&mut self, x: i32, y: i32) {
        self.start = (x, y);
    }

    /// Set goal position
    pub fn set_goal(&mut self, x: i32, y: i32) {
        self.goal = (x, y);
    }

    /// Set obstacle at position
    pub fn set_obstacle(&mut self, x: i32, y: i32) {
        if self.is_valid(x, y) {
            self.grid[y as usize][x as usize] = true;
        }
    }

    /// Clear obstacle at position
    pub fn clear_obstacle(&mut self, x: i32, y: i32) {
        if self.is_valid(x, y) {
            self.grid[y as usize][x as usize] = false;
        }
    }

    /// Set entire grid (true = obstacle, false = free)
    pub fn set_grid(&mut self, grid: Vec<Vec<bool>>) {
        if grid.len() == self.height && grid[0].len() == self.width {
            self.grid = grid;
        }
    }

    /// Set heuristic function
    pub fn set_heuristic(&mut self, heuristic: Heuristic) {
        self.heuristic = heuristic;
    }

    /// Set heuristic weight (1.0 = optimal, >1.0 = faster but suboptimal)
    pub fn set_heuristic_weight(&mut self, weight: f64) {
        self.heuristic_weight = weight.max(1.0);
    }

    /// Allow or disallow diagonal movement
    pub fn set_allow_diagonal(&mut self, allow: bool) {
        self.allow_diagonal = allow;
    }

    /// Clear all obstacles
    pub fn clear_obstacles(&mut self) {
        for row in &mut self.grid {
            row.fill(false);
        }
    }

    /// Plan path from start to goal
    pub fn plan(&self) -> Option<Vec<(i32, i32)>> {
        // Validate start and goal
        if !self.is_valid(self.start.0, self.start.1) || !self.is_valid(self.goal.0, self.goal.1) {
            return None;
        }

        if self.is_obstacle(self.start.0, self.start.1)
            || self.is_obstacle(self.goal.0, self.goal.1)
        {
            return None;
        }

        // Check if already at goal
        if self.start == self.goal {
            return Some(vec![self.start]);
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut nodes = HashMap::new();

        // Initialize start node
        let start_node = Node {
            x: self.start.0,
            y: self.start.1,
            g_cost: 0.0,
            h_cost: self.heuristic_cost(self.start.0, self.start.1),
            f_cost: self.heuristic_cost(self.start.0, self.start.1),
            parent: None,
        };

        open_set.push(start_node.clone());
        nodes.insert((self.start.0, self.start.1), start_node);

        while let Some(current) = open_set.pop() {
            let current_pos = (current.x, current.y);

            // Check if reached goal
            if current_pos == self.goal {
                return Some(self.reconstruct_path(&nodes, current_pos));
            }

            // Skip if already processed
            if closed_set.contains(&current_pos) {
                continue;
            }

            closed_set.insert(current_pos);

            // Explore neighbors
            for (nx, ny) in self.get_neighbors(current.x, current.y) {
                let neighbor_pos = (nx, ny);

                if closed_set.contains(&neighbor_pos) {
                    continue;
                }

                // Calculate cost to neighbor
                let move_cost = if (nx - current.x).abs() + (ny - current.y).abs() == 2 {
                    std::f64::consts::SQRT_2 // Diagonal movement
                } else {
                    1.0 // Straight movement
                };

                let tentative_g = current.g_cost + move_cost;

                // Check if this path is better
                let should_update = if let Some(existing) = nodes.get(&neighbor_pos) {
                    tentative_g < existing.g_cost
                } else {
                    true
                };

                if should_update {
                    let h_cost = self.heuristic_cost(nx, ny);
                    let neighbor_node = Node {
                        x: nx,
                        y: ny,
                        g_cost: tentative_g,
                        h_cost,
                        f_cost: tentative_g + h_cost,
                        parent: Some(current_pos),
                    };

                    open_set.push(neighbor_node.clone());
                    nodes.insert(neighbor_pos, neighbor_node);
                }
            }
        }

        None // No path found
    }

    fn is_valid(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32
    }

    fn is_obstacle(&self, x: i32, y: i32) -> bool {
        if !self.is_valid(x, y) {
            return true;
        }
        self.grid[y as usize][x as usize]
    }

    fn heuristic_cost(&self, x: i32, y: i32) -> f64 {
        let dx = (self.goal.0 - x).abs() as f64;
        let dy = (self.goal.1 - y).abs() as f64;

        let cost = match self.heuristic {
            Heuristic::Euclidean => (dx * dx + dy * dy).sqrt(),
            Heuristic::Manhattan => dx + dy,
            Heuristic::Diagonal => dx.max(dy) + (std::f64::consts::SQRT_2 - 1.0) * dx.min(dy),
        };

        cost * self.heuristic_weight
    }

    fn get_neighbors(&self, x: i32, y: i32) -> Vec<(i32, i32)> {
        let mut neighbors = Vec::new();

        // 4-directional movement
        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];

        for (dx, dy) in directions {
            let nx = x + dx;
            let ny = y + dy;

            if self.is_valid(nx, ny) && !self.is_obstacle(nx, ny) {
                neighbors.push((nx, ny));
            }
        }

        // Diagonal movement
        if self.allow_diagonal {
            let diagonals = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

            for (dx, dy) in diagonals {
                let nx = x + dx;
                let ny = y + dy;

                if self.is_valid(nx, ny) && !self.is_obstacle(nx, ny) {
                    // Check for diagonal corner cutting
                    let check_x = self.is_obstacle(x + dx, y);
                    let check_y = self.is_obstacle(x, y + dy);

                    if !check_x && !check_y {
                        neighbors.push((nx, ny));
                    }
                }
            }
        }

        neighbors
    }

    fn reconstruct_path(
        &self,
        nodes: &HashMap<(i32, i32), Node>,
        mut current: (i32, i32),
    ) -> Vec<(i32, i32)> {
        let mut path = vec![current];

        while let Some(node) = nodes.get(&current) {
            if let Some(parent) = node.parent {
                path.push(parent);
                current = parent;
            } else {
                break;
            }
        }

        path.reverse();
        path
    }

    /// Calculate path length
    pub fn path_length(path: &[(i32, i32)]) -> f64 {
        path.windows(2)
            .map(|w| {
                let dx = (w[1].0 - w[0].0) as f64;
                let dy = (w[1].1 - w[0].1) as f64;
                (dx * dx + dy * dy).sqrt()
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_pathfinding() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(0, 0);
        astar.set_goal(9, 9);

        let path = astar.plan();
        assert!(path.is_some());

        let path = path.unwrap();
        assert_eq!(path[0], (0, 0));
        assert_eq!(path[path.len() - 1], (9, 9));
    }

    #[test]
    fn test_with_obstacle() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(0, 0);
        astar.set_goal(9, 0);

        // Create wall
        for y in 0..8 {
            astar.set_obstacle(5, y);
        }

        let path = astar.plan();
        assert!(path.is_some());

        let path = path.unwrap();
        // Path should go around obstacle
        assert!(path.len() > 10);
    }

    #[test]
    fn test_no_path() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(0, 0);
        astar.set_goal(9, 9);

        // Create complete wall
        for y in 0..10 {
            astar.set_obstacle(5, y);
        }

        let path = astar.plan();
        assert!(path.is_none());
    }

    #[test]
    fn test_start_is_goal() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(5, 5);
        astar.set_goal(5, 5);

        let path = astar.plan();
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 1);
    }

    #[test]
    fn test_heuristic_weight() {
        let mut astar = AStar::new(20, 20);
        astar.set_start(0, 0);
        astar.set_goal(19, 19);

        // Optimal (weight = 1.0)
        astar.set_heuristic_weight(1.0);
        let path1 = astar.plan().unwrap();
        let len1 = AStar::path_length(&path1);

        // Faster but suboptimal (weight = 2.0)
        astar.set_heuristic_weight(2.0);
        let path2 = astar.plan().unwrap();
        let len2 = AStar::path_length(&path2);

        // Both should find paths
        assert!(path1.len() > 0);
        assert!(path2.len() > 0);

        // Weighted might be slightly longer (or equal in open space)
        assert!(len2 >= len1 * 0.9); // Within 10% tolerance
    }
}
