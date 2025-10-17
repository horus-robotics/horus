use horus_core::{Node, NodeInfo, Hub};
use crate::{PathPlan, Odometry, LaserScan};
use std::collections::{HashMap, BinaryHeap, HashSet};
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

/// Path Planner Node - A* and RRT path planning for autonomous navigation
///
/// Plans collision-free paths from current position to goal using A* algorithm
/// for grid-based environments and RRT for complex obstacle spaces.
pub struct PathPlannerNode {
    plan_publisher: Hub<PathPlan>,
    odometry_subscriber: Hub<Odometry>,
    lidar_subscriber: Hub<LaserScan>,
    goal_subscriber: Hub<PathPlan>, // Receives goal positions

    // Current state
    current_pose: (f64, f64, f64), // (x, y, theta)
    goal_pose: (f64, f64, f64),
    grid_map: Vec<Vec<bool>>, // Occupancy grid: true = obstacle, false = free

    // Configuration
    grid_resolution: f64, // meters per cell
    grid_width: usize,
    grid_height: usize,
    robot_radius: f64,
    planning_algorithm: PlanningAlgorithm,

    // Path planning state
    current_path: Vec<(f64, f64)>,
    path_valid: bool,
    replanning_threshold: f64, // replan if deviation > threshold

    // A* parameters
    heuristic_weight: f64,

    // RRT parameters
    rrt_max_iterations: usize,
    rrt_step_size: f64,
    rrt_goal_bias: f64,
}

#[derive(Clone, Copy)]
enum PlanningAlgorithm {
    AStar,
    RRT,
}

#[derive(Clone, Debug)]
struct AStarNode {
    x: i32,
    y: i32,
    g_cost: f64, // Cost from start
    h_cost: f64, // Heuristic cost to goal
    f_cost: f64, // Total cost
    parent: Option<(i32, i32)>,
}

impl Eq for AStarNode {}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PathPlannerNode {
    /// Create a new path planner node with default topics
    pub fn new() -> Self {
        Self::new_with_topics("path_plan", "odom", "lidar_scan", "goal")
    }

    /// Create a new path planner node with custom topics
    pub fn new_with_topics(plan_topic: &str, odom_topic: &str, lidar_topic: &str, goal_topic: &str) -> Self {
        Self {
            plan_publisher: Hub::new(plan_topic).expect("Failed to create path plan publisher"),
            odometry_subscriber: Hub::new(odom_topic).expect("Failed to subscribe to odometry"),
            lidar_subscriber: Hub::new(lidar_topic).expect("Failed to subscribe to lidar"),
            goal_subscriber: Hub::new(goal_topic).expect("Failed to subscribe to goals"),

            current_pose: (0.0, 0.0, 0.0),
            goal_pose: (0.0, 0.0, 0.0),
            grid_map: Vec::new(),

            grid_resolution: 0.1, // 10cm resolution
            grid_width: 200,      // 20m x 20m grid
            grid_height: 200,
            robot_radius: 0.3,    // 30cm robot radius
            planning_algorithm: PlanningAlgorithm::AStar,

            current_path: Vec::new(),
            path_valid: false,
            replanning_threshold: 0.5, // 50cm deviation

            heuristic_weight: 1.0,

            rrt_max_iterations: 1000,
            rrt_step_size: 0.5,
            rrt_goal_bias: 0.1,
        }
    }

    /// Set grid map parameters
    pub fn set_grid_config(&mut self, resolution: f64, width: usize, height: usize) {
        self.grid_resolution = resolution;
        self.grid_width = width;
        self.grid_height = height;

        // Initialize empty grid
        self.grid_map = vec![vec![false; width]; height];
    }

    /// Set robot radius for collision checking
    pub fn set_robot_radius(&mut self, radius: f64) {
        self.robot_radius = radius;
    }

    /// Set planning algorithm
    pub fn set_algorithm(&mut self, use_rrt: bool) {
        self.planning_algorithm = if use_rrt {
            PlanningAlgorithm::RRT
        } else {
            PlanningAlgorithm::AStar
        };
    }

    /// Set goal position
    pub fn set_goal(&mut self, x: f64, y: f64, theta: f64) {
        self.goal_pose = (x, y, theta);
        self.path_valid = false; // Invalidate current path
    }

    /// Get current path
    pub fn get_path(&self) -> &Vec<(f64, f64)> {
        &self.current_path
    }

    /// Check if path is valid
    pub fn is_path_valid(&self) -> bool {
        self.path_valid
    }

    fn world_to_grid(&self, world_x: f64, world_y: f64) -> (i32, i32) {
        let grid_x = (world_x / self.grid_resolution + self.grid_width as f64 / 2.0) as i32;
        let grid_y = (world_y / self.grid_resolution + self.grid_height as f64 / 2.0) as i32;
        (grid_x, grid_y)
    }

    fn grid_to_world(&self, grid_x: i32, grid_y: i32) -> (f64, f64) {
        let world_x = (grid_x as f64 - self.grid_width as f64 / 2.0) * self.grid_resolution;
        let world_y = (grid_y as f64 - self.grid_height as f64 / 2.0) * self.grid_resolution;
        (world_x, world_y)
    }

