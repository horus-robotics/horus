//! Debug iceoryx2 multi-process IPC
//!
//! Simple test to identify issues with iceoryx2 setup

use horus_library::messages::cmd_vel::CmdVel;
use iceoryx2::prelude::*;
use std::env;
use std::time::{Duration, Instant};

const ITERATIONS: usize = 100;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "producer" => run_producer(&args[2]),
            "consumer" => run_consumer(&args[2]),
            _ => eprintln!("Usage: {} [producer|consumer] <topic>", args[0]),
        }
        return;
    }

    println!("=== iceoryx2 Debug Test ===\n");

    // Test 1: Basic service creation
    println!("Test 1: Creating iceoryx2 node...");
    let node = match NodeBuilder::new().create::<ipc::Service>() {
        Ok(n) => {
            println!("  ✓ Node created successfully");
            n
        }
        Err(e) => {
            eprintln!("  ✗ Failed to create node: {:?}", e);
            return;
        }
    };

    // Test 2: Service creation
    println!("\nTest 2: Creating pub/sub service...");
    let topic = format!("ice_debug_test_{}", std::process::id());
    let service_name = match ServiceName::new(&topic) {
        Ok(s) => {
            println!("  ✓ Service name created: {}", topic);
            s
        }
        Err(e) => {
            eprintln!("  ✗ Failed to create service name: {:?}", e);
            return;
        }
    };

    let service = match node
        .service_builder(&service_name)
        .publish_subscribe::<CmdVel>()
        .open_or_create()
    {
        Ok(s) => {
            println!("  ✓ Service created successfully");
            s
        }
        Err(e) => {
            eprintln!("  ✗ Failed to create service: {:?}", e);
            return;
        }
    };

    // Test 3: Publisher creation
    println!("\nTest 3: Creating publisher...");
    let publisher = match service.publisher_builder().create() {
        Ok(p) => {
            println!("  ✓ Publisher created successfully");
            p
        }
        Err(e) => {
            eprintln!("  ✗ Failed to create publisher: {:?}", e);
            return;
        }
    };

    // Test 4: Subscriber creation
    println!("\nTest 4: Creating subscriber...");
    let subscriber = match service.subscriber_builder().create() {
        Ok(s) => {
            println!("  ✓ Subscriber created successfully");
            s
        }
        Err(e) => {
            eprintln!("  ✗ Failed to create subscriber: {:?}", e);
            return;
        }
    };

    // Test 5: Send and receive in same process
    println!("\nTest 5: Single-process send/receive...");
    let msg = CmdVel::new(1.5, 0.8);

    match publisher.loan_uninit() {
        Ok(sample) => {
            let sample = sample.write_payload(msg);
            match sample.send() {
                Ok(_) => println!("  ✓ Message sent"),
                Err(e) => eprintln!("  ✗ Failed to send: {:?}", e),
            }
        }
        Err(e) => {
            eprintln!("  ✗ Failed to loan sample: {:?}", e);
            return;
        }
    }

    std::thread::sleep(Duration::from_millis(10));

    match subscriber.receive() {
        Ok(Some(sample)) => {
            let payload = sample.payload();
            println!(
                "  ✓ Received: linear={}, angular={}",
                payload.linear, payload.angular
            );
        }
        Ok(None) => {
            eprintln!("  ✗ No message received");
        }
        Err(e) => {
            eprintln!("  ✗ Receive error: {:?}", e);
        }
    }

    // Test 6: Multi-process test (WRONG ORDER - consumer first)
    println!("\n=== Multi-Process Test (Consumer First - WILL FAIL) ===");
    test_multiprocess_order("consumer_first");

    // Test 7: Multi-process test (CORRECT ORDER - producer first)
    println!("\n=== Multi-Process Test (Producer First - SHOULD WORK) ===");
    test_multiprocess_order("producer_first");
}

