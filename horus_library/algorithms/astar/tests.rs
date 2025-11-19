#[cfg(test)]
mod tests {
    use super::super::{AStar, Heuristic};

    #[test]
    fn test_straight_line_path() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(0, 0);
        astar.set_goal(9, 9);

        let path = astar.plan();
        assert!(path.is_some());

        let path = path.unwrap();
        assert_eq!(path[0], (0, 0));
        assert_eq!(path[path.len() - 1], (9, 9));

        // Diagonal path should be roughly sqrt(2) * 9
        let length = AStar::path_length(&path);
        assert!(length > 12.0 && length < 13.0);
    }

    #[test]
    fn test_path_around_obstacle() {
        let mut astar = AStar::new(20, 20);
        astar.set_start(0, 10);
        astar.set_goal(19, 10);

        // Create vertical wall with gap
        for y in 0..7 {
            astar.set_obstacle(10, y);
        }
        for y in 13..20 {
            astar.set_obstacle(10, y);
        }

        let path = astar.plan();
        assert!(path.is_some());

        let path = path.unwrap();
        // Path must go through gap (y between 7-13 at x=10)
        let through_gap = path.iter().any(|(x, y)| *x == 10 && *y >= 7 && *y <= 13);
        assert!(through_gap);
    }

    #[test]
    fn test_no_path_exists() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(0, 0);
        astar.set_goal(9, 9);

        // Create complete wall blocking path
        for y in 0..10 {
            astar.set_obstacle(5, y);
        }

        let path = astar.plan();
        assert!(path.is_none());
    }

    #[test]
    fn test_start_equals_goal() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(5, 5);
        astar.set_goal(5, 5);

        let path = astar.plan();
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 1);
    }

    #[test]
    fn test_manhattan_heuristic() {
        let mut astar = AStar::new(20, 20);
        astar.set_start(0, 0);
        astar.set_goal(10, 10);
        astar.set_heuristic(Heuristic::Manhattan);

        let path = astar.plan();
        assert!(path.is_some());
    }

    #[test]
    fn test_diagonal_heuristic() {
        let mut astar = AStar::new(20, 20);
        astar.set_start(0, 0);
        astar.set_goal(10, 10);
        astar.set_heuristic(Heuristic::Diagonal);

        let path = astar.plan();
        assert!(path.is_some());
    }

    #[test]
    fn test_no_diagonal_movement() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(0, 0);
        astar.set_goal(5, 5);
        astar.set_allow_diagonal(false);

        let path = astar.plan();
        assert!(path.is_some());

        let path = path.unwrap();
        // Without diagonals, path length should be Manhattan distance
        let length = AStar::path_length(&path);
        assert!((length - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_weighted_astar() {
        let mut astar = AStar::new(30, 30);
        astar.set_start(0, 0);
        astar.set_goal(29, 29);

        // Optimal path
        astar.set_heuristic_weight(1.0);
        let optimal = astar.plan().unwrap();
        let optimal_len = AStar::path_length(&optimal);

        // Weighted path (faster but suboptimal)
        astar.set_heuristic_weight(2.0);
        let weighted = astar.plan().unwrap();
        let weighted_len = AStar::path_length(&weighted);

        // Both should find paths
        assert!(optimal.len() > 0);
        assert!(weighted.len() > 0);

        // Weighted might be slightly longer (or same in open space)
        assert!(weighted_len >= optimal_len * 0.95);
    }

    #[test]
    fn test_clear_obstacle() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(0, 5);
        astar.set_goal(9, 5);

        // Block path
        for x in 3..7 {
            astar.set_obstacle(x, 5);
        }

        // Should fail
        assert!(astar.plan().is_none());

        // Clear middle obstacle
        astar.clear_obstacle(5, 5);

        // Should succeed now
        assert!(astar.plan().is_some());
    }

    #[test]
    fn test_obstacle_at_start() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(5, 5);
        astar.set_goal(9, 9);
        astar.set_obstacle(5, 5);  // Start is blocked

        let path = astar.plan();
        assert!(path.is_none());
    }

    #[test]
    fn test_obstacle_at_goal() {
        let mut astar = AStar::new(10, 10);
        astar.set_start(0, 0);
        astar.set_goal(9, 9);
        astar.set_obstacle(9, 9);  // Goal is blocked

        let path = astar.plan();
        assert!(path.is_none());
    }

    #[test]
    fn test_large_grid() {
        let mut astar = AStar::new(100, 100);
        astar.set_start(10, 10);
        astar.set_goal(90, 90);

        let path = astar.plan();
        assert!(path.is_some());

        let path = path.unwrap();
        assert!(path.len() > 0);
        assert_eq!(path[0], (10, 10));
        assert_eq!(path[path.len() - 1], (90, 90));
    }

    #[test]
    fn test_narrow_corridor() {
        let mut astar = AStar::new(20, 20);
        astar.set_start(0, 10);
        astar.set_goal(19, 10);

        // Create narrow corridor
        for x in 0..20 {
            astar.set_obstacle(x, 9);
            astar.set_obstacle(x, 11);
        }

        let path = astar.plan();
        assert!(path.is_some());

        // All path points should be at y=10 (the corridor)
        let path = path.unwrap();
        for (_, y) in &path {
            assert_eq!(*y, 10);
        }
    }

    #[test]
    fn test_maze_like_environment() {
        let mut astar = AStar::new(15, 15);
        astar.set_start(1, 1);
        astar.set_goal(13, 13);

        // Create maze-like obstacles
        for i in (2..13).step_by(2) {
            for j in 2..13 {
                if j != 7 {  // Leave gaps
                    astar.set_obstacle(i, j);
                }
            }
        }

        let path = astar.plan();
        assert!(path.is_some());
    }

    #[test]
    fn test_set_grid() {
        let mut astar = AStar::new(10, 10);

        // Create custom grid
        let mut grid = vec![vec![false; 10]; 10];
        for i in 0..10 {
            grid[5][i] = true;  // Horizontal wall
        }
        grid[5][5] = false;  // Gap in wall

        astar.set_grid(grid);
        astar.set_start(0, 0);
        astar.set_goal(9, 9);

        let path = astar.plan();
        assert!(path.is_some());
    }

    #[test]
    fn test_path_length_calculation() {
        let path = vec![
            (0, 0),
            (1, 0),  // +1
            (2, 0),  // +1
            (2, 1),  // +1
        ];

        let length = AStar::path_length(&path);
        assert!((length - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_clear_all_obstacles() {
        let mut astar = AStar::new(10, 10);

        // Add many obstacles
        for i in 0..10 {
            astar.set_obstacle(5, i);
        }

        astar.set_start(0, 5);
        astar.set_goal(9, 5);
        assert!(astar.plan().is_none());

        // Clear all
        astar.clear_obstacles();
        assert!(astar.plan().is_some());
    }
}
