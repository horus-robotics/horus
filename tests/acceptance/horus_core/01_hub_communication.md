# User Acceptance Test: Hub Communication (Pub/Sub)

## Feature
Lock-free, zero-copy shared memory pub/sub communication between nodes.

## User Story
As a robotics developer, I want fast and reliable message passing between nodes so that my robot can respond to sensor data in real-time with minimal latency.

## Test Scenarios

### Scenario 1: Basic Publish and Subscribe
**Given:** Two nodes share a topic name
**When:** Publisher sends a message
**Then:**
- [ ] Subscriber receives the message
- [ ] Data integrity is maintained
- [ ] Latency is sub-microsecond
- [ ] No data corruption

**Acceptance Criteria:**
```rust
// Publisher
let hub = Hub::<i32>::new("test_topic")?;
hub.send(42, None)?;

// Subscriber (same topic)
let hub = Hub::<i32>::new("test_topic")?;
let msg = hub.recv(None);
assert_eq!(msg, Some(42));
```

### Scenario 2: Multiple Subscribers
**Given:** One publisher, three subscribers on same topic
**When:** Publisher sends message
**Then:**
- [ ] All three subscribers receive the message
- [ ] Each subscriber gets independent copy
- [ ] No message interference between subscribers

**Acceptance Criteria:**
```rust
let pub_hub = Hub::<i32>::new("multi")?;
let sub1 = Hub::<i32>::new("multi")?;
let sub2 = Hub::<i32>::new("multi")?;
let sub3 = Hub::<i32>::new("multi")?;

pub_hub.send(100, None)?;

assert_eq!(sub1.recv(None), Some(100));
assert_eq!(sub2.recv(None), Some(100));
assert_eq!(sub3.recv(None), Some(100));
```

### Scenario 3: Multiple Publishers
**Given:** Three publishers, one subscriber on same topic
**When:** Each publisher sends a message
**Then:**
- [ ] Subscriber receives all messages (eventually)
- [ ] Order may vary but all messages arrive
- [ ] No messages are lost

**Acceptance Criteria:**
```rust
let pub1 = Hub::<i32>::new("topic")?;
let pub2 = Hub::<i32>::new("topic")?;
let pub3 = Hub::<i32>::new("topic")?;
let sub = Hub::<i32>::new("topic")?;

pub1.send(1, None)?;
pub2.send(2, None)?;
pub3.send(3, None)?;

let mut received = vec![];
while let Some(msg) = sub.recv(None) {
    received.push(msg);
}
assert_eq!(received.len(), 3);
assert!(received.contains(&1));
assert!(received.contains(&2));
assert!(received.contains(&3));
```

### Scenario 4: No Subscriber (Message Buffering)
**Given:** Publisher sends before subscriber exists
**When:** Subscriber connects later
**Then:**
- [ ] Messages are buffered in shared memory
- [ ] Subscriber receives buffered messages
- [ ] Ring buffer maintains recent N messages

**Acceptance Criteria:**
```rust
let pub_hub = Hub::<i32>::new("buffered")?;
pub_hub.send(1, None)?;
pub_hub.send(2, None)?;
pub_hub.send(3, None)?;

// Subscriber created after messages sent
let sub_hub = Hub::<i32>::new("buffered")?;
assert_eq!(sub_hub.recv(None), Some(1));
assert_eq!(sub_hub.recv(None), Some(2));
assert_eq!(sub_hub.recv(None), Some(3));
```

### Scenario 5: Ring Buffer Overflow
**Given:** Hub has capacity of 1024 slots
**When:** Publisher sends 2000 messages without subscriber reading
**Then:**
- [ ] Oldest messages are overwritten
- [ ] Most recent 1024 messages are available
- [ ] No crash or undefined behavior
- [ ] Send returns Ok(()) even when overwriting

### Scenario 6: Empty Receive
**Given:** No messages have been published
**When:** Subscriber calls `recv()`
**Then:**
- [ ] Returns `None` immediately
- [ ] Non-blocking operation
- [ ] No waiting or timeout

**Acceptance Criteria:**
```rust
let hub = Hub::<i32>::new("empty")?;
assert_eq!(hub.recv(None), None);
```

### Scenario 7: Large Messages
**Given:** Message is 120KB (PointCloud with 10K points)
**When:** Publisher sends large message
**Then:**
- [ ] Message is sent successfully
- [ ] Subscriber receives complete message
- [ ] Latency is < 200μs
- [ ] Data integrity is perfect

**Acceptance Criteria:**
```rust
#[derive(Clone, Serialize, Deserialize)]
struct PointCloud {
    points: [Point3D; 10000],
}

let hub = Hub::<PointCloud>::new("lidar")?;
let cloud = PointCloud { points: [Point3D::default(); 10000] };
hub.send(cloud.clone(), None)?;

let received = hub.recv(None).unwrap();
assert_eq!(received.points.len(), cloud.points.len());
```

### Scenario 8: High Frequency Publishing
**Given:** Node publishes at 1000 Hz
**When:** Publishing for extended period
**Then:**
- [ ] No message loss (within buffer capacity)
- [ ] Consistent latency
- [ ] No memory leaks
- [ ] CPU usage remains stable

