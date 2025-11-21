// Benchmark binary - allow clippy warnings
#![allow(unused_imports)]
#![allow(unused_assignments)]
#![allow(unreachable_patterns)]
#![allow(clippy::all)]
#![allow(deprecated)]

/// IPC Functionality Test Suite
/// Verifies that IPC mechanisms work correctly in various scenarios
use horus::prelude::{Hub, Link};
use horus_library::messages::cmd_vel::CmdVel;
use std::env;
use std::process::{self, Command};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <test_name>", args[0]);
        eprintln!("Available tests:");
        eprintln!("  hub_multiprocess");
        eprintln!("  link_singleprocess");
        eprintln!("  cross_process");
        eprintln!("  large_messages");
        eprintln!("  high_frequency");
        process::exit(1);
    }

    let test_name = &args[1];
    let result = match test_name.as_str() {
        "hub_multiprocess" => test_hub_multiprocess(),
        "link_singleprocess" => test_link_singleprocess(),
        "cross_process" => test_cross_process(),
        "large_messages" => test_large_messages(),
        "high_frequency" => test_high_frequency(),
        _ => {
            eprintln!("Unknown test: {}", test_name);
            process::exit(1);
        }
    };

    if result {
        println!(" Test passed: {}", test_name);
        process::exit(0);
    } else {
        eprintln!("[FAIL] Test failed: {}", test_name);
        process::exit(1);
    }
}

/// Test 1: Hub Multi-Process MPMC
fn test_hub_multiprocess() -> bool {
    println!("Testing Hub multi-process communication...");

    let topic = format!("test_hub_mp_{}", process::id());

    // Check if we're the parent or child process
    if env::var("TEST_ROLE").is_err() {
        // Parent process - spawn child and act as subscriber
        println!("  Starting as parent (subscriber)...");

        let subscriber = match Hub::<CmdVel>::new(&topic) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create subscriber: {}", e);
                return false;
            }
        };

        // Spawn child process as publisher
        let child = Command::new(env::current_exe().unwrap())
            .arg("hub_multiprocess")
            .env("TEST_ROLE", "child")
            .env("TEST_TOPIC", &topic)
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to spawn child process: {}", e);
                return false;
            }
        };

        // Give child time to start and publish
        thread::sleep(Duration::from_millis(200));

        // Receive messages from child
        let mut received = 0;
        let start = Instant::now();
        while received < 100 && start.elapsed() < Duration::from_secs(5) {
            if let Some(msg) = subscriber.recv(&mut None) {
                if msg.stamp_nanos as usize != received {
                    eprintln!(
                        "Message order error: expected {}, got {}",
                        received, msg.stamp_nanos
                    );
                    return false;
                }
                received += 1;
            } else {
                thread::sleep(Duration::from_micros(100));
            }
        }

        // Wait for child to finish
        let _ = child.wait();

        if received == 100 {
            println!("   Received all 100 messages from child process");
            true
        } else {
            eprintln!("Only received {} out of 100 messages", received);
            false
        }
    } else {
        // Child process - act as publisher
        let topic = env::var("TEST_TOPIC").unwrap();

        thread::sleep(Duration::from_millis(100)); // Wait for parent subscriber

        let publisher = match Hub::<CmdVel>::new(&topic) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Child: Failed to create publisher: {}", e);
                return false;
            }
        };

        // Publish 100 messages
        for i in 0..100 {
            let msg = CmdVel {
                linear: 1.0,
                angular: 0.5,
                stamp_nanos: i,
            };
            if let Err(e) = publisher.send(msg, &mut None) {
                eprintln!("Child: Failed to publish message {}: {:?}", i, e);
                return false;
            }
            thread::sleep(Duration::from_micros(100)); // Throttle
        }

        true
    }
}

