//! Integration test for domain randomization
//!
//! Tests that domain randomization properly affects environment physics,
//! visuals, and produces varied training scenarios without breaking the simulation.

use bevy::prelude::*;
use sim3d::rl::{
    domain_randomization::*,
    tasks::*,
    Action, RLTask,
};

#[test]
fn test_physics_randomization() {
    let config = DomainRandomizationConfig {
        enabled: true,
        randomize_physics: true,
        randomize_visual: false,
        randomize_env: false,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config);

    // Sample multiple times
    let mut masses = Vec::new();
    let mut frictions = Vec::new();
    let mut gravities = Vec::new();

    for _ in 0..20 {
        let params = sample_physics_params(&world);
        masses.push(params.mass);
        frictions.push(params.friction);
        gravities.push(params.gravity);
    }

    // Check that we got variation
    let mass_min = masses.iter().cloned().fold(f32::INFINITY, f32::min);
    let mass_max = masses.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(mass_max > mass_min, "Mass should vary");
    println!("Mass range: {} to {}", mass_min, mass_max);

    let friction_min = frictions.iter().cloned().fold(f32::INFINITY, f32::min);
    let friction_max = frictions.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(friction_max > friction_min, "Friction should vary");
    println!("Friction range: {} to {}", friction_min, friction_max);

    let gravity_min = gravities.iter().cloned().fold(f32::INFINITY, f32::min);
    let gravity_max = gravities.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(gravity_max > gravity_min, "Gravity should vary");
    println!("Gravity range: {} to {}", gravity_min, gravity_max);

    // Verify all values are within expected ranges
    for mass in masses {
        assert!(mass >= 0.8 && mass <= 1.2, "Mass out of range: {}", mass);
    }

    for friction in frictions {
        assert!(friction >= 0.3 && friction <= 0.9, "Friction out of range: {}", friction);
    }

    for gravity in gravities {
        assert!(gravity >= -12.0 && gravity <= -8.0, "Gravity out of range: {}", gravity);
    }
}

#[test]
fn test_visual_randomization() {
    let config = DomainRandomizationConfig {
        enabled: true,
        randomize_physics: false,
        randomize_visual: true,
        randomize_env: false,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config);

    // Sample multiple times
    let mut intensities = Vec::new();
    let mut color_temps = Vec::new();

    for _ in 0..20 {
        let params = sample_visual_params(&world);
        intensities.push(params.light_intensity);
        color_temps.push(params.color_temperature);
    }

    // Check variation
    let intensity_min = intensities.iter().cloned().fold(f32::INFINITY, f32::min);
    let intensity_max = intensities.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(intensity_max > intensity_min, "Light intensity should vary");
    println!("Intensity range: {} to {}", intensity_min, intensity_max);

    let temp_min = color_temps.iter().cloned().fold(f32::INFINITY, f32::min);
    let temp_max = color_temps.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(temp_max > temp_min, "Color temperature should vary");
    println!("Color temp range: {} to {}", temp_min, temp_max);

    // Verify ranges
    for intensity in intensities {
        assert!(intensity >= 500.0 && intensity <= 2000.0, "Intensity out of range: {}", intensity);
    }

    for temp in color_temps {
        assert!(temp >= 3000.0 && temp <= 7000.0, "Color temp out of range: {}", temp);
    }
}

#[test]
fn test_environment_randomization() {
    let config = DomainRandomizationConfig {
        enabled: true,
        randomize_physics: false,
        randomize_visual: false,
        randomize_env: true,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config);

    // Sample multiple times
    let mut positions = Vec::new();
    let mut rotations = Vec::new();
    let mut scales = Vec::new();

    for _ in 0..20 {
        let params = sample_environment_params(&world);
        positions.push(params.object_position);
        rotations.push(params.object_rotation);
        scales.push(params.object_scale);
    }

    // Check that positions vary
    let first_pos = positions[0];
    let all_same = positions.iter().all(|p| (p - first_pos).length() < 1e-6);
    assert!(!all_same, "Object positions should vary");

    // Check rotations vary
    let first_rot = rotations[0];
    let all_same_rot = rotations.iter().all(|r| r.abs_diff_eq(first_rot, 1e-6));
    assert!(!all_same_rot, "Object rotations should vary");

    // Check scales are within range
    for scale in scales {
        assert!(scale >= 0.9 && scale <= 1.1, "Scale out of range: {}", scale);
    }

    println!("Environment randomization working correctly");
}

#[test]
fn test_full_domain_randomization() {
    let config = DomainRandomizationConfig {
        enabled: true,
        randomize_physics: true,
        randomize_visual: true,
        randomize_env: true,
        physics_variation: 0.2,
        visual_variation: 0.3,
        env_variation: 0.15,
    };

    let mut world = World::new();
    world.insert_resource(config);

    // Run multiple sampling iterations
    for iteration in 0..10 {
        let physics = sample_physics_params(&world);
        let visual = sample_visual_params(&world);
        let env = sample_environment_params(&world);

        // Verify all parameters are valid
        assert!(physics.mass > 0.0);
        assert!(physics.friction >= 0.0);
        assert!(physics.restitution >= 0.0 && physics.restitution <= 1.0);
        assert!(visual.light_intensity > 0.0);
        assert!(visual.color_temperature > 0.0);
        assert!(env.object_scale > 0.0);

        if iteration == 0 {
            println!("Sample parameters:");
            println!("  Mass: {}", physics.mass);
            println!("  Friction: {}", physics.friction);
            println!("  Light: {} lux", visual.light_intensity);
            println!("  Color temp: {} K", visual.color_temperature);
            println!("  Scale: {}", env.object_scale);
        }
    }
}

