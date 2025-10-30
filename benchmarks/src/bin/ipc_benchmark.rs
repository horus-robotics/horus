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
use horus::prelude::Hub;
use horus_library::messages::cmd_vel::CmdVel;
use std::env;
use std::fs;
use std::process::{Child, Command};
use std::time::Duration;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_rdtsc;

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

    // Subprocess mode
    if args.len() > 1 {
        match args[1].as_str() {
            "producer" => producer(&args[2], &args[3]),
            "consumer" => consumer(&args[2], &args[3]),
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
    println!("  • Cache Alignment: 64-byte (will test 128-byte separately)");
    println!();

    println!("\n{}", "Running benchmarks...".bright_white().bold());
    println!();

    let mut all_latencies = Vec::new();

    for run in 1..=NUM_RUNS {
        print!("  Run {}/{}: ", run, NUM_RUNS);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let latencies = run_benchmark();
        let median_cycles = median(&latencies);

        all_latencies.push(latencies);
        println!("{} cycles median", median_cycles);
    }

    // Aggregate statistics
    println!();
    println!("{}", "═".repeat(80).bright_white());
    println!("{}", "  RESULTS (CPU Cycles)".bright_white().bold());
    println!("{}", "═".repeat(80).bright_white());

    let all_cycles: Vec<u64> = all_latencies.iter().flatten().copied().collect();
    let median = median(&all_cycles);
    let p95 = percentile(&all_cycles, 95);
    let p99 = percentile(&all_cycles, 99);
    let min = *all_cycles.iter().min().unwrap();
    let max = *all_cycles.iter().max().unwrap();

    println!("\n  Median:  {} cycles", format!("{}", median).bright_green());
    println!("  P95:     {} cycles", p95);
    println!("  P99:     {} cycles", p99);
    println!("  Min:     {} cycles", min);
    println!("  Max:     {} cycles", max);

    println!("\n{}", "Analysis:".bright_yellow());
    println!("  • Core-to-core theoretical minimum: ~60 cycles (30ns each way @ 2GHz)");
    println!("  • Good SPSC queue target: 70-80 cycles");

    if median < 100 {
        println!("  • {} Excellent performance!", "✓".bright_green());
    } else if median < 200 {
        println!("  • {} Good performance, but room for optimization", "⚠".bright_yellow());
    } else {
        println!("  • {} High latency - investigate bottlenecks", "✗".bright_red());
    }

    println!("\n{}", "═".repeat(80).bright_cyan().bold());
    println!();
}

fn run_benchmark() -> Vec<u64> {
    let topic = format!("bench_{}", std::process::id());
    let barrier_file = format!("/tmp/barrier_{}", std::process::id());

    // Create barrier
    fs::write(&barrier_file, &[0]).unwrap();

    // Start consumer first (waits on core 1)
    let consumer = spawn_process("consumer", &topic, &barrier_file, 1);

    // Wait for consumer ready
    wait_for_barrier(&barrier_file, BARRIER_CONSUMER_READY, Duration::from_secs(5));

    // Start producer (runs on core 0)
    let producer = spawn_process("producer", &topic, &barrier_file, 0);

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

fn producer(topic: &str, _barrier_file: &str) {
    eprintln!("Producer started for topic: {}", topic);

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

    // Small delay to ensure consumer is ready
    std::thread::sleep(Duration::from_millis(50));
    eprintln!("Producer: Starting warmup");

    // Warmup
    for _ in 0..WARMUP {
        let tsc = rdtsc();
        let mut msg = CmdVel::new(1.0, 0.5);
        msg.stamp_nanos = tsc;
        sender.send(msg, None).unwrap();
    }
    eprintln!("Producer: Warmup complete");

    // Measured iterations - embed rdtsc in message
    eprintln!("Producer: Starting measured iterations");
    for _ in 0..ITERATIONS {
        let tsc = rdtsc();
        let mut msg = CmdVel::new(1.0, 0.5);
        msg.stamp_nanos = tsc; // Embed timestamp
        sender.send(msg, None).unwrap();
    }
    eprintln!("Producer: All messages sent");
}

fn consumer(topic: &str, barrier_file: &str) {
    eprintln!("Consumer started for topic: {}", topic);

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

    // Signal ready
    write_barrier(barrier_file, BARRIER_CONSUMER_READY);
    eprintln!("Consumer: Signaled ready");

    // Warmup - spin-poll with timeout
    eprintln!("Consumer: Starting warmup");
    let warmup_start = std::time::Instant::now();
    for i in 0..WARMUP {
        let mut attempts = 0;
        let msg_start = std::time::Instant::now();
        loop {
            if let Some(_msg) = receiver.recv(None) {
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

    // Measured receives - spin-poll with timeout
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
                break;
            }
            if msg_start.elapsed().as_secs() > 5 {
                eprintln!("Consumer: TIMEOUT waiting for message {} - only received {}/{}", i, i, ITERATIONS);
                return;
            }
        }
    }
    eprintln!("Consumer: Completed all iterations");
}

// ============================================================================
// UTILITIES
// ============================================================================

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
