// GPU Rigid Body Integration
// Semi-implicit Euler integration for rigid body dynamics

struct Uniforms {
    dt: f32,
    linear_damping: f32,
    num_objects: u32,
    padding: u32,
}

@group(0) @binding(0) var<storage, read_write> positions: array<vec3<f32>>;
@group(0) @binding(1) var<storage, read_write> velocities: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read> forces: array<vec3<f32>>;
@group(0) @binding(3) var<storage, read> masses: array<f32>;
@group(0) @binding(4) var<uniform> uniforms: Uniforms;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;

    if (idx >= uniforms.num_objects) {
        return;
    }

    let mass = masses[idx];
    let force = forces[idx];
    let vel = velocities[idx];
    let pos = positions[idx];

    // Semi-implicit Euler: v' = v + (F/m) * dt
    let acceleration = force / mass;
    var new_velocity = vel + acceleration * uniforms.dt;

    // Apply damping
    new_velocity = new_velocity * (1.0 - uniforms.linear_damping * uniforms.dt);

    // Update position: p' = p + v' * dt
    let new_position = pos + new_velocity * uniforms.dt;

    // Write back
    velocities[idx] = new_velocity;
    positions[idx] = new_position;
}
