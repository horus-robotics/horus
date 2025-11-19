//! Basic A* Pathfinding Example
//!
//! Demonstrates simple pathfinding in an open grid.

use horus_library::algorithms::astar::{AStar, Heuristic};

fn main() {
    println!("=== Basic A* Pathfinding ===\n");

    // Create 20x20 grid
    let mut astar = AStar::new(20, 20);

    // Set start and goal
    astar.set_start(2, 2);
    astar.set_goal(17, 17);

    // Configure for optimal pathfinding
    astar.set_heuristic(Heuristic::Euclidean);
    astar.set_allow_diagonal(true);

    // Plan path
    if let Some(path) = astar.plan() {
        println!("✓ Path found!");
        println!("  Waypoints: {}", path.len());
        println!("  Path length: {:.2} units\n", AStar::path_length(&path));

        // Visualize path
        visualize_grid(&astar, &path);

        // Print path coordinates
        println!("\nPath coordinates:");
        for (i, (x, y)) in path.iter().enumerate() {
            if i % 5 == 0 {
                print!("\n  ");
            }
            print!("({:2},{:2}) ", x, y);
        }
        println!();
    } else {
        println!("✗ No path found!");
    }
}

fn visualize_grid(astar: &AStar, path: &[(i32, i32)]) {
    println!("Grid visualization:");
    println!("  S = Start, G = Goal, * = Path, . = Free\n");

    let start = (2, 2);
    let goal = (17, 17);

    for y in 0..20 {
        print!("  ");
        for x in 0..20 {
            let pos = (x, y);
            if pos == start {
                print!("S ");
            } else if pos == goal {
                print!("G ");
            } else if path.contains(&pos) {
                print!("* ");
            } else {
                print!(". ");
            }
        }
        println!();
    }
}
