#!/usr/bin/env python3
"""
Demo: Soft Real-Time Scheduling Features
Shows deadline monitoring and violation tracking
"""
import horus
import time
import threading

print("=" * 70)
print("HORUS Soft Real-Time Scheduling Demo")
print("=" * 70)

# Create nodes with varying execution times
class FastNode(horus.Node):
    """Node that executes quickly (within deadline)"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        time.sleep(0.002)  # 2ms - should be within 10ms deadline

class SlowNode(horus.Node):
    """Node that occasionally violates deadline"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        # Occasionally take longer than deadline
        if self.tick_count % 5 == 0:
            time.sleep(0.025)  # 25ms - exceeds 10ms deadline
        else:
            time.sleep(0.005)  # 5ms - within deadline

class VariableNode(horus.Node):
    """Node with highly variable execution time"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        # Variable execution time
        if self.tick_count % 3 == 0:
            time.sleep(0.030)  # 30ms - exceeds deadline
        elif self.tick_count % 2 == 0:
            time.sleep(0.015)  # 15ms - exceeds deadline
        else:
            time.sleep(0.008)  # 8ms - within deadline

# Create scheduler with deadline monitoring enabled
config = horus.SchedulerConfig.standard()
config.tick_rate = 20.0  # 20 Hz
config.deadline_monitoring = True  # Enable deadline warnings

scheduler = horus.Scheduler.from_config(config)

# Create nodes
fast = FastNode()
slow = SlowNode()
variable = VariableNode()

# Add nodes to scheduler
scheduler.add(fast, priority=1)
scheduler.add(slow, priority=2)
scheduler.add(variable, priority=3)

print(f"\nConfiguration:")
print(f"  Tick Rate: {config.tick_rate} Hz")
print(f"  Deadline Monitoring: {config.deadline_monitoring}")
print()

# Get node names after adding
names = scheduler.get_node_names()
print(f"Registered nodes: {names}")
print()

# Set deadlines for each node
print("Setting per-node deadlines...")
for name in names:
    scheduler.set_node_deadline(name, 10.0)  # 10ms deadline
    print(f"  {name}: 10ms deadline")

print()
print("=" * 70)
print("Running scheduler for 5 seconds...")
print("Watch for deadline violation warnings below:")
print("=" * 70)
print()

# Run scheduler for a short duration
def stop_after(seconds):
    time.sleep(seconds)
    scheduler.stop()

stop_thread = threading.Thread(target=lambda: stop_after(5), daemon=True)
stop_thread.start()

try:
    scheduler.run()
except Exception as e:
    print(f"Scheduler error: {e}")

# Analyze results
print()
print("=" * 70)
print("Results Analysis:")
print("=" * 70)

nodes = scheduler.get_all_nodes()
for node_info in nodes:
    print(f"\n{node_info['name']}:")
    print(f"  Total ticks: {node_info.get('total_ticks', 0)}")
    print(f"  Deadline: {node_info.get('deadline_ms', 'N/A')}ms")
    print(f"  Deadline misses: {node_info.get('deadline_misses', 0)}")
    print(f"  Last tick duration: {node_info.get('last_tick_duration_ms', 0):.3f}ms")
    print(f"  Avg tick duration: {node_info.get('avg_tick_duration_ms', 0):.3f}ms")
    print(f"  Min tick duration: {node_info.get('min_tick_duration_ms', 0):.3f}ms")
    print(f"  Max tick duration: {node_info.get('max_tick_duration_ms', 0):.3f}ms")

    # Calculate deadline miss rate
    total_ticks = node_info.get('total_ticks', 0)
    deadline_misses = node_info.get('deadline_misses', 0)
    if total_ticks > 0:
        miss_rate = (deadline_misses / total_ticks) * 100
        print(f"  Miss rate: {miss_rate:.1f}%")

        if miss_rate == 0:
            print(f"  Status: ✓ All deadlines met")
        elif miss_rate < 10:
            print(f"  Status: ⚠ Occasional deadline misses")
        else:
            print(f"  Status: ✗ Frequent deadline misses")

# Summary
print()
print("=" * 70)
print("Summary:")
print("=" * 70)

total_ticks = sum(n.get('total_ticks', 0) for n in nodes)
total_misses = sum(n.get('deadline_misses', 0) for n in nodes)
nodes_with_misses = sum(1 for n in nodes if n.get('deadline_misses', 0) > 0)

print(f"\nTotal ticks: {total_ticks}")
print(f"Total deadline misses: {total_misses}")
print(f"Nodes with deadline violations: {nodes_with_misses}/{len(nodes)}")

if total_ticks > 0:
    overall_miss_rate = (total_misses / total_ticks) * 100
    print(f"Overall miss rate: {overall_miss_rate:.1f}%")

print()
print("=" * 70)
print("Demo complete!")
print("=" * 70)
print("\nKey Features Demonstrated:")
print("  ✓ Deadline monitoring enabled in SchedulerConfig")
print("  ✓ Per-node deadline configuration with set_node_deadline()")
print("  ✓ Automatic deadline violation detection and warnings")
print("  ✓ Deadline miss tracking and statistics")
print("  ✓ Performance metrics (min/max/avg tick duration)")
