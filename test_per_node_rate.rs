// Test per-node rate control in Rust scheduler
use horus::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Shared counter to track ticks
#[derive(Clone)]
struct TickCounter {
    count: Arc<Mutex<u64>>,
}

impl TickCounter {
    fn new() -> Self {
        Self {
            count: Arc::new(Mutex::new(0)),
        }
    }

    fn increment(&self) {
        if let Ok(mut count) = self.count.lock() {
            *count += 1;
        }
    }

    fn get(&self) -> u64 {
        self.count.lock().map(|c| *c).unwrap_or(0)
    }
}

// Fast node (100Hz)
struct FastNode {
    counter: TickCounter,
}

impl Node for FastNode {
    fn name(&self) -> &str {
        "fast_node"
    }

    fn tick(&mut self, _info: Option<&NodeInfo>) {
        self.counter.increment();
    }
}

// Medium node (50Hz)
struct MediumNode {
    counter: TickCounter,
}

impl Node for MediumNode {
    fn name(&self) -> &str {
        "medium_node"
    }

    fn tick(&mut self, _info: Option<&NodeInfo>) {
        self.counter.increment();
    }
}

// Slow node (10Hz)
struct SlowNode {
    counter: TickCounter,
}

impl Node for SlowNode {
    fn name(&self) -> &str {
        "slow_node"
    }

    fn tick(&mut self, _info: Option<&NodeInfo>) {
        self.counter.increment();
    }
}

fn main() {
    println!("🧪 Testing Per-Node Rate Control");
    println!("{}", "=".repeat(60));

    // Create counters
    let fast_counter = TickCounter::new();
    let medium_counter = TickCounter::new();
    let slow_counter = TickCounter::new();

    // Create nodes
    let fast_node = FastNode {
        counter: fast_counter.clone(),
    };
    let medium_node = MediumNode {
        counter: medium_counter.clone(),
    };
    let slow_node = SlowNode {
        counter: slow_counter.clone(),
    };

    // Create scheduler
    let mut scheduler = Scheduler::new();

    println!("\n📋 Configuration:");
    println!("  - fast_node:   100 Hz (expected ~100 ticks/sec)");
    println!("  - medium_node:  50 Hz (expected ~50 ticks/sec)");
    println!("  - slow_node:    10 Hz (expected ~10 ticks/sec)");

    // Add nodes with different rates
    scheduler
        .add(Box::new(fast_node), 0, None)
        .set_node_rate("fast_node", 100.0)
        .add(Box::new(medium_node), 1, None)
        .set_node_rate("medium_node", 50.0)
        .add(Box::new(slow_node), 2, None)
        .set_node_rate("slow_node", 10.0);

    println!("\n⏱️  Running for 1 second...\n");

    // Run for 1 second
    if let Err(e) = scheduler.run_for(Duration::from_secs(1)) {
        eprintln!("❌ Scheduler error: {}", e);
        std::process::exit(1);
    }

    // Get results
    let fast_ticks = fast_counter.get();
    let medium_ticks = medium_counter.get();
    let slow_ticks = slow_counter.get();

    println!("\n{}", "=".repeat(60));
    println!("📊 Results after 1 second:");
    println!("{}", "-".repeat(60));
    println!("  fast_node:   {:3} ticks (expected ~100)", fast_ticks);
    println!("  medium_node: {:3} ticks (expected ~50)", medium_ticks);
    println!("  slow_node:   {:3} ticks (expected ~10)", slow_ticks);
    println!("{}", "=".repeat(60));

    // Validate results (allow 20% tolerance)
    let mut all_passed = true;

    // Fast node: expect ~100 ticks (80-120 acceptable)
    if fast_ticks >= 80 && fast_ticks <= 120 {
        println!("✅ fast_node rate: PASS (within 20% of target)");
    } else {
        println!("❌ fast_node rate: FAIL (outside acceptable range)");
        all_passed = false;
    }

    // Medium node: expect ~50 ticks (40-60 acceptable)
    if medium_ticks >= 40 && medium_ticks <= 60 {
        println!("✅ medium_node rate: PASS (within 20% of target)");
    } else {
        println!("❌ medium_node rate: FAIL (outside acceptable range)");
        all_passed = false;
    }

    // Slow node: expect ~10 ticks (8-12 acceptable)
    if slow_ticks >= 8 && slow_ticks <= 12 {
        println!("✅ slow_node rate: PASS (within 20% of target)");
    } else {
        println!("❌ slow_node rate: FAIL (outside acceptable range)");
        all_passed = false;
    }

    // Verify rate ratios
    let fast_medium_ratio = fast_ticks as f64 / medium_ticks as f64;
    let medium_slow_ratio = medium_ticks as f64 / slow_ticks as f64;

    println!("\n📐 Rate Ratios:");
    println!("  fast/medium: {:.2} (expected ~2.0)", fast_medium_ratio);
    println!("  medium/slow: {:.2} (expected ~5.0)", medium_slow_ratio);

    if fast_medium_ratio >= 1.6 && fast_medium_ratio <= 2.4 {
        println!("✅ fast/medium ratio: PASS");
    } else {
        println!("❌ fast/medium ratio: FAIL");
        all_passed = false;
    }

    if medium_slow_ratio >= 4.0 && medium_slow_ratio <= 6.0 {
        println!("✅ medium/slow ratio: PASS");
    } else {
        println!("❌ medium/slow ratio: FAIL");
        all_passed = false;
    }

    println!("\n{}", "=".repeat(60));
    if all_passed {
        println!("🎉 All tests PASSED!");
        println!("✅ Per-node rate control is working correctly!");
        std::process::exit(0);
    } else {
        println!("❌ Some tests FAILED!");
        println!("⚠️  Per-node rate control may have issues.");
        std::process::exit(1);
    }
}
