#!/usr/bin/env python3
"""
Complete Warehouse Robot System - Python
This demonstrates a full robotics application with:
- Vision processing nodes (QR Scanner, Object Detector)
- Localization nodes (SLAM, Position Estimator)
- Task management nodes (Task Scheduler, Path Executor)
- Safety nodes (Collision Detector, Emergency Handler)
"""

import time
import random
import math
from horus import Scheduler, Node
from horus.library import Pose2D, CmdVel

# ============================================================================
# VISION PROCESSING NODES
# ============================================================================

class QrScannerNode(Node):
    """Simulates QR code scanner for warehouse inventory"""

    def __init__(self):
        super().__init__(name="QrScannerNode", pubs=["vision/qr_codes"])
        self.scan_count = 0
        self.known_codes = ["SHELF-A01", "SHELF-B12", "SHELF-C33", "DOCK-01"]

    def tick(self):
        self.scan_count += 1

        # Simulate scanning QR codes
        if self.scan_count % 100 == 0:
            code = random.choice(self.known_codes)
            confidence = random.uniform(0.85, 0.99)
            data = {
                "code": code,
                "confidence": confidence,
                "timestamp": time.time()
            }
            self.send("vision/qr_codes", data)


class ObjectDetectorNode(Node):
    """Simulates object detection for obstacle awareness"""

    def __init__(self):
        super().__init__(name="ObjectDetectorNode", pubs=["vision/objects"])
        self.frame_count = 0

    def tick(self):
        self.frame_count += 1

        # Simulate detecting objects in camera view
        if self.frame_count % 50 == 0:
            num_objects = random.randint(0, 5)
            objects = []

            for i in range(num_objects):
                obj = {
                    "class": random.choice(["person", "forklift", "pallet", "box"]),
                    "confidence": random.uniform(0.7, 0.95),
                    "bbox": [
                        random.randint(0, 640),
                        random.randint(0, 480),
                        random.randint(50, 200),
                        random.randint(50, 200)
                    ]
                }
                objects.append(obj)

            self.send("vision/objects", {"objects": objects, "timestamp": time.time()})


# ============================================================================
# LOCALIZATION NODES
# ============================================================================

class SlamNode(Node):
    """Simulates SLAM (Simultaneous Localization and Mapping)"""

    def __init__(self):
        super().__init__(
            name="SlamNode",
            pubs={
                "localization/map": {},  # Generic hub for map data
                "localization/pose": {"type": Pose2D}  # Typed hub for pose
            }
        )
        self.position = [0.0, 0.0, 0.0]  # x, y, theta
        self.map_size = 0

    def tick(self):
        # Simulate robot movement
        self.position[0] += random.uniform(-0.1, 0.1)
        self.position[1] += random.uniform(-0.1, 0.1)
        self.position[2] += random.uniform(-0.05, 0.05)

        # Publish pose frequently (using typed Pose2D - proper logging!)
        pose = Pose2D(
            x=self.position[0],
            y=self.position[1],
            theta=self.position[2]
        )
        self.send("localization/pose", pose)

        # Publish map updates less frequently (generic hub)
        self.map_size += random.randint(0, 5)
        if random.random() < 0.1:  # 10% chance
            map_data = {
                "width": 100,
                "height": 100,
                "resolution": 0.05,
                "occupied_cells": self.map_size
            }
            self.send("localization/map", map_data)


class PositionEstimatorNode(Node):
    """Fuses SLAM and other sensors for position estimate"""

    def __init__(self):
        super().__init__(
            name="PositionEstimatorNode",
            subs={
                "localization/pose": {"type": Pose2D},  # Typed sub
                "vision/qr_codes": {}  # Generic sub
            },
            pubs=["localization/position_estimate"]
        )
        self.last_slam_pose = None
        self.last_qr_correction = None

    def tick(self):
        # Get SLAM pose (now receives typed Pose2D object)
        if self.has_msg("localization/pose"):
            self.last_slam_pose = self.get("localization/pose")

        # Get QR code corrections
        if self.has_msg("vision/qr_codes"):
            qr_data = self.get("vision/qr_codes")
            self.last_qr_correction = qr_data

        # Fuse data and publish estimate
        if self.last_slam_pose:
            estimate = {
                "x": self.last_slam_pose.x,  # Accessing Pose2D attributes
                "y": self.last_slam_pose.y,
                "theta": self.last_slam_pose.theta,
                "confidence": 0.9 if self.last_qr_correction else 0.7
            }
            self.send("localization/position_estimate", estimate)


# ============================================================================
# TASK MANAGEMENT NODES
# ============================================================================

class TaskSchedulerNode(Node):
    """Assigns tasks to robots"""

    def __init__(self):
        super().__init__(
            name="TaskSchedulerNode",
            pubs=["tasks/current_task", "tasks/status"]
        )
        self.task_id = 0
        self.tasks = ["PICK_A01", "MOVE_TO_DOCK", "PICK_B12", "DELIVER_C33"]

    def tick(self):
        # Generate new task periodically
        if random.random() < 0.02:  # 2% chance
            self.task_id += 1
            task = {
                "id": self.task_id,
                "type": random.choice(self.tasks),
                "priority": random.randint(1, 5),
                "assigned_time": time.time()
            }
            self.send("tasks/current_task", task)
            self.send("tasks/status", {"active_tasks": self.task_id})


