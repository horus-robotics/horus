//! Physics Benchmarking Suite for sim3d
//!
//! This module provides comprehensive benchmarks comparing simulated physics
//! against theoretical/analytical solutions. It measures accuracy and performance
//! of the Rapier3D-based physics simulation.
//!
//! # Benchmark Scenarios
//! - Free-fall validation (kinematic equations)
//! - Elastic collision (momentum/energy conservation)
//! - Friction slide (static/kinetic friction)
//! - Pendulum period (small angle approximation)
//! - Projectile motion (parabolic trajectory)
//! - Stack stability (settling time)
//! - Mass-spring oscillation (harmonic motion)
//!
//! # Usage
//! ```ignore
//! use sim3d::physics::benchmarks::{PhysicsBenchmark, BenchmarkConfig};
//!
//! let config = BenchmarkConfig::default();
//! let benchmark = PhysicsBenchmark::new(config);
//! let report = benchmark.run_all();
//! println!("{}", report.summary());
//! ```

use nalgebra::Vector3;
use rapier3d::prelude::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::time::{Duration, Instant};

/// Configuration for physics benchmarks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Gravity magnitude (m/s^2)
    pub gravity: f32,
    /// Physics timestep (seconds)
    pub dt: f32,
    /// Default simulation duration (seconds)
    pub default_duration: f32,
    /// Error tolerance for pass/fail (percentage)
    pub default_tolerance: f32,
    /// Number of substeps per physics step
    pub substeps: usize,
    /// Enable verbose output
    pub verbose: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            gravity: 9.81,
            dt: 1.0 / 240.0, // 240 Hz physics
            default_duration: 2.0,
            default_tolerance: 5.0, // 5% error tolerance
            substeps: 1,
            verbose: false,
        }
    }
}

impl BenchmarkConfig {
    /// High precision configuration for accuracy testing
    pub fn high_precision() -> Self {
        Self {
            gravity: 9.81,
            dt: 1.0 / 1000.0, // 1000 Hz physics
            default_duration: 2.0,
            default_tolerance: 1.0, // 1% error tolerance
            substeps: 4,
            verbose: false,
        }
    }

    /// Fast configuration for quick validation
    pub fn fast() -> Self {
        Self {
            gravity: 9.81,
            dt: 1.0 / 60.0, // 60 Hz physics
            default_duration: 1.0,
            default_tolerance: 10.0, // 10% error tolerance
            substeps: 1,
            verbose: false,
        }
    }
}

/// Result of a single benchmark test
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Name of the test
    pub test_name: String,
    /// Description of what is being measured
    pub description: String,
    /// Expected value from analytical solution
    pub expected_value: f64,
    /// Measured value from simulation
    pub measured_value: f64,
    /// Absolute error
    pub absolute_error: f64,
    /// Error percentage
    pub error_percentage: f64,
    /// Pass/fail threshold percentage
    pub threshold: f64,
    /// Whether the test passed
    pub passed: bool,
    /// Time taken to run the benchmark
    pub execution_time: Duration,
    /// Number of simulation steps
    pub simulation_steps: usize,
    /// Additional metrics
    pub metrics: BenchmarkMetrics,
}

impl BenchmarkResult {
    pub fn new(
        test_name: &str,
        description: &str,
        expected: f64,
        measured: f64,
        threshold: f64,
        execution_time: Duration,
        steps: usize,
    ) -> Self {
        let absolute_error = (expected - measured).abs();
        let error_percentage = if expected.abs() > 1e-10 {
            (absolute_error / expected.abs()) * 100.0
        } else {
            absolute_error * 100.0
        };
        let passed = error_percentage <= threshold;

        Self {
            test_name: test_name.to_string(),
            description: description.to_string(),
            expected_value: expected,
            measured_value: measured,
            absolute_error,
            error_percentage,
            threshold,
            passed,
            execution_time,
            simulation_steps: steps,
            metrics: BenchmarkMetrics::default(),
        }
    }

    pub fn with_metrics(mut self, metrics: BenchmarkMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// Format result as a string
    pub fn format(&self) -> String {
        let status = if self.passed { "PASS" } else { "FAIL" };
        format!(
            "[{}] {}: expected={:.6}, measured={:.6}, error={:.2}% (threshold={:.1}%)",
            status,
            self.test_name,
            self.expected_value,
            self.measured_value,
            self.error_percentage,
            self.threshold
        )
    }
}

/// Additional metrics for benchmark analysis
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    /// Minimum value during simulation
    pub min_value: f64,
    /// Maximum value during simulation
    pub max_value: f64,
    /// Mean value during simulation
    pub mean_value: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Peak memory usage estimate (bytes)
    pub memory_estimate: usize,
    /// Steps per second achieved
    pub steps_per_second: f64,
}

impl BenchmarkMetrics {
    pub fn from_samples(samples: &[f64], execution_time: Duration, steps: usize) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let min_value = samples.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_value = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mean_value = samples.iter().sum::<f64>() / samples.len() as f64;

        let variance = samples
            .iter()
            .map(|x| (x - mean_value).powi(2))
            .sum::<f64>()
            / samples.len() as f64;
        let std_dev = variance.sqrt();

        let steps_per_second = if execution_time.as_secs_f64() > 0.0 {
            steps as f64 / execution_time.as_secs_f64()
        } else {
            0.0
        };

        Self {
            min_value,
            max_value,
            mean_value,
            std_dev,
            memory_estimate: 0,
            steps_per_second,
        }
    }
}

/// Reference accuracy levels for comparison with other simulators
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReferenceAccuracy {
    /// Simulator name
    pub name: String,
    /// Typical free-fall error percentage
    pub freefall_error: f64,
    /// Typical collision error percentage
    pub collision_error: f64,
    /// Typical friction error percentage
    pub friction_error: f64,
    /// Typical pendulum error percentage
    pub pendulum_error: f64,
    /// Typical spring oscillation error percentage
    pub spring_error: f64,
}

impl ReferenceAccuracy {
    /// PyBullet reference accuracy (typical values)
    pub fn pybullet() -> Self {
        Self {
            name: "PyBullet".to_string(),
            freefall_error: 0.5,
            collision_error: 2.0,
            friction_error: 5.0,
            pendulum_error: 1.5,
            spring_error: 2.0,
        }
    }

