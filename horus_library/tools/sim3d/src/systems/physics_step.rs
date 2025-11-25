use crate::physics::PhysicsWorld;
use bevy::prelude::*;

/// Physics accumulator resource for fixed timestep simulation
#[derive(Resource, Default)]
pub struct PhysicsAccumulator {
    pub accumulated_time: f32,
}

pub fn physics_step_system(
    mut physics_world: ResMut<PhysicsWorld>,
    mut accumulator: ResMut<PhysicsAccumulator>,
    time: Res<Time>,
) {
    const FIXED_DT: f32 = 1.0 / 240.0; // 240 Hz physics

    accumulator.accumulated_time += time.delta_secs();
    while accumulator.accumulated_time >= FIXED_DT {
        physics_world.step();
        accumulator.accumulated_time -= FIXED_DT;
    }
}
