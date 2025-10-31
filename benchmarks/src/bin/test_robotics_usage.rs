/// Robotics Usage Test Suite
/// Verifies typical robotics usage patterns work correctly

use horus::prelude::{Hub, Link, Node, NodeInfo, Scheduler};
use horus_library::messages::cmd_vel::CmdVel;
use std::env;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <test_name>", args[0]);
        eprintln!("Available tests:");
        eprintln!("  sensor_flow");
        eprintln!("  actuator_commands");
        eprintln!("  control_loop_1khz");
        eprintln!("  transforms");
        eprintln!("  state_machine");
        process::exit(1);
    }

    let test_name = &args[1];
    let result = match test_name.as_str() {
        "sensor_flow" => test_sensor_flow(),
        "actuator_commands" => test_actuator_commands(),
        "control_loop_1khz" => test_control_loop_1khz(),
        "transforms" => test_transforms(),
        "state_machine" => test_state_machine(),
        _ => {
            eprintln!("Unknown test: {}", test_name);
            process::exit(1);
        }
    };

    if result {
        println!("✓ Test passed: {}", test_name);
        process::exit(0);
    } else {
        eprintln!("✗ Test failed: {}", test_name);
        process::exit(1);
    }
}

/// Test 1: Sensor Data Flow
fn test_sensor_flow() -> bool {
    println!("Testing sensor data flow pattern...");

    // Simulate Sensor → Processor → Command pipeline using CmdVel
    let sensor_topic = format!("test_sensor_{}", process::id());
    let processed_topic = format!("test_processed_{}", process::id());
    let cmd_topic = format!("test_cmd_{}", process::id());

    let sensor_pub = match Hub::<CmdVel>::new(&sensor_topic) {
        Ok(p) => p,
    };

    let sensor_sub = match Hub::<CmdVel>::new(&sensor_topic) {
    };

    let processed_pub = match Hub::<CmdVel>::new(&processed_topic) {
        Ok(p) => p,
    };

    let processed_sub = match Hub::<CmdVel>::new(&processed_topic) {
    };

    let cmd_pub = match Hub::<CmdVel>::new(&cmd_topic) {
        Ok(p) => p,
    };

    let cmd_sub = match Hub::<CmdVel>::new(&cmd_topic) {
    };

    println!("  ✓ Created sensor data pipeline (Sensor → Processor → Cmd)");

    thread::sleep(Duration::from_millis(100));

    // Spawn processor thread
    let processor_handle = thread::spawn(move || {
        let mut processed = 0;
        let start = Instant::now();
        while processed < 100 && start.elapsed() < Duration::from_secs(5) {
            if let Some(sensor) = sensor_sub.recv(None) {
                let processed_msg = CmdVel::with_timestamp(
                    sensor.linear * 0.8,
                    sensor.angular * 0.9,
                    sensor.stamp_nanos,
                );
                if processed_pub.send(processed_msg, None).is_ok() {
                    processed += 1;
                }
            } else {
                thread::sleep(Duration::from_micros(100));
            }
        }
        processed == 100
    });

    // Spawn controller thread
    let controller_handle = thread::spawn(move || {
        let mut processed = 0;
        let start = Instant::now();
        while processed < 100 && start.elapsed() < Duration::from_secs(5) {
            if let Some(msg) = processed_sub.recv(None) {
                let cmd = CmdVel::with_timestamp(
                    msg.linear * 0.5,
                    msg.angular * 0.3,
                    msg.stamp_nanos,
                );
                if cmd_pub.send(cmd, None).is_ok() {
                    processed += 1;
                }
            } else {
                thread::sleep(Duration::from_micros(100));
            }
        }
        processed == 100
    });

    // Main thread: publish sensor data and receive commands
    let mut received_cmds = 0;
    for i in 0..100 {
        let sensor = CmdVel::with_timestamp(1.0 + (i as f32 * 0.01), 0.5, i);

        if let Err(e) = sensor_pub.send(sensor, None) {
            eprintln!("Failed to publish sensor data {}: {:?}", i, e);
            return false;
        }

        if let Some(_cmd) = cmd_sub.recv(None) {
            received_cmds += 1;
        }

        thread::sleep(Duration::from_micros(500));
    }

    // Wait for remaining commands
    let start = Instant::now();
    while received_cmds < 100 && start.elapsed() < Duration::from_secs(2) {
        if cmd_sub.recv(None).is_some() {
            received_cmds += 1;
        } else {
            thread::sleep(Duration::from_micros(100));
        }
    }

    let processor_result = processor_handle.join().unwrap();
    let controller_result = controller_handle.join().unwrap();

    if processor_result && controller_result && received_cmds >= 95 {
        println!("  ✓ Processor handled 100 sensor messages");
        println!("  ✓ Controller processed 100 messages");
        println!("  ✓ Received {} command messages", received_cmds);
        true
    } else {
        eprintln!("Pipeline incomplete: processor={}, controller={}, cmds={}",
                 processor_result, controller_result, received_cmds);
        false
    }
}

