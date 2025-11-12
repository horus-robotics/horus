#!/usr/bin/env python3
"""
Demo: Fault Tolerance Features
- Circuit Breaker: Opens after max consecutive failures
- Auto-Restart: Attempts to restart failed nodes after 5 seconds

Usage: python3 demo_fault_tolerance.py
Press Ctrl+C to stop
"""
import horus

print("=" * 70)
print("HORUS Fault Tolerance Demo")
print("=" * 70)
print("\nThis demo shows:")
print("  1. Circuit Breaker - Opens after 3 consecutive failures")
print("  2. Auto-Restart - Attempts restart every 5 seconds")
print("\nPress Ctrl+C to stop\n")
print("=" * 70)

class DemoNode(horus.Node):
    """Demo node that shows fault tolerance in action"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        print(f"[{self.tick_count:3d}] Node executing...")

        # Fail every 10 ticks to demonstrate circuit breaker
        if self.tick_count % 10 in [1, 2, 3]:
            raise RuntimeError(f"Demo failure at tick {self.tick_count}")

# Create scheduler with fault tolerance enabled
config = horus.SchedulerConfig.standard()
config.tick_rate = 1.0  # 1 Hz for easy observation
config.circuit_breaker = True
config.max_failures = 3
config.auto_restart = True

print(f"Configuration:")
print(f"  Tick Rate: {config.tick_rate} Hz")
print(f"  Circuit Breaker: {config.circuit_breaker}")
print(f"  Max Failures: {config.max_failures}")
print(f"  Auto-Restart: {config.auto_restart}")
print("\nStarting scheduler...\n")

scheduler = horus.Scheduler.from_config(config)
node = DemoNode()
scheduler.add(node, priority=1)

try:
    scheduler.run()
except KeyboardInterrupt:
    print("\n\nStopping...")
    scheduler.stop()

print(f"\nTotal ticks executed: {node.tick_count}")
print("Demo complete!")