### Scenario 9: Custom Capacity
**Given:** User needs larger buffer
**When:** User creates Hub with `new_with_capacity("topic", 4096)`
**Then:**
- [ ] Hub buffer has 4096 slots
- [ ] Can buffer 4096 messages before overflow
- [ ] Performance is maintained

**Acceptance Criteria:**
```rust
let hub = Hub::<i32>::new_with_capacity("large", 4096)?;
// Send 4000 messages
for i in 0..4000 {
    hub.send(i, None)?;
}
// All messages should still be available
```

### Scenario 10: Type Safety
**Given:** Two Hubs with different types on same topic name
**When:** Both try to communicate
**Then:**
- [ ] Compile-time error prevents type mismatch
- [ ] Rust's type system enforces correctness
- [ ] No runtime type errors possible

**Acceptance Criteria:**
```rust
// This should not compile:
let hub_i32 = Hub::<i32>::new("topic")?;
let hub_f64 = Hub::<f64>::new("topic")?;
hub_i32.send(42, None)?;
let x: f64 = hub_f64.recv(None).unwrap();  // Type mismatch!
```

## Performance Tests

### Scenario 11: Latency Benchmarks
**Given:** Production benchmarking setup
**When:** Measuring round-trip latency
**Then:**
- [ ] 16B messages: < 300ns
- [ ] 304B messages: < 800ns
- [ ] 1.5KB messages: < 2μs
- [ ] 120KB messages: < 200μs

### Scenario 12: Throughput Test
**Given:** Maximum throughput scenario
**When:** Publisher sends continuously
**Then:**
- [ ] Achieves > 1M msg/sec for small messages
- [ ] No dropped messages (within buffer capacity)
- [ ] Latency remains consistent

### Scenario 13: Memory Footprint
**Given:** Hub created with default capacity
**When:** Measuring shared memory usage
**Then:**
- [ ] Memory usage is predictable
- [ ] Capacity × message_size + overhead
- [ ] Shared memory visible in /dev/shm/horus/

**Acceptance Criteria:**
```bash
$ ls -lh /dev/shm/horus/
-rw-rw-rw- 1 user user 64K test_topic
```

## Error Handling

### Scenario 14: Shared Memory Creation Failure
**Given:** Insufficient shared memory space
**When:** User creates Hub
**Then:**
- [ ] Returns `Err(HorusError::SharedMemory(...))`
- [ ] Clear error message
- [ ] No partial initialization

### Scenario 15: Permission Denied
**Given:** User lacks permission for /dev/shm/horus/
**When:** User creates Hub
**Then:**
- [ ] Returns `Err(HorusError::PermissionDenied(...))`
- [ ] Error explains permission issue
- [ ] Suggests solution (check permissions)

## Cleanup and Resource Management

### Scenario 16: Hub Dropped
**Given:** Hub goes out of scope
**When:** Drop trait executes
**Then:**
- [ ] Shared memory reference is cleaned up
- [ ] No memory leaks
- [ ] Other Hubs on same topic still work

### Scenario 17: Process Crash
**Given:** Process with Hub crashes
**When:** New process creates same Hub
**Then:**
- [ ] Old shared memory is reused or cleaned
- [ ] No stale data issues
- [ ] System remains stable

### Scenario 18: Manual Cleanup
**Given:** User wants to clean shared memory
**When:** User deletes `/dev/shm/horus/*`
**Then:**
- [ ] All topic data is removed
- [ ] Fresh Hubs can be created
- [ ] No corruption issues

## Edge Cases

### Edge Case 1: Topic Name with Special Characters
**Given:** Topic name is "robot/sensors/lidar"
**When:** Hub is created
**Then:**
- [ ] Topic name is sanitized for filesystem
- [ ] Hub creation succeeds
- [ ] Pub/sub works correctly

### Edge Case 2: Very Long Topic Name
**Given:** Topic name is 256 characters
**When:** Hub is created
**Then:**
- [ ] Either succeeds or fails with clear error
- [ ] No buffer overflow
- [ ] Error suggests shorter name

### Edge Case 3: Zero-Capacity Hub
**Given:** User tries `new_with_capacity("topic", 0)`
**When:** Hub creation is attempted
**Then:**
- [ ] Returns error or panics with clear message
- [ ] Does not create invalid Hub

### Edge Case 4: Same Process, Multiple Hubs
**Given:** Same process creates 100 different Hubs
**When:** All publish simultaneously
**Then:**
- [ ] All Hubs work independently
- [ ] No interference between topics
- [ ] System remains stable

## Non-Functional Requirements

- [ ] Zero-copy within shared memory (no serialization overhead)
- [ ] Lock-free atomic operations only
- [ ] Cache-line aligned data structures
- [ ] Deterministic latency (no garbage collection pauses)
- [ ] Cross-platform: Linux, macOS (where POSIX shm available)
- [ ] No dynamic allocation in send/recv paths
- [ ] Thread-safe (multiple threads can use same Hub)
