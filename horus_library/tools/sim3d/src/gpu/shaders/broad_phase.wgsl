// GPU Broad-Phase Collision Detection
// Performs AABB overlap tests in parallel

struct AABB {
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
}

struct CollisionPair {
    object_a: u32,
    object_b: u32,
}

struct Uniforms {
    num_objects: u32,
    padding: vec3<u32>,
}

@group(0) @binding(0) var<storage, read> aabbs: array<AABB>;
@group(0) @binding(1) var<storage, read_write> collision_pairs: array<u32>;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

// Check if two AABBs overlap
fn aabb_overlap(a: AABB, b: AABB) -> bool {
    return (a.max_x >= b.min_x && a.min_x <= b.max_x) &&
           (a.max_y >= b.min_y && a.min_y <= b.max_y) &&
           (a.max_z >= b.min_z && a.min_z <= b.max_z);
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;

    // Bounds check
    if (i >= uniforms.num_objects) {
        return;
    }

    let aabb_i = aabbs[i];

    // Check against all other objects (j > i to avoid duplicates)
    for (var j = i + 1u; j < uniforms.num_objects; j++) {
        let aabb_j = aabbs[j];

        if (aabb_overlap(aabb_i, aabb_j)) {
            // Atomically add collision pair
            let pair_idx = atomicAdd(&collision_pairs[0], 1u);
            let base_idx = (pair_idx * 2u) + 1u;

            collision_pairs[base_idx] = i;
            collision_pairs[base_idx + 1u] = j;
        }
    }
}
