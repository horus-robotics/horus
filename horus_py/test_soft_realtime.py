#!/usr/bin/env python3
"""
Test script for soft real-time scheduling features
"""
import horus
import time
import threading

print("=" * 70)
print("Soft Real-Time Scheduling Test")
print("=" * 70)

# Create test nodes
class FastNode(horus.Node):
    """Fast node - always meets deadline"""
    def tick(self):
        time.sleep(0.002)  # 2ms

class SlowNode(horus.Node):
    """Slow node - occasionally misses deadline"""
    def __init__(self):
        super().__init__()
        self.count = 0

    def tick(self):
        self.count += 1
        if self.count % 5 == 0:
            time.sleep(0.025)  # 25ms - miss
        else:
            time.sleep(0.005)  # 5ms - ok

# Create scheduler with deadline monitoring enabled
config = horus.SchedulerConfig.standard()
config.tick_rate = 20.0  # 20 Hz
config.deadline_monitoring = True

scheduler = horus.Scheduler.from_config(config)

fast = FastNode()
slow = SlowNode()

scheduler.add(fast, priority=1)
scheduler.add(slow, priority=2)

print(f"\nConfiguration:")
print(f"  Tick Rate: {config.tick_rate} Hz")
print(f"  Deadline Monitoring: {config.deadline_monitoring}")

# Get node names and set deadlines
names = scheduler.get_node_names()
print(f"\nNodes: {names}")

for name in names:
    scheduler.set_node_deadline(name, 10.0)  # 10ms deadline
    print(f"Set {name} deadline to 10ms")

print("\n" + "=" * 70)
print("Running for 2 seconds...")
print("=" * 70 + "\n")

# Run for 2 seconds
def stop_after(sec):
    time.sleep(sec)
    scheduler.stop()

threading.Thread(target=lambda: stop_after(2), daemon=True).start()
scheduler.run()

# Results
print("\n" + "=" * 70)
print("Results:")
print("=" * 70)

nodes = scheduler.get_all_nodes()
for node in nodes:
    total = node.get('total_ticks', 0)
    misses = node.get('deadline_misses', 0)
    print(f"\n{node['name']}:")
    print(f"  Total ticks: {total}")
    print(f"  Deadline misses: {misses}")
    print(f"  Miss rate: {(misses/total*100) if total > 0 else 0:.1f}%")
    print(f"  Avg duration: {node.get('avg_tick_duration_ms', 0):.3f}ms")

print("\n" + "=" * 70)
print("Test complete!")
print("=" * 70)
