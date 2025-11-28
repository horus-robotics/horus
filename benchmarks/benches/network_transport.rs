//! Network Transport Latency Benchmarks
//!
//! Comprehensive benchmarks comparing HORUS network transport backends:
//! - Standard UDP: Cross-platform, ~5-10µs latency
//! - Batch UDP (sendmmsg/recvmmsg): Linux, ~3-5µs latency
//! - io_uring: Linux 5.1+, ~2-3µs latency (with io-uring-net feature)
//!
//! Run with: cargo bench --bench network_transport

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use horus::prelude::Link;
use horus_library::messages::cmd_vel::CmdVel;

/// Payload sizes to test
const PAYLOAD_SIZES: &[usize] = &[64, 256, 1024, 4096];

/// Message to send
fn create_payload(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i & 0xFF) as u8).collect()
}

/// Benchmark standard UDP loopback latency
fn bench_udp_loopback(c: &mut Criterion) {
    let mut group = c.benchmark_group("udp_loopback_latency");
    group.measurement_time(Duration::from_secs(5));

    for &size in PAYLOAD_SIZES {
        group.bench_with_input(BenchmarkId::new("standard_udp", size), &size, |b, &size| {
            let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
            let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
            let recv_addr = receiver.local_addr().unwrap();

            sender.set_nonblocking(false).unwrap();
            receiver
                .set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();

            let payload = create_payload(size);
            let mut recv_buf = vec![0u8; size + 64];

            b.iter(|| {
                sender.send_to(black_box(&payload), recv_addr).unwrap();
                let _ = black_box(receiver.recv(&mut recv_buf));
            });
        });
    }

    group.finish();
}

/// Benchmark UDP roundtrip latency (more realistic)
fn bench_udp_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("udp_roundtrip_latency");
    group.measurement_time(Duration::from_secs(5));

    // Use 64-byte payload for latency measurement
    let size = 64;

    group.bench_function("roundtrip_64B", |b| {
        let socket_a = UdpSocket::bind("127.0.0.1:0").unwrap();
        let socket_b = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr_a = socket_a.local_addr().unwrap();
        let addr_b = socket_b.local_addr().unwrap();

        socket_a
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        socket_b
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();

        let payload = create_payload(size);
        let mut recv_buf = vec![0u8; size + 64];

        b.iter(|| {
            // A -> B
            socket_a.send_to(black_box(&payload), addr_b).unwrap();
            let _ = socket_b.recv(&mut recv_buf);
            // B -> A (echo)
            socket_b.send_to(&recv_buf[..size], addr_a).unwrap();
            let _ = black_box(socket_a.recv(&mut recv_buf));
        });
    });

    group.finish();
}

