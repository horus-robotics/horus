#!/bin/bash
# Test IPC and robotics applications with horus run

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# HORUS binary
HORUS="/home/lord-patpak/horus/HORUS/target/debug/horus"

# Test directory
TEST_DIR=$(mktemp -d /tmp/horus_test_ipc_XXXXXX)
trap "rm -rf $TEST_DIR" EXIT

cd "$TEST_DIR"

echo "=== Testing IPC and Robotics Applications ==="
echo ""

# Helper functions
pass() {
    echo -e "${GREEN} PASS${NC}: $1"
    ((TESTS_PASSED++))
}

fail() {
    echo -e "${RED} FAIL${NC}: $1"
    echo "   Error: $2"
    ((TESTS_FAILED++))
}

# Test 1: Simple node with Node trait
echo "Test 1: Rust Node trait implementation..."
cat > simple_node.rs << 'EOF'
struct SimpleNode {
    counter: u32,
}

trait Node {
    fn init() -> Self;
    fn tick(&mut self);
}

impl Node for SimpleNode {
    fn init() -> Self {
        Self { counter: 0 }
    }

    fn tick(&mut self) {
        self.counter += 1;
        println!("Tick #{}", self.counter);
    }
}

fn main() {
    let mut node = SimpleNode::init();
    for _ in 0..3 {
        node.tick();
    }
}
EOF

OUTPUT=$($HORUS run simple_node.rs 2>&1)
if echo "$OUTPUT" | grep -q "Tick #1" && echo "$OUTPUT" | grep -q "Tick #3"; then
    pass "Node trait implementation works"
else
    fail "Node trait" "Node implementation failed"
fi

