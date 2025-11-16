//! Link SPSC Performance Benchmarks
//!
//! Comprehensive benchmarks for the Link (Single Producer Single Consumer) IPC mechanism.
//! Tests latency, throughput, and compares with Hub to verify the claimed 2-4x speedup.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use horus::prelude::{Hub, Link};
use horus_library::messages::{
    cmd_vel::CmdVel,
    sensor::{Imu, LaserScan},
};

/// Small message (16 bytes) - CmdVel
/// Target: <100ns for Link vs ~296ns for Hub
fn bench_link_small_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("link_small_16B");
    group.throughput(Throughput::Bytes(std::mem::size_of::<CmdVel>() as u64));

    // Link: send + recv
    group.bench_function("Link::send_recv", |b| {
        let topic = format!("bench_link_small_{}", std::process::id());
        let producer: Link<CmdVel> = Link::producer(&topic).unwrap();
        let consumer: Link<CmdVel> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            let msg = CmdVel::new(1.5, 0.8);
            producer.send(black_box(msg), &mut None).unwrap();
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    // Hub: send + recv (for comparison)
    group.bench_function("Hub::send_recv", |b| {
        let topic = format!("bench_hub_small_{}", std::process::id());
        let sender: Hub<CmdVel> = Hub::new(&topic).unwrap();
        let receiver: Hub<CmdVel> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let msg = CmdVel::new(1.5, 0.8);
            sender.send(black_box(msg), &mut None).unwrap();
            let _ = black_box(receiver.recv(&mut None));
        });
    });

    // Link: send only (consumer exists but doesn't recv to isolate send performance)
    group.bench_function("Link::send_only", |b| {
        let topic = format!("bench_link_send_{}", std::process::id());
        let producer: Link<CmdVel> = Link::producer(&topic).unwrap();
        let _consumer: Link<CmdVel> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            let msg = CmdVel::new(1.5, 0.8);
            // Ignore error if buffer is full
            let _ = producer.send(black_box(msg), &mut None);
        });
    });

    // Link: recv only
    group.bench_function("Link::recv_only", |b| {
        let topic = format!("bench_link_recv_{}", std::process::id());
        let producer: Link<CmdVel> = Link::producer(&topic).unwrap();
        let consumer: Link<CmdVel> = Link::consumer(&topic).unwrap();

        // Pre-fill with messages
        for _ in 0..1000 {
            producer.send(CmdVel::new(1.5, 0.8), None).unwrap();
        }

        b.iter(|| {
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    group.finish();
}

/// Medium message (~304 bytes) - IMU data
fn bench_link_medium_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("link_medium_304B");
    group.throughput(Throughput::Bytes(std::mem::size_of::<Imu>() as u64));

    // Link: send + recv
    group.bench_function("Link::send_recv", |b| {
        let topic = format!("bench_link_imu_{}", std::process::id());
        let producer: Link<Imu> = Link::producer(&topic).unwrap();
        let consumer: Link<Imu> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            let mut imu = Imu::new();
            imu.set_orientation_from_euler(0.1, 0.2, 0.3);
            producer.send(black_box(imu), &mut None).unwrap();
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    // Hub: send + recv (for comparison)
    group.bench_function("Hub::send_recv", |b| {
        let topic = format!("bench_hub_imu_{}", std::process::id());
        let sender: Hub<Imu> = Hub::new(&topic).unwrap();
        let receiver: Hub<Imu> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let mut imu = Imu::new();
            imu.set_orientation_from_euler(0.1, 0.2, 0.3);
            sender.send(black_box(imu), &mut None).unwrap();
            let _ = black_box(receiver.recv(&mut None));
        });
    });

    group.finish();
}

/// Large message (~1.5KB) - LaserScan
fn bench_link_large_message(c: &mut Criterion) {
    let mut group = c.benchmark_group("link_large_1.5KB");
    group.throughput(Throughput::Bytes(std::mem::size_of::<LaserScan>() as u64));

    // Link: send + recv
    group.bench_function("Link::send_recv", |b| {
        let topic = format!("bench_link_laser_{}", std::process::id());
        let producer: Link<LaserScan> = Link::producer(&topic).unwrap();
        let consumer: Link<LaserScan> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            let mut scan = LaserScan::new();
            for i in 0..360 {
                scan.ranges[i] = 5.0 + (i as f32 * 0.01);
            }
            producer.send(black_box(scan), &mut None).unwrap();
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    // Hub: send + recv (for comparison)
    group.bench_function("Hub::send_recv", |b| {
        let topic = format!("bench_hub_laser_{}", std::process::id());
        let sender: Hub<LaserScan> = Hub::new(&topic).unwrap();
        let receiver: Hub<LaserScan> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let mut scan = LaserScan::new();
            for i in 0..360 {
                scan.ranges[i] = 5.0 + (i as f32 * 0.01);
            }
            sender.send(black_box(scan), &mut None).unwrap();
            let _ = black_box(receiver.recv(&mut None));
        });
    });

    group.finish();
}

/// Zero-copy loan() API benchmark
fn bench_link_zero_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("link_zero_copy");
    group.throughput(Throughput::Bytes(std::mem::size_of::<CmdVel>() as u64));

    // Standard send (with clone)
    group.bench_function("Link::send_with_clone", |b| {
        let topic = format!("bench_link_clone_{}", std::process::id());
        let producer: Link<CmdVel> = Link::producer(&topic).unwrap();
        let _consumer: Link<CmdVel> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            let msg = CmdVel::new(1.5, 0.8);
            let _ = producer.send(black_box(msg), &mut None);
        });
    });

    // Zero-copy loan API
    group.bench_function("Link::loan_zero_copy", |b| {
        let topic = format!("bench_link_loan_{}", std::process::id());
        let producer: Link<CmdVel> = Link::producer(&topic).unwrap();
        let _consumer: Link<CmdVel> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            if let Ok(sample) = producer.loan() {
                sample.write(black_box(CmdVel::new(1.5, 0.8)));
            }
        });
    });

    group.finish();
}

