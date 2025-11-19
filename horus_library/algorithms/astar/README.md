# A* (A-Star) Pathfinding Algorithm

Grid-based optimal pathfinding using heuristic search.

## Overview

A* is a grid-based path planning algorithm that finds the optimal path from start to goal using a heuristic function to guide the search. It combines the benefits of Dijkstra's algorithm (guaranteed optimality) with greedy best-first search (efficiency through heuristics).

## Features

- **Optimal Pathfinding**: Guarantees shortest path with admissible heuristics
- **8-Directional Movement**: Supports diagonal movement with proper cost calculation
- **Multiple Heuristics**: Euclidean, Manhattan, and Diagonal distance functions
- **Configurable Weight**: Trade optimality for speed with weighted A*
- **Obstacle Avoidance**: Built-in collision detection
- **Efficient**: Priority queue-based implementation

## Algorithm Details

### Core Equation

```
f(n) = g(n) + h(n)

where:
  g(n) = actual cost from start to node n
  h(n) = heuristic estimate from n to goal
  f(n) = total estimated cost through node n
```

### Properties

- **Complete**: Always finds a path if one exists
- **Optimal**: Finds shortest path when h(n) is admissible (h(n) ≤ true cost)
- **Time Complexity**: O(b^d) worst case, O(n log n) typical
- **Space Complexity**: O(n) where n is number of nodes

## API Reference

### Construction

```rust
use horus_library::algorithms::astar::AStar;

// Create A* planner with grid size
let mut astar = AStar::new(width, height);
```

### Configuration

```rust
// Set start and goal positions
astar.set_start(0, 0);
astar.set_goal(99, 99);

// Add obstacles
astar.set_obstacle(50, 50);
astar.clear_obstacle(50, 50);

// Set entire grid (true = obstacle, false = free)
let grid = vec![vec![false; 100]; 100];
astar.set_grid(grid);

// Configure heuristic
astar.set_heuristic(Heuristic::Euclidean);  // Best for 8-directional
astar.set_heuristic(Heuristic::Manhattan);  // Best for 4-directional
astar.set_heuristic(Heuristic::Diagonal);   // Chebyshev distance

// Set heuristic weight (1.0 = optimal, >1.0 = faster)
astar.set_heuristic_weight(1.5);  // 50% faster, slightly suboptimal

// Allow or disallow diagonal movement
astar.set_allow_diagonal(true);   // 8-directional (default)
astar.set_allow_diagonal(false);  // 4-directional only

// Clear all obstacles
astar.clear_obstacles();
```

### Planning

```rust
// Plan path
if let Some(path) = astar.plan() {
    println!("Found path with {} waypoints", path.len());
    for (x, y) in path {
        println!("  ({}, {})", x, y);
    }
}

// Calculate path length
let length = AStar::path_length(&path);
println!("Path length: {:.2} meters", length);
```

## Examples

### Basic Pathfinding

```rust
use horus_library::algorithms::astar::{AStar, Heuristic};

fn main() {
    let mut astar = AStar::new(100, 100);

    // Set start and goal
    astar.set_start(10, 10);
    astar.set_goal(90, 90);

    // Plan path
    if let Some(path) = astar.plan() {
        println!("Path found with {} waypoints", path.len());
        println!("Path length: {:.2}", AStar::path_length(&path));
    } else {
        println!("No path found!");
    }
}
```

### Obstacle Avoidance

```rust
use horus_library::algorithms::astar::AStar;

fn main() {
    let mut astar = AStar::new(100, 100);

    astar.set_start(10, 50);
    astar.set_goal(90, 50);

    // Create vertical wall obstacle
    for y in 20..80 {
        astar.set_obstacle(50, y);
    }

    // Add opening in wall
    for y in 45..55 {
        astar.clear_obstacle(50, y);
    }

    // Find path through opening
    if let Some(path) = astar.plan() {
        println!("Found path around obstacle!");
        println!("Path length: {:.2}", AStar::path_length(&path));
    }
}
```

### Warehouse Navigation

