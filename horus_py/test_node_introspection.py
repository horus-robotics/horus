#!/usr/bin/env python3
"""
Test script for node introspection features
"""
import horus
import time
import threading

print("=" * 70)
print("Node Introspection Test")
print("=" * 70)

# Create some test nodes
class SensorNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        time.sleep(0.01)  # Simulate some work

class ControlNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        # Occasionally fail to test fault tolerance
        if self.tick_count in [5, 6]:
            raise RuntimeError("Test failure")

class ActuatorNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        time.sleep(0.005)  # Simulate some work

# Create scheduler and add nodes
config = horus.SchedulerConfig.standard()
config.tick_rate = 20.0  # 20 Hz
config.circuit_breaker = True
config.max_failures = 3

scheduler = horus.Scheduler.from_config(config)

sensor = SensorNode()
control = ControlNode()
actuator = ActuatorNode()

scheduler.add(sensor, priority=1)
scheduler.add(control, priority=2)
scheduler.add(actuator, priority=3)

print(f"\nConfiguration:")
print(f"  Tick Rate: {config.tick_rate} Hz")
print(f"  Circuit Breaker: {config.circuit_breaker}")
print(f"  Max Failures: {config.max_failures}")
print()

# Test introspection before running
print("=" * 70)
print("Before Running:")
print("=" * 70)

# Test get_node_count()
count = scheduler.get_node_count()
print(f"\nTotal nodes: {count}")

# Test get_node_names()
names = scheduler.get_node_names()
print(f"Node names: {names}")

# Test has_node()
print(f"Has 'node_' prefix: {scheduler.has_node('node_')}")
print(f"Node exists check: {any(scheduler.has_node(name) for name in names)}")

# Test get_all_nodes()
print("\nAll nodes (before running):")
nodes = scheduler.get_all_nodes()
for node_info in nodes:
    print(f"  {node_info['name']}:")
    print(f"    Priority: {node_info['priority']}")
    print(f"    Rate: {node_info['rate_hz']} Hz")
    print(f"    Total ticks: {node_info.get('total_ticks', 0)}")

# Run scheduler for a short duration
print("\n" + "=" * 70)
print("Running scheduler for 3 seconds...")
print("=" * 70)

def stop_after(seconds):
    time.sleep(seconds)
    scheduler.stop()

stop_thread = threading.Thread(target=lambda: stop_after(3), daemon=True)
stop_thread.start()

try:
    scheduler.run()
except Exception as e:
    print(f"Scheduler error: {e}")

# Test introspection after running
print("\n" + "=" * 70)
print("After Running:")
print("=" * 70)

# Test get_all_nodes() with runtime stats
print("\nAll nodes (after running):")
nodes = scheduler.get_all_nodes()
for node_info in nodes:
    print(f"\n  {node_info['name']}:")
    print(f"    Priority: {node_info['priority']}")
    print(f"    Rate: {node_info['rate_hz']} Hz")
    print(f"    State: {node_info.get('state', 'unknown')}")
    print(f"    Uptime: {node_info.get('uptime_seconds', 0):.2f}s")
    print(f"    Total ticks: {node_info.get('total_ticks', 0)}")
    print(f"    Successful ticks: {node_info.get('successful_ticks', 0)}")
    print(f"    Failed ticks: {node_info.get('failed_ticks', 0)}")
    print(f"    Failure count: {node_info.get('failure_count', 0)}")
    print(f"    Consecutive failures: {node_info.get('consecutive_failures', 0)}")
    print(f"    Circuit open: {node_info.get('circuit_open', False)}")
    print(f"    Avg tick duration: {node_info.get('avg_tick_duration_ms', 0):.3f}ms")

# Test get_node_stats() for specific node
print("\n" + "=" * 70)
print("Detailed Stats for Control Node:")
print("=" * 70)

# Find the control node name
control_name = None
for node_info in nodes:
    if node_info['priority'] == 2:
        control_name = node_info['name']
        break

if control_name:
    stats = scheduler.get_node_stats(control_name)
    print(f"\n{control_name} detailed stats:")
    for key, value in sorted(stats.items()):
        if isinstance(value, float):
            print(f"  {key}: {value:.3f}")
        else:
            print(f"  {key}: {value}")
else:
    print("Control node not found")

# Summary
print("\n" + "=" * 70)
print("Summary:")
print("=" * 70)

total_ticks = sum(n.get('total_ticks', 0) for n in nodes)
total_failures = sum(n.get('failure_count', 0) for n in nodes)
nodes_with_circuit_open = sum(1 for n in nodes if n.get('circuit_open', False))

print(f"\nTotal ticks across all nodes: {total_ticks}")
print(f"Total failures across all nodes: {total_failures}")
print(f"Nodes with open circuit breaker: {nodes_with_circuit_open}")
print(f"Active nodes: {scheduler.get_node_count()}")

print("\n" + "=" * 70)
print("Test complete!")
print("=" * 70)
