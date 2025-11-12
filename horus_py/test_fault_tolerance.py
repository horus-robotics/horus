#!/usr/bin/env python3
"""
Test fault tolerance features: circuit breaker and auto-restart
"""
import horus
import time

# Test 1: Circuit breaker opens after max failures
print("=" * 60)
print("Test 1: Circuit Breaker")
print("=" * 60)

class FailingNode(horus.Node):
    """A node that always fails"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self, info):
        self.tick_count += 1
        print(f"FailingNode tick #{self.tick_count}")
        raise RuntimeError(f"Intentional failure #{self.tick_count}")

# Create config with circuit breaker enabled and low max failures
config = horus.SchedulerConfig.standard()
config.tick_rate = 10.0  # 10 Hz for faster testing
config.circuit_breaker = True
config.max_failures = 3
config.auto_restart = False  # Disable auto-restart for this test

scheduler = horus.Scheduler.from_config(config)
failing_node = FailingNode()
scheduler.add(failing_node, priority=1)

print(f"Config: tick_rate={config.tick_rate}Hz, max_failures={config.max_failures}")
print("Starting scheduler (will run for 5 seconds)...")
print("Expected: Node should fail 3 times, then circuit breaker opens\n")

# Run for 5 seconds
import threading
def stop_after_delay():
    time.sleep(5)
    scheduler.stop()

stop_thread = threading.Thread(target=stop_after_delay, daemon=True)
stop_thread.start()

try:
    scheduler.run()
except Exception as e:
    print(f"Scheduler stopped: {e}")

print(f"\nTest 1 Results:")
print(f"  Total ticks executed: {failing_node.tick_count}")
print(f"  Expected: ~3 ticks (circuit breaker should open after 3 failures)")
print(f"  Status: {'✓ PASS' if failing_node.tick_count <= 4 else '✗ FAIL'}")

# Test 2: Auto-restart
print("\n" + "=" * 60)
print("Test 2: Auto-Restart")
print("=" * 60)

class SometimesFailingNode(horus.Node):
    """A node that fails initially but eventually succeeds"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0
        self.fail_until = 8  # Fail for first 8 ticks, then succeed

    def tick(self, info):
        self.tick_count += 1
        if self.tick_count <= self.fail_until:
            print(f"SometimesFailingNode tick #{self.tick_count} - FAILING")
            raise RuntimeError(f"Failure #{self.tick_count}")
        else:
            print(f"SometimesFailingNode tick #{self.tick_count} - SUCCESS")

# Create config with auto-restart enabled
config2 = horus.SchedulerConfig.standard()
config2.tick_rate = 2.0  # 2 Hz for testing
config2.circuit_breaker = True
config2.max_failures = 3
config2.auto_restart = True

scheduler2 = horus.Scheduler.from_config(config2)
sometimes_failing_node = SometimesFailingNode()
scheduler2.add(sometimes_failing_node, priority=1)

print(f"Config: tick_rate={config2.tick_rate}Hz, max_failures={config2.max_failures}, auto_restart=True")
print("Starting scheduler (will run for 20 seconds)...")
print("Expected:")
print("  - Node fails 3 times")
print("  - Circuit breaker opens")
print("  - After 5s, auto-restart attempts")
print("  - Node fails again 3 times")
print("  - Circuit breaker opens again")
print("  - After 5s, auto-restart attempts")
print("  - Eventually node succeeds and runs normally\n")

def stop_after_delay2():
    time.sleep(20)
    scheduler2.stop()

stop_thread2 = threading.Thread(target=stop_after_delay2, daemon=True)
stop_thread2.start()

try:
    scheduler2.run()
except Exception as e:
    print(f"Scheduler stopped: {e}")

print(f"\nTest 2 Results:")
print(f"  Total ticks executed: {sometimes_failing_node.tick_count}")
print(f"  Expected: At least 9 ticks (8 failures + at least 1 success)")
print(f"  Status: {'✓ PASS' if sometimes_failing_node.tick_count >= 9 else '✗ FAIL'}")

# Test 3: Successful node doesn't trigger circuit breaker
print("\n" + "=" * 60)
print("Test 3: Successful Node (No Circuit Breaker)")
print("=" * 60)

class SuccessfulNode(horus.Node):
    """A node that always succeeds"""
    def __init__(self):
        super().__init__()
        self.tick_count = 0

    def tick(self, info):
        self.tick_count += 1
        if self.tick_count % 10 == 0:
            print(f"SuccessfulNode tick #{self.tick_count}")

config3 = horus.SchedulerConfig.standard()
config3.tick_rate = 50.0  # 50 Hz
config3.circuit_breaker = True
config3.max_failures = 3

scheduler3 = horus.Scheduler.from_config(config3)
successful_node = SuccessfulNode()
scheduler3.add(successful_node, priority=1)

print(f"Config: tick_rate={config3.tick_rate}Hz")
print("Starting scheduler (will run for 2 seconds)...")
print("Expected: Node runs successfully without circuit breaker activation\n")

def stop_after_delay3():
    time.sleep(2)
    scheduler3.stop()

stop_thread3 = threading.Thread(target=stop_after_delay3, daemon=True)
stop_thread3.start()

try:
    scheduler3.run()
except Exception as e:
    print(f"Scheduler stopped: {e}")

print(f"\nTest 3 Results:")
print(f"  Total ticks executed: {successful_node.tick_count}")
print(f"  Expected: ~100 ticks (2 seconds * 50 Hz)")
print(f"  Status: {'✓ PASS' if 80 <= successful_node.tick_count <= 120 else '✗ FAIL'}")

print("\n" + "=" * 60)
print("All tests complete!")
print("=" * 60)