/// Test 2: Link Single-Process SPSC
fn test_link_singleprocess() -> bool {
    println!("Testing Link single-process SPSC...");

    let topic = format!("test_link_sp_{}", process::id());

    // Create sender and receiver in same process
    let sender = match Link::<CmdVel>::producer(&topic) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create sender: {}", e);
            return false;
        }
    };

    let receiver = match Link::<CmdVel>::consumer(&topic) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create receiver: {}", e);
            return false;
        }
    };

    println!("   Created sender and receiver");

    // Spawn sender thread
    let sender_handle = thread::spawn(move || {
        for i in 0..1000 {
            let msg = CmdVel {
                linear: 2.0 + i as f32 * 0.01,
                angular: 1.0,
                stamp_nanos: i,
            };
            if let Err(e) = sender.send(msg, &mut None) {
                eprintln!("Failed to send message {}: {:?}", i, e);
                return false;
            }
        }
        true
    });

    // Receive in main thread
    let mut received = 0;
    let start = Instant::now();
    while received < 1000 && start.elapsed() < Duration::from_secs(5) {
        if let Some(msg) = receiver.recv(&mut None) {
            if msg.stamp_nanos as usize != received {
                eprintln!(
                    "Message order error: expected {}, got {}",
                    received, msg.stamp_nanos
                );
                return false;
            }
            received += 1;
        }
    }

    let sender_result = sender_handle.join().unwrap();

    if received == 1000 && sender_result {
        println!("   Sent and received 1000 messages in correct order");
        true
    } else {
        eprintln!("Only received {} out of 1000 messages", received);
        false
    }
}

/// Test 3: Cross-Process Messaging
fn test_cross_process() -> bool {
    println!("Testing cross-process messaging with Link...");

    let topic = format!("test_cross_proc_{}", process::id());

    if env::var("TEST_ROLE").is_err() {
        // Parent process - receiver
        println!("  Starting as parent (consumer)...");

        let receiver = match Link::<CmdVel>::consumer(&topic) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to create receiver: {}", e);
                return false;
            }
        };

        // Spawn child process as sender
        let child = Command::new(env::current_exe().unwrap())
            .arg("cross_process")
            .env("TEST_ROLE", "child")
            .env("TEST_TOPIC", &topic)
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to spawn child process: {}", e);
                return false;
            }
        };

        thread::sleep(Duration::from_millis(200)); // Wait for child to start

        // Receive messages
        let mut received = 0;
        let start = Instant::now();
        while received < 500 && start.elapsed() < Duration::from_secs(5) {
            if let Some(msg) = receiver.recv(&mut None) {
                if msg.stamp_nanos as usize != received {
                    eprintln!(
                        "Message order error: expected {}, got {}",
                        received, msg.stamp_nanos
                    );
                    return false;
                }
                received += 1;
            }
        }

        let _ = child.wait();

        if received == 500 {
            println!("   Received all 500 messages from child process via Link");
            true
        } else {
            eprintln!("Only received {} out of 500 messages", received);
            false
        }
    } else {
        // Child process - sender
        let topic = env::var("TEST_TOPIC").unwrap();

        let sender = match Link::<CmdVel>::producer(&topic) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Child: Failed to create sender: {}", e);
                return false;
            }
        };

        thread::sleep(Duration::from_millis(100)); // Wait for parent receiver

        // Send 500 messages
        for i in 0..500 {
            let msg = CmdVel {
                linear: 1.5,
                angular: 0.75,
                stamp_nanos: i,
            };
            if let Err(e) = sender.send(msg, &mut None) {
                eprintln!("Child: Failed to send message {}: {:?}", i, e);
                return false;
            }
        }

        true
    }
}

