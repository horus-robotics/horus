#!/usr/bin/env python3
"""
sim2d Dynamic Obstacle Control Example

Demonstrates how to dynamically add and remove obstacles in sim2d simulator
at runtime using HORUS topics.

Features:
- Add rectangular obstacles with custom colors
- Add circular obstacles
- Remove obstacles by position
- Interactive menu for live control

Requirements:
- sim2d simulator running (cargo run --package sim2d)
- HORUS Python bindings installed

Usage:
    python3 dynamic_obstacles.py
"""

from horus import Hub
import time
import sys


class ObstacleController:
    """Controls dynamic obstacles in sim2d simulator"""

    def __init__(self):
        self.hub = Hub("/sim2d/obstacle_cmd")
        print("Connected to /sim2d/obstacle_cmd topic")
        print("Make sure sim2d is running!\n")

    def add_rectangle(self, x, y, width, height, color=None):
        """Add a rectangular obstacle

        Args:
            x, y: Position in meters
            width, height: Size in meters
            color: Optional [R, G, B] with values 0.0-1.0
        """
        cmd = {
            "action": "add",
            "obstacle": {
                "pos": [x, y],
                "shape": "rectangle",
                "size": [width, height],
                "color": color
            }
        }
        self.hub.send(cmd)
        color_str = f" (color: RGB{color})" if color else ""
        print(f"✓ Added rectangle at ({x}, {y}) size [{width}x{height}]{color_str}")

    def add_circle(self, x, y, radius, color=None):
        """Add a circular obstacle

        Args:
            x, y: Position in meters
            radius: Radius in meters
            color: Optional [R, G, B] with values 0.0-1.0
        """
        cmd = {
            "action": "add",
            "obstacle": {
                "pos": [x, y],
                "shape": "circle",
                "size": [radius, radius],  # First value used as radius
                "color": color
            }
        }
        self.hub.send(cmd)
        color_str = f" (color: RGB{color})" if color else ""
        print(f"✓ Added circle at ({x}, {y}) radius {radius}{color_str}")

    def remove_obstacle(self, x, y):
        """Remove obstacle at position (10cm tolerance)

        Args:
            x, y: Position in meters
        """
        cmd = {
            "action": "remove",
            "obstacle": {
                "pos": [x, y],
                "shape": "rectangle",  # Shape doesn't matter for removal
                "size": [0.0, 0.0]
            }
        }
        self.hub.send(cmd)
        print(f"✓ Sent remove command for obstacle at ({x}, {y})")


def demo_simple():
    """Simple demo: add a few obstacles"""
    print("\n" + "=" * 70)
    print("DEMO: Simple Obstacle Spawning")
    print("=" * 70)

    controller = ObstacleController()

    print("\n1. Adding red rectangular obstacle...")
    controller.add_rectangle(3.0, 2.0, 1.5, 1.0, color=[0.8, 0.2, 0.2])
    time.sleep(1)

    print("\n2. Adding green circular obstacle...")
    controller.add_circle(-2.0, 4.0, 0.8, color=[0.2, 0.8, 0.2])
    time.sleep(1)

    print("\n3. Adding blue rectangular obstacle...")
    controller.add_rectangle(-4.0, -3.0, 2.0, 0.5, color=[0.2, 0.2, 0.8])
    time.sleep(1)

    print("\nDemo complete! Obstacles should be visible in sim2d.")


def demo_grid():
    """Create a grid of colored obstacles"""
    print("\n" + "=" * 70)
    print("DEMO: Grid of Obstacles")
    print("=" * 70)

    controller = ObstacleController()

    print("\nCreating 3x3 grid of circular obstacles...")
    colors = [
        [1.0, 0.0, 0.0],  # Red
        [0.0, 1.0, 0.0],  # Green
        [0.0, 0.0, 1.0],  # Blue
        [1.0, 1.0, 0.0],  # Yellow
        [1.0, 0.0, 1.0],  # Magenta
        [0.0, 1.0, 1.0],  # Cyan
        [1.0, 0.5, 0.0],  # Orange
        [0.5, 0.0, 1.0],  # Purple
        [0.0, 1.0, 0.5],  # Mint
    ]

    idx = 0
    for i in range(-1, 2):
        for j in range(-1, 2):
            x = i * 2.0
            y = j * 2.0
            controller.add_circle(x, y, 0.4, color=colors[idx])
            idx += 1
            time.sleep(0.3)

    print("\nGrid created! Watch them appear in sim2d.")


