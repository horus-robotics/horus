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

# ============================================================================
# VISION PROCESSING NODES
# ============================================================================

class QrScannerNode(Node):
    """Simulates QR code scanner for warehouse inventory"""

    def __init__(self):
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
        super().__init__(name="ObjectDetectorNode", pubs=['vision/objects'])
        self.frame_count = 0

    def name(self) -> str:
        return "ObjectDetectorNode"

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

            self.send({"objects": objects, "timestamp": time.time()})


# ============================================================================
# LOCALIZATION NODES
# ============================================================================

class SlamNode(Node):
    """Simulates SLAM (Simultaneous Localization and Mapping)"""

    def __init__(self):
        super().__init__(name="SlamNode", pubs=['localization/map', 'localization/pose'])
        self.position = [0.0, 0.0, 0.0]  # x, y, theta
        self.map_size = 0

    def name(self) -> str:
        return "SlamNode"

    def tick(self):
        # Simulate robot movement
        self.position[0] += random.uniform(-0.01, 0.01)
        self.position[1] += random.uniform(-0.01, 0.01)
        self.position[2] += random.uniform(-0.05, 0.05)

        # Publish pose estimate
        pose = {
            "x": self.position[0],
            "y": self.position[1],
            "theta": self.position[2],
            "covariance": [0.01, 0.01, 0.02],
            "timestamp": time.time()
        }
        self.send("topic", pose)

        # Periodically publish map updates
        self.map_size += 1
        if self.map_size % 200 == 0:
            map_data = {
                "resolution": 0.05,  # 5cm per cell
                "width": 200,
                "height": 200,
                "occupied_cells": self.map_size,
                "timestamp": time.time()
            }
            self.send("topic", map_data)


class PositionEstimatorNode(Node):
    """Fuses multiple sensors for position estimation"""

    def __init__(self):
        super().__init__(name="PositionEstimatorNode", pubs=['localization/position_estimate'], subs=['localization/pose', 'vision/qr_codes'])
        self.last_qr_correction = 0

    def name(self) -> str:
        return "PositionEstimatorNode"

    def tick(self):
        # Get SLAM pose
        slam_pose = self.recv()

        # Check for QR code corrections
        qr_data = self.recv()
        if qr_data:
            self.last_qr_correction = time.time()

        if slam_pose:
            # Publish fused estimate
            estimate = {
                "x": slam_pose["x"],
                "y": slam_pose["y"],
                "theta": slam_pose["theta"],
                "confidence": 0.95 if (time.time() - self.last_qr_correction) < 5.0 else 0.75,
                "source": "fused",
                "timestamp": time.time()
            }
            self.send("topic", estimate)


# ============================================================================
# TASK MANAGEMENT NODES
# ============================================================================

class TaskSchedulerNode(Node):
    """Manages warehouse tasks and assigns priorities"""

    def __init__(self):
        super().__init__(name="TaskSchedulerNode", pubs=['tasks/current_task', 'tasks/status'])
        self.current_task = None
        self.task_queue = [
            {"id": 1, "type": "pick", "shelf": "A01", "item": "SKU-12345"},
            {"id": 2, "type": "pick", "shelf": "B12", "item": "SKU-67890"},
            {"id": 3, "type": "deliver", "dock": "DOCK-01"},
            {"id": 4, "type": "return", "shelf": "C33"},
        ]
        self.task_index = 0
        self.tick_count = 0

    def name(self) -> str:
        return "TaskSchedulerNode"

    def tick(self):
        self.tick_count += 1

        # Assign new task every 500 ticks
        if self.tick_count % 500 == 0:
            if self.task_queue:
                self.current_task = self.task_queue[self.task_index % len(self.task_queue)]
                self.task_index += 1
                self.send("topic", self.current_task)

        # Publish status
        status = {
            "queue_size": len(self.task_queue),
            "active_task": self.current_task["id"] if self.current_task else None,
            "completed_today": self.task_index,
            "timestamp": time.time()
        }
        self.send("topic", status)


class PathExecutorNode(Node):
    """Executes planned paths to reach task locations"""

    def __init__(self):
        super().__init__(name="PathExecutorNode", pubs=['control/cmd_vel'], subs=['tasks/current_task', 'localization/position_estimate'])
        self.current_target = None

    def name(self) -> str:
        return "PathExecutorNode"

    def tick(self):
        # Get current task
        task = self.recv()
        if task:
            # Simple goal: navigate to shelf or dock
            if task["type"] == "pick":
                # Shelf locations (hardcoded for demo)
                shelf_positions = {
                    "A01": (5.0, 2.0),
                    "B12": (10.0, 5.0),
                    "C33": (15.0, 8.0)
                }
                self.current_target = shelf_positions.get(task["shelf"], (0, 0))
            elif task["type"] == "deliver":
                self.current_target = (20.0, 10.0)  # Dock location

        # Get current position
        pose = self.recv()

        if pose and self.current_target:
            # Simple proportional controller
            dx = self.current_target[0] - pose["x"]
            dy = self.current_target[1] - pose["y"]
            distance = math.sqrt(dx**2 + dy**2)

            if distance > 0.1:
                linear = min(distance * 0.5, 1.0)
                angular = math.atan2(dy, dx) - pose["theta"]

                # Normalize angular to [-pi, pi]
                while angular > math.pi:
                    angular -= 2 * math.pi
                while angular < -math.pi:
                    angular += 2 * math.pi

                cmd = {
                    "linear": linear,
                    "angular": angular * 0.5,
                    "timestamp": time.time()
                }
                self.send("topic", cmd)


