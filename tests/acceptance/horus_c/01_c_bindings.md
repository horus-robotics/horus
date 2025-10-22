# User Acceptance Test: C Bindings (Alpha - Under Development)

## Feature Status
⚠️ **C bindings are in alpha and under active development. This test suite represents the target functionality.**

## User Story
As a C developer working with hardware drivers, I want to use HORUS from C so that I can integrate with existing embedded systems code.

## Installation Tests

### Scenario 1: Build C Bindings
**Given:** User has HORUS repository
**When:** Building C bindings
**Then:**
- [ ] Make compiles successfully
- [ ] libhorus_c.so (or .dylib) generated
- [ ] Header files available
- [ ] No compilation warnings

**Acceptance Criteria:**
```bash
$ cd horus_c
$ make
gcc -c ...
✓ Built libhorus_c.so
✓ Headers: horus.h
```

### Scenario 2: Link Against C Library
**Given:** User has C project
**When:** Linking with -lhorus_c
**Then:**
- [ ] Links successfully
- [ ] No undefined symbols
- [ ] Executable runs

**Acceptance Criteria:**
```bash
$ gcc -o my_app main.c -lhorus_c
$ ./my_app
# Runs successfully
```

## Basic Hub Tests

### Scenario 3: Create Hub from C
**Given:** User includes horus.h
**When:** Calling horus_hub_create()
**Then:**
- [ ] Hub is created
- [ ] Returns non-NULL pointer
- [ ] Topic is registered

**Acceptance Criteria:**
```c
#include <horus.h>

int main() {
    horus_hub_t* hub = horus_hub_create("test_topic", HORUS_TYPE_INT32);
    if (hub == NULL) {
        fprintf(stderr, "Failed to create hub\n");
        return 1;
    }

    // Use hub...

    horus_hub_destroy(hub);
    return 0;
}
```

### Scenario 4: Send Message from C
**Given:** Hub is created
**When:** Calling horus_hub_send()
**Then:**
- [ ] Message is sent
- [ ] Returns success code
- [ ] Data reaches shared memory

**Acceptance Criteria:**
```c
int32_t data = 42;
int result = horus_hub_send(hub, &data, sizeof(data));
if (result != HORUS_OK) {
    fprintf(stderr, "Send failed\n");
}
```

### Scenario 5: Receive Message from C
**Given:** Message is available
**When:** Calling horus_hub_recv()
**Then:**
- [ ] Message is received
- [ ] Data is copied to buffer
- [ ] Returns number of bytes read

**Acceptance Criteria:**
```c
int32_t data;
int bytes = horus_hub_recv(hub, &data, sizeof(data));
if (bytes > 0) {
    printf("Received: %d\n", data);
}
```

### Scenario 6: Destroy Hub
**Given:** Hub is no longer needed
**When:** Calling horus_hub_destroy()
**Then:**
- [ ] Resources freed
- [ ] No memory leaks
- [ ] Shared memory cleaned up

**Acceptance Criteria:**
```c
horus_hub_destroy(hub);
hub = NULL;  // Good practice
```

## Error Handling Tests

### Scenario 7: Error Codes
**Given:** Operation fails
**When:** Checking return value
**Then:**
- [ ] Error code is meaningful
- [ ] HORUS_OK for success
- [ ] HORUS_ERROR_* for failures

**Acceptance Criteria:**
```c
#define HORUS_OK 0
#define HORUS_ERROR_INVALID_TOPIC -1
#define HORUS_ERROR_PERMISSION_DENIED -2
#define HORUS_ERROR_NO_MESSAGE -3
// etc.

int result = horus_hub_create(...);
if (result != HORUS_OK) {
    switch (result) {
        case HORUS_ERROR_INVALID_TOPIC:
            // Handle
            break;
        // ...
    }
}
```

### Scenario 8: Error Messages
**Given:** Error occurred
**When:** Calling horus_get_last_error()
**Then:**
- [ ] Human-readable error message
- [ ] Provides context
- [ ] Thread-safe

**Acceptance Criteria:**
```c
if (horus_hub_send(hub, &data, sizeof(data)) != HORUS_OK) {
    const char* err = horus_get_last_error();
    fprintf(stderr, "Error: %s\n", err);
}
```

## Type Support Tests

### Scenario 9: Primitive Types
**Given:** C bindings support common types
**When:** Creating Hubs for different types
**Then:**
- [ ] int32_t works
- [ ] float works
- [ ] double works
- [ ] uint8_t works

**Acceptance Criteria:**
```c
horus_hub_t* int_hub = horus_hub_create("int", HORUS_TYPE_INT32);
horus_hub_t* float_hub = horus_hub_create("float", HORUS_TYPE_FLOAT);
horus_hub_t* double_hub = horus_hub_create("double", HORUS_TYPE_DOUBLE);
```

### Scenario 10: Struct Types
**Given:** User has custom struct
**When:** Sending via Hub
**Then:**
- [ ] Struct is serialized
- [ ] Data integrity maintained
- [ ] Receiver can deserialize

**Acceptance Criteria:**
```c
typedef struct {
    float linear;
    float angular;
} cmd_vel_t;

horus_hub_t* hub = horus_hub_create("cmd_vel", HORUS_TYPE_CUSTOM);
cmd_vel_t cmd = {1.0f, 0.5f};
horus_hub_send(hub, &cmd, sizeof(cmd));
```