fn test_multiprocess_order(test_name: &str) {
    let topic_mp = format!("ice_mp_{}_{}", test_name, std::process::id());

    let (producer, consumer) = if test_name == "producer_first" {
        println!("Starting producer subprocess FIRST...");
        let producer = std::process::Command::new(env::current_exe().unwrap())
            .arg("producer")
            .arg(&topic_mp)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn producer");

        println!("Waiting for producer to create publisher...");
        std::thread::sleep(Duration::from_millis(500));

        println!("Now starting consumer subprocess...");
        let consumer = std::process::Command::new(env::current_exe().unwrap())
            .arg("consumer")
            .arg(&topic_mp)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn consumer");

        (producer, consumer)
    } else {
        println!("Starting consumer subprocess FIRST...");
        let consumer = std::process::Command::new(env::current_exe().unwrap())
            .arg("consumer")
            .arg(&topic_mp)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn consumer");

        println!("Waiting for consumer to initialize...");
        std::thread::sleep(Duration::from_millis(500));

        println!("Now starting producer subprocess...");
        let producer = std::process::Command::new(env::current_exe().unwrap())
            .arg("producer")
            .arg(&topic_mp)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn producer");

        (producer, consumer)
    };

    println!("Waiting for completion...");
    let producer_result = producer.wait_with_output().unwrap();
    let consumer_result = consumer.wait_with_output().unwrap();

    println!("\nProducer output:");
    println!("{}", String::from_utf8_lossy(&producer_result.stdout));
    if !producer_result.stderr.is_empty() {
        eprintln!("Producer stderr:");
        eprintln!("{}", String::from_utf8_lossy(&producer_result.stderr));
    }

    println!("\nConsumer output:");
    println!("{}", String::from_utf8_lossy(&consumer_result.stdout));
    if !consumer_result.stderr.is_empty() {
        eprintln!("Consumer stderr:");
        eprintln!("{}", String::from_utf8_lossy(&consumer_result.stderr));
    }

    if producer_result.status.success() && consumer_result.status.success() {
        println!("\n✓ Multi-process test PASSED");
    } else {
        println!("\n✗ Multi-process test FAILED");
    }
}

fn run_producer(topic: &str) {
    println!("[Producer] Starting with topic: {}", topic);

    let node = NodeBuilder::new().create::<ipc::Service>().unwrap();
    let service_name = ServiceName::new(topic).unwrap();

    println!("[Producer] Creating service...");
    let service = node
        .service_builder(&service_name)
        .publish_subscribe::<CmdVel>()
        // Buffer all messages to prevent drops
        .history_size(ITERATIONS * 2)
        .subscriber_max_buffer_size(ITERATIONS * 2)
        .open_or_create()
        .unwrap();

    println!("[Producer] Creating publisher...");
    let publisher = service.publisher_builder().create().unwrap();

    println!("[Producer] Waiting for subscriber to connect...");
    // Wait for at least one subscriber to connect
    std::thread::sleep(Duration::from_millis(1000));

    println!("[Producer] Sending {} messages...", ITERATIONS);
    let start = Instant::now();

    for i in 0..ITERATIONS {
        let msg = CmdVel::new(1.0 + i as f32 * 0.01, 0.5);

        match publisher.loan_uninit() {
            Ok(sample) => {
                let sample = sample.write_payload(msg);
                if let Err(e) = sample.send() {
                    eprintln!("[Producer] Send error at {}: {:?}", i, e);
                    return;
                }
            }
            Err(e) => {
                eprintln!("[Producer] Loan error at {}: {:?}", i, e);
                return;
            }
        }

        if i % 20 == 0 {
            print!(".");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
    }

    let elapsed = start.elapsed();
    println!("\n[Producer] Sent {} messages in {:?}", ITERATIONS, elapsed);
    println!(
        "[Producer] Throughput: {:.2} msg/s",
        ITERATIONS as f64 / elapsed.as_secs_f64()
    );
}

fn run_consumer(topic: &str) {
    println!("[Consumer] Starting with topic: {}", topic);

    let node = NodeBuilder::new().create::<ipc::Service>().unwrap();
    let service_name = ServiceName::new(topic).unwrap();

    println!("[Consumer] Creating service...");
    let service = node
        .service_builder(&service_name)
        .publish_subscribe::<CmdVel>()
        // Buffer all messages to prevent drops
        .history_size(ITERATIONS * 2)
        .subscriber_max_buffer_size(ITERATIONS * 2)
        .open_or_create()
        .unwrap();

    println!("[Consumer] Creating subscriber...");
    let subscriber = service.subscriber_builder().create().unwrap();

    println!("[Consumer] Waiting for messages...");
    let start = Instant::now();
    let mut count = 0;
    let mut last_linear = 0.0;

    while count < ITERATIONS {
        match subscriber.receive() {
            Ok(Some(sample)) => {
                let payload = sample.payload();
                last_linear = payload.linear;
                count += 1;

                if count % 20 == 0 {
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                }
            }
            Ok(None) => {
                // No message available yet, continue waiting
                std::thread::sleep(Duration::from_micros(10));
            }
            Err(e) => {
                eprintln!("[Consumer] Receive error at {}: {:?}", count, e);
                return;
            }
        }

        // Safety timeout
        if start.elapsed() > Duration::from_secs(10) {
            eprintln!(
                "[Consumer] Timeout! Only received {}/{} messages",
                count, ITERATIONS
            );
            return;
        }
    }

    let elapsed = start.elapsed();
    println!("\n[Consumer] Received {} messages in {:?}", count, elapsed);
    println!("[Consumer] Last message linear velocity: {}", last_linear);
    println!(
        "[Consumer] Throughput: {:.2} msg/s",
        count as f64 / elapsed.as_secs_f64()
    );
}