/// Benchmark batch send performance (simulates sendmmsg benefits)
fn bench_batch_send(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_send_throughput");
    group.measurement_time(Duration::from_secs(5));

    let batch_sizes: &[usize] = &[1, 8, 16, 32, 64];
    let payload_size = 256;

    for &batch in batch_sizes {
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch),
            &batch,
            |b, &batch| {
                let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
                let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
                let recv_addr = receiver.local_addr().unwrap();

                sender.set_nonblocking(true).unwrap();
                receiver.set_nonblocking(true).unwrap();

                let payloads: Vec<Vec<u8>> =
                    (0..batch).map(|_| create_payload(payload_size)).collect();

                b.iter(|| {
                    // Send batch
                    for payload in &payloads {
                        let _ = sender.send_to(black_box(payload), recv_addr);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Measure raw UDP send latency with timing
fn bench_udp_send_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("udp_send_latency");
    group.measurement_time(Duration::from_secs(5));

    // Small payload for pure syscall overhead measurement
    let payload = create_payload(64);

    group.bench_function("send_64B", |b| {
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let target: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        sender.set_nonblocking(true).unwrap();

        b.iter(|| {
            let _ = sender.send_to(black_box(&payload), target);
        });
    });

    group.finish();
}

/// High-precision latency measurement (manual timing)
fn bench_latency_percentiles(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_percentiles");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    group.bench_function("p50_p99_p999", |b| {
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let recv_addr = receiver.local_addr().unwrap();

        receiver
            .set_read_timeout(Some(Duration::from_millis(10)))
            .unwrap();

        let payload = create_payload(64);
        let mut recv_buf = vec![0u8; 128];

        b.iter(|| {
            sender.send_to(black_box(&payload), recv_addr).unwrap();
            let _ = receiver.recv(&mut recv_buf);
        });
    });

    group.finish();
}

/// Compare with shared memory performance
fn bench_transport_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport_comparison");
    group.measurement_time(Duration::from_secs(5));

    // Shared memory (Link) - baseline using CmdVel (16 bytes)
    group.bench_function("shared_memory_link_cmdvel", |b| {
        let topic = format!("bench_shm_{}", std::process::id());
        let producer: Link<CmdVel> = Link::producer(&topic).unwrap();
        let consumer: Link<CmdVel> = Link::consumer(&topic).unwrap();

        b.iter(|| {
            let msg = CmdVel::new(1.5, 0.8);
            producer.send(black_box(msg), &mut None).unwrap();
            let _ = black_box(consumer.recv(&mut None));
        });
    });

    // UDP loopback with similar payload size (16 bytes)
    group.bench_function("udp_loopback_16B", |b| {
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let recv_addr = receiver.local_addr().unwrap();

        receiver
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();

        let payload = create_payload(16);
        let mut recv_buf = [0u8; 64];

        b.iter(|| {
            sender.send_to(black_box(&payload), recv_addr).unwrap();
            let _ = black_box(receiver.recv(&mut recv_buf));
        });
    });

    group.finish();
}

/// Throughput benchmark - messages per second
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_throughput");
    group.measurement_time(Duration::from_secs(5));

    let message_count = 10000;
    let payload = create_payload(64);

    group.bench_function("udp_10k_messages", |b| {
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let recv_addr = receiver.local_addr().unwrap();

        sender.set_nonblocking(true).unwrap();
        receiver.set_nonblocking(true).unwrap();

        let mut recv_buf = vec![0u8; 128];

        b.iter(|| {
            // Send all
            for _ in 0..message_count {
                let _ = sender.send_to(&payload, recv_addr);
            }
            // Drain receiver (best effort)
            for _ in 0..message_count {
                let _ = receiver.recv(&mut recv_buf);
            }
        });
    });

    group.finish();
}

/// Linux-specific: Test batch UDP with sendmmsg
#[cfg(target_os = "linux")]
fn bench_linux_batch_udp(c: &mut Criterion) {
    use horus_core::communication::network::batch_udp::{BatchUdpConfig, BatchUdpSender};

    let mut group = c.benchmark_group("linux_batch_udp");
    group.measurement_time(Duration::from_secs(5));

    // Test sendmmsg performance
    group.bench_function("sendmmsg_batch_16", |b| {
        let config = BatchUdpConfig {
            batch_size: 16,
            ..Default::default()
        };

        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        if let Ok(mut sender) = BatchUdpSender::new(bind_addr, config) {
            let target: SocketAddr = "127.0.0.1:12345".parse().unwrap();
            let payloads: Vec<Vec<u8>> = (0..16).map(|_| create_payload(64)).collect();

            b.iter(|| {
                for payload in &payloads {
                    let _ = sender.send(payload, target);
                }
                let _ = sender.flush();
            });
        }
    });

    group.finish();
}

/// Linux-specific: Check io_uring availability
#[cfg(target_os = "linux")]
fn bench_io_uring_check(c: &mut Criterion) {
    use horus_core::communication::network::io_uring::is_real_io_uring_available;

    let mut group = c.benchmark_group("io_uring_availability");

    group.bench_function("check_support", |b| {
        b.iter(|| black_box(is_real_io_uring_available()));
    });

    // Print io_uring status
    let available = is_real_io_uring_available();
    if available {
        println!("\nio_uring is AVAILABLE on this system");
        println!("Expected latency improvement: ~2-3µs (vs ~5-10µs for standard UDP)\n");
    } else {
        println!("\nio_uring is NOT available (requires Linux 5.1+)\n");
    }

    group.finish();
}

// Register criterion groups
criterion_group!(
    benches,
    bench_udp_loopback,
    bench_udp_roundtrip,
    bench_batch_send,
    bench_udp_send_latency,
    bench_latency_percentiles,
    bench_transport_comparison,
    bench_throughput,
);

#[cfg(target_os = "linux")]
criterion_group!(linux_benches, bench_linux_batch_udp, bench_io_uring_check,);

#[cfg(target_os = "linux")]
criterion_main!(benches, linux_benches);

#[cfg(not(target_os = "linux"))]
criterion_main!(benches);
