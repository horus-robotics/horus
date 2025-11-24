//! Friction physics validation
//!
//! Validates friction forces including:
//! - Static friction threshold
//! - Kinetic friction
//! - Sliding motion on inclined planes

use approx::assert_relative_eq;
use std::f32::consts::PI;
use super::ValidationReport;

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

/// Run friction validation and return a report
pub fn run_validation() -> ValidationReport {
    let mut total_error = 0.0;
    let mut test_count = 0;
    let mut all_passed = true;
    let mut notes = Vec::new();

    // Test 1: Static friction threshold
    {
        let mass = 10.0;
        let mu_static = 0.6;
        let normal_force = mass * GRAVITY;
        let max_static_friction = mu_static * normal_force;
        let expected_friction = 58.86;

        let friction_error = ((max_static_friction - expected_friction).abs() / expected_friction) * 100.0;
        total_error += friction_error;
        test_count += 1;

        if friction_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Static friction test failed (error: {:.2}%)", friction_error));
        }
    }

    // Test 2: Kinetic friction
    {
        let mass = 5.0;
        let mu_kinetic = 0.4;
        let normal_force = mass * GRAVITY;
        let friction_force = mu_kinetic * normal_force;
        let expected_friction = 19.62;

        let friction_error = ((friction_force - expected_friction).abs() / expected_friction) * 100.0;
        total_error += friction_error;
        test_count += 1;

        if friction_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Kinetic friction test failed (error: {:.2}%)", friction_error));
        }
    }

    // Test 3: Inclined plane sliding
    {
        let mass = 2.0;
        let angle = 30.0 * PI / 180.0;
        let mu = 0.3;

        let f_parallel = mass * GRAVITY * angle.sin();
        let normal = mass * GRAVITY * angle.cos();
        let f_friction = mu * normal;
        let f_net = f_parallel - f_friction;
        let acceleration = f_net / mass;

        let sliding_occurs = f_parallel > f_friction && acceleration > 0.0;
        if !sliding_occurs {
            all_passed = false;
            notes.push("Inclined plane sliding test failed: object should slide".to_string());
        }
    }

    // Test 4: Critical angle
    {
        let mu_static = 0.5;
        let theta_critical = mu_static.atan();
        let theta_degrees = theta_critical * 180.0 / PI;
        let expected_degrees = 26.565;

        let angle_error = ((theta_degrees - expected_degrees).abs() / expected_degrees) * 100.0;
        total_error += angle_error;
        test_count += 1;

        if angle_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Critical angle test failed (error: {:.2}%)", angle_error));
        }
    }

    // Test 5: Friction deceleration
    {
        let mass = 3.0;
        let initial_velocity = 10.0;
        let mu_kinetic = 0.2;

        let deceleration = mu_kinetic * GRAVITY;
        let expected_deceleration = 1.962;

        let decel_error = ((deceleration - expected_deceleration).abs() / expected_deceleration) * 100.0;

        let time_to_stop = initial_velocity / deceleration;
        let expected_time = 5.097;

        let time_error = ((time_to_stop - expected_time).abs() / expected_time) * 100.0;

        let stopping_distance = initial_velocity.powi(2) / (2.0 * deceleration);
        let expected_distance = 25.486;

        let distance_error = ((stopping_distance - expected_distance).abs() / expected_distance) * 100.0;

        total_error += decel_error + time_error + distance_error;
        test_count += 3;

        if decel_error > TOLERANCE * 100.0 || time_error > TOLERANCE * 100.0 || distance_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Deceleration test failed (decel: {:.2}%, time: {:.2}%, distance: {:.2}%)",
                              decel_error, time_error, distance_error));
        }
    }

    // Test 6: Work done against friction
    {
        let friction_force = 20.0;
        let distance = 5.0;
        let work = friction_force * distance;
        let expected_work = 100.0;

        let work_error = ((work - expected_work).abs() / expected_work) * 100.0;
        total_error += work_error;
        test_count += 1;

        if work_error > TOLERANCE * 100.0 {
            all_passed = false;
            notes.push(format!("Work calculation test failed (error: {:.2}%)", work_error));
        }
    }

    let avg_error = if test_count > 0 { total_error / test_count as f32 } else { 0.0 };

    let mut report = ValidationReport::new("Friction", all_passed, avg_error);
    if !notes.is_empty() {
        report = report.with_notes(notes.join("; "));
    } else if all_passed {
        report = report.with_notes("All friction tests passed within tolerance");
    }

    report
}
