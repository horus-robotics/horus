//! Pendulum physics validation
//!
//! Validates periodic motion and energy conservation principles.
//! Tests simple pendulum equation: T = 2π√(L/g)

use approx::assert_relative_eq;
use std::f32::consts::PI;

const GRAVITY: f32 = 9.81;
const TOLERANCE: f32 = 0.05; // 5% tolerance

/// Test pendulum period formula
#[test]
fn test_pendulum_period() {
    let length = 1.0; // 1 meter
    
    // T = 2π√(L/g)
    let expected_period = 2.0 * PI * (length / GRAVITY).sqrt();
    
    // For 1m pendulum, period should be ~2.006 seconds
    assert_relative_eq!(expected_period, 2.006, epsilon = 0.01);
}

/// Test small angle approximation
#[test]
fn test_small_angle_approximation() {
    // For small angles, sin(θ) ≈ θ
    let small_angle = 0.1; // radians (~5.7 degrees)
    
    let sin_theta = small_angle.sin();
    let error = (sin_theta - small_angle).abs() / small_angle;
    
    // Error should be < 1% for small angles
    assert!(error < 0.01);
}

/// Test energy conservation
#[test]
fn test_energy_conservation_principle() {
    let mass = 1.0; // kg
    let length = 1.0; // m
    let initial_angle = PI / 6.0; // 30 degrees
    
    // Initial potential energy (relative to lowest point)
    let height = length * (1.0 - initial_angle.cos());
    let pe_initial = mass * GRAVITY * height;
    
    // At lowest point, all PE converts to KE
    // KE = 0.5 * m * v^2 = PE_initial
    let velocity_at_bottom = (2.0 * GRAVITY * height).sqrt();
    let ke_at_bottom = 0.5 * mass * velocity_at_bottom * velocity_at_bottom;
    
    assert_relative_eq!(ke_at_bottom, pe_initial, epsilon = 0.001);
}

/// Test angular frequency
#[test]
fn test_angular_frequency() {
    let length = 1.0;
    
    // ω = √(g/L)
    let angular_frequency = (GRAVITY / length).sqrt();
    
    // Period T = 2π/ω
    let period = 2.0 * PI / angular_frequency;
    
    let expected_period = 2.0 * PI * (length / GRAVITY).sqrt();
    assert_relative_eq!(period, expected_period, epsilon = 0.001);
}