    fn is_valid_grid_cell(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.grid_width as i32 && y >= 0 && y < self.grid_height as i32
    }

    fn is_cell_free(&self, x: i32, y: i32) -> bool {
        if !self.is_valid_grid_cell(x, y) {
            return false;
        }
        !self.grid_map[y as usize][x as usize]
    }

    fn update_occupancy_grid(&mut self, lidar_data: &LaserScan) {
        // Clear previous obstacles (simplified - real implementation would use probabilistic updates)
        for row in &mut self.grid_map {
            row.fill(false);
        }

        let (robot_x, robot_y, robot_theta) = self.current_pose;

        // Process lidar points
        for (i, &range) in lidar_data.ranges.iter().enumerate() {
            if range > 0.1 && range < lidar_data.range_max {
                let angle = lidar_data.angle_min as f64 + i as f64 * lidar_data.angle_increment as f64 + robot_theta;

                let obstacle_x = robot_x + range as f64 * angle.cos();
                let obstacle_y = robot_y + range as f64 * angle.sin();

                let (grid_x, grid_y) = self.world_to_grid(obstacle_x, obstacle_y);

                if self.is_valid_grid_cell(grid_x, grid_y) {
                    self.grid_map[grid_y as usize][grid_x as usize] = true;

                    // Inflate obstacles by robot radius
                    let inflation_cells = (self.robot_radius / self.grid_resolution).ceil() as i32;
                    for dy in -inflation_cells..=inflation_cells {
                        for dx in -inflation_cells..=inflation_cells {
                            let nx = grid_x + dx;
                            let ny = grid_y + dy;
                            if self.is_valid_grid_cell(nx, ny) {
                                let dist = ((dx * dx + dy * dy) as f64).sqrt() * self.grid_resolution;
                                if dist <= self.robot_radius {
                                    self.grid_map[ny as usize][nx as usize] = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn euclidean_distance(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
    }

    fn plan_path_astar(&mut self) -> Vec<(f64, f64)> {
        let (start_x, start_y, _) = self.current_pose;
        let (goal_x, goal_y, _) = self.goal_pose;

        let (start_grid_x, start_grid_y) = self.world_to_grid(start_x, start_y);
        let (goal_grid_x, goal_grid_y) = self.world_to_grid(goal_x, goal_y);

        if !self.is_cell_free(start_grid_x, start_grid_y) || !self.is_cell_free(goal_grid_x, goal_grid_y) {
            return Vec::new(); // Start or goal is in obstacle
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

        let start_node = AStarNode {
            x: start_grid_x,
            y: start_grid_y,
            g_cost: 0.0,
            h_cost: self.euclidean_distance(
                start_grid_x as f64, start_grid_y as f64,
                goal_grid_x as f64, goal_grid_y as f64
            ),
            f_cost: 0.0,
            parent: None,
        };

        open_set.push(start_node);

        let directions = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

        while let Some(current) = open_set.pop() {
            let current_pos = (current.x, current.y);

            if current.x == goal_grid_x && current.y == goal_grid_y {
                // Reconstruct path
                let mut path = Vec::new();
                let mut pos = current_pos;

                loop {
                    let (world_x, world_y) = self.grid_to_world(pos.0, pos.1);
                    path.push((world_x, world_y));

                    if let Some(parent) = came_from.get(&pos) {
                        pos = *parent;
                    } else {
                        break;
                    }
                }

                path.reverse();
                return path;
            }

            closed_set.insert(current_pos);

            for &(dx, dy) in &directions {
                let neighbor_x = current.x + dx;
                let neighbor_y = current.y + dy;
                let neighbor_pos = (neighbor_x, neighbor_y);

                if closed_set.contains(&neighbor_pos) || !self.is_cell_free(neighbor_x, neighbor_y) {
                    continue;
                }

                let movement_cost = if dx.abs() + dy.abs() == 2 {
                    1.414 // Diagonal movement
                } else {
                    1.0   // Orthogonal movement
                };

                let tentative_g_cost = current.g_cost + movement_cost;
                let h_cost = self.euclidean_distance(
                    neighbor_x as f64, neighbor_y as f64,
                    goal_grid_x as f64, goal_grid_y as f64
                ) * self.heuristic_weight;

                let neighbor_node = AStarNode {
                    x: neighbor_x,
                    y: neighbor_y,
                    g_cost: tentative_g_cost,
                    h_cost,
                    f_cost: tentative_g_cost + h_cost,
                    parent: Some(current_pos),
                };

                came_from.insert(neighbor_pos, current_pos);
                open_set.push(neighbor_node);
            }
        }

        Vec::new() // No path found
    }

    fn plan_path_rrt(&mut self) -> Vec<(f64, f64)> {
        // Simplified RRT implementation
        let (start_x, start_y, _) = self.current_pose;
        let (goal_x, goal_y, _) = self.goal_pose;

        let mut tree: Vec<(f64, f64)> = vec![(start_x, start_y)];
        let mut parents: Vec<Option<usize>> = vec![None];

        for iteration in 0..self.rrt_max_iterations {
            // Sample random point or goal with bias (using simple PRNG)
            let rand_value = ((iteration as u64 * 1103515245 + 12345) % (1u64 << 31)) as f64 / (1u64 << 31) as f64;
            let (rand_x, rand_y) = if rand_value < self.rrt_goal_bias {
                (goal_x, goal_y)
            } else {
                let x_rand = ((iteration as u64 * 1664525 + 1013904223) % (1u64 << 31)) as f64 / (1u64 << 31) as f64;
                let y_rand = ((iteration as u64 * 2147483647 + 1) % (1u64 << 31)) as f64 / (1u64 << 31) as f64;
                let x = (x_rand - 0.5) * self.grid_width as f64 * self.grid_resolution;
                let y = (y_rand - 0.5) * self.grid_height as f64 * self.grid_resolution;
                (x, y)
            };

            // Find nearest node in tree
            let mut nearest_idx = 0;
            let mut nearest_dist = self.euclidean_distance(tree[0].0, tree[0].1, rand_x, rand_y);

            for (i, &(tx, ty)) in tree.iter().enumerate().skip(1) {
                let dist = self.euclidean_distance(tx, ty, rand_x, rand_y);
                if dist < nearest_dist {
                    nearest_dist = dist;
                    nearest_idx = i;
                }
            }

            let (nearest_x, nearest_y) = tree[nearest_idx];

            // Extend towards random point
            let direction_x = (rand_x - nearest_x) / nearest_dist;
            let direction_y = (rand_y - nearest_y) / nearest_dist;

            let new_x = nearest_x + direction_x * self.rrt_step_size.min(nearest_dist);
            let new_y = nearest_y + direction_y * self.rrt_step_size.min(nearest_dist);

            // Check if new point is collision-free
            let (grid_x, grid_y) = self.world_to_grid(new_x, new_y);
            if self.is_cell_free(grid_x, grid_y) {
                tree.push((new_x, new_y));
                parents.push(Some(nearest_idx));

                // Check if we reached the goal
                if self.euclidean_distance(new_x, new_y, goal_x, goal_y) < self.grid_resolution * 2.0 {
                    // Reconstruct path
                    let mut path = Vec::new();
                    let mut current_idx = Some(tree.len() - 1);

                    while let Some(idx) = current_idx {
                        path.push(tree[idx]);
                        current_idx = parents[idx];
                    }

                    path.reverse();
                    return path;
                }
            }
        }

        Vec::new() // No path found
    }

    fn check_path_deviation(&self) -> bool {
        if self.current_path.is_empty() {
            return false;
        }

        let (current_x, current_y, _) = self.current_pose;

        // Find closest point on path
        let mut min_distance = f64::INFINITY;
        for &(path_x, path_y) in &self.current_path {
            let distance = self.euclidean_distance(current_x, current_y, path_x, path_y);
            min_distance = min_distance.min(distance);
        }

        min_distance > self.replanning_threshold
    }

    fn publish_path(&self) {
        if self.current_path.is_empty() {
            return;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let path_plan = PathPlan {
            waypoints: self.current_path.iter().map(|&(x, y)| [x as f32, y as f32, 0.0]).collect(),
            goal_pose: [self.goal_pose.0 as f32, self.goal_pose.1 as f32, self.goal_pose.2 as f32],
            path_length: self.current_path.len() as u32,
            timestamp: current_time,
        };

        let _ = self.plan_publisher.send(path_plan, None);
    }
}

impl Node for PathPlannerNode {
    fn name(&self) -> &'static str {
        "PathPlannerNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Update current pose from odometry
        if let Some(odom) = self.odometry_subscriber.recv(None) {
            self.current_pose = (
                odom.pose.x,
                odom.pose.y,
                odom.pose.theta,
            );
        }

        // Handle new goal commands
        if let Some(goal) = self.goal_subscriber.recv(None) {
            if !goal.waypoints.is_empty() {
                let goal_point = goal.waypoints.last().unwrap();
                self.set_goal(goal_point[0] as f64, goal_point[1] as f64, goal.goal_pose[2] as f64);
            }
        }

        // Update occupancy grid from lidar
        if let Some(lidar) = self.lidar_subscriber.recv(None) {
            self.update_occupancy_grid(&lidar);
        }

        // Check if we need to replan
        let should_replan = !self.path_valid ||
                           self.current_path.is_empty() ||
                           self.check_path_deviation();

        if should_replan {
            self.current_path = match self.planning_algorithm {
                PlanningAlgorithm::AStar => self.plan_path_astar(),
                PlanningAlgorithm::RRT => self.plan_path_rrt(),
            };

            self.path_valid = !self.current_path.is_empty();

            // Publish new path
            if self.path_valid {
                self.publish_path();
            }
        }
    }
}

impl Default for PathPlannerNode {
    fn default() -> Self {
        let mut node = Self::new();
        node.set_grid_config(0.1, 200, 200); // 10cm resolution, 20x20m grid
        node
    }
}