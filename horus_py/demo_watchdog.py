#!/usr/bin/env python3
"""
Demo: Watchdog Timer Safety Monitor
Shows how to use watchdog timers to detect unresponsive nodes
"""
import horus
import time
import threading

print("=" * 70)
print("HORUS Watchdog Timer Demo")
print("=" * 70)

# Create nodes with different behaviors
class HealthyNode(horus.Node):
    """Always responds normally"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        time.sleep(0.01)  # Normal work

class FlakyNode(horus.Node):
    """Occasionally hangs (simulates stuck I/O)"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        # Simulate occasional hang
        if self.tick_count in [15, 30]:
            print(f"FlakyNode hanging on tick {self.tick_count}...")
            time.sleep(2.0)  # Hang for 2 seconds
        else:
            time.sleep(0.01)

class CriticalNode(horus.Node):
    """Mission-critical node that must never hang"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        time.sleep(0.005)

# Create scheduler with watchdog enabled
config = horus.SchedulerConfig.standard()
config.tick_rate = 20.0  # 20 Hz
config.watchdog_enabled = True  # Enable global watchdog
config.watchdog_timeout_ms = 1000  # Default 1000ms timeout

scheduler = horus.Scheduler.from_config(config)

print(f"\nConfiguration:")
print(f"  Tick Rate: {config.tick_rate} Hz")
print(f"  Watchdog Enabled: {config.watchdog_enabled}")
print(f"  Default Timeout: {config.watchdog_timeout_ms}ms")
print()

# Create nodes
healthy = HealthyNode()
flaky = FlakyNode()
critical = CriticalNode()

# Add nodes to scheduler
scheduler.add(healthy, priority=1)
scheduler.add(flaky, priority=2)
scheduler.add(critical, priority=3)

# Get node names
names = scheduler.get_node_names()
print(f"Registered nodes: {names}")
print()

# Configure watchdogs for specific nodes
print("Configuring watchdogs...")
print(f"  {names[0]} (healthy): Watchdog disabled")
scheduler.set_node_watchdog(names[0], False)

print(f"  {names[1]} (flaky): Watchdog enabled, 500ms timeout")
scheduler.set_node_watchdog(names[1], True, 500)

print(f"  {names[2]} (critical): Watchdog enabled, 100ms timeout")
scheduler.set_node_watchdog(names[2], True, 100)

print()
print("=" * 70)
print("Running scheduler for 3 seconds...")
print("Watch for watchdog expiration warnings:")
print("=" * 70)
print()

# Run scheduler
def stop_after(seconds):
    time.sleep(seconds)
    scheduler.stop()

stop_thread = threading.Thread(target=lambda: stop_after(3), daemon=True)
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
    print(f"  Watchdog enabled: {node_info.get('watchdog_enabled', False)}")
    print(f"  Watchdog timeout: {node_info.get('watchdog_timeout_ms', 'N/A')}ms")
    print(f"  Watchdog expired: {node_info.get('watchdog_expired', False)}")

    if node_info.get('watchdog_enabled'):
        time_since_feed = node_info.get('watchdog_time_since_feed_ms', 0)
        print(f"  Time since last feed: {time_since_feed}ms")

        expired = node_info.get('watchdog_expired', False)
        if expired:
            print(f"  Status: ✗ WATCHDOG EXPIRED")
        else:
            print(f"  Status: ✓ Watchdog OK")

# Summary
print()
print("=" * 70)
print("Summary:")
print("=" * 70)

total_expired = sum(1 for n in nodes if n.get('watchdog_expired', False))
total_enabled = sum(1 for n in nodes if n.get('watchdog_enabled', False))

print(f"\nWatchdogs enabled: {total_enabled}/{len(nodes)}")
print(f"Watchdogs expired: {total_expired}/{total_enabled}")

print()
print("=" * 70)
print("Demo complete!")
print("=" * 70)
print("\nKey Features Demonstrated:")
print("  ✓ Global watchdog configuration in SchedulerConfig")
print("  ✓ Per-node watchdog enable/disable")
print("  ✓ Configurable per-node timeouts")
print("  ✓ Automatic watchdog feeding on successful ticks")
print("  ✓ Watchdog expiration detection and warnings")
print("  ✓ Watchdog status tracking via introspection")
