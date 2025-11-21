// Benchmark for Sensor performance
// Run with: cargo bench --bench sensor_bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn benchmark_gps_update(c: &mut Criterion) {
    c.bench_function("gps_sensor_update", |b| {
        b.iter(|| {
            // Placeholder: would update GPS sensor
            black_box(42)
        });
    });
}

fn benchmark_gps_velocity_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("gps_velocity_methods");

    for method in ["simple_diff", "weighted_avg", "linear_regression"].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(method), method, |b, _method| {
            b.iter(|| {
                // Placeholder: would compute velocity using specified method
                black_box(42)
            });
        });
    }

    group.finish();
}

fn benchmark_imu_update(c: &mut Criterion) {
    c.bench_function("imu_sensor_update", |b| {
        b.iter(|| {
            // Placeholder: would update IMU sensor
            black_box(42)
        });
    });
}

fn benchmark_force_torque_update(c: &mut Criterion) {
    c.bench_function("force_torque_sensor_update", |b| {
        b.iter(|| {
            // Placeholder: would update force/torque sensor
            black_box(42)
        });
    });
}

fn benchmark_lidar_raycast(c: &mut Criterion) {
    let mut group = c.benchmark_group("lidar_ray_count");

    for ray_count in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(ray_count),
            ray_count,
            |b, &count| {
                b.iter(|| {
                    // Placeholder: would cast N rays for Lidar
                    black_box(count * 10)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_gps_update,
    benchmark_gps_velocity_computation,
    benchmark_imu_update,
    benchmark_force_torque_update,
    benchmark_lidar_raycast
);
criterion_main!(benches);