class PathExecutorNode(Node):
    """Executes path planning and sends velocity commands"""

    def __init__(self):
        super().__init__(
            name="PathExecutorNode",
            subs=["tasks/current_task", "localization/position_estimate"],
            pubs={"control/cmd_vel": {"type": CmdVel}}  # Typed pub
        )
        self.current_task = None
        self.current_position = None

    def tick(self):
        # Get current task
        if self.has_msg("tasks/current_task"):
            self.current_task = self.get("tasks/current_task")

        # Get current position
        if self.has_msg("localization/position_estimate"):
            self.current_position = self.get("localization/position_estimate")

        # Execute task (simplified) - using typed CmdVel
        if self.current_task and self.current_position:
            cmd = CmdVel(
                linear=random.uniform(0.0, 0.5),
                angular=random.uniform(-0.3, 0.3)
            )
            self.send("control/cmd_vel", cmd)


# ============================================================================
# SAFETY NODES
# ============================================================================

class CollisionDetectorNode(Node):
    """Monitors for potential collisions"""

    def __init__(self):
        super().__init__(
            name="CollisionDetectorNode",
            subs=["vision/objects"],
            pubs=["safety/collision_alert"]
        )

    def tick(self):
        if self.has_msg("vision/objects"):
            objects = self.get("vision/objects")
            
            # Check if any objects are too close
            for obj in objects.get("objects", []):
                if obj["class"] in ["person", "forklift"]:
                    # Simulate danger detection
                    if random.random() < 0.1:  # 10% chance
                        alert = {
                            "severity": "high",
                            "object_type": obj["class"],
                            "timestamp": time.time()
                        }
                        self.send("safety/collision_alert", alert)


class EmergencyHandlerNode(Node):
    """Handles emergency stops and safety overrides"""

    def __init__(self):
        super().__init__(
            name="EmergencyHandlerNode",
            subs={
                "safety/collision_alert": {},  # Generic sub
                "control/cmd_vel": {"type": CmdVel}  # Typed sub
            },
            pubs={
                "control/cmd_vel_safe": {"type": CmdVel},  # Typed pub
                "safety/status": {}  # Generic pub
            }
        )
        self.emergency_stop = False

    def tick(self):
        # Check for collision alerts
        if self.has_msg("safety/collision_alert"):
            alert = self.get("safety/collision_alert")
            if alert["severity"] == "high":
                self.emergency_stop = True
                self.send("safety/status", {"emergency_stop": True})

        # Monitor velocity commands (now receives typed CmdVel)
        if self.has_msg("control/cmd_vel"):
            cmd = self.get("control/cmd_vel")

            if self.emergency_stop:
                # Override with stop command (typed CmdVel)
                safe_cmd = CmdVel(linear=0.0, angular=0.0)
            else:
                # Pass through the typed CmdVel
                safe_cmd = cmd

            self.send("control/cmd_vel_safe", safe_cmd)

        # Clear emergency stop after some time
        if self.emergency_stop and random.random() < 0.05:
            self.emergency_stop = False


# ============================================================================
# MONITORING NODE
# ============================================================================

class PerformanceMonitorNode(Node):
    """Monitors system performance"""

    def __init__(self):
        super().__init__(
            name="PerformanceMonitorNode",
            pubs=["system/performance"]
        )
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        
        if self.tick_count % 100 == 0:
            stats = {
                "ticks": self.tick_count,
                "uptime": time.time(),
                "cpu_usage": random.uniform(10, 60)
            }
            self.send("system/performance", stats)


# ============================================================================
# MAIN
# ============================================================================

if __name__ == "__main__":
    print("Starting Warehouse Robot Fleet System...")
    
    # Create scheduler
    scheduler = Scheduler()
    
    # Add vision nodes
    scheduler.add(QrScannerNode(), priority=0, logging=True)
    scheduler.add(ObjectDetectorNode(), priority=0, logging=True)
    
    # Add localization nodes
    scheduler.add(SlamNode(), priority=1, logging=True)
    scheduler.add(PositionEstimatorNode(), priority=1, logging=True)
    
    # Add task management nodes
    scheduler.add(TaskSchedulerNode(), priority=2, logging=True)
    scheduler.add(PathExecutorNode(), priority=2, logging=True)
    
    # Add safety nodes
    scheduler.add(CollisionDetectorNode(), priority=3, logging=True)
    scheduler.add(EmergencyHandlerNode(), priority=3, logging=True)
    
    # Add monitoring
    scheduler.add(PerformanceMonitorNode(), priority=4, logging=True)
    
    print("All nodes initialized. Running for 10 seconds...")
    
    # Run for 10 seconds
    scheduler.run(duration=10.0)
    
    print("System shutdown complete.")
