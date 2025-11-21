//! Friction physics validation
//!
//! Validates friction forces including:
//! - Static friction threshold
//! - Kinetic friction
//! - Sliding motion on inclined planes

use approx::assert_relative_eq;
use std::f32::consts::PI;

const GRAVITY: f32 = 9.81;
const TOLERANCE: f32 = 0.02;

/// Test static friction threshold
#[test]
fn test_static_friction_threshold() {
    let mass = 10.0; // kg
    let mu_static = 0.6; // coefficient of static friction
    
    let normal_force = mass * GRAVITY;
    let max_static_friction = mu_static * normal_force;
    
    // Object won't move if applied force < max static friction
    assert_relative_eq!(max_static_friction, 58.86, epsilon = 0.01);
    
    // Applied force just below threshold - no motion
    let force_below = 58.0;
    assert!(force_below < max_static_friction);
    
    // Applied force above threshold - motion starts
    let force_above = 60.0;
    assert!(force_above > max_static_friction);
}

/// Test kinetic friction
#[test]
fn test_kinetic_friction() {
    let mass = 5.0; // kg
    let mu_kinetic = 0.4; // coefficient of kinetic friction
    
    let normal_force = mass * GRAVITY;
    let friction_force = mu_kinetic * normal_force;
    
    assert_relative_eq!(friction_force, 19.62, epsilon = 0.01);
    
    // Kinetic friction is typically less than static friction
    let mu_static = 0.6;
    assert!(mu_kinetic < mu_static);
}

/// Test sliding on inclined plane
#[test]
fn test_inclined_plane() {
    let mass = 2.0; // kg
    let angle = 30.0 * PI / 180.0; // 30 degrees
    let mu = 0.3; // coefficient of friction
    
    // Component of gravity parallel to plane
    let f_parallel = mass * GRAVITY * angle.sin();
    
    // Normal force
    let normal = mass * GRAVITY * angle.cos();
    
    // Friction force
    let f_friction = mu * normal;
    
    // Net force down the plane
    let f_net = f_parallel - f_friction;
    
    // Acceleration down the plane
    let acceleration = f_net / mass;
    
    // Should slide if f_parallel > f_friction
    assert!(f_parallel > f_friction);
    assert!(acceleration > 0.0);
}

/// Test critical angle for sliding
#[test]
fn test_critical_angle() {
    let mu_static = 0.5;
    
    // Critical angle where object just starts to slide
    // tan(θ_critical) = μ_s
    let theta_critical = mu_static.atan();
    
    // Convert to degrees
    let theta_degrees = theta_critical * 180.0 / PI;
    
    assert_relative_eq!(theta_degrees, 26.565, epsilon = 0.01);
    
    // Object slides if angle > critical angle
    let test_angle = 30.0 * PI / 180.0;
    assert!(test_angle > theta_critical);
}

/// Test deceleration due to friction
#[test]
fn test_friction_deceleration() {
    let mass = 3.0; // kg
    let initial_velocity = 10.0; // m/s
    let mu_kinetic = 0.2;
    
    // Friction force opposes motion
    let friction_force = mu_kinetic * mass * GRAVITY;
    
    // Deceleration = friction_force / mass = μ * g
    let deceleration = mu_kinetic * GRAVITY;
    
    assert_relative_eq!(deceleration, 1.962, epsilon = 0.01);
    
    // Time to stop = v / a
    let time_to_stop = initial_velocity / deceleration;
    
    assert_relative_eq!(time_to_stop, 5.097, epsilon = 0.01);
    
    // Distance to stop = v² / (2a)
    let stopping_distance = initial_velocity.powi(2) / (2.0 * deceleration);
    
    assert_relative_eq!(stopping_distance, 25.486, epsilon = 0.01);
}

/// Test work done against friction
#[test]
fn test_friction_work() {
    let friction_force = 20.0; // N
    let distance = 5.0; // m
    
    // Work = Force * distance
    let work = friction_force * distance;
    
    assert_relative_eq!(work, 100.0, epsilon = 0.001);
    
    // This work is converted to heat
    // Energy is conserved but transforms from kinetic to thermal
}