#[test]
fn test_randomization_with_rl_task() {
    let config = DomainRandomizationConfig {
        enabled: true,
        randomize_physics: true,
        randomize_visual: false,
        randomize_env: true,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config);

    // Run task with randomization
    let mut task = ReachingTask::new(10, 6);

    // Reset multiple times and verify each reset produces valid state
    let mut observations = Vec::new();

    for episode in 0..5 {
        // Apply randomization before reset
        let _physics = sample_physics_params(&world);
        let _env = sample_environment_params(&world);

        let obs = task.reset(&mut world);
        observations.push(obs.clone());

        // Run a few steps
        for _ in 0..10 {
            let action = Action::Continuous(vec![0.1; 6]);
            let result = task.step(&mut world, &action);

            assert!(result.reward.is_finite(), "Episode {} produced invalid reward", episode);
            assert!(result.observation.len() == 10);

            if result.done || result.truncated {
                break;
            }
        }
    }

    println!("Completed 5 episodes with domain randomization");
}

#[test]
fn test_randomization_disabled() {
    let config = DomainRandomizationConfig {
        enabled: false,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config);

    // When disabled, should get consistent values
    let params1 = sample_physics_params(&world);
    let params2 = sample_physics_params(&world);

    // Default values should be returned
    assert_eq!(params1.mass, params2.mass);
    assert_eq!(params1.friction, params2.friction);
    assert_eq!(params1.gravity, params2.gravity);

    println!("Randomization correctly disabled");
}

#[test]
fn test_selective_randomization() {
    // Test enabling only certain types of randomization
    let config_physics_only = DomainRandomizationConfig {
        enabled: true,
        randomize_physics: true,
        randomize_visual: false,
        randomize_env: false,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config_physics_only);

    let physics1 = sample_physics_params(&world);
    let physics2 = sample_physics_params(&world);

    // Physics should vary
    let physics_varies = (physics1.mass - physics2.mass).abs() > 1e-6
        || (physics1.friction - physics2.friction).abs() > 1e-6;

    // May or may not vary on first try due to randomness, but should vary over multiple samples
    let mut any_variation = physics_varies;
    for _ in 0..10 {
        let p = sample_physics_params(&world);
        if (p.mass - physics1.mass).abs() > 1e-6 {
            any_variation = true;
            break;
        }
    }

    assert!(any_variation, "Physics randomization should produce variation");
}

#[test]
fn test_color_temperature_conversion() {
    let config = DomainRandomizationConfig {
        enabled: true,
        randomize_visual: true,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config);

    // Sample and check RGB conversion
    for _ in 0..10 {
        let params = sample_visual_params(&world);
        let rgb = params.light_color;

        // RGB values should be normalized [0, 1]
        assert!(rgb.r >= 0.0 && rgb.r <= 1.0, "Red out of range: {}", rgb.r);
        assert!(rgb.g >= 0.0 && rgb.g <= 1.0, "Green out of range: {}", rgb.g);
        assert!(rgb.b >= 0.0 && rgb.b <= 1.0, "Blue out of range: {}", rgb.b);

        // Color should not be pure black or white (except in extreme cases)
        let sum = rgb.r + rgb.g + rgb.b;
        assert!(sum > 0.1, "Color too dark");
    }
}

#[test]
fn test_randomization_reproducibility() {
    // Test that randomization produces different results on subsequent calls
    let config = DomainRandomizationConfig {
        enabled: true,
        randomize_physics: true,
        physics_variation: 0.2,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config);

    let mut params_set = Vec::new();
    for _ in 0..20 {
        let params = sample_physics_params(&world);
        params_set.push((params.mass, params.friction));
    }

    // Check that not all samples are identical
    let first = params_set[0];
    let all_same = params_set.iter().all(|p| {
        (p.0 - first.0).abs() < 1e-6 && (p.1 - first.1).abs() < 1e-6
    });

    assert!(!all_same, "Randomization should produce varied samples");

    // Check that we have reasonable distribution (not all at extremes)
    let avg_mass: f32 = params_set.iter().map(|p| p.0).sum::<f32>() / params_set.len() as f32;
    assert!(avg_mass >= 0.85 && avg_mass <= 1.15, "Average mass out of expected range: {}", avg_mass);
}

#[test]
fn test_distractor_objects() {
    let config = DomainRandomizationConfig {
        enabled: true,
        randomize_env: true,
        ..Default::default()
    };

    let mut world = World::new();
    world.insert_resource(config);

    // Sample environment parameters multiple times
    let mut distractor_counts = Vec::new();
    for _ in 0..20 {
        let params = sample_environment_params(&world);
        distractor_counts.push(params.num_distractors);
    }

    // Should have variety in distractor counts
    let min_distractors = *distractor_counts.iter().min().unwrap();
    let max_distractors = *distractor_counts.iter().max().unwrap();

    println!("Distractor range: {} to {}", min_distractors, max_distractors);

    // All should be within valid range
    for count in distractor_counts {
        assert!(count <= 3, "Too many distractors: {}", count);
    }
}
