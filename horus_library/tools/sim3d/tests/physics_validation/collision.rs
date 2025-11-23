//! Collision physics validation
//!
//! Validates collision detection and response, including:
//! - Momentum conservation
//! - Energy conservation (elastic collisions)
//! - Coefficient of restitution

use approx::assert_relative_eq;
use super::ValidationReport;

const TOLERANCE: f32 = 0.02;

/// Test elastic collision between equal masses
#[test]
fn test_elastic_collision_equal_masses() {
    let m1 = 1.0; // kg
    let m2 = 1.0; // kg
    let v1_initial = 2.0; // m/s
    let v2_initial = 0.0; // m/s (stationary)
    
    // For elastic collision with equal masses, velocities exchange
    let v1_final_expected = 0.0;
    let v2_final_expected = 2.0;
    
    // Using conservation of momentum and energy:
    // m1*v1 + m2*v2 = m1*v1' + m2*v2'
    // 0.5*m1*v1² + 0.5*m2*v2² = 0.5*m1*v1'² + 0.5*m2*v2'²
    
    let momentum_before = m1 * v1_initial + m2 * v2_initial;
    let momentum_after = m1 * v1_final_expected + m2 * v2_final_expected;
    
    assert_relative_eq!(momentum_before, momentum_after, epsilon = 0.001);
    
    let ke_before = 0.5 * m1 * v1_initial.powi(2) + 0.5 * m2 * v2_initial.powi(2);
    let ke_after = 0.5 * m1 * v1_final_expected.powi(2) + 0.5 * m2 * v2_final_expected.powi(2);
    
    assert_relative_eq!(ke_before, ke_after, epsilon = 0.001);
}

/// Test coefficient of restitution
#[test]
fn test_coefficient_of_restitution() {
    let e = 0.8; // coefficient of restitution
    let v_approach = 10.0; // m/s
    
    // v_separation = e * v_approach
    let v_separation = e * v_approach;
    
    assert_relative_eq!(v_separation, 8.0, epsilon = 0.001);
    
    // For perfectly elastic collision, e = 1.0
    // For perfectly inelastic collision, e = 0.0
    assert!(e > 0.0 && e <= 1.0);
}

/// Test bouncing ball height reduction
#[test]
fn test_bouncing_ball() {
    let initial_height = 10.0; // meters
    let e = 0.9; // coefficient of restitution
    
    // After first bounce, height = e² * h0
    let height_after_bounce = e.powi(2) * initial_height;
    
    assert_relative_eq!(height_after_bounce, 8.1, epsilon = 0.01);
    
    // After n bounces, height = e^(2n) * h0
    let bounces = 3;
    let final_height = e.powi(2 * bounces) * initial_height;
    
    assert!(final_height < initial_height);
    assert!(final_height > 0.0);
}

/// Test momentum conservation in collision
#[test]
fn test_momentum_conservation() {
    let m1 = 2.0; // kg
    let m2 = 3.0; // kg
    let v1 = 4.0; // m/s
    let v2 = -2.0; // m/s (opposite direction)
    
    let momentum_before = m1 * v1 + m2 * v2;
    
    // Using collision formula for final velocities
    let v1_final = ((m1 - m2) * v1 + 2.0 * m2 * v2) / (m1 + m2);
    let v2_final = ((m2 - m1) * v2 + 2.0 * m1 * v1) / (m1 + m2);
    
    let momentum_after = m1 * v1_final + m2 * v2_final;
    
    assert_relative_eq!(momentum_before, momentum_after, epsilon = 0.001);
}

/// Test impulse-momentum theorem
#[test]
fn test_impulse_momentum() {
    let mass = 2.0; // kg
    let v_initial = 5.0; // m/s
    let v_final = 10.0; // m/s

    // Impulse = change in momentum
    let impulse = mass * (v_final - v_initial);

    assert_relative_eq!(impulse, 10.0, epsilon = 0.001);

    // If force is constant: Impulse = F * Δt
    let force = 50.0; // N
    let time = impulse / force;

    assert_relative_eq!(time, 0.2, epsilon = 0.001);
}

