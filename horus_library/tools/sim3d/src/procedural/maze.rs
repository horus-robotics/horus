use bevy::prelude::*;
use std::collections::VecDeque;

/// Maze generation algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MazeAlgorithm {
    /// Depth-first search (long winding corridors)
    DepthFirstSearch,
    /// Prim's algorithm (more branching)
    Prim,
    /// Kruskal's algorithm (creates loops)
    Kruskal,
    /// Binary tree (simple, biased)
    BinaryTree,
    /// Recursive division
    RecursiveDivision,
}

/// Maze cell type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellType {
    Wall,
    Path,
    Start,
    Goal,
}

/// Maze configuration
#[derive(Clone, Debug, Resource)]
pub struct MazeConfig {
    pub width: u32,
    pub height: u32,
    pub algorithm: MazeAlgorithm,
    pub wall_height: f32,
    pub corridor_width: f32,
    pub seed: u32,
    pub sparseness: f32, // 0.0 = dense, 1.0 = sparse
}

impl Default for MazeConfig {
    fn default() -> Self {
        Self {
            width: 20,
            height: 20,
            algorithm: MazeAlgorithm::DepthFirstSearch,
            wall_height: 2.0,
            corridor_width: 2.0,
            seed: 0,
            sparseness: 0.0,
        }
    }
}

/// Maze grid
#[derive(Clone, Debug)]
pub struct Maze {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<CellType>,
    pub start: (u32, u32),
    pub goal: (u32, u32),
}