```rust
use horus_library::algorithms::astar::{AStar, Heuristic};

fn main() {
    // 50x50 meter warehouse at 0.1m resolution = 500x500 grid
    let mut astar = AStar::new(500, 500);

    // Configure for warehouse
    astar.set_heuristic(Heuristic::Euclidean);
    astar.set_allow_diagonal(true);

    // Loading dock to storage area
    astar.set_start(25, 25);   // 2.5m, 2.5m
    astar.set_goal(450, 450);  // 45m, 45m

    // Add shelving units as obstacles
    add_shelving(&mut astar);

    // Plan path
    if let Some(path) = astar.plan() {
        let length = AStar::path_length(&path) * 0.1;  // Convert to meters
        println!("Warehouse path: {:.1}m with {} waypoints", length, path.len());
    }
}

fn add_shelving(astar: &mut AStar) {
    // Add 10 shelving units
    for shelf in 0..10 {
        let x_start = 100 + shelf * 40;
        for y in 100..400 {
            astar.set_obstacle(x_start, y);
            astar.set_obstacle(x_start + 1, y);
        }
    }
}
```

### Weighted A* for Faster Planning

```rust
use horus_library::algorithms::astar::AStar;

fn main() {
    let mut astar = AStar::new(200, 200);

    astar.set_start(0, 0);
    astar.set_goal(199, 199);

    // Optimal path (weight = 1.0)
    astar.set_heuristic_weight(1.0);
    let optimal_path = astar.plan().unwrap();
    let optimal_length = AStar::path_length(&optimal_path);

    // Faster path (weight = 2.0) - explores fewer nodes
    astar.set_heuristic_weight(2.0);
    let fast_path = astar.plan().unwrap();
    let fast_length = AStar::path_length(&fast_path);

    println!("Optimal: {:.2} units", optimal_length);
    println!("Fast: {:.2} units ({:.1}% longer)",
        fast_length, (fast_length / optimal_length - 1.0) * 100.0);
}
```

### Maze Solving

```rust
use horus_library::algorithms::astar::AStar;

fn main() {
    let mut astar = AStar::new(50, 50);

    // Load maze from file or generate procedurally
    load_maze(&mut astar, "maze.txt");

    astar.set_start(1, 1);
    astar.set_goal(48, 48);

    if let Some(path) = astar.plan() {
        println!("Maze solved in {} steps!", path.len());
        visualize_path(&path);
    } else {
        println!("Maze is unsolvable!");
    }
}

fn load_maze(astar: &mut AStar, filename: &str) {
    // Load maze from file
    // '#' = obstacle, '.' = free space
}

fn visualize_path(path: &[(i32, i32)]) {
    // Print path visualization to console
}
```

## Heuristic Functions

### Euclidean Distance

**Best for**: 8-directional movement (diagonal allowed)

```rust
astar.set_heuristic(Heuristic::Euclidean);
```

**Formula**: `√(dx² + dy²)`

**Properties**:
- Admissible (always ≤ true cost)
- Consistent (monotonic)
- Most accurate for diagonal movement

### Manhattan Distance

**Best for**: 4-directional movement (no diagonals)

```rust
astar.set_heuristic(Heuristic::Manhattan);
```

**Formula**: `|dx| + |dy|`

**Properties**:
- Admissible for grid-based movement
- Fast to compute
- Optimal for 4-directional grids

### Diagonal Distance (Chebyshev)

**Best for**: Uniform cost diagonal movement

```rust
astar.set_heuristic(Heuristic::Diagonal);
```

**Formula**: `max(|dx|, |dy|) + (√2 - 1) * min(|dx|, |dy|)`

**Properties**:
- Accounts for diagonal cost
- More accurate than Manhattan for 8-directional
- Slightly slower than Euclidean

## Performance Tuning

### Grid Resolution

Finer grids = more accuracy, slower planning:

```rust
// Coarse (fast, less accurate)
let astar = AStar::new(50, 50);    // 2m resolution for 100x100m area

// Medium (balanced)
let astar = AStar::new(100, 100);  // 1m resolution

// Fine (slow, very accurate)
let astar = AStar::new(500, 500);  // 0.2m resolution
```

