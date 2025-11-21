//! 2D Occupancy Grid Mapping
//!
//! Grid-based environment representation for robot navigation.
//!
//! # Features
//!
//! - Binary occupancy (free/occupied)
//! - Probabilistic occupancy (0-1)
//! - Ray tracing for sensor integration
//! - Grid-world coordinate conversion
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::occupancy_grid::OccupancyGrid;
//!
//! let mut grid = OccupancyGrid::new(100, 100, 0.1);  // 10m x 10m at 0.1m resolution
//!
//! // Set occupied cell
//! grid.set_occupied(50, 50);
//!
//! // Check if cell is free
//! if grid.is_free(60, 60) {
//!     println!("Cell is free!");
//! }
//! ```

/// 2D Occupancy Grid
pub struct OccupancyGrid {
    width: usize,
    height: usize,
    resolution: f64,     // meters per cell
    grid: Vec<Vec<f64>>, // 0.0 = free, 1.0 = occupied
    origin: (f64, f64),  // World coordinates of grid[0][0]
}

impl OccupancyGrid {
    /// Create new occupancy grid
    ///
    /// # Arguments
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `resolution` - Cell size in meters
    pub fn new(width: usize, height: usize, resolution: f64) -> Self {
        Self {
            width,
            height,
            resolution,
            grid: vec![vec![0.0; width]; height],
            origin: (0.0, 0.0),
        }
    }

    /// Set origin (world coordinates of grid[0][0])
    pub fn set_origin(&mut self, x: f64, y: f64) {
        self.origin = (x, y);
    }

    /// Set cell as occupied
    pub fn set_occupied(&mut self, x: usize, y: usize) {
        if self.is_valid(x, y) {
            self.grid[y][x] = 1.0;
        }
    }

    /// Set cell as free
    pub fn set_free(&mut self, x: usize, y: usize) {
        if self.is_valid(x, y) {
            self.grid[y][x] = 0.0;
        }
    }

    /// Set cell occupancy probability (0.0 - 1.0)
    pub fn set_probability(&mut self, x: usize, y: usize, prob: f64) {
        if self.is_valid(x, y) {
            self.grid[y][x] = prob.clamp(0.0, 1.0);
        }
    }

    /// Get cell occupancy probability
    pub fn get_probability(&self, x: usize, y: usize) -> f64 {
        if self.is_valid(x, y) {
            self.grid[y][x]
        } else {
            1.0 // Out of bounds = occupied
        }
    }

    /// Check if cell is free (probability < 0.5)
    pub fn is_free(&self, x: usize, y: usize) -> bool {
        self.get_probability(x, y) < 0.5
    }

    /// Check if cell is occupied (probability >= 0.5)
    pub fn is_occupied(&self, x: usize, y: usize) -> bool {
        self.get_probability(x, y) >= 0.5
    }

    /// Clear grid (set all cells to free)
    pub fn clear(&mut self) {
        for row in &mut self.grid {
            row.fill(0.0);
        }
    }

    /// Convert world coordinates to grid coordinates
    pub fn world_to_grid(&self, world_x: f64, world_y: f64) -> (i32, i32) {
        let grid_x = ((world_x - self.origin.0) / self.resolution) as i32;
        let grid_y = ((world_y - self.origin.1) / self.resolution) as i32;
        (grid_x, grid_y)
    }

    /// Convert grid coordinates to world coordinates
    pub fn grid_to_world(&self, grid_x: i32, grid_y: i32) -> (f64, f64) {
        let world_x = grid_x as f64 * self.resolution + self.origin.0;
        let world_y = grid_y as f64 * self.resolution + self.origin.1;
        (world_x, world_y)
    }

    /// Check if grid coordinates are valid
    pub fn is_valid(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    /// Ray trace from start to end, marking cells
    pub fn ray_trace(&mut self, start: (f64, f64), end: (f64, f64), mark_free: bool) {
        let (start_x, start_y) = self.world_to_grid(start.0, start.1);
        let (end_x, end_y) = self.world_to_grid(end.0, end.1);

        let cells = bresenham_line(start_x, start_y, end_x, end_y);

        for (i, (x, y)) in cells.iter().enumerate() {
            if *x >= 0 && *x < self.width as i32 && *y >= 0 && *y < self.height as i32 {
                let ux = *x as usize;
                let uy = *y as usize;

                if i == cells.len() - 1 {
                    // End point = obstacle
                    self.set_occupied(ux, uy);
                } else if mark_free {
                    // Intermediate points = free
                    self.set_free(ux, uy);
                }
            }
        }
    }

    /// Get grid dimensions
    pub fn get_dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Get resolution
    pub fn get_resolution(&self) -> f64 {
        self.resolution
    }
}

/// Bresenham's line algorithm
fn bresenham_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        cells.push((x, y));

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;

        if e2 > -dy {
            err -= dy;
            x += sx;
        }

        if e2 < dx {
            err += dx;
            y += sy;
        }
    }

    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_grid() {
        let grid = OccupancyGrid::new(100, 100, 0.1);
        assert_eq!(grid.get_dimensions(), (100, 100));
        assert_eq!(grid.get_resolution(), 0.1);
    }

    #[test]
    fn test_set_occupied() {
        let mut grid = OccupancyGrid::new(10, 10, 1.0);
        grid.set_occupied(5, 5);

        assert!(grid.is_occupied(5, 5));
        assert!(!grid.is_free(5, 5));
    }

    #[test]
    fn test_set_free() {
        let mut grid = OccupancyGrid::new(10, 10, 1.0);
        grid.set_occupied(5, 5);
        grid.set_free(5, 5);

        assert!(grid.is_free(5, 5));
        assert!(!grid.is_occupied(5, 5));
    }

    #[test]
    fn test_probability() {
        let mut grid = OccupancyGrid::new(10, 10, 1.0);
        grid.set_probability(5, 5, 0.7);

        assert_eq!(grid.get_probability(5, 5), 0.7);
        assert!(grid.is_occupied(5, 5));
    }

    #[test]
    fn test_world_to_grid() {
        let grid = OccupancyGrid::new(100, 100, 0.1);

        let (gx, gy) = grid.world_to_grid(1.0, 2.0);
        assert_eq!(gx, 10);
        assert_eq!(gy, 20);
    }

    #[test]
    fn test_grid_to_world() {
        let grid = OccupancyGrid::new(100, 100, 0.1);

        let (wx, wy) = grid.grid_to_world(10, 20);
        assert!((wx - 1.0).abs() < 0.01);
        assert!((wy - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_clear() {
        let mut grid = OccupancyGrid::new(10, 10, 1.0);

        grid.set_occupied(5, 5);
        grid.clear();

        assert!(grid.is_free(5, 5));
    }

    #[test]
    fn test_out_of_bounds() {
        let grid = OccupancyGrid::new(10, 10, 1.0);

        // Out of bounds should be considered occupied
        assert!(!grid.is_valid(15, 15));
        assert!(grid.is_occupied(15, 15));
    }

    #[test]
    fn test_ray_trace() {
        let mut grid = OccupancyGrid::new(20, 20, 1.0);

        grid.ray_trace((0.0, 0.0), (10.0, 0.0), true);

        // End point should be occupied
        assert!(grid.is_occupied(10, 0));

        // Intermediate points should be free
        assert!(grid.is_free(5, 0));
    }

    #[test]
    fn test_origin() {
        let mut grid = OccupancyGrid::new(10, 10, 1.0);
        grid.set_origin(-5.0, -5.0);

        let (gx, gy) = grid.world_to_grid(0.0, 0.0);
        assert_eq!(gx, 5);
        assert_eq!(gy, 5);
    }
}
