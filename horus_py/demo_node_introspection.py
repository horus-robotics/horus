#!/usr/bin/env python3
"""
Demo: Node Introspection Features
Shows how to query information about nodes in the scheduler
"""
import horus

print("=" * 70)
print("HORUS Node Introspection Demo")
print("=" * 70)

# Create some demo nodes
class SensorNode(horus.Node):
    def tick(self):
        pass

class ControlNode(horus.Node):
    def tick(self):
        pass

class ActuatorNode(horus.Node):
    def tick(self):
        pass

# Create scheduler and add nodes
config = horus.SchedulerConfig.standard()
scheduler = horus.Scheduler.from_config(config)

sensor = SensorNode()
control = ControlNode()
actuator = ActuatorNode()

scheduler.add(sensor, priority=1)
scheduler.add(control, priority=2)
scheduler.add(actuator, priority=3)

print(f"\nScheduler Configuration:")
print(f"  Tick Rate: {config.tick_rate} Hz")
print(f"  Circuit Breaker: {config.circuit_breaker}")
print(f"  Max Failures: {config.max_failures}")
print(f"  Auto-Restart: {config.auto_restart}")

# Test 1: Get node count
print("\n" + "=" * 70)
print("1. Node Count")
print("=" * 70)
count = scheduler.get_node_count()
print(f"Total nodes: {count}")

# Test 2: Get node names
print("\n" + "=" * 70)
print("2. Node Names")
print("=" * 70)
names = scheduler.get_node_names()
for i, name in enumerate(names, 1):
    print(f"  {i}. {name}")

# Test 3: Check if nodes exist
print("\n" + "=" * 70)
print("3. Node Existence Check")
print("=" * 70)
test_names = names + ["nonexistent_node"]
for name in test_names:
    exists = scheduler.has_node(name)
    status = "✓ EXISTS" if exists else "✗ NOT FOUND"
    print(f"  {name}: {status}")

# Test 4: Get all nodes info
print("\n" + "=" * 70)
print("4. All Nodes Information")
print("=" * 70)
nodes = scheduler.get_all_nodes()
for node_info in nodes:
    print(f"\n  {node_info['name']}:")
    print(f"    Priority: {node_info['priority']}")
    print(f"    Rate: {node_info['rate_hz']} Hz")
    print(f"    Logging: {node_info['logging_enabled']}")
    print(f"    State: {node_info.get('state', 'unknown')}")
    print(f"    Total ticks: {node_info.get('total_ticks', 0)}")
    print(f"    Failure count: {node_info.get('failure_count', 0)}")
    print(f"    Circuit open: {node_info.get('circuit_open', False)}")

# Test 5: Get specific node stats
print("\n" + "=" * 70)
print("5. Specific Node Statistics")
print("=" * 70)
if names:
    first_node = names[0]
    stats = scheduler.get_node_stats(first_node)
    print(f"\nDetailed stats for '{first_node}':")
    for key in sorted(stats.keys()):
        value = stats[key]
        if isinstance(value, float):
            print(f"  {key:25s}: {value:.3f}")
        elif isinstance(value, bool):
            print(f"  {key:25s}: {value}")
        else:
            print(f"  {key:25s}: {value}")

# Test 6: Summary statistics
print("\n" + "=" * 70)
print("6. Summary Statistics")
print("=" * 70)
total_ticks = sum(n.get('total_ticks', 0) for n in nodes)
avg_rate = sum(n['rate_hz'] for n in nodes) / len(nodes) if nodes else 0
priorities = [n['priority'] for n in nodes]

print(f"\n  Total nodes: {len(nodes)}")
print(f"  Total ticks: {total_ticks}")
print(f"  Average rate: {avg_rate:.1f} Hz")
print(f"  Priority range: {min(priorities)} - {max(priorities)}")
print(f"  Nodes with logging: {sum(1 for n in nodes if n['logging_enabled'])}")

# Test 7: Query by priority
print("\n" + "=" * 70)
print("7. Nodes by Priority")
print("=" * 70)
by_priority = {}
for node_info in nodes:
    priority = node_info['priority']
    if priority not in by_priority:
        by_priority[priority] = []
    by_priority[priority].append(node_info['name'])

for priority in sorted(by_priority.keys()):
    print(f"  Priority {priority}: {', '.join(by_priority[priority])}")

print("\n" + "=" * 70)
print("Demo complete!")
print("=" * 70)
print("\nIntrospection Features Demonstrated:")
print("  ✓ get_node_count() - Get total number of nodes")
print("  ✓ get_node_names() - Get list of all node names")
print("  ✓ has_node(name) - Check if a specific node exists")
print("  ✓ get_all_nodes() - Get detailed info for all nodes")
print("  ✓ get_node_stats(name) - Get detailed stats for a specific node")