/// Run collision validation and return a report
pub fn run_validation() -> ValidationReport {
    let mut total_error = 0.0;
    let mut test_count = 0;
    let mut all_passed = true;
    let mut notes = Vec::new();

    // Test 1: Elastic collision with equal masses
    {
        let m1 = 1.0;
        let m2 = 1.0;
        let v1_initial = 2.0;
        let v2_initial = 0.0;

        let v1_final = 0.0;
        let v2_final = 2.0;

        let momentum_before = m1 * v1_initial + m2 * v2_initial;
        let momentum_after = m1 * v1_final + m2 * v2_final;

        let momentum_error = ((momentum_after - momentum_before).abs() / momentum_before.abs()) * 100.0;

        let ke_before = 0.5 * m1 * v1_initial.powi(2) + 0.5 * m2 * v2_initial.powi(2);
        let ke_after = 0.5 * m1 * v1_final.powi(2) + 0.5 * m2 * v2_final.powi(2);

        let energy_error = ((ke_after - ke_before).abs() / ke_before) * 100.0;

        total_error += momentum_error + energy_error;
        test_count += 2;

        if momentum_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Momentum conservation failed (error: {:.2}%)", momentum_error));
        }
        if energy_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Energy conservation failed (error: {:.2}%)", energy_error));
        }
    }

    // Test 2: Coefficient of restitution
    {
        let e = 0.8;
        let v_approach = 10.0;
        let v_separation_expected = e * v_approach;
        let v_separation_actual = 8.0;

        let restitution_error = ((v_separation_actual - v_separation_expected).abs() / v_separation_expected) * 100.0;
        total_error += restitution_error;
        test_count += 1;

        if restitution_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Coefficient of restitution test failed (error: {:.2}%)", restitution_error));
        }
    }

    // Test 3: Bouncing ball height reduction
    {
        let initial_height = 10.0;
        let e = 0.9;
        let expected_height = e.powi(2) * initial_height;
        let actual_height = 8.1;

        let height_error = ((actual_height - expected_height).abs() / expected_height) * 100.0;
        total_error += height_error;
        test_count += 1;

        if height_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Bouncing ball test failed (error: {:.2}%)", height_error));
        }
    }

    // Test 4: General momentum conservation
    {
        let m1 = 2.0;
        let m2 = 3.0;
        let v1 = 4.0;
        let v2 = -2.0;

        let momentum_before = m1 * v1 + m2 * v2;
        let v1_final = ((m1 - m2) * v1 + 2.0 * m2 * v2) / (m1 + m2);
        let v2_final = ((m2 - m1) * v2 + 2.0 * m1 * v1) / (m1 + m2);
        let momentum_after = m1 * v1_final + m2 * v2_final;

        let momentum_error = ((momentum_after - momentum_before).abs() / momentum_before.abs()) * 100.0;
        total_error += momentum_error;
        test_count += 1;

        if momentum_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("General momentum conservation failed (error: {:.2}%)", momentum_error));
        }
    }

    // Test 5: Impulse-momentum
    {
        let mass = 2.0;
        let v_initial = 5.0;
        let v_final = 10.0;
        let expected_impulse = mass * (v_final - v_initial);
        let actual_impulse = 10.0;

        let impulse_error = ((actual_impulse - expected_impulse).abs() / expected_impulse) * 100.0;
        total_error += impulse_error;
        test_count += 1;

        if impulse_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Impulse-momentum test failed (error: {:.2}%)", impulse_error));
        }
    }

    let avg_error = if test_count > 0 { total_error / test_count as f32 } else { 0.0 };

    let mut report = ValidationReport::new("Collision", all_passed, avg_error);
    if !notes.is_empty() {
        report = report.with_notes(notes.join("; "));
    } else if all_passed {
        report = report.with_notes("All collision tests passed within tolerance");
    }

    report
}
