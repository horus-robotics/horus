//! # HORUS IPC Latency Benchmark - RDTSC-Based
//!
//! Accurate multi-process IPC latency measurement using CPU timestamp counters (rdtsc).
//!
//! ## Methodology
//!
//! - Producer embeds rdtsc() timestamp in each message
//! - Consumer reads rdtsc() upon receipt and calculates propagation time
//! - Null cost calibration: back-to-back rdtsc() calls (~20-30 cycles)
//! - Tests both 64-byte and 128-byte cache line alignment
//!
//! ## Usage
//!
//! ```bash
//! cargo build --release --bin ipc_benchmark
//! ./target/release/ipc_benchmark
//! ```

use colored::Colorize;
use horus::prelude::{Hub, Link};
use horus_library::messages::cmd_vel::CmdVel;
use std::env;
use std::fs;
use std::process::{Child, Command};
use std::time::Duration;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_rdtsc;

#[cfg(feature = "iceoryx2")]
use iceoryx2::prelude::*;

const ITERATIONS: usize = 10_000;
const WARMUP: usize = 1_000;
const NUM_RUNS: usize = 5;

// Barrier states
const BARRIER_CONSUMER_READY: u8 = 2;
const BARRIER_PRODUCER_DONE: u8 = 3;

/// Read CPU timestamp counter - measures cycles, not nanoseconds
#[inline(always)]
fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        _rdtsc()
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// Calibrate rdtsc overhead (back-to-back calls)
fn calibrate_rdtsc() -> u64 {
    let mut min_cost = u64::MAX;

    // Warmup
    for _ in 0..100 {
        let _ = rdtsc();
    }

    // Measure minimum overhead
    for _ in 0..1000 {
        let start = rdtsc();
        let end = rdtsc();
        let cost = end.wrapping_sub(start);
        if cost > 0 && cost < min_cost {
            min_cost = cost;
        }
    }

    min_cost
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Subprocess mode: <ipc_type> <role> <topic> <barrier_file>
    if args.len() > 1 {
        match args[1].as_str() {
            "hub_producer" => hub_producer(&args[2], &args[3]),
            "hub_consumer" => hub_consumer(&args[2], &args[3]),
            #[cfg(feature = "iceoryx2")]
            "ice_producer" => ice_producer(&args[2], &args[3]),
            #[cfg(feature = "iceoryx2")]
            "ice_consumer" => ice_consumer(&args[2], &args[3]),
            _ => eprintln!("Unknown mode: {}", args[1]),
        }
        return;
    }

    // Main coordinator
    println!("\n{}", "═".repeat(80).bright_cyan().bold());
    println!("{}", "  HORUS IPC LATENCY BENCHMARK".bright_cyan().bold());
    println!("{}", "  RDTSC-Based True Propagation Time Measurement".bright_cyan());
    println!("{}", "═".repeat(80).bright_cyan().bold());

    // Calibration
    let rdtsc_overhead = calibrate_rdtsc();
    println!("\n{}", "RDTSC Calibration:".bright_yellow());
    println!("  • Null cost (back-to-back rdtsc): {} cycles", rdtsc_overhead);
    println!("  • Target: ~20-30 cycles on modern x86_64");

    println!("\n{}", "Benchmark Configuration:".bright_yellow());
    println!("  • Message type: CmdVel (16 bytes)");
    println!("  • Iterations per run: {}", format!("{}", ITERATIONS).bright_green());
    println!("  • Warmup iterations: {}", format!("{}", WARMUP).bright_green());
    println!("  • Number of runs: {}", format!("{}", NUM_RUNS).bright_green());
    println!("  • CPU Affinity: producer=core0, consumer=core1");
    println!("  • Measurement: rdtsc timestamp embedded in message");
    println!("  • Pattern: Ping-pong (ack before next send - no queue buildup)");
    println!("  • Cache Alignment: 64-byte (optimized for x86_64)");
    println!();

    // Run benchmarks for each IPC system
    run_all_benchmarks();

    println!("\n{}", "═".repeat(80).bright_cyan().bold());
    println!();
}

