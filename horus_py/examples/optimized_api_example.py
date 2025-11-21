#!/usr/bin/env python3
"""
HORUS Python Bindings - Typed Hub API Example

Demonstrates the new type-based Hub API where the message type
determines the topic name automatically.

NEW API (Type-based):
    hub = Hub(Pose2D)         # Type determines topic ("robot_pose")
    hub.send(pose, node)      # Type-safe send

OLD API (Name-based):
    hub = Hub("robot_pose")   # Manual topic naming
    hub.send(data, node)      # Untyped data

Benefits of Typed Hub:
1. Type safety - compiler catches errors
2. Auto topic naming - types have __topic_name__
3. Zero-copy IPC - direct struct serialization
4. Cross-language - works with Rust/C++/etc
5. Better IDE support - autocomplete, type hints
"""

from horus import Hub, Pose2D, CmdVel, Node, run
import math
import time


def example_1_typed_pose():
    """
    Demonstrate typed Hub with Pose2D messages

    Type determines topic: Pose2D → "robot_pose"
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 1: Typed Hub with Pose2D")
    print("=" * 70)

    # NEW API: Hub(MessageType) - type determines topic
    pose_hub = Hub(Pose2D)
    print(f"Created Hub for type: Pose2D")
    print(f"Topic name (from __topic_name__): {pose_hub.topic()}")

    # Create and send typed message
    pose = Pose2D(x=1.5, y=2.3, theta=0.785)
    success = pose_hub.send(pose)
    print(f"Sent Pose2D: x={pose.x}, y={pose.y}, theta={pose.theta}")
    print(f"Success: {success}")

    # Receive typed message
    received = pose_hub.recv()
    if received:
        print(f"Received Pose2D: x={received.x}, y={received.y}, theta={received.theta}")
    else:
        print("No message available (expected - same hub)")


def example_2_typed_cmdvel():
    """
    Demonstrate typed Hub with CmdVel messages

    Type determines topic: CmdVel → "cmd_vel"
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 2: Typed Hub with CmdVel")
    print("=" * 70)

    # NEW API: Hub(MessageType)
    cmd_hub = Hub(CmdVel)
    print(f"Created Hub for type: CmdVel")
    print(f"Topic name (from __topic_name__): {cmd_hub.topic()}")

    # Create and send velocity command
    cmd = CmdVel(linear=1.5, angular=0.5)
    success = cmd_hub.send(cmd)
    print(f"Sent CmdVel: linear={cmd.linear}, angular={cmd.angular}")
    print(f"Success: {success}")

    # Receive typed message
    received = cmd_hub.recv()
    if received:
        print(f"Received CmdVel: linear={received.linear}, angular={received.angular}")
    else:
        print("No message available (expected - same hub)")