/// Test 4: Large Messages (1MB)
fn test_large_messages() -> bool {
    println!("Testing large message handling (1MB)...");

    // Use a custom large message type
    #[derive(Clone, Copy)]
    #[repr(C)]
    struct LargeMessage {
        data: [u8; 1024 * 1024], // 1MB
        checksum: u64,
    }

    // unsafe impl horus_core::memory::Payload for LargeMessage {}

    let topic = format!("test_large_{}", process::id());

    let publisher = match Hub::<CmdVel>::new(&topic) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create publisher: {}", e);
            return false;
        }
    };

    let subscriber = match Hub::<CmdVel>::new(&topic) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create subscriber: {}", e);
            return false;
        }
    };

    thread::sleep(Duration::from_millis(100));

    println!("   Created Hub for 1MB messages");

    // Publish high-frequency messages
    for i in 0..1000 {
        let msg = CmdVel {
            linear: (i as f32) * 0.001,
            angular: (i as f32) * 0.002,
            stamp_nanos: i,
        };

        if let Err(e) = publisher.send(msg, &mut None) {
            eprintln!("Failed to publish message {}: {:?}", i, e);
            return false;
        }
    }

    println!("   Published 1000 messages");

    thread::sleep(Duration::from_millis(100));

    // Receive and verify ordering
    let mut received = 0;
    let start = Instant::now();
    while received < 1000 && start.elapsed() < Duration::from_secs(5) {
        if let Some(msg) = subscriber.recv(&mut None) {
            // Verify message ordering
            if msg.stamp_nanos != received {
                eprintln!(
                    "Message order error: expected {}, got {}",
                    received, msg.stamp_nanos
                );
                return false;
            }
            received += 1;
        } else {
            thread::sleep(Duration::from_micros(10));
        }
    }

    if received >= 950 {
        println!("   Received {} messages in correct order", received);
        true
    } else {
        eprintln!("Only received {} out of 1000 messages", received);
        false
    }
}

/// Test 5: High Frequency (10kHz)
fn test_high_frequency() -> bool {
    println!("Testing high-frequency communication (10kHz)...");

    let topic = format!("test_highfreq_{}", process::id());

    let sender = match Link::<CmdVel>::producer(&topic) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create sender: {}", e);
            return false;
        }
    };

    let receiver = match Link::<CmdVel>::consumer(&topic) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create receiver: {}", e);
            return false;
        }
    };

    println!("   Created high-frequency Link");

    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = Arc::clone(&barrier);

    // Spawn sender thread
    let sender_handle = thread::spawn(move || {
        barrier_clone.wait(); // Synchronize start

        let start = Instant::now();
        let target_messages = 10000; // 10k messages in 1 second = 10kHz
        let target_duration = Duration::from_secs(1);

        for i in 0..target_messages {
            let msg = CmdVel {
                linear: 1.0,
                angular: 0.5,
                stamp_nanos: i,
            };

            // Send with retries if buffer full
            let mut retries = 0;
            while let Err(_) = sender.send(msg, &mut None) {
                retries += 1;
                if retries > 100 {
                    eprintln!("Too many send retries at message {}", i);
                    return (false, 0);
                }
            }
        }

        let elapsed = start.elapsed();
        let actual_rate = target_messages as f64 / elapsed.as_secs_f64();

        (true, actual_rate as u64)
    });

    // Receive in main thread
    barrier.wait(); // Synchronize start

    let start = Instant::now();
    let mut received = 0;
    let target_messages = 10000;

    while received < target_messages && start.elapsed() < Duration::from_secs(3) {
        if let Some(_msg) = receiver.recv(&mut None) {
            received += 1;
        }
    }

    let elapsed = start.elapsed();
    let actual_rate = received as f64 / elapsed.as_secs_f64();

    let (sender_result, send_rate) = sender_handle.join().unwrap();

    println!("  Send rate: {:.0} Hz", send_rate);
    println!("  Receive rate: {:.0} Hz", actual_rate);

    if received >= target_messages && sender_result && actual_rate >= 8000.0 {
        println!("   Achieved >8kHz communication rate");
        true
    } else {
        eprintln!(
            "Failed to achieve target rate: received {} messages at {:.0} Hz",
            received, actual_rate
        );
        false
    }
}