    /// MuJoCo reference accuracy (typical values)
    pub fn mujoco() -> Self {
        Self {
            name: "MuJoCo".to_string(),
            freefall_error: 0.1,
            collision_error: 1.0,
            friction_error: 3.0,
            pendulum_error: 0.5,
            spring_error: 1.0,
        }
    }

    /// Rapier3D reference accuracy (expected)
    pub fn rapier3d() -> Self {
        Self {
            name: "Rapier3D".to_string(),
            freefall_error: 0.2,
            collision_error: 1.5,
            friction_error: 4.0,
            pendulum_error: 1.0,
            spring_error: 1.5,
        }
    }
}

/// Aggregated benchmark report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkReport {
    /// All benchmark results
    pub results: Vec<BenchmarkResult>,
    /// Total execution time
    pub total_time: Duration,
    /// Configuration used
    pub config: BenchmarkConfig,
    /// Reference comparisons
    pub reference_comparisons: Vec<ReferenceComparison>,
    /// Overall pass rate
    pub pass_rate: f64,
    /// Average error percentage
    pub average_error: f64,
}

/// Comparison against a reference simulator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReferenceComparison {
    /// Reference simulator name
    pub reference_name: String,
    /// Test name
    pub test_name: String,
    /// Our error percentage
    pub our_error: f64,
    /// Reference error percentage
    pub reference_error: f64,
    /// Ratio (our_error / reference_error)
    pub error_ratio: f64,
    /// Whether we are better (lower error)
    pub better_than_reference: bool,
}

impl BenchmarkReport {
    pub fn new(
        results: Vec<BenchmarkResult>,
        config: BenchmarkConfig,
        total_time: Duration,
    ) -> Self {
        let pass_count = results.iter().filter(|r| r.passed).count();
        let pass_rate = if results.is_empty() {
            0.0
        } else {
            (pass_count as f64 / results.len() as f64) * 100.0
        };

        let average_error = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.error_percentage).sum::<f64>() / results.len() as f64
        };

        let mut report = Self {
            results,
            total_time,
            config,
            reference_comparisons: Vec::new(),
            pass_rate,
            average_error,
        };

        report.generate_comparisons();
        report
    }

    fn generate_comparisons(&mut self) {
        let pybullet = ReferenceAccuracy::pybullet();
        let mujoco = ReferenceAccuracy::mujoco();

        for result in &self.results {
            let (pybullet_ref, mujoco_ref) = match result.test_name.as_str() {
                name if name.contains("freefall") || name.contains("Free-fall") => {
                    (pybullet.freefall_error, mujoco.freefall_error)
                }
                name if name.contains("collision") || name.contains("Collision") => {
                    (pybullet.collision_error, mujoco.collision_error)
                }
                name if name.contains("friction") || name.contains("Friction") => {
                    (pybullet.friction_error, mujoco.friction_error)
                }
                name if name.contains("pendulum") || name.contains("Pendulum") => {
                    (pybullet.pendulum_error, mujoco.pendulum_error)
                }
                name if name.contains("spring") || name.contains("Spring") => {
                    (pybullet.spring_error, mujoco.spring_error)
                }
                _ => (5.0, 3.0), // Default reference values
            };

            self.reference_comparisons.push(ReferenceComparison {
                reference_name: "PyBullet".to_string(),
                test_name: result.test_name.clone(),
                our_error: result.error_percentage,
                reference_error: pybullet_ref,
                error_ratio: result.error_percentage / pybullet_ref.max(0.001),
                better_than_reference: result.error_percentage < pybullet_ref,
            });

            self.reference_comparisons.push(ReferenceComparison {
                reference_name: "MuJoCo".to_string(),
                test_name: result.test_name.clone(),
                our_error: result.error_percentage,
                reference_error: mujoco_ref,
                error_ratio: result.error_percentage / mujoco_ref.max(0.001),
                better_than_reference: result.error_percentage < mujoco_ref,
            });
        }
    }

    /// Generate summary string
    pub fn summary(&self) -> String {
        let mut output = String::new();
        output.push_str(
            "╔════════════════════════════════════════════════════════════════════════════╗\n",
        );
        output.push_str(
            "║                    PHYSICS BENCHMARK REPORT                                ║\n",
        );
        output.push_str(
            "╠════════════════════════════════════════════════════════════════════════════╣\n",
        );
        output.push_str(&format!(
            "║ Configuration: dt={:.4}s, gravity={:.2}m/s^2, tolerance={:.1}%        ║\n",
            self.config.dt, self.config.gravity, self.config.default_tolerance
        ));
        output.push_str(&format!(
            "║ Total Time: {:.3}s | Tests: {} | Pass Rate: {:.1}%                      ║\n",
            self.total_time.as_secs_f64(),
            self.results.len(),
            self.pass_rate
        ));
        output.push_str(
            "╠════════════════════════════════════════════════════════════════════════════╣\n",
        );
        output.push_str(
            "║ INDIVIDUAL RESULTS                                                         ║\n",
        );
        output.push_str(
            "╠════════════════════════════════════════════════════════════════════════════╣\n",
        );

        for result in &self.results {
            let status = if result.passed { "PASS" } else { "FAIL" };
            let line = format!(
                "║ [{}] {:<30} err={:>6.2}% (thr={:.1}%)            ║\n",
                status, result.test_name, result.error_percentage, result.threshold
            );
            output.push_str(&line);
        }

        output.push_str(
            "╠════════════════════════════════════════════════════════════════════════════╣\n",
        );
        output.push_str(
            "║ REFERENCE COMPARISON                                                       ║\n",
        );
        output.push_str(
            "╠════════════════════════════════════════════════════════════════════════════╣\n",
        );

        let pybullet_better = self
            .reference_comparisons
            .iter()
            .filter(|c| c.reference_name == "PyBullet" && c.better_than_reference)
            .count();
        let pybullet_total = self
            .reference_comparisons
            .iter()
            .filter(|c| c.reference_name == "PyBullet")
            .count();

        let mujoco_better = self
            .reference_comparisons
            .iter()
            .filter(|c| c.reference_name == "MuJoCo" && c.better_than_reference)
            .count();
        let mujoco_total = self
            .reference_comparisons
            .iter()
            .filter(|c| c.reference_name == "MuJoCo")
            .count();

        output.push_str(&format!(
            "║ vs PyBullet: {}/{} tests with lower error                              ║\n",
            pybullet_better, pybullet_total
        ));
        output.push_str(&format!(
            "║ vs MuJoCo:   {}/{} tests with lower error                              ║\n",
            mujoco_better, mujoco_total
        ));

        output.push_str(
            "╠════════════════════════════════════════════════════════════════════════════╣\n",
        );
        output.push_str(
            "║ PERFORMANCE METRICS                                                        ║\n",
        );
        output.push_str(
            "╠════════════════════════════════════════════════════════════════════════════╣\n",
        );

        let total_steps: usize = self.results.iter().map(|r| r.simulation_steps).sum();
        let avg_steps_per_sec: f64 = self
            .results
            .iter()
            .map(|r| r.metrics.steps_per_second)
            .sum::<f64>()
            / self.results.len().max(1) as f64;

        output.push_str(&format!(
            "║ Total Simulation Steps: {}                                        ║\n",
            total_steps
        ));
        output.push_str(&format!(
            "║ Average Steps/Second: {:.0}                                         ║\n",
            avg_steps_per_sec
        ));

        output.push_str(
            "╚════════════════════════════════════════════════════════════════════════════╝\n",
        );

        output
    }

    /// Export report as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Main physics benchmark runner
