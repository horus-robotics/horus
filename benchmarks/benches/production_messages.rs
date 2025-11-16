//! Production-realistic benchmarks using actual HORUS message types
//!
//! Tests latency and throughput with real message structures:
//! - CmdVel (control commands)
//! - LaserScan (lidar data)
//! - IMU (inertial measurement)
//! - Odometry (pose + velocity)
//! - PointCloud (3D perception)
//! - BatteryState (status monitoring)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use horus::communication::horus::Hub;
use horus_library::messages::{
    cmd_vel::CmdVel,
    geometry::Point3,
    perception::PointCloud,
    sensor::{BatteryState, Imu, LaserScan, Odometry},
};
use std::time::{Duration, Instant};

/// Benchmark CmdVel (16 bytes - control command)
fn bench_cmdvel_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cmdvel");
    group.throughput(Throughput::Bytes(std::mem::size_of::<CmdVel>() as u64));

    group.bench_function("send_recv", |b| {
        let topic = format!("bench_cmdvel_{}", std::process::id());
        let sender: Hub<CmdVel> = Hub::new(&topic).unwrap();
        let receiver: Hub<CmdVel> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let msg = CmdVel::new(1.5, 0.8);
            sender.send(black_box(msg), &mut None).unwrap();
            let _ = black_box(receiver.recv(&mut None));
        });
    });

    group.bench_function("send_only", |b| {
        let topic = format!("bench_cmdvel_send_{}", std::process::id());
        let sender: Hub<CmdVel> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let msg = CmdVel::new(1.5, 0.8);
            sender.send(black_box(msg), &mut None).unwrap();
        });
    });

    group.finish();
}

/// Benchmark LaserScan (~1.5KB - 360 floats + metadata)
fn bench_laserscan_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("laserscan");
    group.throughput(Throughput::Bytes(std::mem::size_of::<LaserScan>() as u64));

    group.bench_function("send_recv", |b| {
        let topic = format!("bench_laser_{}", std::process::id());
        let sender: Hub<LaserScan> = Hub::new(&topic).unwrap();
        let receiver: Hub<LaserScan> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let mut scan = LaserScan::new();
            // Populate with realistic data
            for i in 0..360 {
                scan.ranges[i] = 5.0 + (i as f32 * 0.01);
            }
            sender.send(black_box(scan), &mut None).unwrap();
            let _ = black_box(receiver.recv(&mut None));
        });
    });

    group.finish();
}

/// Benchmark IMU (~500 bytes - orientation, angular velocity, linear acceleration + covariances)
fn bench_imu_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("imu");
    group.throughput(Throughput::Bytes(std::mem::size_of::<Imu>() as u64));

    group.bench_function("send_recv", |b| {
        let topic = format!("bench_imu_{}", std::process::id());
        let sender: Hub<Imu> = Hub::new(&topic).unwrap();
        let receiver: Hub<Imu> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let mut imu = Imu::new();
            imu.set_orientation_from_euler(0.1, 0.2, 0.3);
            imu.angular_velocity = [0.01, 0.02, 0.03];
            imu.linear_acceleration = [9.8, 0.1, 0.1];
            sender.send(black_box(imu), &mut None).unwrap();
            let _ = black_box(receiver.recv(&mut None));
        });
    });

    group.finish();
}

/// Benchmark Odometry (~700 bytes - pose, twist, covariances)
fn bench_odometry_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("odometry");
    group.throughput(Throughput::Bytes(std::mem::size_of::<Odometry>() as u64));

    group.bench_function("send_recv", |b| {
        let topic = format!("bench_odom_{}", std::process::id());
        let sender: Hub<Odometry> = Hub::new(&topic).unwrap();
        let receiver: Hub<Odometry> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let mut odom = Odometry::new();
            odom.pose.x = 1.5;
            odom.pose.y = 2.3;
            odom.pose.theta = 0.8;
            odom.twist.linear_x = 0.5;
            odom.twist.angular_z = 0.1;
            sender.send(black_box(odom), &mut None).unwrap();
            let _ = black_box(receiver.recv(&mut None));
        });
    });

    group.finish();
}

/// Benchmark BatteryState (small message with arrays)
fn bench_battery_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("battery");
    group.throughput(Throughput::Bytes(std::mem::size_of::<BatteryState>() as u64));

    group.bench_function("send_recv", |b| {
        let topic = format!("bench_battery_{}", std::process::id());
        let sender: Hub<BatteryState> = Hub::new(&topic).unwrap();
        let receiver: Hub<BatteryState> = Hub::new(&topic).unwrap();

        b.iter(|| {
            let battery = BatteryState::new(12.6, 75.0);
            sender.send(black_box(battery), &mut None).unwrap();
            let _ = black_box(receiver.recv(&mut None));
        });
    });

    group.finish();
}