/// Throughput test: how many messages per second
fn bench_link_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("link_throughput");

    // Link throughput
    group.bench_function("Link::messages_per_sec", |b| {
        let topic = format!("bench_link_throughput_{}", std::process::id());
        let producer: Link<f32> = Link::producer(&topic).unwrap();
        let consumer: Link<f32> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            // Send 1000 messages
            for i in 0..1000 {
                producer.send(black_box(i as f32), &mut None).unwrap();
            }
            // Receive 1000 messages
            for _ in 0..1000 {
                let _ = black_box(consumer.recv(&mut None));
            }
        });
    });

    // Hub throughput (for comparison)
    group.bench_function("Hub::messages_per_sec", |b| {
        let topic = format!("bench_hub_throughput_{}", std::process::id());
        let sender: Hub<f32> = Hub::new(&topic).unwrap();
        let receiver: Hub<f32> = Hub::new(&topic).unwrap();

        b.iter(|| {
            // Send 1000 messages
            for i in 0..1000 {
                sender.send(black_box(i as f32), &mut None).unwrap();
            }
            // Receive 1000 messages
            for _ in 0..1000 {
                let _ = black_box(receiver.recv(&mut None));
            }
        });
    });

    group.finish();
}

/// Primitive types benchmark
fn bench_link_primitives(c: &mut Criterion) {
    let mut group = c.benchmark_group("link_primitives");

    // u8 (1 byte)
    group.bench_function("Link::u8_1B", |b| {
        let topic = format!("bench_link_u8_{}", std::process::id());
        let producer: Link<u8> = Link::producer(&topic).unwrap();
        let consumer: Link<u8> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            producer.send(black_box(42u8), &mut None).unwrap();
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    // f32 (4 bytes)
    group.bench_function("Link::f32_4B", |b| {
        let topic = format!("bench_link_f32_{}", std::process::id());
        let producer: Link<f32> = Link::producer(&topic).unwrap();
        let consumer: Link<f32> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            producer.send(black_box(3.14f32), &mut None).unwrap();
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    // f64 (8 bytes)
    group.bench_function("Link::f64_8B", |b| {
        let topic = format!("bench_link_f64_{}", std::process::id());
        let producer: Link<f64> = Link::producer(&topic).unwrap();
        let consumer: Link<f64> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            producer.send(black_box(3.14159265359f64), &mut None).unwrap();
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    // [f32; 3] (12 bytes - typical Vec3)
    group.bench_function("Link::vec3_12B", |b| {
        let topic = format!("bench_link_vec3_{}", std::process::id());
        let producer: Link<[f32; 3]> = Link::producer(&topic).unwrap();
        let consumer: Link<[f32; 3]> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            producer.send(black_box([1.0, 2.0, 3.0]), None).unwrap();
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    group.finish();
}

/// Buffer capacity stress test
fn bench_link_buffer_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("link_buffer_handling");

    group.bench_function("Link::buffer_fill_and_drain", |b| {
        let topic = format!("bench_link_buffer_{}", std::process::id());
        let producer: Link<i32> = Link::producer(&topic).unwrap();
        let consumer: Link<i32> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            // Fill buffer to capacity-1 (1023 messages)
            for i in 0..1023 {
                producer.send(black_box(i), &mut None).unwrap();
            }
            // Drain buffer
            for _ in 0..1023 {
                let _ = black_box(consumer.recv(&mut None));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_link_small_message,
    bench_link_medium_message,
    bench_link_large_message,
    bench_link_zero_copy,
    bench_link_throughput,
    bench_link_primitives,
    bench_link_buffer_full,
);
criterion_main!(benches);