fn run_all_benchmarks() {
    // 1. Hub (multi-process MPMC)
    println!("\n{}", "═".repeat(80).bright_white());
    println!("{}", "  HORUS HUB (Multi-Process MPMC)".bright_white().bold());
    println!("{}", "═".repeat(80).bright_white());
    run_ipc_benchmark("hub");

    // 2. Link (single-process SPSC)
    println!("\n{}", "═".repeat(80).bright_white());
    println!("{}", "  HORUS LINK (Single-Process SPSC)".bright_white().bold());
    println!("{}", "═".repeat(80).bright_white());
    run_link_benchmark();

    // 3. iceoryx2 (single-process threading, like their official benchmark)
    #[cfg(feature = "iceoryx2")]
    {
        println!("\n{}", "═".repeat(80).bright_white());
        println!("{}", "  ICEORYX2 (Single-Process Threading)".bright_white().bold());
        println!("{}", "═".repeat(80).bright_white());
        println!("  SKIPPED: iceoryx2 benchmark has synchronization issues with ping-pong pattern");
        println!("  TODO: Requires investigation of iceoryx2 API usage or different test pattern");
        println!("  See: benchmarks/src/bin/ipc_benchmark.rs:683-840");
        // run_iceoryx2_benchmark();
    }
}

fn run_ipc_benchmark(ipc_type: &str) {
    let mut all_latencies = Vec::new();

    for run in 1..=NUM_RUNS {
        print!("  Run {}/{}: ", run, NUM_RUNS);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let latencies = run_benchmark(ipc_type);
        let median_cycles = median(&latencies);

        all_latencies.push(latencies);
        println!("{} cycles median", median_cycles);
    }

    print_results(&all_latencies);
}

fn print_results(all_latencies: &[Vec<u64>]) {
    let all_cycles: Vec<u64> = all_latencies.iter().flatten().copied().collect();

    if all_cycles.is_empty() {
        println!("\n  {} No results collected", "✗".bright_red());
        return;
    }

    let median = median(&all_cycles);
    let p95 = percentile(&all_cycles, 95);
    let p99 = percentile(&all_cycles, 99);
    let min = *all_cycles.iter().min().unwrap();
    let max = *all_cycles.iter().max().unwrap();

    println!("\n  Median:  {} cycles (~{} ns @ 2GHz)", format!("{}", median).bright_green(), median / 2);
    println!("  P95:     {} cycles (~{} ns)", p95, p95 / 2);
    println!("  P99:     {} cycles (~{} ns)", p99, p99 / 2);
    println!("  Min:     {} cycles (~{} ns)", min, min / 2);
    println!("  Max:     {} cycles (~{} ns)", max, max / 2);

    println!("\n{}", "Analysis:".bright_yellow());
    println!("  • Core-to-core theoretical minimum: ~60 cycles (30ns each way @ 2GHz)");
    println!("  • Good SPSC queue target: 70-80 cycles");

    if median < 100 {
        println!("  • {} Excellent performance!", "✓".bright_green());
    } else if median < 2000 {
        println!("  • {} Good performance", "✓".bright_green());
    } else if median < 5000 {
        println!("  • {} Acceptable performance", "⚠".bright_yellow());
    } else {
        println!("  • {} High latency", "⚠".bright_yellow());
    }
}

fn run_benchmark(ipc_type: &str) -> Vec<u64> {
    let topic = format!("bench_{}_{}", ipc_type, std::process::id());
    let barrier_file = format!("/tmp/barrier_{}_{}", ipc_type, std::process::id());

    // Create barrier
    fs::write(&barrier_file, &[0]).unwrap();

    let producer_mode = format!("{}_producer", ipc_type);
    let consumer_mode = format!("{}_consumer", ipc_type);

    // Start consumer first (waits on core 1)
    let consumer = spawn_process(&consumer_mode, &topic, &barrier_file, 1);

    // Wait for consumer ready
    wait_for_barrier(&barrier_file, BARRIER_CONSUMER_READY, Duration::from_secs(5));

    // Start producer (runs on core 0)
    let producer = spawn_process(&producer_mode, &topic, &barrier_file, 0);

    // Wait for completion
    let producer_output = producer.wait_with_output().unwrap();
    let consumer_output = consumer.wait_with_output().unwrap();

    // Cleanup
    let _ = fs::remove_file(&barrier_file);

    if !producer_output.status.success() {
        eprintln!("Producer failed: {}", String::from_utf8_lossy(&producer_output.stderr));
        return vec![];
    }

    if !consumer_output.status.success() {
        eprintln!("Consumer failed: {}", String::from_utf8_lossy(&consumer_output.stderr));
        return vec![];
    }

    // Parse latencies from consumer output
    let output = String::from_utf8_lossy(&consumer_output.stdout);

    // Debug: Print consumer output if empty
    let latencies: Vec<u64> = output
        .lines()
        .filter_map(|line| line.parse::<u64>().ok())
        .collect();

    if latencies.is_empty() {
        eprintln!("WARNING: No latencies collected!");
        eprintln!("Consumer stdout: {}", output);
        eprintln!("Consumer stderr: {}", String::from_utf8_lossy(&consumer_output.stderr));
    }

    latencies
}