/// Test 2: Actuator Commands
fn test_actuator_commands() -> bool {
    println!("Testing actuator command pattern...");

    let topic = format!("test_actuator_{}", process::id());

    // Create command link (high priority, low latency)
    let cmd_sender = match Link::<CmdVel>::producer_with_capacity(&topic, 256) {
    };

    let cmd_receiver = match Link::<CmdVel>::consumer(&topic) {
        Ok(r) => r,
    };

    println!("  ✓ Created actuator command Link");

    // Simulate motor driver receiving commands
    let driver_handle = thread::spawn(move || {
        let mut last_timestamp = 0;
        let mut received = 0;
        let start = Instant::now();

        while received < 500 && start.elapsed() < Duration::from_secs(5) {
            if let Some(cmd) = cmd_receiver.recv(None) {
                // Verify commands are in order
                if cmd.stamp_nanos < last_timestamp {
                    eprintln!("Command out of order: {} < {}", cmd.stamp_nanos, last_timestamp);
                    return false;
                }
                last_timestamp = cmd.stamp_nanos;
                received += 1;

                // Simulate motor actuation delay
                thread::sleep(Duration::from_micros(10));
            }
        }
        received == 500
    });

    // Send commands at regular intervals
    for i in 0..500 {
        let cmd = CmdVel {
            linear: (i as f32 * 0.01).sin(),
            angular: (i as f32 * 0.02).cos(),
            stamp_nanos: i,
        };

        if let Err(e) = cmd_sender.send(cmd, None) {
            eprintln!("Failed to send command {}: {:?}", i, e);
            return false;
        }

        thread::sleep(Duration::from_micros(50)); // 20kHz command rate
    }

    let driver_result = driver_handle.join().unwrap();

    if driver_result {
        println!("  ✓ Sent 500 actuator commands");
        println!("  ✓ All commands received in correct order");
        true
    } else {
        eprintln!("Motor driver did not receive all commands");
        false
    }
}

/// Test 3: Control Loop at 1kHz
fn test_control_loop_1khz() -> bool {
    println!("Testing 1kHz control loop...");

    let counter = Arc::new(Mutex::new(0u32));
    let counter_clone = Arc::clone(&counter);
    let timestamps = Arc::new(Mutex::new(Vec::new()));
    let timestamps_clone = Arc::clone(&timestamps);

    let control_loop = move |_ctx: &mut horus::prelude::NodeInfo| {
        let mut c = counter_clone.lock().unwrap();
        *c += 1;

        let mut ts = timestamps_clone.lock().unwrap();
        ts.push(Instant::now());
    };

    let mut scheduler = Scheduler::new();
    };

    println!("  ✓ Created 1kHz scheduler");

    if let Err(e) = scheduler.add_node("control_loop".to_string(), Box::new(control_loop)) {
        eprintln!("Failed to add node: {}", e);
        return false;
    }

    // Run scheduler in background thread
    let _scheduler_handle = thread::spawn(move || {
        scheduler.spin();
    });

    // Run for 1 second
    thread::sleep(Duration::from_secs(1));

    let final_count = *counter.lock().unwrap();
    let ts = timestamps.lock().unwrap();

    println!("  ✓ Control loop executed {} times in 1 second", final_count);

    // Should execute ~1000 times at 1kHz (allow 10% tolerance)
    if final_count < 900 || final_count > 1100 {
        eprintln!("Execution count out of range: {} (expected ~1000)", final_count);
        return false;
    }

    // Calculate jitter
    if ts.len() > 1 {
        let mut intervals = Vec::new();
        for i in 1..ts.len() {
            let interval = ts[i].duration_since(ts[i-1]);
            intervals.push(interval.as_micros());
        }

        intervals.sort();
        let median = intervals[intervals.len() / 2];
        let p99 = intervals[(intervals.len() * 99) / 100];

        println!("  ✓ Median interval: {}µs (target: 1000µs)", median);
        println!("  ✓ P99 interval: {}µs", p99);

        // Check if jitter is acceptable (<1ms = <1000µs for P99)
        if p99 > 2000 {
            eprintln!("Control loop jitter too high: P99 = {}µs", p99);
            return false;
        }
    }

    println!("  ✓ Control loop timing acceptable");
    true
}