# Test 2: Publisher node pattern
echo "Test 2: Publisher node pattern..."
cat > pub_pattern.rs << 'EOF'
fn main() {
    println!("Publisher starting...");

    for i in 1..=5 {
        println!("Publishing message {}", i);
        // Simulated publish
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    println!("Publisher completed");
}
EOF

OUTPUT=$($HORUS run pub_pattern.rs 2>&1)
if echo "$OUTPUT" | grep -q "Publishing message 1" && echo "$OUTPUT" | grep -q "Publisher completed"; then
    pass "Publisher pattern works"
else
    fail "Publisher pattern" "Publisher failed"
fi

# Test 3: Sensor simulation
echo "Test 3: Sensor node simulation..."
cp /home/lord-patpak/horus/HORUS/tests/horus_run/fixtures/sensor_node.py .

OUTPUT=$($HORUS run sensor_node.py 2>&1)
if echo "$OUTPUT" | grep -q "Sensor Node Starting" && echo "$OUTPUT" | grep -q "Temperature"; then
    pass "Sensor simulation works"
else
    fail "Sensor simulation" "Sensor node failed"
fi

# Test 4: Control loop pattern
echo "Test 4: Control loop pattern..."
cat > control_loop.rs << 'EOF'
fn main() {
    println!("Control loop starting");

    let mut position: f64 = 0.0;
    let target: f64 = 10.0;

    for iteration in 1..=5 {
        let error = target - position;
        let control = error * 0.3;  // Simple P controller

        position += control;

        println!("Iteration {}: pos={:.2}, error={:.2}", iteration, position, error);

        if error.abs() < 0.01 {
            break;
        }
    }

    println!("Control loop completed");
}
EOF

OUTPUT=$($HORUS run control_loop.rs 2>&1)
if echo "$OUTPUT" | grep -q "Control loop starting" && echo "$OUTPUT" | grep -q "Iteration"; then
    pass "Control loop pattern works"
else
    fail "Control loop" "Control loop failed"
fi

# Test 5: Multi-threaded simulation
echo "Test 5: Multi-threaded node simulation..."
cat > multithread.rs << 'EOF'
use std::thread;
use std::time::Duration;

fn main() {
    println!("Starting multi-threaded simulation");

    let handle1 = thread::spawn(|| {
        for i in 1..=3 {
            println!("Thread 1: tick {}", i);
            thread::sleep(Duration::from_millis(10));
        }
    });

    let handle2 = thread::spawn(|| {
        for i in 1..=3 {
            println!("Thread 2: tick {}", i);
            thread::sleep(Duration::from_millis(10));
        }
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("Multi-threaded simulation completed");
}
EOF

OUTPUT=$($HORUS run multithread.rs 2>&1)
if echo "$OUTPUT" | grep -q "Thread 1" && echo "$OUTPUT" | grep -q "Thread 2"; then
    pass "Multi-threaded simulation works"
else
    fail "Multi-threaded" "Threading failed"
fi

# Test 6: State machine pattern
echo "Test 6: State machine pattern..."
cat > state_machine.py << 'EOF'
#!/usr/bin/env python3

class RobotState:
    IDLE = "idle"
    MOVING = "moving"
    STOPPED = "stopped"

def main():
    state = RobotState.IDLE
    print(f"Initial state: {state}")

    transitions = [RobotState.MOVING, RobotState.STOPPED, RobotState.IDLE]

    for new_state in transitions:
        state = new_state
        print(f"Transitioned to: {state}")

    print("State machine completed")
    return 0

if __name__ == "__main__":
    exit(main())
EOF

OUTPUT=$($HORUS run state_machine.py 2>&1)
if echo "$OUTPUT" | grep -q "Initial state" && echo "$OUTPUT" | grep -q "Transitioned to"; then
    pass "State machine pattern works"
else
    fail "State machine" "State transitions failed"
fi

# Test 7: Data processing pipeline
echo "Test 7: Data processing pipeline..."
cat > pipeline.rs << 'EOF'
fn filter_data(data: Vec<f64>) -> Vec<f64> {
    data.into_iter().filter(|x| *x > 0.0).collect()
}

fn transform_data(data: Vec<f64>) -> Vec<f64> {
    data.into_iter().map(|x| x * 2.0).collect()
}

fn main() {
    println!("Data pipeline starting");

    let raw_data = vec![-1.0, 2.0, -3.0, 4.0, 5.0];
    let filtered = filter_data(raw_data);
    let transformed = transform_data(filtered);

    println!("Processed data: {:?}", transformed);
    println!("Pipeline completed");
}
EOF

OUTPUT=$($HORUS run pipeline.rs 2>&1)
if echo "$OUTPUT" | grep -q "Data pipeline" && echo "$OUTPUT" | grep -q "Pipeline completed"; then
    pass "Data processing pipeline works"
else
    fail "Pipeline" "Pipeline processing failed"
fi

# Test 8: Shared memory access pattern
echo "Test 8: Shared memory pattern (Arc/Mutex)..."
cat > shared_mem.rs << 'EOF'
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    println!("Shared memory test starting");

    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for i in 0..3 {
        let counter_clone = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut num = counter_clone.lock().unwrap();
            *num += 1;
            println!("Thread {} incremented counter", i);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Final counter: {}", *counter.lock().unwrap());
    println!("Shared memory test completed");
}
EOF

OUTPUT=$($HORUS run shared_mem.rs 2>&1)
if echo "$OUTPUT" | grep -q "Shared memory test" && echo "$OUTPUT" | grep -q "Final counter: 3"; then
    pass "Shared memory pattern works"
else
    fail "Shared memory" "Shared memory test failed"
fi

# Test 9: Timing and scheduling
echo "Test 9: Timing and scheduling pattern..."
cat > timing.rs << 'EOF'
use std::time::{Duration, Instant};

fn main() {
    println!("Timing test starting");

    let start = Instant::now();

    // Simulate work
    std::thread::sleep(Duration::from_millis(50));

    let elapsed = start.elapsed();

    println!("Elapsed time: {:?}", elapsed);
    println!("Timing test completed");
}
EOF

OUTPUT=$($HORUS run timing.rs 2>&1)
if echo "$OUTPUT" | grep -q "Timing test" && echo "$OUTPUT" | grep -q "Elapsed time"; then
    pass "Timing and scheduling works"
else
    fail "Timing" "Timing test failed"
fi

# Test 10: Error handling in robotics context
echo "Test 10: Error handling in nodes..."
cat > error_handling.rs << 'EOF'
fn sensor_read() -> Result<f64, String> {
    // Simulate successful read
    Ok(42.5)
}

fn main() {
    println!("Error handling test");

    match sensor_read() {
        Ok(value) => println!("Sensor value: {}", value),
        Err(e) => println!("Sensor error: {}", e),
    }

    println!("Error handling completed");
}
EOF

OUTPUT=$($HORUS run error_handling.rs 2>&1)
if echo "$OUTPUT" | grep -q "Sensor value: 42.5" && echo "$OUTPUT" | grep -q "Error handling completed"; then
    pass "Error handling works"
else
    fail "Error handling" "Error handling failed"
fi

# Summary
echo ""
echo "================================"
echo "IPC and Robotics Tests Summary"
echo "================================"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All IPC and robotics tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