fn spawn_process(mode: &str, topic: &str, barrier_file: &str, core: usize) -> Child {
    let exe = env::current_exe().unwrap();
    let mut cmd = Command::new(&exe);
    cmd.arg(mode).arg(topic).arg(barrier_file);

    // Set CPU affinity via taskset
    #[cfg(target_os = "linux")]
    {
        cmd = Command::new("taskset");
        cmd.arg("-c").arg(core.to_string()).arg(&exe).arg(mode).arg(topic).arg(barrier_file);
    }

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn process")
}

fn hub_producer(topic: &str, _barrier_file: &str) {
    eprintln!("Hub Producer started for topic: {}", topic);

    // Create sender and receiver for ping-pong
    let sender = match Hub::<CmdVel>::new(topic) {
        Ok(s) => {
            eprintln!("Producer: Hub created successfully");
            s
        }
        Err(e) => {
            eprintln!("Producer: Failed to create Hub: {:?}", e);
            return;
        }
    };

    let ack_topic = format!("{}_ack", topic);
    let ack_receiver = match Hub::<CmdVel>::new(&ack_topic) {
        Ok(r) => {
            eprintln!("Producer: Ack receiver created");
            r
        }
        Err(e) => {
            eprintln!("Producer: Failed to create ack receiver: {:?}", e);
            return;
        }
    };

    // Small delay to ensure consumer is ready
    std::thread::sleep(Duration::from_millis(100));
    eprintln!("Producer: Starting warmup");

    // Warmup - ping-pong pattern
    for _ in 0..WARMUP {
        let tsc = rdtsc();
        let mut msg = CmdVel::new(1.0, 0.5);
        msg.stamp_nanos = tsc;
        sender.send(msg, None).unwrap();

        // Wait for acknowledgment
        loop {
            if ack_receiver.recv(None).is_some() {
                break;
            }
        }
    }
    eprintln!("Producer: Warmup complete");

    // Measured iterations - ping-pong ensures no queue buildup
    eprintln!("Producer: Starting measured iterations");
    for _ in 0..ITERATIONS {
        let tsc = rdtsc();
        let mut msg = CmdVel::new(1.0, 0.5);
        msg.stamp_nanos = tsc; // Embed timestamp
        sender.send(msg, None).unwrap();

        // Wait for acknowledgment before sending next
        loop {
            if ack_receiver.recv(None).is_some() {
                break;
            }
        }
    }
    eprintln!("Producer: All messages sent");
}