# ============================================================================
# SAFETY NODES
# ============================================================================

class CollisionDetectorNode(Node):
    """Monitors for potential collisions"""

    def __init__(self):
        super().__init__(name="CollisionDetectorNode", pubs=['safety/collision_alert'], subs=['vision/objects'])
        self.alerts_sent = 0

    def name(self) -> str:
        return "CollisionDetectorNode"

    def tick(self):
        detection = self.recv()

        if detection and detection.get("objects"):
            # Check for objects in danger zone
            for obj in detection["objects"]:
                if obj["class"] in ["person", "forklift"]:
                    # Check if object is close (in center of image)
                    x, y, w, h = obj["bbox"]
                    center_x = x + w/2

                    if 200 < center_x < 440:  # Center zone of 640px image
                        self.alerts_sent += 1
                        alert = {
                            "type": "collision_warning",
                            "object_class": obj["class"],
                            "distance_estimate": 2.0 / (w / 100.0),  # Rough estimate
                            "severity": "high" if obj["class"] == "person" else "medium",
                            "timestamp": time.time()
                        }
                        self.send("topic", alert)


class EmergencyHandlerNode(Node):
    """Handles emergency stops and safety overrides"""

    def __init__(self):
        super().__init__(name="EmergencyHandlerNode", pubs=['control/cmd_vel_safe', 'safety/status'], subs=['safety/collision_alert', 'control/cmd_vel'])
        self.emergency_active = False
        self.last_alert_time = 0

    def name(self) -> str:
        return "EmergencyHandlerNode"

    def tick(self):
        # Check for collision alerts
        alert = self.recv()
        if alert:
            self.emergency_active = True
            self.last_alert_time = time.time()

        # Clear emergency after 3 seconds
        if self.emergency_active and (time.time() - self.last_alert_time) > 3.0:
            self.emergency_active = False

        # Get command velocity
        cmd = self.recv()

        if self.emergency_active:
            # Override with stop command
            safe_cmd = {
                "linear": 0.0,
                "angular": 0.0,
                "timestamp": time.time()
            }
            self.send("topic", safe_cmd)
        elif cmd:
            # Pass through
            self.send("topic", cmd)

        # Publish safety status
        status = {
            "emergency_active": self.emergency_active,
            "last_alert": self.last_alert_time,
            "system_status": "SAFE" if not self.emergency_active else "EMERGENCY_STOP",
            "timestamp": time.time()
        }
        self.send("topic", status)


# ============================================================================
# PERFORMANCE MONITORING NODE
# ============================================================================

class PerformanceMonitorNode(Node):
    """Monitors system performance metrics"""

    def __init__(self):
        super().__init__(name="PerformanceMonitorNode", pubs=['system/performance'])
        self.start_time = time.time()
        self.tick_count = 0
        self.last_report = time.time()

    def name(self) -> str:
        return "PerformanceMonitorNode"

    def tick(self):
        self.tick_count += 1

        # Report every 200 ticks
        if self.tick_count % 200 == 0:
            now = time.time()
            elapsed = now - self.last_report
            tick_rate = 200 / elapsed if elapsed > 0 else 0

            stats = {
                "uptime": now - self.start_time,
                "tick_rate": tick_rate,
                "total_ticks": self.tick_count,
                "cpu_percent": random.uniform(20, 60),  # Simulated
                "memory_mb": random.uniform(150, 300),  # Simulated
                "timestamp": now
            }
            self.send("topic", stats)
            self.last_report = now


# ============================================================================
# MAIN
# ============================================================================

def main():
    print("🏭 Starting Warehouse Robot System (Python)")
    print(" Dashboard available at: http://localhost:8080")
    print(" Run 'horus dashboard' in another terminal to monitor\n")

    scheduler = Scheduler()

    # Vision Processing Layer (Priority 0-9: Highest)
    scheduler.add(QrScannerNode(), priority=0, logging=True)
    scheduler.add(ObjectDetectorNode(), priority=1, logging=True)

    # Localization Layer (Priority 10-19: High)
    scheduler.add(SlamNode(), priority=10, logging=True)
    scheduler.add(PositionEstimatorNode(), priority=11, logging=True)

    # Task Management Layer (Priority 20-29: Medium)
    scheduler.add(TaskSchedulerNode(), priority=20, logging=True)
    scheduler.add(PathExecutorNode(), priority=21, logging=True)

    # Safety Layer (Priority 30-34: High within layer)
    scheduler.add(CollisionDetectorNode(), priority=30, logging=True)
    scheduler.add(EmergencyHandlerNode(), priority=31, logging=True)

    # Monitoring Layer (Priority 40+: Low)
    scheduler.add(PerformanceMonitorNode(), priority=40, logging=True)

    print(" All nodes registered:")
    print("   - 2 Vision nodes")
    print("   - 2 Localization nodes")
    print("   - 2 Task management nodes")
    print("   - 2 Safety nodes")
    print("   - 1 Performance monitoring node")
    print("\n Starting scheduler...\n")

    scheduler.run()


if __name__ == "__main__":
    main()