/// Benchmark PointCloud (variable size - large messages)
fn bench_pointcloud_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("pointcloud");

    for (name, num_points) in &[("100pts", 100), ("1000pts", 1000), ("10000pts", 10000)] {
        group.throughput(Throughput::Bytes((*num_points * 12) as u64)); // 3 * f32 per point

        group.bench_with_input(
            BenchmarkId::new("send_recv", name),
            num_points,
            |b, &num_points| {
                let topic = format!("bench_cloud_{}_{}", name, std::process::id());
                let sender: Hub<PointCloud> = Hub::new(&topic).unwrap();
                let receiver: Hub<PointCloud> = Hub::new(&topic).unwrap();

                b.iter(|| {
                    // Create realistic point cloud
                    let points: Vec<Point3> = (0..num_points)
                        .map(|i| {
                            let t = i as f64 * 0.1;
                            Point3::new(t.sin(), t.cos(), t * 0.1)
                        })
                        .collect();

                    let cloud = PointCloud::xyz(&points);
                    sender.send(black_box(cloud), &mut None).unwrap();
                    let _ = black_box(receiver.recv(&mut None));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark sustained throughput with CmdVel at different frequencies
fn bench_cmdvel_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("cmdvel_throughput");
    group.measurement_time(Duration::from_secs(5));

    for (name, hz) in &[("10Hz", 10), ("100Hz", 100), ("1000Hz", 1000)] {
        group.bench_function(*name, |b| {
            let topic = format!("bench_cmdvel_throughput_{}_{}", name, std::process::id());
            let sender: Hub<CmdVel> = Hub::new(&topic).unwrap();
            let receiver: Hub<CmdVel> = Hub::new(&topic).unwrap();

            let cycle_time = Duration::from_secs_f64(1.0 / *hz as f64);

            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for i in 0..iters {
                    let start = Instant::now();

                    let msg = CmdVel::new(1.0 + (i as f32 * 0.01), 0.5);
                    sender.send(msg, &mut None).unwrap();
                    let _ = receiver.recv(&mut None);

                    total += start.elapsed();

                    // Simulate target frequency
                    if start.elapsed() < cycle_time {
                        std::thread::sleep(cycle_time - start.elapsed());
                    }
                }
                total
            });
        });
    }

    group.finish();
}

/// Benchmark LaserScan at typical lidar frequency (10Hz)
fn bench_laserscan_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("laserscan_throughput");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("10Hz_sustained", |b| {
        let topic = format!("bench_laser_throughput_{}", std::process::id());
        let sender: Hub<LaserScan> = Hub::new(&topic).unwrap();
        let receiver: Hub<LaserScan> = Hub::new(&topic).unwrap();

        let cycle_time = Duration::from_millis(100); // 10Hz

        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for i in 0..iters {
                let start = Instant::now();

                let mut scan = LaserScan::new();
                for j in 0..360 {
                    scan.ranges[j] = 5.0 + ((i + j as u64) as f32 * 0.01);
                }

                sender.send(scan, &mut None).unwrap();
                let _ = receiver.recv(&mut None);

                total += start.elapsed();

                if start.elapsed() < cycle_time {
                    std::thread::sleep(cycle_time - start.elapsed());
                }
            }
            total
        });
    });

    group.finish();
}

/// Benchmark mixed message types (realistic robot scenario)
fn bench_mixed_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_messages");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("robot_loop_100Hz", |b| {
        let cmd_topic = format!("bench_mix_cmd_{}", std::process::id());
        let imu_topic = format!("bench_mix_imu_{}", std::process::id());
        let battery_topic = format!("bench_mix_battery_{}", std::process::id());

        let cmd_sender: Hub<CmdVel> = Hub::new(&cmd_topic).unwrap();
        let cmd_receiver: Hub<CmdVel> = Hub::new(&cmd_topic).unwrap();

        let imu_sender: Hub<Imu> = Hub::new(&imu_topic).unwrap();
        let imu_receiver: Hub<Imu> = Hub::new(&imu_topic).unwrap();

        let battery_sender: Hub<BatteryState> = Hub::new(&battery_topic).unwrap();
        let battery_receiver: Hub<BatteryState> = Hub::new(&battery_topic).unwrap();

        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;

            for i in 0..iters {
                let start = Instant::now();

                // CmdVel at 100Hz
                let cmd = CmdVel::new(1.0, 0.5);
                cmd_sender.send(cmd, &mut None).unwrap();
                let _ = cmd_receiver.recv(&mut None);

                // IMU at 100Hz
                if i.is_multiple_of(1) {
                    let mut imu = Imu::new();
                    imu.angular_velocity = [0.01, 0.02, 0.03];
                    imu_sender.send(imu, &mut None).unwrap();
                    let _ = imu_receiver.recv(&mut None);
                }

                // Battery at 1Hz (every 100 iterations at 100Hz)
                if i.is_multiple_of(100) {
                    let battery = BatteryState::new(12.4, 70.0);
                    battery_sender.send(battery, &mut None).unwrap();
                    let _ = battery_receiver.recv(&mut None);
                }

                total += start.elapsed();
            }

            total
        });
    });

    group.finish();
}

/// Benchmark burst handling with LaserScan
fn bench_burst_laserscan(c: &mut Criterion) {
    let mut group = c.benchmark_group("burst_laserscan");

    group.bench_function("burst_10_scans", |b| {
        let topic = format!("bench_burst_laser_{}", std::process::id());
        let sender: Hub<LaserScan> = Hub::new(&topic).unwrap();
        let receiver: Hub<LaserScan> = Hub::new(&topic).unwrap();

        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;

            for _ in 0..iters {
                let start = Instant::now();

                // Send burst of 10 scans
                for i in 0..10 {
                    let mut scan = LaserScan::new();
                    for j in 0..360 {
                        scan.ranges[j] = 5.0 + ((i + j) as f32 * 0.01);
                    }
                    sender.send(scan, &mut None).unwrap();
                }

                // Receive all
                for _ in 0..10 {
                    let _ = receiver.recv(&mut None);
                }

                total += start.elapsed();
            }

            total
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cmdvel_latency,
    bench_laserscan_latency,
    bench_imu_latency,
    bench_odometry_latency,
    bench_battery_latency,
    bench_pointcloud_latency,
    bench_cmdvel_throughput,
    bench_laserscan_throughput,
    bench_mixed_messages,
    bench_burst_laserscan,
);

criterion_main!(benches);