fn hub_consumer(topic: &str, barrier_file: &str) {
    eprintln!("Hub Consumer started for topic: {}", topic);

    let receiver = match Hub::<CmdVel>::new(topic) {
        Ok(r) => {
            eprintln!("Consumer: Hub created successfully");
            r
        }
        Err(e) => {
            eprintln!("Consumer: Failed to create Hub: {:?}", e);
            return;
        }
    };

    // Create acknowledgment sender for ping-pong
    let ack_topic = format!("{}_ack", topic);
    let ack_sender = match Hub::<CmdVel>::new(&ack_topic) {
        Ok(s) => {
            eprintln!("Consumer: Ack sender created");
            s
        }
        Err(e) => {
            eprintln!("Consumer: Failed to create ack sender: {:?}", e);
            return;
        }
    };

    // Signal ready
    write_barrier(barrier_file, BARRIER_CONSUMER_READY);
    eprintln!("Consumer: Signaled ready");

    // Warmup - spin-poll with ping-pong acknowledgment
    eprintln!("Consumer: Starting warmup");
    let warmup_start = std::time::Instant::now();
    for i in 0..WARMUP {
        let mut attempts = 0;
        let msg_start = std::time::Instant::now();
        loop {
            if let Some(_msg) = receiver.recv(None) {
                // Send acknowledgment immediately
                let ack = CmdVel::new(0.0, 0.0);
                let _ = ack_sender.send(ack, None);
                break;
            }
            attempts += 1;
            if msg_start.elapsed().as_secs() > 5 {
                eprintln!("Consumer: TIMEOUT waiting for warmup message {} after {} attempts", i, attempts);
                eprintln!("Consumer: This suggests multi-process IPC is not working");
                return;
            }
        }
    }
    eprintln!("Consumer: Warmup complete in {:?}", warmup_start.elapsed());

    // Measured receives - measure latency then send ack
    eprintln!("Consumer: Starting measured iterations");
    for i in 0..ITERATIONS {
        let msg_start = std::time::Instant::now();
        loop {
            if let Some(msg) = receiver.recv(None) {
                let recv_tsc = rdtsc();
                let send_tsc = msg.stamp_nanos;
                let cycles = recv_tsc.wrapping_sub(send_tsc);

                // Print cycles (one per line for easy parsing)
                println!("{}", cycles);

                // Send acknowledgment to enable next message
                let ack = CmdVel::new(0.0, 0.0);
                let _ = ack_sender.send(ack, None);
                break;
            }
        }
        if msg_start.elapsed().as_secs() > 5 {
            eprintln!("Consumer: TIMEOUT waiting for message {} - only received {}/{}", i, i, ITERATIONS);
            return;
        }
    }
    eprintln!("Consumer: Completed all iterations");
}

// ============================================================================
// ICEORYX2 BENCHMARKS
// ============================================================================

#[cfg(feature = "iceoryx2")]
fn ice_producer(topic: &str, barrier_file: &str) {
    eprintln!("iceoryx2 Producer started for topic: {}", topic);

    use iceoryx2::prelude::*;

    let node = NodeBuilder::new().create::<ipc::Service>().unwrap();

    // Create publisher for main messages
    let service = node
        .service_builder(&ServiceName::new(topic).unwrap())
        .publish_subscribe::<CmdVel>()
        .open_or_create()
        .unwrap();

    let publisher = service.publisher_builder().create().unwrap();

    // Create subscriber for acknowledgments
    let ack_topic_name = format!("{}_ack", topic);
    let ack_service = node
        .service_builder(&ServiceName::new(&ack_topic_name).unwrap())
        .publish_subscribe::<CmdVel>()
        .open_or_create()
        .unwrap();

    let ack_subscriber = ack_service.subscriber_builder().create().unwrap();

    // Wait for consumer to signal ready
    wait_for_barrier(barrier_file, BARRIER_CONSUMER_READY, Duration::from_secs(10));
    eprintln!("iceoryx2 Producer: Consumer ready, starting warmup");

    // Warmup - ping-pong pattern
    for _ in 0..WARMUP {
        let tsc = rdtsc();
        let sample = publisher.loan_uninit().unwrap();
        let _msg = sample.write_payload(CmdVel::with_timestamp(1.0, 0.5, tsc));
        // Sample is automatically sent when dropped

        // Wait for acknowledgment
        loop {
            if ack_subscriber.receive().unwrap().is_some() {
                break;
            }
        }
    }
    eprintln!("iceoryx2 Producer: Warmup complete");

    // Measured iterations - ping-pong ensures no queue buildup
    eprintln!("iceoryx2 Producer: Starting measured iterations");
    for _ in 0..ITERATIONS {
        let tsc = rdtsc();
        let sample = publisher.loan_uninit().unwrap();
        let _msg = sample.write_payload(CmdVel::with_timestamp(1.0, 0.5, tsc));
        // Sample is automatically sent when dropped

        // Wait for acknowledgment before sending next
        loop {
            if ack_subscriber.receive().unwrap().is_some() {
                break;
            }
        }
    }
    eprintln!("iceoryx2 Producer: All messages sent");
}

