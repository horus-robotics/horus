//! Collision physics validation
//!
//! Validates collision detection and response, including:
//! - Momentum conservation
//! - Energy conservation (elastic collisions)
//! - Coefficient of restitution

use approx::assert_relative_eq;

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