/// Test 4: Transform Broadcasting
fn test_transforms() -> bool {
    println!("Testing broadcast pattern (simulated with CmdVel)...");

    let broadcast_topic = format!("test_broadcast_{}", process::id());

    let broadcaster = match Hub::<CmdVel>::new(&broadcast_topic) {
        Ok(p) => p,
    };

    let listener = match Hub::<CmdVel>::new(&broadcast_topic) {
    };

    println!("  ✓ Created broadcast Hub");

    thread::sleep(Duration::from_millis(100));

    // Broadcast 200 messages at 100Hz
    for i in 0..200 {
        let msg = CmdVel::with_timestamp((i as f32) * 0.01, (i as f32) * 0.02, i);

        if let Err(e) = broadcaster.send(msg, None) {
            eprintln!("Failed to broadcast message {}: {:?}", i, e);
            return false;
        }
        thread::sleep(Duration::from_millis(10)); // 100Hz
    }

    println!("  ✓ Broadcast 200 messages");

    // Receive and verify
    let mut received = 0;
    let start = Instant::now();
    while received < 200 && start.elapsed() < Duration::from_secs(5) {
        if listener.recv(None).is_some() {
            received += 1;
        } else {
            thread::sleep(Duration::from_micros(100));
        }
    }

    if received >= 190 {
        println!("  ✓ Received {} broadcasts", received);
        true
    } else {
        eprintln!("Only received {} out of 200 broadcasts", received);
        false
    }
}

/// Test 5: State Machine
fn test_state_machine() -> bool {
    println!("Testing state machine execution...");

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum RobotState {
        Idle,
        Moving,
        Turning,
        Stopped,
    }

    let state = Arc::new(Mutex::new(RobotState::Idle));
    let state_clone = Arc::clone(&state);
    let transitions = Arc::new(Mutex::new(0u32));
    let transitions_clone = Arc::clone(&transitions);

    let state_machine = move |_ctx: &mut horus::prelude::NodeInfo| {
        let mut s = state_clone.lock().unwrap();
        let mut t = transitions_clone.lock().unwrap();

        // Simple state machine: Idle → Moving → Turning → Stopped → Idle
        *s = match *s {
            RobotState::Idle => {
                *t += 1;
                RobotState::Moving
            }
            RobotState::Moving => {
                *t += 1;
                RobotState::Turning
            }
            RobotState::Turning => {
                *t += 1;
                RobotState::Stopped
            }
            RobotState::Stopped => {
                *t += 1;
                RobotState::Idle
            }
        };
    };

    let mut scheduler = Scheduler::new();
    };

    if let Err(e) = scheduler.add_node("state_machine".to_string(), Box::new(state_machine)) {
        eprintln!("Failed to add state machine node: {}", e);
        return false;
    }

    println!("  ✓ Created state machine (100Hz)");

    let _scheduler_handle = thread::spawn(move || {
        scheduler.spin();
    });

    // Run for 500ms
    thread::sleep(Duration::from_millis(500));

    let final_state = *state.lock().unwrap();
    let total_transitions = *transitions.lock().unwrap();

    println!("  ✓ State machine made {} transitions", total_transitions);
    println!("  ✓ Final state: {:?}", final_state);

    // Should have made ~50 transitions at 100Hz over 500ms
    if total_transitions >= 40 && total_transitions <= 60 {
        println!("  ✓ State machine execution rate correct");
        true
    } else {
        eprintln!("Unexpected transition count: {} (expected ~50)", total_transitions);
        false
    }
}