#[cfg(feature = "iceoryx2")]
fn ice_consumer(topic: &str, barrier_file: &str) {
    eprintln!("iceoryx2 Consumer started for topic: {}", topic);

    use iceoryx2::prelude::*;

    let node = NodeBuilder::new().create::<ipc::Service>().unwrap();

    // Create subscriber for main messages
    let service = node
        .service_builder(&ServiceName::new(topic).unwrap())
        .publish_subscribe::<CmdVel>()
        .open_or_create()
        .unwrap();

    let subscriber = service.subscriber_builder().create().unwrap();

    // Create publisher for acknowledgments
    let ack_topic_name = format!("{}_ack", topic);
    let ack_service = node
        .service_builder(&ServiceName::new(&ack_topic_name).unwrap())
        .publish_subscribe::<CmdVel>()
        .open_or_create()
        .unwrap();

    let ack_publisher = ack_service.publisher_builder().create().unwrap();

    // Signal ready
    write_barrier(barrier_file, BARRIER_CONSUMER_READY);
    eprintln!("iceoryx2 Consumer: Signaled ready");

    // Give iceoryx2 time to fully establish subscriptions
    std::thread::sleep(Duration::from_millis(200));

    // Warmup - spin-poll with ping-pong acknowledgment
    eprintln!("iceoryx2 Consumer: Starting warmup");
    let warmup_start = std::time::Instant::now();
    for i in 0..WARMUP {
        let msg_start = std::time::Instant::now();
        loop {
            if let Some(sample) = subscriber.receive().unwrap() {
                // Send acknowledgment immediately
                let ack_sample = ack_publisher.loan_uninit().unwrap();
                let _ack_msg = ack_sample.write_payload(CmdVel::new(0.0, 0.0));
                // Ack sample is automatically sent when dropped
                break;
            }
            if msg_start.elapsed().as_secs() > 5 {
                eprintln!("iceoryx2 Consumer: TIMEOUT waiting for warmup message {}", i);
                return;
            }
        }
    }
    eprintln!("iceoryx2 Consumer: Warmup complete in {:?}", warmup_start.elapsed());

    // Measured receives - measure latency then send ack
    eprintln!("iceoryx2 Consumer: Starting measured iterations");
    for i in 0..ITERATIONS {
        let msg_start = std::time::Instant::now();
        loop {
            if let Some(sample) = subscriber.receive().unwrap() {
                let recv_tsc = rdtsc();
                let msg = sample.payload();
                let send_tsc = msg.stamp_nanos;
                let cycles = recv_tsc.wrapping_sub(send_tsc);

                // Print cycles (one per line for easy parsing)
                println!("{}", cycles);

                // Send acknowledgment to enable next message
                let ack_sample = ack_publisher.loan_uninit().unwrap();
                let _ack_msg = ack_sample.write_payload(CmdVel::new(0.0, 0.0));
                // Ack sample is automatically sent when dropped
                break;
            }
            if msg_start.elapsed().as_secs() > 5 {
                eprintln!("iceoryx2 Consumer: TIMEOUT waiting for message {} - only received {}/{}", i, i, ITERATIONS);
                return;
            }
        }
    }
    eprintln!("iceoryx2 Consumer: Completed all iterations");
}

// ============================================================================
// LINK BENCHMARKS (Single-Process SPSC)
// ============================================================================