def example_3_pub_sub_nodes():
    """
    Demonstrate publisher/subscriber pattern with typed hubs

    Shows cross-node communication with typed messages
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 3: Publisher/Subscriber Nodes")
    print("=" * 70)

    # Global hubs for pub/sub
    _pose_pub = Hub(Pose2D)
    _pose_sub = Hub(Pose2D)

    # Publisher node
    def publisher_tick(node):
        t = time.time()
        x = 2.0 * math.cos(t * 0.5)
        y = 2.0 * math.sin(t * 0.5)
        theta = t * 0.5

        pose = Pose2D(x=x, y=y, theta=theta)
        _pose_pub.send(pose, node)
        node.log_info(f"Published: ({x:.2f}, {y:.2f}, {theta:.2f})")

    # Subscriber node
    def subscriber_tick(node):
        pose = _pose_sub.recv(node)
        if pose:
            node.log_info(f"Received: ({pose.x:.2f}, {pose.y:.2f}, {pose.theta:.2f})")

    # Create nodes
    publisher = Node(name="publisher", tick=publisher_tick, rate=10)
    subscriber = Node(name="subscriber", tick=subscriber_tick, rate=10)

    print("Running publisher/subscriber for 3 seconds...")
    print("(Ctrl+C to stop early)")

    # Run for 3 seconds
    try:
        run(publisher, subscriber, duration=3, logging=True)
    except KeyboardInterrupt:
        print("\nStopped by user")

    print("Done!")


def example_4_robot_control():
    """
    Real-world example: Robot pose estimation and control

    Demonstrates typical robotics pattern:
    - Sensor node publishes pose
    - Controller node reads pose, publishes commands
    """
    print("\n" + "=" * 70)
    print("EXAMPLE 4: Robot Control Loop")
    print("=" * 70)

    _pose_hub = Hub(Pose2D)
    _cmd_hub = Hub(CmdVel)

    # Simulated sensor (odometry)
    def sensor_tick(node):
        t = time.time()
        pose = Pose2D(
            x=math.cos(t) * 2.0,
            y=math.sin(t) * 2.0,
            theta=t
        )
        _pose_hub.send(pose, node)

    # Controller - simple proportional control
    def controller_tick(node):
        pose = _pose_hub.recv(node)
        if pose:
            # Simple controller: drive toward origin
            distance = math.sqrt(pose.x**2 + pose.y**2)
            angle_to_origin = math.atan2(-pose.y, -pose.x)
            angle_error = angle_to_origin - pose.theta

            # Generate velocity command
            linear = min(distance * 0.5, 1.0)  # Proportional, capped at 1.0 m/s
            angular = angle_error * 2.0  # Proportional turning

            cmd = CmdVel(linear=linear, angular=angular)
            _cmd_hub.send(cmd, node)

            node.log_info(f"Distance: {distance:.2f}m, Command: {linear:.2f}m/s, {angular:.2f}rad/s")

    # Create nodes
    sensor = Node(name="sensor", tick=sensor_tick, rate=30)
    controller = Node(name="controller", tick=controller_tick, rate=30)

    print("Running robot control for 3 seconds...")
    print("(Ctrl+C to stop early)")

    try:
        run(sensor, controller, duration=3, logging=True)
    except KeyboardInterrupt:
        print("\nStopped by user")

    print("Done!")


def main():
    print("\n" + "=" * 70)
    print(" HORUS Python Bindings - Typed Hub API Examples")
    print("=" * 70)
    print("\nThis demonstrates the new type-based Hub API:")
    print("  • Hub(MessageType) - type determines topic")
    print("  • hub.send(message) - type-safe sending")
    print("  • hub.recv() - type-safe receiving")
    print("  • Zero-copy IPC with typed structs")
    print("  • Cross-language compatibility (Rust/Python/C++)")

    example_1_typed_pose()
    example_2_typed_cmdvel()
    example_3_pub_sub_nodes()
    example_4_robot_control()

    print("\n" + "=" * 70)
    print(" Summary: Typed Hub Benefits")
    print("=" * 70)
    print("\nType safety: Compiler catches mismatched types")
    print("Auto topic naming: Types have __topic_name__ attribute")
    print("Zero-copy: Direct struct serialization (no pickle overhead)")
    print("Cross-language: Same API in Rust, Python, C++")
    print("IDE support: Autocomplete and type hints")
    print("\nSupported message types (horus.library):")
    print("  Geometry: Pose2D, CmdVel, Twist, Transform, Point3, Vector3, Quaternion")
    print("  Control: MotorCommand, DifferentialDriveCommand, ServoCommand")
    print("           PwmCommand, StepperCommand, PidConfig")
    print("  Sensors: LaserScan, Imu, Odometry, Range, BatteryState, NavSatFix")
    print("  I/O: DigitalIO, AnalogIO")
    print("  Input: JoystickInput, KeyboardInput")
    print("  Diagnostics: Status, EmergencyStop, Heartbeat, ResourceUsage")
    print("\nFor complete API documentation, see horus_library/python/src/")
    print("All types support zero-copy serialization and cross-language communication")
    print("=" * 70 + "\n")


if __name__ == "__main__":
    main()
