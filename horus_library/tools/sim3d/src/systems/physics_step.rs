use bevy::prelude::*;
use crate::physics::PhysicsWorld;

static mut ACCUMULATOR: f32 = 0.0;

pub fn physics_step_system(mut physics_world: ResMut<PhysicsWorld>, time: Res<Time>) {
    const FIXED_DT: f32 = 1.0 / 240.0; // 240 Hz physics

    unsafe {
        ACCUMULATOR += time.delta_secs();
        while ACCUMULATOR >= FIXED_DT {
            physics_world.step();
            ACCUMULATOR -= FIXED_DT;
        }
    }
}