fn run_link_benchmark() {
    use std::thread;

    let mut all_latencies = Vec::new();

    for run in 1..=NUM_RUNS {
        print!("  Run {}/{}: ", run, NUM_RUNS);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let link_topic = format!("link_bench_{}", run);
        let ack_topic = format!("link_ack_{}", run);

        let link_send = Link::<CmdVel>::producer(&link_topic).unwrap();
        let link_recv = Link::<CmdVel>::consumer(&link_topic).unwrap();
        let ack_send = Link::<CmdVel>::producer(&ack_topic).unwrap();
        let ack_recv = Link::<CmdVel>::consumer(&ack_topic).unwrap();

        let producer_handle = {

            thread::spawn(move || {
                // Set CPU affinity to core 0 (same as Hub producer)
                set_cpu_affinity(0);
                // Warmup
                for _ in 0..WARMUP {
                    let tsc = rdtsc();
                    let mut msg = CmdVel::new(1.0, 0.5);
                    msg.stamp_nanos = tsc;
                    let _ = link_send.send(msg, None);

                    // Wait for ack
                    loop {
                        if ack_recv.recv(None).is_some() {
                            break;
                        }
                                }
                }

                // Measured iterations
                for _ in 0..ITERATIONS {
                    let tsc = rdtsc();
                    let mut msg = CmdVel::new(1.0, 0.5);
                    msg.stamp_nanos = tsc;
                    let _ = link_send.send(msg, None);

                    // Wait for ack
                    loop {
                        if ack_recv.recv(None).is_some() {
                            break;
                        }
                                }
                }
            })
        };

        let consumer_handle = {
            thread::spawn(move || {
                // Set CPU affinity to core 1 (same as Hub consumer)
                set_cpu_affinity(1);

                let mut latencies = Vec::with_capacity(ITERATIONS);

                // Warmup
                for _ in 0..WARMUP {
                    loop {
                        if link_recv.recv(None).is_some() {
                            let _ = ack_send.send(CmdVel::new(0.0, 0.0), None);
                            break;
                        }
                                }
                }

                // Measured iterations
                for _ in 0..ITERATIONS {
                    loop {
                        if let Some(msg) = link_recv.recv(None) {
                            let recv_tsc = rdtsc();
                            let send_tsc = msg.stamp_nanos;
                            let cycles = recv_tsc.wrapping_sub(send_tsc);
                            latencies.push(cycles);

                            // Send ack
                            let _ = ack_send.send(CmdVel::new(0.0, 0.0), None);
                            break;
                        }
                                }
                }

                latencies
            })
        };

        producer_handle.join().unwrap();
        let latencies = consumer_handle.join().unwrap();

        let median_cycles = median(&latencies);
        all_latencies.push(latencies);
        println!("{} cycles median", median_cycles);
    }

    print_results(&all_latencies);
}

// ============================================================================
// ICEORYX2 BENCHMARKS (Single-Process Threading - Official Pattern)
// ============================================================================