def demo_animation():
    """Animated obstacle spawning and removal"""
    print("\n" + "=" * 70)
    print("DEMO: Animated Obstacles")
    print("=" * 70)

    controller = ObstacleController()

    print("\nCreating moving wave of obstacles...")
    positions = []

    # Spawn wave
    for i in range(5):
        x = -4.0 + i * 2.0
        y = 0.0
        controller.add_rectangle(x, y, 0.5, 2.0, color=[0.0, 0.5 + i*0.1, 1.0])
        positions.append((x, y))
        time.sleep(0.5)

    print("\nRemoving obstacles in reverse order...")
    time.sleep(1)

    # Remove in reverse
    for x, y in reversed(positions):
        controller.remove_obstacle(x, y)
        time.sleep(0.5)

    print("\nAnimation complete!")


def interactive_menu():
    """Interactive menu for manual control"""
    print("\n" + "=" * 70)
    print("INTERACTIVE OBSTACLE CONTROL")
    print("=" * 70)

    controller = ObstacleController()

    while True:
        print("\n" + "-" * 70)
        print("Options:")
        print("  1. Add rectangle")
        print("  2. Add circle")
        print("  3. Remove obstacle")
        print("  4. Run simple demo")
        print("  5. Run grid demo")
        print("  6. Run animation demo")
        print("  q. Quit")
        print("-" * 70)

        choice = input("Enter choice: ").strip().lower()

        if choice == 'q':
            print("Exiting...")
            break

        elif choice == '1':
            try:
                x = float(input("X position (meters): "))
                y = float(input("Y position (meters): "))
                w = float(input("Width (meters): "))
                h = float(input("Height (meters): "))

                use_color = input("Use custom color? (y/n): ").strip().lower()
                color = None
                if use_color == 'y':
                    r = float(input("Red (0.0-1.0): "))
                    g = float(input("Green (0.0-1.0): "))
                    b = float(input("Blue (0.0-1.0): "))
                    color = [r, g, b]

                controller.add_rectangle(x, y, w, h, color)
            except ValueError as e:
                print(f"Error: {e}")

        elif choice == '2':
            try:
                x = float(input("X position (meters): "))
                y = float(input("Y position (meters): "))
                radius = float(input("Radius (meters): "))

                use_color = input("Use custom color? (y/n): ").strip().lower()
                color = None
                if use_color == 'y':
                    r = float(input("Red (0.0-1.0): "))
                    g = float(input("Green (0.0-1.0): "))
                    b = float(input("Blue (0.0-1.0): "))
                    color = [r, g, b]

                controller.add_circle(x, y, radius, color)
            except ValueError as e:
                print(f"Error: {e}")

        elif choice == '3':
            try:
                x = float(input("X position (meters): "))
                y = float(input("Y position (meters): "))
                controller.remove_obstacle(x, y)
            except ValueError as e:
                print(f"Error: {e}")

        elif choice == '4':
            demo_simple()

        elif choice == '5':
            demo_grid()

        elif choice == '6':
            demo_animation()

        else:
            print("Invalid choice. Try again.")


def main():
    print("=" * 70)
    print(" sim2d Dynamic Obstacle Control")
    print("=" * 70)
    print("\nThis script allows you to add/remove obstacles in a running sim2d")
    print("simulator. Make sure sim2d is already running before proceeding!")
    print("\nMessage Format:")
    print("  Topic: /sim2d/obstacle_cmd")
    print("  Actions: 'add' or 'remove'")
    print("  Shapes: 'rectangle' or 'circle'")
    print("  Colors: [R, G, B] with values 0.0-1.0 (optional)")

    if len(sys.argv) > 1:
        mode = sys.argv[1]
        if mode == "simple":
            demo_simple()
        elif mode == "grid":
            demo_grid()
        elif mode == "animation":
            demo_animation()
        elif mode == "interactive":
            interactive_menu()
        else:
            print(f"\nUnknown mode: {mode}")
            print("Available modes: simple, grid, animation, interactive")
    else:
        print("\nUsage:")
        print("  python3 dynamic_obstacles.py simple       - Run simple demo")
        print("  python3 dynamic_obstacles.py grid         - Create obstacle grid")
        print("  python3 dynamic_obstacles.py animation    - Run animation demo")
        print("  python3 dynamic_obstacles.py interactive  - Interactive menu")
        print("\nDefaulting to interactive mode...\n")
        time.sleep(1)
        interactive_menu()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nInterrupted by user. Exiting...")
        sys.exit(0)
