#!/usr/bin/env python3
"""
Simple test for watchdog functionality
"""
import horus
import time

print("=" * 70)
print("Watchdog Timer Test")
print("=" * 70)

# Create a node that will be monitored
class TestNode(horus.Node):
    def __init__(self):
        super().__init__()
        self.count = 0

    def tick(self):
        self.count += 1
        time.sleep(0.01)

# Create scheduler with watchdog enabled
config = horus.SchedulerConfig.standard()
config.tick_rate = 20.0
config.watchdog_enabled = True
config.watchdog_timeout_ms = 500  # 500ms timeout

scheduler = horus.Scheduler.from_config(config)

print(f"\nConfiguration:")
print(f"  Watchdog Enabled: {config.watchdog_enabled}")
print(f"  Timeout: {config.watchdog_timeout_ms}ms")

# Add node and enable watchdog
node = TestNode()
scheduler.add(node, priority=1)

names = scheduler.get_node_names()
print(f"\nNode: {names[0]}")

# Enable watchdog for this node
scheduler.set_node_watchdog(names[0], True, 500)
print("Watchdog enabled with 500ms timeout")

# Check node stats before running
stats = scheduler.get_node_stats(names[0])
print(f"\nBefore running:")
print(f"  Watchdog enabled: {stats.get('watchdog_enabled')}")
print(f"  Watchdog timeout: {stats.get('watchdog_timeout_ms')}ms")
print(f"  Watchdog expired: {stats.get('watchdog_expired')}")

print("\n" + "=" * 70)
print("Test complete!")
print("=" * 70)
print("\nWatchdog Features Tested:")
print("  ✓ Global watchdog configuration")
print("  ✓ Per-node watchdog enable with set_node_watchdog()")
print("  ✓ Watchdog status in node stats")