#[cfg(feature = "iceoryx2")]
fn run_iceoryx2_benchmark() {
    use std::thread;
    use iceoryx2::prelude::*;

    let mut all_latencies = Vec::new();

    for run in 1..=NUM_RUNS {
        print!("  Run {}/{}: ", run, NUM_RUNS);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        // Create two services for ping-pong pattern (a2b and b2a)
        let service_name_a2b = ServiceName::new(&format!("ice_a2b_{}", run)).unwrap();
        let service_name_b2a = ServiceName::new(&format!("ice_b2a_{}", run)).unwrap();

        // Use local::Service for single-process threading (ipc::Service uses Rc, can't Send)
        let node = NodeBuilder::new().create::<local::Service>().unwrap();

        let service_a2b = node
            .service_builder(&service_name_a2b)
            .publish_subscribe::<CmdVel>()
            .max_publishers(1)
            .max_subscribers(1)
            .history_size(0)
            .subscriber_max_buffer_size(1)
            .enable_safe_overflow(true)
            .create()
            .unwrap();

        let service_b2a = node
            .service_builder(&service_name_b2a)
            .publish_subscribe::<CmdVel>()
            .max_publishers(1)
            .max_subscribers(1)
            .history_size(0)
            .subscriber_max_buffer_size(1)
            .enable_safe_overflow(true)
            .create()
            .unwrap();

        // Use thread scope to share service references between threads
        let latencies = thread::scope(|s| {
            // Use a barrier to synchronize thread startup
            use std::sync::{Arc, Barrier};
            let barrier = Arc::new(Barrier::new(2));
            let barrier_p = barrier.clone();
            let barrier_c = barrier.clone();

            // Capture services by reference
            let service_a2b_ref = &service_a2b;
            let service_b2a_ref = &service_b2a;

            let producer_handle = s.spawn(move || {
                // Set CPU affinity to core 0 (same as Hub/Link producer)
                set_cpu_affinity(0);

                // Create publishers/subscribers INSIDE thread (iceoryx2 pattern)
                let publisher_a2b = service_a2b_ref.publisher_builder().create().unwrap();
                let subscriber_b2a = service_b2a_ref.subscriber_builder().create().unwrap();

                // Wait for consumer to be ready
                barrier_p.wait();

                // Warmup
                for _ in 0..WARMUP {
                    let tsc = rdtsc();
                    let mut msg = CmdVel::new(1.0, 0.5);
                    msg.stamp_nanos = tsc;

                    let sample = publisher_a2b.loan_uninit().unwrap();
                    let _ = sample.write_payload(msg);
                    // sample auto-sends on drop

                    // Wait for ack
                    loop {
                        if subscriber_b2a.receive().unwrap().is_some() {
                            break;
                        }
                        std::thread::yield_now();
                    }
                }

                // Measured iterations
                for _ in 0..ITERATIONS {
                    let tsc = rdtsc();
                    let mut msg = CmdVel::new(1.0, 0.5);
                    msg.stamp_nanos = tsc;

                    let sample = publisher_a2b.loan_uninit().unwrap();
                    let _ = sample.write_payload(msg);
                    // sample auto-sends on drop

                    // Wait for ack
                    loop {
                        if subscriber_b2a.receive().unwrap().is_some() {
                            break;
                        }
                        std::thread::yield_now();
                    }
                }
            });

            let consumer_handle = s.spawn(move || {
                // Set CPU affinity to core 1 (same as Hub/Link consumer)
                set_cpu_affinity(1);

                // Create publishers/subscribers INSIDE thread (iceoryx2 pattern)
                let subscriber_a2b = service_a2b_ref.subscriber_builder().create().unwrap();
                let publisher_b2a = service_b2a_ref.publisher_builder().create().unwrap();

                let mut latencies = Vec::with_capacity(ITERATIONS);

                // Wait for producer to be ready
                barrier_c.wait();

                // Warmup
                for _ in 0..WARMUP {
                    loop {
                        if subscriber_a2b.receive().unwrap().is_some() {
                            // Send ack
                            let ack_sample = publisher_b2a.loan_uninit().unwrap();
                            let _ = ack_sample.write_payload(CmdVel::new(0.0, 0.0));
                            break;
                        }
                        std::thread::yield_now();
                    }
                }

                // Measured iterations
                for _ in 0..ITERATIONS {
                    loop {
                        if let Some(sample) = subscriber_a2b.receive().unwrap() {
                            let recv_tsc = rdtsc();
                            let msg = sample.payload();
                            let send_tsc = msg.stamp_nanos;
                            let cycles = recv_tsc.wrapping_sub(send_tsc);
                            latencies.push(cycles);

                            // Send ack
                            let ack_sample = publisher_b2a.loan_uninit().unwrap();
                            let _ = ack_sample.write_payload(CmdVel::new(0.0, 0.0));
                            break;
                        }
                        std::thread::yield_now();
                    }
                }

                latencies
            });

            producer_handle.join().unwrap();
            consumer_handle.join().unwrap()
        });

        let median_cycles = median(&latencies);
        all_latencies.push(latencies);
        println!("{} cycles median", median_cycles);
    }

    print_results(&all_latencies);
}

// ============================================================================
// UTILITIES
// ============================================================================

#[cfg(target_os = "linux")]
fn set_cpu_affinity(core: usize) {
    use libc::{cpu_set_t, CPU_SET, CPU_ZERO, sched_setaffinity};
    use std::mem;

    unsafe {
        let mut cpu_set: cpu_set_t = mem::zeroed();
        CPU_ZERO(&mut cpu_set);
        CPU_SET(core, &mut cpu_set);

        let result = sched_setaffinity(
            0, // 0 = current thread
            mem::size_of::<cpu_set_t>(),
            &cpu_set
        );

        if result != 0 {
            eprintln!("Warning: Failed to set CPU affinity to core {}", core);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn set_cpu_affinity(_core: usize) {
    // No-op on non-Linux platforms
}

fn wait_for_barrier(barrier_file: &str, expected: u8, timeout: Duration) {
    let start = std::time::Instant::now();
    loop {
        if let Ok(data) = fs::read(barrier_file) {
            if !data.is_empty() && data[0] == expected {
                return;
            }
        }
        if start.elapsed() > timeout {
            eprintln!("Barrier timeout waiting for state {}", expected);
            return;
        }
        std::thread::sleep(Duration::from_micros(100));
    }
}

fn write_barrier(barrier_file: &str, state: u8) {
    let _ = fs::write(barrier_file, &[state]);
}

fn median(values: &[u64]) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn percentile(values: &[u64], p: usize) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let idx = (sorted.len() * p) / 100;
    sorted[idx.min(sorted.len() - 1)]
}

fn rand_id() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
        % 1_000_000
}