### Heuristic Weight

Trade optimality for speed:

```rust
astar.set_heuristic_weight(1.0);   // Optimal, slower
astar.set_heuristic_weight(1.5);   // ~50% faster, slightly longer path
astar.set_heuristic_weight(2.0);   // ~2x faster, noticeably longer path
```

### Movement Restrictions

4-directional is faster than 8-directional:

```rust
astar.set_allow_diagonal(false);  // ~30% faster, but paths longer
```

## Troubleshooting

### Issue: No path found

**Possible causes**:
- Start or goal is inside obstacle
- No collision-free path exists
- Grid bounds exceeded

**Solutions**:
```rust
// Verify start/goal are free
assert!(!astar.is_obstacle(start_x, start_y));
assert!(!astar.is_obstacle(goal_x, goal_y));

// Check grid bounds
assert!(start_x >= 0 && start_x < width);
assert!(start_y >= 0 && start_y < height);

// Verify obstacles don't block all paths
astar.clear_obstacles();  // Test without obstacles
```

### Issue: Path cuts through obstacles

**Cause**: Diagonal corner cutting enabled

**Solution**:
```rust
// Algorithm already prevents corner cutting
// If still occurring, check obstacle grid is correct
let grid = astar.get_grid();  // Verify obstacles are set
```

### Issue: Planning too slow

**Solutions**:
```rust
// 1. Reduce grid resolution
let astar = AStar::new(100, 100);  // Instead of 500x500

// 2. Increase heuristic weight
astar.set_heuristic_weight(1.5);

// 3. Disable diagonal movement
astar.set_allow_diagonal(false);

// 4. Use faster heuristic
astar.set_heuristic(Heuristic::Manhattan);
```

### Issue: Path not optimal

**Causes**:
- Heuristic weight > 1.0
- Wrong heuristic for movement type
- Diagonal movement disabled

**Solutions**:
```rust
// Ensure optimal settings
astar.set_heuristic_weight(1.0);
astar.set_heuristic(Heuristic::Euclidean);
astar.set_allow_diagonal(true);
```

## Algorithm Comparison

| Feature | A* | Dijkstra | RRT |
|---------|----|-----------| ----|
| Optimality | Yes* | Yes | No |
| Speed | Fast | Slow | Variable |
| Grid Required | Yes | Yes | No |
| Best For | Structured environments | Multiple goals | Complex spaces |

*Optimal when heuristic weight = 1.0

## Integration with HORUS Nodes

A* is used internally by `PathPlannerNode`:

```rust
use horus_library::nodes::PathPlannerNode;

let mut planner = PathPlannerNode::new()?;
planner.set_algorithm(false);  // Use A* (true = RRT)
```

For direct algorithm access:

```rust
use horus_library::algorithms::astar::AStar;

// Create standalone A* instance
let mut astar = AStar::new(100, 100);

// Convert world coordinates to grid coordinates
fn world_to_grid(world_x: f64, world_y: f64, resolution: f64) -> (i32, i32) {
    let grid_x = (world_x / resolution) as i32;
    let grid_y = (world_y / resolution) as i32;
    (grid_x, grid_y)
}

// Use with real robot coordinates
let (start_grid_x, start_grid_y) = world_to_grid(robot_x, robot_y, 0.1);
let (goal_grid_x, goal_grid_y) = world_to_grid(goal_x, goal_y, 0.1);

astar.set_start(start_grid_x, start_grid_y);
astar.set_goal(goal_grid_x, goal_grid_y);
```

## See Also

- **RRT Algorithm**: Sampling-based planning for complex environments
- **Dijkstra Algorithm**: Multi-goal pathfinding without heuristic
- **Path Planner Node**: High-level navigation node using A*
- [A* Wikipedia](https://en.wikipedia.org/wiki/A*_search_algorithm)
- [PathPlanner Node Documentation](../../nodes/path_planner/README.md)