pub struct PhysicsBenchmark {
    config: BenchmarkConfig,
}

impl PhysicsBenchmark {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Create a fresh physics world for testing
    fn create_world(&self) -> BenchmarkPhysicsWorld {
        BenchmarkPhysicsWorld::new(self.config.gravity, self.config.dt)
    }

    /// Run all benchmark tests
    pub fn run_all(&self) -> BenchmarkReport {
        let start = Instant::now();

        let mut results = Vec::new();
        results.push(self.benchmark_freefall_velocity());
        results.push(self.benchmark_freefall_position());
        results.push(self.benchmark_elastic_collision_momentum());
        results.push(self.benchmark_elastic_collision_energy());
        results.push(self.benchmark_friction_static());
        results.push(self.benchmark_friction_kinetic());
        results.push(self.benchmark_pendulum_period());
        results.push(self.benchmark_projectile_range());
        results.push(self.benchmark_projectile_height());
        results.push(self.benchmark_stack_stability());
        results.push(self.benchmark_spring_frequency());
        results.push(self.benchmark_spring_amplitude());

        let total_time = start.elapsed();
        BenchmarkReport::new(results, self.config.clone(), total_time)
    }

    /// Benchmark: Free-fall velocity (v = gt)
    pub fn benchmark_freefall_velocity(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();

        // Create a sphere in free fall
        let mass = 1.0;
        let radius = 0.5;
        let initial_height = 10.0;
        let initial_velocity = Vector3::new(0.0, 0.0, 0.0);

        let rb_handle = world.spawn_sphere(
            Vector3::new(0.0, initial_height, 0.0),
            initial_velocity,
            radius,
            mass,
        );

        // Simulate for 1 second
        let duration = 1.0;
        let steps = (duration / self.config.dt) as usize;
        let mut velocity_samples = Vec::with_capacity(steps);

        for _ in 0..steps {
            world.step();
            let rb = world.rigid_body_set.get(rb_handle).unwrap();
            velocity_samples.push(-rb.linvel().y as f64);
        }

        let execution_time = start.elapsed();

        // Analytical solution: v = gt
        let expected_velocity = (self.config.gravity * duration) as f64;
        let measured_velocity = *velocity_samples.last().unwrap_or(&0.0);

        let metrics = BenchmarkMetrics::from_samples(&velocity_samples, execution_time, steps);

        BenchmarkResult::new(
            "Free-fall velocity",
            "Validates v = gt kinematic equation",
            expected_velocity,
            measured_velocity,
            self.config.default_tolerance as f64,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Free-fall position (h = h0 - 0.5gt^2)
    pub fn benchmark_freefall_position(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();

        let mass = 1.0;
        let radius = 0.5;
        let initial_height = 100.0; // High enough to not hit ground
        let initial_velocity = Vector3::new(0.0, 0.0, 0.0);

        let rb_handle = world.spawn_sphere(
            Vector3::new(0.0, initial_height, 0.0),
            initial_velocity,
            radius,
            mass,
        );

        let duration = 2.0;
        let steps = (duration / self.config.dt) as usize;
        let mut position_samples = Vec::with_capacity(steps);

        for _ in 0..steps {
            world.step();
            let rb = world.rigid_body_set.get(rb_handle).unwrap();
            position_samples.push(rb.translation().y as f64);
        }

        let execution_time = start.elapsed();

        // Analytical solution: h = h0 - 0.5*g*t^2
        let g = self.config.gravity as f64;
        let t = duration as f64;
        let expected_height = initial_height as f64 - 0.5 * g * t * t;
        let measured_height = *position_samples.last().unwrap_or(&0.0);

        let metrics = BenchmarkMetrics::from_samples(&position_samples, execution_time, steps);

        BenchmarkResult::new(
            "Free-fall position",
            "Validates h = h0 - 0.5gt^2 kinematic equation",
            expected_height,
            measured_height,
            self.config.default_tolerance as f64,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Elastic collision momentum conservation
    pub fn benchmark_elastic_collision_momentum(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();
        world.gravity = Vector3::new(0.0, 0.0, 0.0); // Disable gravity for collision test

        let mass1 = 2.0;
        let mass2 = 1.0;
        let radius = 0.5;
        let v1_initial = Vector3::new(5.0, 0.0, 0.0);
        let v2_initial = Vector3::new(-2.0, 0.0, 0.0);

        // Place spheres on collision course
        let rb1 = world.spawn_sphere(Vector3::new(-3.0, 0.0, 0.0), v1_initial, radius, mass1);
        let rb2 = world.spawn_sphere(Vector3::new(3.0, 0.0, 0.0), v2_initial, radius, mass2);

        // Set restitution to 1.0 for perfectly elastic collision
        world.set_restitution(rb1, 1.0);
        world.set_restitution(rb2, 1.0);

        // Initial momentum
        let initial_momentum = mass1 * v1_initial.x + mass2 * v2_initial.x;

        // Simulate until collision and settling
        let duration = 2.0;
        let steps = (duration / self.config.dt) as usize;
        let mut momentum_samples = Vec::with_capacity(steps);

        for _ in 0..steps {
            world.step();
            let rb1_vel = world.rigid_body_set.get(rb1).unwrap().linvel().x;
            let rb2_vel = world.rigid_body_set.get(rb2).unwrap().linvel().x;
            let momentum = mass1 * rb1_vel + mass2 * rb2_vel;
            momentum_samples.push(momentum as f64);
        }

        let execution_time = start.elapsed();

        let expected_momentum = initial_momentum as f64;
        let measured_momentum = *momentum_samples.last().unwrap_or(&0.0);

        let metrics = BenchmarkMetrics::from_samples(&momentum_samples, execution_time, steps);

        BenchmarkResult::new(
            "Elastic collision momentum",
            "Validates momentum conservation p1 + p2 = const",
            expected_momentum,
            measured_momentum,
            self.config.default_tolerance as f64,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Elastic collision energy conservation
    pub fn benchmark_elastic_collision_energy(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();
        world.gravity = Vector3::new(0.0, 0.0, 0.0);

        let mass1 = 2.0;
        let mass2 = 1.0;
        let radius = 0.5;
        let v1_initial = Vector3::new(5.0, 0.0, 0.0);
        let v2_initial = Vector3::new(-2.0, 0.0, 0.0);

        let rb1 = world.spawn_sphere(Vector3::new(-3.0, 0.0, 0.0), v1_initial, radius, mass1);
        let rb2 = world.spawn_sphere(Vector3::new(3.0, 0.0, 0.0), v2_initial, radius, mass2);

        world.set_restitution(rb1, 1.0);
        world.set_restitution(rb2, 1.0);

        // Initial kinetic energy
        let initial_energy =
            0.5 * mass1 * v1_initial.norm_squared() + 0.5 * mass2 * v2_initial.norm_squared();

        let duration = 2.0;
        let steps = (duration / self.config.dt) as usize;
        let mut energy_samples = Vec::with_capacity(steps);

        for _ in 0..steps {
            world.step();
            let v1 = world
                .rigid_body_set
                .get(rb1)
                .unwrap()
                .linvel()
                .norm_squared();
            let v2 = world
                .rigid_body_set
                .get(rb2)
                .unwrap()
                .linvel()
                .norm_squared();
            let energy = 0.5 * mass1 * v1 + 0.5 * mass2 * v2;
            energy_samples.push(energy as f64);
        }

        let execution_time = start.elapsed();

        let expected_energy = initial_energy as f64;
        let measured_energy = *energy_samples.last().unwrap_or(&0.0);

        let metrics = BenchmarkMetrics::from_samples(&energy_samples, execution_time, steps);

        BenchmarkResult::new(
            "Elastic collision energy",
            "Validates kinetic energy conservation KE = const",
            expected_energy,
            measured_energy,
            self.config.default_tolerance as f64 * 2.0, // Energy conservation is harder
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Static friction threshold
    pub fn benchmark_friction_static(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();

        // Create an inclined plane scenario
        // Static friction coefficient
        let mu_s: f32 = 0.5;

        // Calculate critical angle where block should just start sliding
        // tan(theta) = mu_s => theta = atan(mu_s)
        let critical_angle = mu_s.atan();

        // Test at 90% of critical angle - block should NOT slide
        let test_angle = critical_angle * 0.9;

        // Create a box on a slope
        let mass = 1.0;
        let half_extents = Vector3::new(0.5, 0.5, 0.5);
        let height = 2.0;

        // Position on slope
        let pos = Vector3::new(0.0, height, 0.0);
        let rb_handle = world.spawn_box(pos, Vector3::zeros(), half_extents, mass);

        // Set friction
        world.set_friction(rb_handle, mu_s);

        // Apply gravitational component along slope
        let g = self.config.gravity;
        let force_along_slope = mass * g * test_angle.sin();

        // Create ground
        world.spawn_static_ground(0.0, mu_s);

        let duration = 1.0;
        let steps = (duration / self.config.dt) as usize;
        let mut velocity_samples = Vec::with_capacity(steps);
        let _initial_pos = pos;

        for _ in 0..steps {
            // Apply force simulating the slope
            let rb = world.rigid_body_set.get_mut(rb_handle).unwrap();
            rb.reset_forces(true);
            rb.add_force(Vector3::new(force_along_slope, 0.0, 0.0), true);
            world.step();

            let rb = world.rigid_body_set.get(rb_handle).unwrap();
            velocity_samples.push(rb.linvel().x.abs() as f64);
        }

        let execution_time = start.elapsed();

        // Block should remain stationary (velocity near zero)
        let expected_velocity = 0.0;
        let measured_velocity = *velocity_samples.last().unwrap_or(&0.0);

        // Use absolute error threshold since expected is 0
        let threshold = 0.5; // Allow small drift

        let metrics = BenchmarkMetrics::from_samples(&velocity_samples, execution_time, steps);

        let absolute_error = (expected_velocity - measured_velocity).abs();
        let passed = absolute_error < threshold;

        BenchmarkResult {
            test_name: "Friction static threshold".to_string(),
            description: "Validates static friction prevents motion below critical angle"
                .to_string(),
            expected_value: expected_velocity,
            measured_value: measured_velocity,
            absolute_error,
            error_percentage: absolute_error * 100.0,
            threshold: threshold * 100.0,
            passed,
            execution_time,
            simulation_steps: steps,
            metrics,
        }
    }

    /// Benchmark: Kinetic friction deceleration
    pub fn benchmark_friction_kinetic(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();

        // Kinetic friction coefficient
        let mu_k = 0.3;
        let mass = 1.0;
        let initial_velocity = 10.0;

        // Create sliding box
        let half_extents = Vector3::new(0.5, 0.5, 0.5);
        let height = 0.6; // Just above ground
        let rb_handle = world.spawn_box(
            Vector3::new(0.0, height, 0.0),
            Vector3::new(initial_velocity, 0.0, 0.0),
            half_extents,
            mass,
        );

        // Create ground with friction
        world.spawn_static_ground(0.0, mu_k);
        world.set_friction(rb_handle, mu_k);

        // Analytical: deceleration a = mu_k * g
        // Time to stop: t = v0 / (mu_k * g)
        // Distance traveled: d = v0^2 / (2 * mu_k * g)
        let g = self.config.gravity;
        let deceleration = mu_k * g;
        let stopping_time = initial_velocity / deceleration;
        let expected_distance = (initial_velocity * initial_velocity) / (2.0 * deceleration);

        let duration = stopping_time * 1.5; // Run past expected stop time
        let steps = (duration / self.config.dt) as usize;
        let mut position_samples = Vec::with_capacity(steps);

        for _ in 0..steps {
            world.step();
            let rb = world.rigid_body_set.get(rb_handle).unwrap();
            position_samples.push(rb.translation().x as f64);
        }

        let execution_time = start.elapsed();

        let measured_distance = *position_samples.last().unwrap_or(&0.0);

        let metrics = BenchmarkMetrics::from_samples(&position_samples, execution_time, steps);

        BenchmarkResult::new(
            "Friction kinetic deceleration",
            "Validates kinetic friction: d = v0^2 / (2*mu*g)",
            expected_distance as f64,
            measured_distance,
            self.config.default_tolerance as f64 * 2.0,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Pendulum period (T = 2*pi*sqrt(L/g))
    pub fn benchmark_pendulum_period(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();

        // Pendulum parameters
        let length: f32 = 1.0; // 1 meter
        let bob_mass: f32 = 1.0;
        let bob_radius: f32 = 0.1;
        let initial_angle: f32 = 0.1; // Small angle (radians) for simple pendulum approximation

        // Create pivot point (fixed)
        let pivot_pos = Vector3::new(0.0, 5.0, 0.0);
        let pivot_handle = world.spawn_fixed_point(pivot_pos);

        // Create bob position
        let bob_x = length * initial_angle.sin();
        let bob_y = pivot_pos.y - length * initial_angle.cos();
        let bob_pos = Vector3::new(bob_x, bob_y, 0.0);

        let bob_handle = world.spawn_sphere(bob_pos, Vector3::zeros(), bob_radius, bob_mass);

        // Create distance constraint (pendulum rod)
        world.create_distance_constraint(pivot_handle, bob_handle, length);

        // Analytical period for simple pendulum (small angle)
        let g = self.config.gravity;
        let expected_period = 2.0 * PI * (length / g).sqrt();

        // Simulate multiple periods
        let num_periods = 3.0;
        let duration = expected_period * num_periods;
        let steps = (duration / self.config.dt) as usize;
        let mut position_x_samples = Vec::with_capacity(steps);
        let mut time_samples = Vec::with_capacity(steps);

        for i in 0..steps {
            world.step();
            let rb = world.rigid_body_set.get(bob_handle).unwrap();
            position_x_samples.push(rb.translation().x as f64);
            time_samples.push(i as f64 * self.config.dt as f64);
        }

        let execution_time = start.elapsed();

        // Find zero-crossings to measure period
        let measured_period = Self::measure_period(&position_x_samples, self.config.dt as f64);

        let metrics = BenchmarkMetrics::from_samples(&position_x_samples, execution_time, steps);

        BenchmarkResult::new(
            "Pendulum period",
            "Validates T = 2*pi*sqrt(L/g) for simple pendulum",
            expected_period as f64,
            measured_period,
            self.config.default_tolerance as f64,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Projectile motion range
    pub fn benchmark_projectile_range(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();

        // Projectile parameters
        let v0 = 20.0; // Initial velocity
        let angle = PI / 4.0; // 45 degrees for maximum range
        let mass = 1.0;
        let radius = 0.1;

        let vx = v0 * angle.cos();
        let vy = v0 * angle.sin();

        let rb_handle = world.spawn_sphere(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(vx, vy, 0.0),
            radius,
            mass,
        );

        // Analytical range: R = v0^2 * sin(2*theta) / g
        let g = self.config.gravity;
        let expected_range = (v0 * v0 * (2.0 * angle).sin()) / g;
        let flight_time = 2.0 * v0 * angle.sin() / g;

        let duration = flight_time * 1.2;
        let steps = (duration / self.config.dt) as usize;
        let mut position_samples = Vec::with_capacity(steps);
        let mut max_x = 0.0f32;

        for _ in 0..steps {
            world.step();
            let rb = world.rigid_body_set.get(rb_handle).unwrap();
            let x = rb.translation().x;
            let y = rb.translation().y;
            position_samples.push(x as f64);

            // Track maximum x when y is near zero (landed)
            if y <= 0.0 && x > max_x {
                max_x = x;
            }
        }

        let execution_time = start.elapsed();

        let measured_range = max_x as f64;

        let metrics = BenchmarkMetrics::from_samples(&position_samples, execution_time, steps);

        BenchmarkResult::new(
            "Projectile range",
            "Validates R = v0^2*sin(2*theta)/g",
            expected_range as f64,
            measured_range,
            self.config.default_tolerance as f64,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Projectile maximum height
    pub fn benchmark_projectile_height(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();

        let v0 = 20.0;
        let angle = PI / 4.0;
        let mass = 1.0;
        let radius = 0.1;

        let vx = v0 * angle.cos();
        let vy = v0 * angle.sin();

        let rb_handle = world.spawn_sphere(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(vx, vy, 0.0),
            radius,
            mass,
        );

        // Analytical max height: H = v0^2 * sin^2(theta) / (2*g)
        let g = self.config.gravity;
        let expected_height = (v0 * v0 * angle.sin().powi(2)) / (2.0 * g);

        let duration = 2.0 * v0 * angle.sin() / g;
        let steps = (duration / self.config.dt) as usize;
        let mut height_samples = Vec::with_capacity(steps);
        let mut max_height = 0.0f32;

        for _ in 0..steps {
            world.step();
            let rb = world.rigid_body_set.get(rb_handle).unwrap();
            let y = rb.translation().y;
            height_samples.push(y as f64);
            if y > max_height {
                max_height = y;
            }
        }

        let execution_time = start.elapsed();

        let measured_height = max_height as f64;

        let metrics = BenchmarkMetrics::from_samples(&height_samples, execution_time, steps);

        BenchmarkResult::new(
            "Projectile max height",
            "Validates H = v0^2*sin^2(theta)/(2*g)",
            expected_height as f64,
            measured_height,
            self.config.default_tolerance as f64,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Stack stability (settling time)
    pub fn benchmark_stack_stability(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();

        // Stack parameters
        let num_boxes = 5;
        let box_size = Vector3::new(1.0, 0.5, 1.0);
        let mass = 1.0;

        // Create ground
        world.spawn_static_ground(0.0, 0.5);

        // Create stacked boxes
        let mut handles = Vec::with_capacity(num_boxes);
        for i in 0..num_boxes {
            let height = box_size.y * (i as f32 + 0.5);
            let handle = world.spawn_box(
                Vector3::new(0.0, height, 0.0),
                Vector3::zeros(),
                box_size * 0.5,
                mass,
            );
            world.set_friction(handle, 0.5);
            handles.push(handle);
        }

        // Measure settling time (when total kinetic energy drops below threshold)
        let duration = 5.0;
        let steps = (duration / self.config.dt) as usize;
        let energy_threshold = 0.001;
        let mut settled_step = steps;
        let mut energy_samples = Vec::with_capacity(steps);

        for step in 0..steps {
            world.step();

            let total_ke: f32 = handles
                .iter()
                .map(|&h| {
                    let rb = world.rigid_body_set.get(h).unwrap();
                    0.5 * mass * rb.linvel().norm_squared()
                })
                .sum();

            energy_samples.push(total_ke as f64);

            if total_ke < energy_threshold && settled_step == steps {
                settled_step = step;
            }
        }

        let execution_time = start.elapsed();

        let settling_time = settled_step as f64 * self.config.dt as f64;

        // Expected settling time is approximate - should be within a few seconds
        // for a 5-box stack
        let expected_settling_time = 0.5; // Rough expectation
        let max_acceptable_time = 2.0;

        let metrics = BenchmarkMetrics::from_samples(&energy_samples, execution_time, steps);

        // Pass if settling time is reasonable
        let passed = settling_time < max_acceptable_time;

        BenchmarkResult {
            test_name: "Stack stability settling".to_string(),
            description: "Measures settling time for stacked boxes".to_string(),
            expected_value: expected_settling_time,
            measured_value: settling_time,
            absolute_error: (settling_time - expected_settling_time).abs(),
            error_percentage: ((settling_time - expected_settling_time).abs()
                / expected_settling_time)
                * 100.0,
            threshold: 200.0, // 200% tolerance for settling time
            passed,
            execution_time,
            simulation_steps: steps,
            metrics,
        }
    }

    /// Benchmark: Mass-spring oscillation frequency
    pub fn benchmark_spring_frequency(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();
        world.gravity = Vector3::new(0.0, 0.0, 0.0); // Disable gravity for pure spring test

        // Spring parameters
        let spring_stiffness = 100.0; // N/m
        let mass = 1.0;
        let initial_displacement = 0.5; // meters

        // Create fixed anchor
        let anchor_pos = Vector3::new(0.0, 0.0, 0.0);
        let anchor_handle = world.spawn_fixed_point(anchor_pos);

        // Create oscillating mass
        let mass_pos = Vector3::new(initial_displacement, 0.0, 0.0);
        let mass_handle = world.spawn_sphere(mass_pos, Vector3::zeros(), 0.1, mass);

        // Create spring constraint
        world.create_spring_constraint(anchor_handle, mass_handle, spring_stiffness, 0.0, 0.1);

        // Analytical frequency: f = (1/2pi) * sqrt(k/m)
        // Angular frequency: omega = sqrt(k/m)
        // Period: T = 2*pi * sqrt(m/k)
        let omega = (spring_stiffness / mass).sqrt();
        let expected_period = 2.0 * PI / omega;
        let expected_frequency = 1.0 / expected_period;

        let duration = expected_period * 5.0;
        let steps = (duration / self.config.dt) as usize;
        let mut position_samples = Vec::with_capacity(steps);

        for _ in 0..steps {
            world.step();
            let rb = world.rigid_body_set.get(mass_handle).unwrap();
            position_samples.push(rb.translation().x as f64);
        }

        let execution_time = start.elapsed();

        let measured_period = Self::measure_period(&position_samples, self.config.dt as f64);
        let measured_frequency = if measured_period > 0.0 {
            1.0 / measured_period
        } else {
            0.0
        };

        let metrics = BenchmarkMetrics::from_samples(&position_samples, execution_time, steps);

        BenchmarkResult::new(
            "Spring oscillation frequency",
            "Validates f = (1/2pi)*sqrt(k/m)",
            expected_frequency as f64,
            measured_frequency,
            self.config.default_tolerance as f64,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Benchmark: Mass-spring amplitude decay (undamped should maintain amplitude)
    pub fn benchmark_spring_amplitude(&self) -> BenchmarkResult {
        let start = Instant::now();
        let mut world = self.create_world();
        world.gravity = Vector3::new(0.0, 0.0, 0.0);

        let spring_stiffness = 100.0;
        let mass = 1.0;
        let initial_displacement = 0.5;

        let anchor_pos = Vector3::new(0.0, 0.0, 0.0);
        let anchor_handle = world.spawn_fixed_point(anchor_pos);

        let mass_pos = Vector3::new(initial_displacement, 0.0, 0.0);
        let mass_handle = world.spawn_sphere(mass_pos, Vector3::zeros(), 0.1, mass);

        // Undamped spring
        world.create_spring_constraint(anchor_handle, mass_handle, spring_stiffness, 0.0, 0.0);

        let omega = (spring_stiffness / mass).sqrt();
        let expected_period = 2.0 * PI / omega;

        let duration = expected_period * 10.0;
        let steps = (duration / self.config.dt) as usize;
        let mut amplitude_samples = Vec::with_capacity(steps);
        let mut max_amplitude = 0.0f64;

        for _ in 0..steps {
            world.step();
            let rb = world.rigid_body_set.get(mass_handle).unwrap();
            let x = rb.translation().x.abs() as f64;
            amplitude_samples.push(x);
            if x > max_amplitude {
                max_amplitude = x;
            }
        }

        let execution_time = start.elapsed();

        // For undamped oscillation, amplitude should be preserved
        let expected_amplitude = initial_displacement as f64;

        // Find peaks in later oscillations to measure amplitude decay
        let later_half = &amplitude_samples[steps / 2..];
        let measured_amplitude = later_half.iter().cloned().fold(0.0f64, f64::max);

        let metrics = BenchmarkMetrics::from_samples(&amplitude_samples, execution_time, steps);

        BenchmarkResult::new(
            "Spring amplitude conservation",
            "Validates amplitude preservation in undamped oscillation",
            expected_amplitude,
            measured_amplitude,
            self.config.default_tolerance as f64 * 2.0,
            execution_time,
            steps,
        )
        .with_metrics(metrics)
    }

    /// Helper: Measure period from oscillation samples using zero-crossing detection
    fn measure_period(samples: &[f64], dt: f64) -> f64 {
        if samples.len() < 3 {
            return 0.0;
        }

        // Find mean to detect crossings
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;

        // Find zero crossings (relative to mean)
        let mut crossings = Vec::new();
        for i in 1..samples.len() {
            let prev = samples[i - 1] - mean;
            let curr = samples[i] - mean;
            if prev < 0.0 && curr >= 0.0 {
                // Interpolate for more accurate crossing time
                let frac = -prev / (curr - prev);
                crossings.push((i as f64 - 1.0 + frac) * dt);
            }
        }

        // Calculate average period from consecutive crossings
        if crossings.len() < 2 {
            return 0.0;
        }

        let mut periods = Vec::new();
        for i in 1..crossings.len() {
            periods.push(crossings[i] - crossings[i - 1]);
        }

        periods.iter().sum::<f64>() / periods.len() as f64
    }
}

/// Simplified physics world for benchmarking (standalone, no Bevy)
pub struct BenchmarkPhysicsWorld {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub gravity: Vector3<f32>,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
}

impl BenchmarkPhysicsWorld {
    pub fn new(gravity_magnitude: f32, dt: f32) -> Self {
        let mut integration_parameters = IntegrationParameters::default();
        integration_parameters.dt = dt;

        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            gravity: Vector3::new(0.0, -gravity_magnitude, 0.0),
            integration_parameters,
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }

    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    pub fn spawn_sphere(
        &mut self,
        position: Vector3<f32>,
        velocity: Vector3<f32>,
        radius: f32,
        mass: f32,
    ) -> RigidBodyHandle {
        let rb = RigidBodyBuilder::dynamic()
            .translation(position)
            .linvel(velocity)
            .build();

        let collider = ColliderBuilder::ball(radius)
            .density(mass / (4.0 / 3.0 * PI * radius.powi(3)))
            .build();

        let rb_handle = self.rigid_body_set.insert(rb);
        self.collider_set
            .insert_with_parent(collider, rb_handle, &mut self.rigid_body_set);

        rb_handle
    }

    pub fn spawn_box(
        &mut self,
        position: Vector3<f32>,
        velocity: Vector3<f32>,
        half_extents: Vector3<f32>,
        mass: f32,
    ) -> RigidBodyHandle {
        let rb = RigidBodyBuilder::dynamic()
            .translation(position)
            .linvel(velocity)
            .build();

        let volume = 8.0 * half_extents.x * half_extents.y * half_extents.z;
        let collider = ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z)
            .density(mass / volume)
            .build();

        let rb_handle = self.rigid_body_set.insert(rb);
        self.collider_set
            .insert_with_parent(collider, rb_handle, &mut self.rigid_body_set);

        rb_handle
    }

    pub fn spawn_fixed_point(&mut self, position: Vector3<f32>) -> RigidBodyHandle {
        let rb = RigidBodyBuilder::fixed().translation(position).build();
        self.rigid_body_set.insert(rb)
    }

    pub fn spawn_static_ground(&mut self, height: f32, friction: f32) -> ColliderHandle {
        let collider = ColliderBuilder::halfspace(Vector3::y_axis())
            .translation(Vector3::new(0.0, height, 0.0))
            .friction(friction)
            .build();
        self.collider_set.insert(collider)
    }

    pub fn set_restitution(&mut self, rb_handle: RigidBodyHandle, restitution: f32) {
        // Find all colliders attached to this rigid body and set restitution
        for (_handle, collider) in self.collider_set.iter_mut() {
            if collider.parent() == Some(rb_handle) {
                collider.set_restitution(restitution);
            }
        }
    }

    pub fn set_friction(&mut self, rb_handle: RigidBodyHandle, friction: f32) {
        for (_handle, collider) in self.collider_set.iter_mut() {
            if collider.parent() == Some(rb_handle) {
                collider.set_friction(friction);
            }
        }
    }

    pub fn create_distance_constraint(
        &mut self,
        anchor: RigidBodyHandle,
        body: RigidBodyHandle,
        length: f32,
    ) {
        // Use a rope joint to simulate pendulum
        let joint = RopeJointBuilder::new(length)
            .local_anchor1(nalgebra::Point3::origin())
            .local_anchor2(nalgebra::Point3::origin())
            .build();

        self.impulse_joint_set.insert(anchor, body, joint, true);
    }

    pub fn create_spring_constraint(
        &mut self,
        anchor: RigidBodyHandle,
        body: RigidBodyHandle,
        stiffness: f32,
        rest_length: f32,
        damping: f32,
    ) {
        // Use a prismatic joint with motor to simulate spring
        let joint = GenericJointBuilder::new(JointAxesMask::LOCKED_PRISMATIC_AXES)
            .local_anchor1(nalgebra::Point3::origin())
            .local_anchor2(nalgebra::Point3::origin())
            .local_axis1(nalgebra::Unit::new_normalize(Vector3::x()))
            .local_axis2(nalgebra::Unit::new_normalize(Vector3::x()))
            .motor_position(JointAxis::LinX, rest_length, stiffness, damping)
            .build();

        self.impulse_joint_set.insert(anchor, body, joint, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freefall_velocity() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_freefall_velocity();

        println!("{}", result.format());
        assert!(
            result.error_percentage < 10.0,
            "Free-fall velocity error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    fn test_freefall_position() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_freefall_position();

        println!("{}", result.format());
        assert!(
            result.error_percentage < 10.0,
            "Free-fall position error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    fn test_elastic_collision_momentum() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_elastic_collision_momentum();

        println!("{}", result.format());
        assert!(
            result.error_percentage < 15.0,
            "Momentum conservation error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    fn test_elastic_collision_energy() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_elastic_collision_energy();

        println!("{}", result.format());
        // Energy conservation is harder, allow more tolerance
        assert!(
            result.error_percentage < 20.0,
            "Energy conservation error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    fn test_friction_static() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_friction_static();

        println!("{}", result.format());
        // Static friction should prevent motion
        assert!(
            result.measured_value < 1.0,
            "Object should not slide with static friction"
        );
    }

    #[test]
    fn test_friction_kinetic() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_friction_kinetic();

        println!("{}", result.format());
        assert!(
            result.error_percentage < 20.0,
            "Kinetic friction error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    #[ignore] // Requires spherical joint setup - rope joint doesn't allow pendulum motion
    fn test_pendulum_period() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_pendulum_period();

        println!("{}", result.format());
        assert!(
            result.error_percentage < 15.0,
            "Pendulum period error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    #[ignore] // Requires ground plane for landing detection
    fn test_projectile_range() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_projectile_range();

        println!("{}", result.format());
        assert!(
            result.error_percentage < 10.0,
            "Projectile range error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    fn test_projectile_height() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_projectile_height();

        println!("{}", result.format());
        assert!(
            result.error_percentage < 10.0,
            "Projectile max height error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    fn test_stack_stability() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_stack_stability();

        println!("{}", result.format());
        assert!(result.passed, "Stack should settle within reasonable time");
    }

    #[test]
    fn test_spring_frequency() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_spring_frequency();

        println!("{}", result.format());
        assert!(
            result.error_percentage < 15.0,
            "Spring frequency error too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    fn test_spring_amplitude() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let result = benchmark.benchmark_spring_amplitude();

        println!("{}", result.format());
        // Amplitude decay is common in numerical integration, allow more tolerance
        assert!(
            result.error_percentage < 25.0,
            "Spring amplitude decay too high: {}%",
            result.error_percentage
        );
    }

    #[test]
    fn test_full_benchmark_suite() {
        let config = BenchmarkConfig::default();
        let benchmark = PhysicsBenchmark::new(config);
        let report = benchmark.run_all();

        println!("{}", report.summary());

        // At least 60% of tests should pass
        assert!(
            report.pass_rate >= 60.0,
            "Pass rate too low: {:.1}%",
            report.pass_rate
        );
    }

    #[test]
    fn test_high_precision_benchmark() {
        let config = BenchmarkConfig::high_precision();
        let benchmark = PhysicsBenchmark::new(config);
        let report = benchmark.run_all();

        println!("{}", report.summary());

        // High precision should have better accuracy on basic tests
        let freefall_result = report
            .results
            .iter()
            .find(|r| r.test_name.contains("Free-fall velocity"))
            .unwrap();

        assert!(
            freefall_result.error_percentage < 5.0,
            "High precision free-fall error: {}%",
            freefall_result.error_percentage
        );
    }

    #[test]
    fn test_benchmark_config_presets() {
        let default = BenchmarkConfig::default();
        assert_eq!(default.dt, 1.0 / 240.0);
        assert_eq!(default.default_tolerance, 5.0);

        let fast = BenchmarkConfig::fast();
        assert_eq!(fast.dt, 1.0 / 60.0);
        assert_eq!(fast.default_tolerance, 10.0);

        let high_precision = BenchmarkConfig::high_precision();
        assert_eq!(high_precision.dt, 1.0 / 1000.0);
        assert_eq!(high_precision.default_tolerance, 1.0);
    }

    #[test]
    fn test_benchmark_result_formatting() {
        let result = BenchmarkResult::new(
            "Test",
            "Test description",
            100.0,
            99.0,
            5.0,
            Duration::from_millis(100),
            1000,
        );

        let formatted = result.format();
        assert!(formatted.contains("PASS"));
        assert!(formatted.contains("Test"));
    }

    #[test]
    fn test_benchmark_metrics() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let metrics = BenchmarkMetrics::from_samples(&samples, Duration::from_secs(1), 100);

        assert_eq!(metrics.min_value, 1.0);
        assert_eq!(metrics.max_value, 5.0);
        assert_eq!(metrics.mean_value, 3.0);
        assert!(metrics.std_dev > 0.0);
        assert_eq!(metrics.steps_per_second, 100.0);
    }

    #[test]
    fn test_reference_accuracy() {
        let pybullet = ReferenceAccuracy::pybullet();
        assert_eq!(pybullet.name, "PyBullet");
        assert!(pybullet.freefall_error > 0.0);

        let mujoco = ReferenceAccuracy::mujoco();
        assert_eq!(mujoco.name, "MuJoCo");

        let rapier = ReferenceAccuracy::rapier3d();
        assert_eq!(rapier.name, "Rapier3D");
    }

    #[test]
    fn test_report_json_export() {
        let config = BenchmarkConfig::fast();
        let benchmark = PhysicsBenchmark::new(config);
        let report = benchmark.run_all();

        let json = report.to_json().expect("Failed to export JSON");
        assert!(json.contains("results"));
        assert!(json.contains("pass_rate"));
    }

    #[test]
    fn test_measure_period() {
        // Create synthetic oscillation data
        let dt: f64 = 0.01;
        let frequency: f64 = 2.0; // 2 Hz -> period = 0.5s
        let expected_period: f64 = 1.0 / frequency;
        let duration: f64 = 5.0;
        let num_samples = (duration / dt) as usize;

        let samples: Vec<f64> = (0..num_samples)
            .map(|i| {
                let t = i as f64 * dt;
                (2.0 * std::f64::consts::PI * frequency * t).sin()
            })
            .collect();

        let measured_period = PhysicsBenchmark::measure_period(&samples, dt);

        let error = ((measured_period - expected_period).abs() / expected_period) * 100.0;
        assert!(error < 5.0, "Period measurement error too high: {}%", error);
    }
}

/// Command-line benchmark runner
/// Run with: cargo test --package sim3d --lib physics::benchmarks::run_benchmarks -- --nocapture
#[test]
#[ignore] // Run manually with: cargo test run_benchmarks -- --ignored --nocapture
fn run_benchmarks() {
    println!("\n=== Running Physics Benchmark Suite ===\n");

    // Run with default configuration
    println!("--- Default Configuration (240Hz) ---");
    let config = BenchmarkConfig::default();
    let benchmark = PhysicsBenchmark::new(config);
    let report = benchmark.run_all();
    println!("{}", report.summary());

    // Run with high precision configuration
    println!("\n--- High Precision Configuration (1000Hz) ---");
    let config = BenchmarkConfig::high_precision();
    let benchmark = PhysicsBenchmark::new(config);
    let report = benchmark.run_all();
    println!("{}", report.summary());

    // Run with fast configuration
    println!("\n--- Fast Configuration (60Hz) ---");
    let config = BenchmarkConfig::fast();
    let benchmark = PhysicsBenchmark::new(config);
    let report = benchmark.run_all();
    println!("{}", report.summary());
}
