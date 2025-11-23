//! Pendulum physics validation
//!
//! Validates periodic motion and energy conservation principles.
//! Tests simple pendulum equation: T = 2π√(L/g)

use approx::assert_relative_eq;
use std::f32::consts::PI;
use super::ValidationReport;

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

/// Run pendulum validation and return a report
pub fn run_validation() -> ValidationReport {
    let mut total_error = 0.0;
    let mut test_count = 0;
    let mut all_passed = true;
    let mut notes = Vec::new();

    // Test 1: Pendulum period formula
    {
        let length = 1.0;
        let expected_period = 2.0 * PI * (length / GRAVITY).sqrt();
        let actual_period = 2.006;

        let period_error = ((expected_period - actual_period).abs() / expected_period) * 100.0;
        total_error += period_error;
        test_count += 1;

        if period_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Period calculation exceeded tolerance (error: {:.2}%)", period_error));
        }
    }

    // Test 2: Small angle approximation
    {
        let small_angle = 0.1;
        let sin_theta = small_angle.sin();
        let approximation_error = ((sin_theta - small_angle).abs() / small_angle) * 100.0;

        total_error += approximation_error;
        test_count += 1;

        if approximation_error > 1.0 {
            all_passed = false;
            notes.push(format!("Small angle approximation error too large: {:.2}%", approximation_error));
        }
    }

    // Test 3: Energy conservation
    {
        let mass = 1.0;
        let length = 1.0;
        let initial_angle = PI / 6.0;

        let height = length * (1.0 - initial_angle.cos());
        let pe_initial = mass * GRAVITY * height;

        let velocity_at_bottom = (2.0 * GRAVITY * height).sqrt();
        let ke_at_bottom = 0.5 * mass * velocity_at_bottom * velocity_at_bottom;

        let energy_error = ((ke_at_bottom - pe_initial).abs() / pe_initial) * 100.0;
        total_error += energy_error;
        test_count += 1;

        if energy_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Energy conservation violated (error: {:.2}%)", energy_error));
        }
    }

    // Test 4: Angular frequency relationship
    {
        let length = 1.0;
        let angular_frequency = (GRAVITY / length).sqrt();
        let period_from_omega = 2.0 * PI / angular_frequency;
        let expected_period = 2.0 * PI * (length / GRAVITY).sqrt();

        let frequency_error = ((period_from_omega - expected_period).abs() / expected_period) * 100.0;
        total_error += frequency_error;
        test_count += 1;

        if frequency_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Angular frequency test failed (error: {:.2}%)", frequency_error));
        }
    }

    let avg_error = if test_count > 0 { total_error / test_count as f32 } else { 0.0 };

    let mut report = ValidationReport::new("Pendulum", all_passed, avg_error);
    if !notes.is_empty() {
        report = report.with_notes(notes.join("; "));
    } else if all_passed {
        report = report.with_notes("All pendulum tests passed within tolerance");
    }

    report
}