impl Maze {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            cells: vec![CellType::Wall; (width * height) as usize],
            start: (1, 1),
            goal: (width - 2, height - 2),
        }
    }

    pub fn get(&self, x: u32, y: u32) -> CellType {
        if x >= self.width || y >= self.height {
            return CellType::Wall;
        }
        self.cells[(y * self.width + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, cell_type: CellType) {
        if x < self.width && y < self.height {
            self.cells[(y * self.width + x) as usize] = cell_type;
        }
    }

    /// Generate maze using specified algorithm
    pub fn generate(config: &MazeConfig) -> Self {
        match config.algorithm {
            MazeAlgorithm::DepthFirstSearch => Self::generate_dfs(config),
            MazeAlgorithm::Prim => Self::generate_prim(config),
            MazeAlgorithm::Kruskal => Self::generate_kruskal(config),
            MazeAlgorithm::BinaryTree => Self::generate_binary_tree(config),
            MazeAlgorithm::RecursiveDivision => Self::generate_recursive_division(config),
        }
    }

    /// Depth-first search maze generation
    fn generate_dfs(config: &MazeConfig) -> Self {
        let mut maze = Maze::new(config.width, config.height);
        let mut rng = fastrand::Rng::with_seed(config.seed as u64);
        let mut stack = Vec::new();
        let mut visited = vec![false; (config.width * config.height) as usize];

        let start_x = 1;
        let start_y = 1;

        stack.push((start_x, start_y));
        visited[(start_y * config.width + start_x) as usize] = true;
        maze.set(start_x, start_y, CellType::Path);

        while !stack.is_empty() {
            let (x, y) = *stack.last().unwrap();
            let mut neighbors = Vec::new();

            // Check all 4 directions
            for (dx, dy) in [(0, -2), (2, 0), (0, 2), (-2, 0)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                if nx > 0
                    && nx < (config.width - 1) as i32
                    && ny > 0
                    && ny < (config.height - 1) as i32
                {
                    let nx = nx as u32;
                    let ny = ny as u32;
                    if !visited[(ny * config.width + nx) as usize] {
                        neighbors.push((nx, ny, dx, dy));
                    }
                }
            }

            if neighbors.is_empty() {
                stack.pop();
            } else {
                let idx = rng.usize(0..neighbors.len());
                let (nx, ny, dx, dy) = neighbors[idx];

                // Carve path
                let wall_x = (x as i32 + dx / 2) as u32;
                let wall_y = (y as i32 + dy / 2) as u32;

                maze.set(wall_x, wall_y, CellType::Path);
                maze.set(nx, ny, CellType::Path);
                visited[(ny * config.width + nx) as usize] = true;

                stack.push((nx, ny));
            }
        }

        maze.set(maze.start.0, maze.start.1, CellType::Start);
        maze.set(maze.goal.0, maze.goal.1, CellType::Goal);

        maze
    }

    /// Prim's algorithm maze generation
    fn generate_prim(config: &MazeConfig) -> Self {
        let mut maze = Maze::new(config.width, config.height);
        let mut rng = fastrand::Rng::with_seed(config.seed as u64);
        let mut walls = Vec::new();
        let mut in_maze = vec![false; (config.width * config.height) as usize];

        // Start with random cell
        let start_x = 1;
        let start_y = 1;
        maze.set(start_x, start_y, CellType::Path);
        in_maze[(start_y * config.width + start_x) as usize] = true;

        // Add walls
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let wx = (start_x as i32 + dx) as u32;
            let wy = (start_y as i32 + dy) as u32;
            if wx > 0 && wx < config.width - 1 && wy > 0 && wy < config.height - 1 {
                walls.push((wx, wy));
            }
        }

        while !walls.is_empty() {
            let idx = rng.usize(0..walls.len());
            let (wx, wy) = walls.remove(idx);

            let mut adjacent_cells = 0;

            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let cx = (wx as i32 + dx) as u32;
                let cy = (wy as i32 + dy) as u32;

                if cx < config.width
                    && cy < config.height
                    && in_maze[(cy * config.width + cx) as usize]
                {
                    adjacent_cells += 1;
                }
            }

            if adjacent_cells == 1 {
                maze.set(wx, wy, CellType::Path);
                in_maze[(wy * config.width + wx) as usize] = true;

                // Add new walls
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    let nx = (wx as i32 + dx) as u32;
                    let ny = (wy as i32 + dy) as u32;

                    if nx > 0 && nx < config.width - 1 && ny > 0 && ny < config.height - 1 {
                        if !in_maze[(ny * config.width + nx) as usize]
                            && maze.get(nx, ny) == CellType::Wall
                        {
                            walls.push((nx, ny));
                        }
                    }
                }
            }
        }

        maze.set(maze.start.0, maze.start.1, CellType::Start);
        maze.set(maze.goal.0, maze.goal.1, CellType::Goal);

        maze
    }

    /// Binary tree algorithm (simple, fast)
    fn generate_binary_tree(config: &MazeConfig) -> Self {
        let mut maze = Maze::new(config.width, config.height);
        let mut rng = fastrand::Rng::with_seed(config.seed as u64);

        for y in 1..config.height - 1 {
            for x in 1..config.width - 1 {
                maze.set(x, y, CellType::Path);

                let can_go_north = y > 1;
                let can_go_east = x < config.width - 2;

                if can_go_north && can_go_east {
                    if rng.bool() {
                        maze.set(x, y - 1, CellType::Path); // North
                    } else {
                        maze.set(x + 1, y, CellType::Path); // East
                    }
                } else if can_go_north {
                    maze.set(x, y - 1, CellType::Path);
                } else if can_go_east {
                    maze.set(x + 1, y, CellType::Path);
                }
            }
        }

        maze.set(maze.start.0, maze.start.1, CellType::Start);
        maze.set(maze.goal.0, maze.goal.1, CellType::Goal);

        maze
    }

    /// Recursive division algorithm
    fn generate_recursive_division(config: &MazeConfig) -> Self {
        let mut maze = Maze::new(config.width, config.height);

        // Start with all paths
        for y in 1..config.height - 1 {
            for x in 1..config.width - 1 {
                maze.set(x, y, CellType::Path);
            }
        }

        let mut rng = fastrand::Rng::with_seed(config.seed as u64);
        Self::recursive_divide(
            &mut maze,
            &mut rng,
            1,
            1,
            config.width - 2,
            config.height - 2,
        );

        maze.set(maze.start.0, maze.start.1, CellType::Start);
        maze.set(maze.goal.0, maze.goal.1, CellType::Goal);

        maze
    }

    fn recursive_divide(
        maze: &mut Maze,
        rng: &mut fastrand::Rng,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        if width < 2 || height < 2 {
            return;
        }

        let horizontal = height > width || (height == width && rng.bool());

        if horizontal {
            let wall_y = y + rng.u32(0..height);
            let passage_x = x + rng.u32(0..width);

            for wx in x..(x + width) {
                if wx != passage_x {
                    maze.set(wx, wall_y, CellType::Wall);
                }
            }

            Self::recursive_divide(maze, rng, x, y, width, wall_y - y);
            Self::recursive_divide(maze, rng, x, wall_y + 1, width, y + height - wall_y - 1);
        } else {
            let wall_x = x + rng.u32(0..width);
            let passage_y = y + rng.u32(0..height);

            for wy in y..(y + height) {
                if wy != passage_y {
                    maze.set(wall_x, wy, CellType::Wall);
                }
            }

            Self::recursive_divide(maze, rng, x, y, wall_x - x, height);
            Self::recursive_divide(maze, rng, wall_x + 1, y, x + width - wall_x - 1, height);
        }
    }

    /// Generate maze using Kruskal's algorithm with union-find
    fn generate_kruskal(config: &MazeConfig) -> Self {
        let mut maze = Maze::new(config.width, config.height);
        let mut rng = fastrand::Rng::with_seed(config.seed as u64);

        // Initialize union-find data structure
        // Each cell at odd coordinates is a separate set
        let num_cells = ((config.width / 2) * (config.height / 2)) as usize;
        let mut parent: Vec<usize> = (0..num_cells).collect();
        let mut rank = vec![0u32; num_cells];

        // Convert 2D coordinates to cell index (only for cells at odd positions)
        let cell_index = |x: u32, y: u32| -> usize {
            ((y / 2) * (config.width / 2) + (x / 2)) as usize
        };

        // Mark all cells at odd coordinates as paths
        for y in (1..config.height).step_by(2) {
            for x in (1..config.width).step_by(2) {
                maze.set(x, y, CellType::Path);
            }
        }

        // Collect all possible walls between cells
        let mut walls = Vec::new();
        for y in (1..config.height - 1).step_by(2) {
            for x in (1..config.width - 1).step_by(2) {
                // Wall to the right
                if x + 2 < config.width {
                    walls.push((x, y, x + 2, y, x + 1, y));
                }
                // Wall below
                if y + 2 < config.height {
                    walls.push((x, y, x, y + 2, x, y + 1));
                }
            }
        }

        // Shuffle walls randomly using Fisher-Yates
        for i in (1..walls.len()).rev() {
            let j = rng.usize(0..=i);
            walls.swap(i, j);
        }

        // Union-Find: find with path compression
        fn find(parent: &mut [usize], mut i: usize) -> usize {
            let mut root = i;
            while parent[root] != root {
                root = parent[root];
            }
            // Path compression
            while parent[i] != root {
                let next = parent[i];
                parent[i] = root;
                i = next;
            }
            root
        }

        // Process walls using Kruskal's algorithm
        for (x1, y1, x2, y2, wx, wy) in walls {
            let idx1 = cell_index(x1, y1);
            let idx2 = cell_index(x2, y2);

            let root1 = find(&mut parent, idx1);
            let root2 = find(&mut parent, idx2);

            if root1 != root2 {
                // Union by rank
                if rank[root1] < rank[root2] {
                    parent[root1] = root2;
                } else if rank[root1] > rank[root2] {
                    parent[root2] = root1;
                } else {
                    parent[root2] = root1;
                    rank[root1] += 1;
                }

                // Remove wall between the two cells
                maze.set(wx, wy, CellType::Path);
            }
        }

        // Mark start and goal
        maze.set(maze.start.0, maze.start.1, CellType::Start);
        maze.set(maze.goal.0, maze.goal.1, CellType::Goal);

        maze
    }

    /// Solve maze using BFS
    pub fn solve(&self) -> Option<Vec<(u32, u32)>> {
        let mut queue = VecDeque::new();
        let mut came_from = vec![None; (self.width * self.height) as usize];
        let mut visited = vec![false; (self.width * self.height) as usize];

        queue.push_back(self.start);
        visited[(self.start.1 * self.width + self.start.0) as usize] = true;

        while let Some((x, y)) = queue.pop_front() {
            if (x, y) == self.goal {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = self.goal;

                while current != self.start {
                    path.push(current);
                    let idx = (current.1 * self.width + current.0) as usize;
                    current = came_from[idx].unwrap();
                }
                path.push(self.start);
                path.reverse();

                return Some(path);
            }

            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let nx = (x as i32 + dx) as u32;
                let ny = (y as i32 + dy) as u32;

                if nx < self.width && ny < self.height {
                    let idx = (ny * self.width + nx) as usize;
                    if !visited[idx] && self.get(nx, ny) != CellType::Wall {
                        visited[idx] = true;
                        came_from[idx] = Some((x, y));
                        queue.push_back((nx, ny));
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maze_creation() {
        let maze = Maze::new(10, 10);
        assert_eq!(maze.width, 10);
        assert_eq!(maze.height, 10);
        assert_eq!(maze.cells.len(), 100);
    }

    #[test]
    fn test_maze_get_set() {
        let mut maze = Maze::new(10, 10);
        maze.set(5, 5, CellType::Path);
        assert_eq!(maze.get(5, 5), CellType::Path);
    }

    #[test]
    fn test_dfs_maze_generation() {
        let config = MazeConfig {
            width: 21,
            height: 21,
            algorithm: MazeAlgorithm::DepthFirstSearch,
            ..Default::default()
        };

        let maze = Maze::generate(&config);

        // Should have paths
        let path_count = maze.cells.iter().filter(|&&c| c != CellType::Wall).count();
        assert!(path_count > 0);

        // Should have start and goal
        assert_eq!(maze.get(maze.start.0, maze.start.1), CellType::Start);
        assert_eq!(maze.get(maze.goal.0, maze.goal.1), CellType::Goal);
    }

    #[test]
    fn test_prim_maze_generation() {
        let config = MazeConfig {
            algorithm: MazeAlgorithm::Prim,
            ..Default::default()
        };

        let maze = Maze::generate(&config);
        let path_count = maze.cells.iter().filter(|&&c| c != CellType::Wall).count();
        assert!(path_count > 0);
    }

    #[test]
    fn test_binary_tree_maze() {
        let config = MazeConfig {
            algorithm: MazeAlgorithm::BinaryTree,
            ..Default::default()
        };

        let maze = Maze::generate(&config);
        let path_count = maze.cells.iter().filter(|&&c| c != CellType::Wall).count();
        assert!(path_count > 0);
    }

    #[test]
    fn test_maze_solver() {
        let config = MazeConfig {
            width: 11,
            height: 11,
            ..Default::default()
        };

        let maze = Maze::generate(&config);
        let solution = maze.solve();

        // Should find a path
        assert!(solution.is_some());

        if let Some(path) = solution {
            assert!(!path.is_empty());
            assert_eq!(path[0], maze.start);
            assert_eq!(*path.last().unwrap(), maze.goal);
        }
    }

    #[test]
    fn test_different_algorithms_produce_different_mazes() {
        let dfs = Maze::generate(&MazeConfig {
            algorithm: MazeAlgorithm::DepthFirstSearch,
            seed: 42,
            ..Default::default()
        });

        let prim = Maze::generate(&MazeConfig {
            algorithm: MazeAlgorithm::Prim,
            seed: 42,
            ..Default::default()
        });

        // Different algorithms should produce different mazes (with same seed)
        assert_ne!(dfs.cells, prim.cells);
    }
}
