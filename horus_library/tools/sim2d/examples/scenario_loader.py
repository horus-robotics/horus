#!/usr/bin/env python3
"""
Example: Loading and Using Scenarios with sim2d Python API

This example demonstrates how to:
1. Load scenarios from YAML files
2. Extract world and robot configurations
3. Run simulations programmatically
4. Monitor robot state and metrics
"""

import sys
import time
from pathlib import Path

# Add sim2d to path (adjust if needed)
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

try:
    from sim2d import Scenario, Sim2DBuilder
    print("✓ sim2d Python API imported successfully")
except ImportError as e:
    print(f"✗ Failed to import sim2d Python API: {e}")
    print("  Make sure sim2d Python bindings are built:")
    print("  cd horus_py && maturin develop")
    sys.exit(1)


def load_and_run_scenario(scenario_path: str, duration: float = 10.0):
    """Load a scenario and run it for specified duration"""

    print(f"\n{'='*60}")
    print(f"Loading scenario: {scenario_path}")
    print(f"{'='*60}\n")

    # Load scenario from file
    try:
        scenario = Scenario.load_from_file(scenario_path)
        print(f"✓ Loaded scenario: {scenario.name}")
        print(f"  Description: {scenario.description}")
        print(f"  World size: {scenario.world.width}m x {scenario.world.height}m")
        print(f"  Obstacles: {len(scenario.world.obstacles)}")
        print(f"  Robots: {len(scenario.robots)}")
        print()
    except Exception as e:
        print(f"✗ Failed to load scenario: {e}")
        return

    # Extract configurations
    world_config = scenario.to_world_config()
    robot_configs = scenario.to_robot_configs()

    if not robot_configs:
        print("✗ No robots in scenario")
        return

    # Display robot information
    for robot_state in scenario.robots:
        print(f"Robot: {robot_state.name}")
        print(f"  Position: ({robot_state.position[0]:.2f}, {robot_state.position[1]:.2f})")
        print(f"  Heading: {robot_state.heading:.2f} rad")
        if robot_state.config:
            print(f"  Kinematics: {robot_state.config.kinematics.kinematic_type}")
            print(f"  Max speed: {robot_state.config.max_speed} m/s")
        print()

    # Build simulation
    print("Building simulation...")
    try:
        sim = (Sim2DBuilder()
               .with_world(world_config)
               .with_robot(robot_configs[0])  # Use first robot
               .robot_name(robot_configs[0].name)
               .topic_prefix(robot_configs[0].topic_prefix)
               .headless(True)  # Run without GUI
               .build())
        print("✓ Simulation built successfully")
    except Exception as e:
        print(f"✗ Failed to build simulation: {e}")
        return

    # Run simulation
    print(f"\nRunning simulation for {duration} seconds...")
    print("(In a real scenario, you would send commands via HORUS topics)")
    print()

    try:
        # Run for specified duration
        sim.run_for(duration)

        # Get final robot pose
        final_pose = sim.get_robot_pose(robot_configs[0].name)
        if final_pose:
            print(f"✓ Simulation completed")
            print(f"  Final pose: ({final_pose[0]:.2f}, {final_pose[1]:.2f}, {final_pose[2]:.2f})")

        # Get metrics
        metrics = sim.get_metrics()
        if metrics:
            print(f"\nPerformance Metrics:")
            print(f"  Path length: {metrics.get('path_length', 0):.2f} m")
            print(f"  Collisions: {metrics.get('collision_count', 0)}")
            print(f"  Time: {metrics.get('elapsed_time', 0):.2f} s")

    except Exception as e:
        print(f"✗ Simulation error: {e}")


def list_available_scenarios(scenarios_dir: str = "scenarios"):
    """List all available scenarios"""

    print(f"\n{'='*60}")
    print("Available Scenarios")
    print(f"{'='*60}\n")

    scenarios_path = Path(scenarios_dir)
    if not scenarios_path.exists():
        print(f"✗ Scenarios directory not found: {scenarios_dir}")
        return []

    scenario_files = sorted(scenarios_path.glob("*.yaml"))

    if not scenario_files:
        print(f"✗ No scenario files found in {scenarios_dir}")
        return []

    for i, scenario_file in enumerate(scenario_files, 1):
        try:
            scenario = Scenario.load_from_file(str(scenario_file))
            print(f"{i}. {scenario_file.name}")
            print(f"   Name: {scenario.name}")
            print(f"   Description: {scenario.description}")
            print(f"   Robots: {len(scenario.robots)}, Obstacles: {len(scenario.world.obstacles)}")
            print()
        except Exception as e:
            print(f"{i}. {scenario_file.name}")
            print(f"   ✗ Failed to load: {e}")
            print()

    return scenario_files


def main():
    """Main function"""

    print("\n" + "="*60)
    print("sim2d Scenario Loader Example")
    print("="*60)

    # List available scenarios
    scenario_files = list_available_scenarios()

    if not scenario_files:
        print("\nNo scenarios available. Create some first!")
        return

    # Example: Load and run first scenario
    if len(sys.argv) > 1:
        # User specified scenario file
        scenario_path = sys.argv[1]
    else:
        # Use first available scenario
        scenario_path = str(scenario_files[0])

    # Load and run scenario
    duration = 10.0  # seconds
    if len(sys.argv) > 2:
        duration = float(sys.argv[2])

    load_and_run_scenario(scenario_path, duration)

    print("\n" + "="*60)
    print("Example completed!")
    print("="*60 + "\n")


if __name__ == "__main__":
    main()