## Cross-Language Tests

### Scenario 11: C to Rust Communication
**Given:** C publisher, Rust subscriber
**When:** C sends message
**Then:**
- [ ] Rust receives message
- [ ] Data matches exactly
- [ ] No type errors

**Acceptance Criteria:**
```c
// C:
int32_t data = 999;
horus_hub_send(hub, &data, sizeof(data));
```
```rust
// Rust:
let hub = Hub::<i32>::new("test")?;
let msg = hub.recv(None);
assert_eq!(msg, Some(999));
```

### Scenario 12: Rust to C Communication
**Given:** Rust publisher, C subscriber
**When:** Rust sends message
**Then:**
- [ ] C receives message
- [ ] Data integrity maintained

```rust
// Rust:
let hub = Hub::<i32>::new("from_rust")?;
hub.send(777, None)?;
```
```c
// C:
int32_t data;
horus_hub_recv(hub, &data, sizeof(data));
printf("Received: %d\n", data);  // Should print 777
```

## Memory Management Tests

### Scenario 13: No Memory Leaks
**Given:** C application uses Hubs
**When:** Running under valgrind
**Then:**
- [ ] No memory leaks detected
- [ ] All allocations freed
- [ ] Clean shutdown

**Acceptance Criteria:**
```bash
$ valgrind --leak-check=full ./my_c_app
...
All heap blocks were freed -- no leaks are possible
```

### Scenario 14: Multiple Hub Lifecycle
**Given:** Creating and destroying many Hubs
**When:** Looping create/destroy 1000 times
**Then:**
- [ ] No resource exhaustion
- [ ] Stable memory usage
- [ ] No crashes

## Thread Safety Tests

### Scenario 15: Concurrent Access
**Given:** Multiple C threads use same Hub
**When:** Publishing from multiple threads
**Then:**
- [ ] Thread-safe operations
- [ ] No data races
- [ ] All messages delivered

**Acceptance Criteria:**
```c
// Thread-safe if documented as such
void* sender_thread(void* arg) {
    horus_hub_t* hub = (horus_hub_t*)arg;
    int32_t data = 42;
    horus_hub_send(hub, &data, sizeof(data));
    return NULL;
}

pthread_t threads[10];
for (int i = 0; i < 10; i++) {
    pthread_create(&threads[i], NULL, sender_thread, hub);
}
```

## API Completeness Tests

### Scenario 16: Essential Functions Available
**Given:** C bindings API
**When:** Reviewing available functions
**Then:**
- [ ] horus_hub_create()
- [ ] horus_hub_destroy()
- [ ] horus_hub_send()
- [ ] horus_hub_recv()
- [ ] horus_get_last_error()
- [ ] horus_version()

### Scenario 17: Future: Node Support
**Given:** Node trait for C (not yet implemented)
**When:** Implemented in future
**Then:**
- [ ] horus_node_create()
- [ ] horus_node_register()
- [ ] Callback functions for init/tick/shutdown

## Documentation Tests

### Scenario 18: Header File Documentation
**Given:** User reads horus.h
**When:** Looking at function declarations
**Then:**
- [ ] All functions have comments
- [ ] Parameters documented
- [ ] Return values explained
- [ ] Examples provided

**Acceptance Criteria:**
```c
/**
 * Create a new Hub for the given topic.
 *
 * @param topic_name Name of the topic (must not be empty)
 * @param type Type of data (HORUS_TYPE_*)
 * @return Hub pointer on success, NULL on failure
 *
 * Example:
 * horus_hub_t* hub = horus_hub_create("sensor_data", HORUS_TYPE_FLOAT);
 */
horus_hub_t* horus_hub_create(const char* topic_name, horus_type_t type);
```

### Scenario 19: Example Programs
**Given:** C bindings distribution
**When:** User looks for examples
**Then:**
- [ ] examples/ directory exists
- [ ] Simple publisher example
- [ ] Simple subscriber example
- [ ] README with instructions

## Current Limitations (Alpha Status)

### Known Limitations
- [ ] Limited API coverage (basic pub/sub only)
- [ ] No Node trait support yet
- [ ] No advanced features (scheduler, etc.)
- [ ] Documentation incomplete
- [ ] Type system limited
- [ ] May change before v1.0

### Future Roadmap
- [ ] Full Node trait support
- [ ] Scheduler integration
- [ ] Extended type system
- [ ] Better error handling
- [ ] Comprehensive examples
- [ ] Performance optimization

## Non-Functional Requirements

- [ ] Zero-copy where possible
- [ ] Minimal overhead over Rust API
- [ ] Valgrind-clean
- [ ] Thread-safe where documented
- [ ] Works on Linux and macOS
- [ ] Compatible with C99 or later
- [ ] No global state (except thread-local errors)

## Testing Strategy

Since C bindings are alpha:
1. Manual testing with example programs
2. Valgrind for memory checks
3. Cross-language integration tests
4. Performance comparisons with Rust
5. User feedback from early adopters

**Note:** This test suite serves as specification for future development. Many tests may not pass until C bindings reach beta or stable status.
