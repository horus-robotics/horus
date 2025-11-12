#!/usr/bin/env python3
"""
Simple test to demonstrate circuit breaker functionality
"""
import horus
import time
import threading

print("=" * 70)
print("Fault Tolerance Test: Circuit Breaker & Auto-Restart")
print("=" * 70)

class FailingNode(horus.Node):
    """A node that always fails"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        print(f"  Tick #{self.tick_count}")
        raise RuntimeError(f"Intentional failure #{self.tick_count}")

# Test 1: Circuit breaker opens after max failures
print("\n1. Testing Circuit Breaker (without auto-restart)")
print("-" * 70)

config = horus.SchedulerConfig.standard()
config.tick_rate = 5.0  # 5 Hz
config.circuit_breaker = True
config.max_failures = 3
config.auto_restart = False

scheduler = horus.Scheduler.from_config(config)
failing_node = FailingNode()
scheduler.add(failing_node, priority=1)

print(f"Config: tick_rate={config.tick_rate}Hz, max_failures={config.max_failures}, auto_restart=False")
print("Starting scheduler...")
print("Expected: Node fails 3 times, then circuit breaker opens and stops execution\n")

def stop_after(seconds):
    time.sleep(seconds)
    scheduler.stop()

stop_thread = threading.Thread(target=lambda: stop_after(3), daemon=True)
stop_thread.start()

try:
    scheduler.run()
except Exception as e:
    pass

print(f"\nResult: Node executed {failing_node.tick_count} ticks")
print(f"Status: {'✓ PASS - Circuit breaker opened' if failing_node.tick_count <= 4 else '✗ FAIL'}")

# Test 2: Auto-restart
print("\n" + "=" * 70)
print("2. Testing Auto-Restart")
print("-" * 70)

class RecoverableNode(horus.Node):
    """A node that fails initially but eventually succeeds"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0
        self.start_time = time.time()

    def tick(self):
        self.tick_count += 1
        elapsed = time.time() - self.start_time

        # Fail for first 8 seconds, then succeed
        if elapsed < 8:
            print(f"  Tick #{self.tick_count} at {elapsed:.1f}s - FAILING")
            raise RuntimeError(f"Temporary failure #{self.tick_count}")
        else:
            print(f"  Tick #{self.tick_count} at {elapsed:.1f}s - SUCCESS")

config2 = horus.SchedulerConfig.standard()
config2.tick_rate = 2.0  # 2 Hz
config2.circuit_breaker = True
config2.max_failures = 3
config2.auto_restart = True

scheduler2 = horus.Scheduler.from_config(config2)
recoverable_node = RecoverableNode()
scheduler2.add(recoverable_node, priority=1)

print(f"Config: tick_rate={config2.tick_rate}Hz, max_failures={config2.max_failures}, auto_restart=True")
print("Starting scheduler...")
print("Expected:")
print("  - Node fails 3 times, circuit breaker opens (~1.5s)")
print("  - After 5s, auto-restart attempts (~6.5s total)")
print("  - Node fails again 3 times, circuit breaker opens (~8s)")
print("  - After 5s, auto-restart attempts (~13s)")
print("  - Node now succeeds and continues running\n")

stop_thread2 = threading.Thread(target=lambda: stop_after(15), daemon=True)
stop_thread2.start()

try:
    scheduler2.run()
except Exception as e:
    pass

print(f"\nResult: Node executed {recoverable_node.tick_count} ticks")
print(f"Status: {'✓ PASS - Auto-restart worked' if recoverable_node.tick_count >= 10 else '✗ FAIL'}")

# Test 3: Successful node
print("\n" + "=" * 70)
print("3. Testing Normal Operation (no failures)")
print("-" * 70)

class SuccessfulNode(horus.Node):
    """A node that always succeeds"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self):
        self.tick_count += 1
        if self.tick_count == 1 or self.tick_count % 20 == 0:
            print(f"  Tick #{self.tick_count}")

config3 = horus.SchedulerConfig.standard()
config3.tick_rate = 30.0  # 30 Hz
config3.circuit_breaker = True
config3.max_failures = 3

scheduler3 = horus.Scheduler.from_config(config3)
successful_node = SuccessfulNode()
scheduler3.add(successful_node, priority=1)

print(f"Config: tick_rate={config3.tick_rate}Hz")
print("Starting scheduler...")
print("Expected: Node runs successfully without circuit breaker activation\n")

stop_thread3 = threading.Thread(target=lambda: stop_after(2), daemon=True)
stop_thread3.start()

try:
    scheduler3.run()
except Exception as e:
    pass

print(f"\nResult: Node executed {successful_node.tick_count} ticks")
expected_ticks = int(config3.tick_rate * 2)
tolerance = int(expected_ticks * 0.3)  # 30% tolerance
print(f"Status: {'✓ PASS - Normal operation' if abs(successful_node.tick_count - expected_ticks) <= tolerance else '✗ FAIL'}")

print("\n" + "=" * 70)
print("All tests complete!")
print("=" * 70)
