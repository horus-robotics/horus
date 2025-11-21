// GPU Raycasting for LiDAR and Depth Sensors
// Möller-Trumbore ray-triangle intersection

struct Uniforms {
    num_rays: u32,
    num_triangles: u32,
    max_distance: f32,
    padding: f32,
}

@group(0) @binding(0) var<storage, read> ray_origins: array<vec3<f32>>;
@group(0) @binding(1) var<storage, read> ray_directions: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read> triangles: array<vec3<f32>>; // Flattened: 3 vertices per triangle
@group(0) @binding(3) var<storage, read_write> hit_distances: array<f32>;
@group(0) @binding(4) var<uniform> uniforms: Uniforms;

const EPSILON: f32 = 0.00001;

// Möller-Trumbore ray-triangle intersection
fn ray_triangle_intersection(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    v0: vec3<f32>,
    v1: vec3<f32>,
    v2: vec3<f32>
) -> f32 {
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = cross(ray_dir, edge2);
    let a = dot(edge1, h);

    // Parallel to triangle
    if (abs(a) < EPSILON) {
        return uniforms.max_distance;
    }

    let f = 1.0 / a;
    let s = ray_origin - v0;
    let u = f * dot(s, h);

    // Outside triangle
    if (u < 0.0 || u > 1.0) {
        return uniforms.max_distance;
    }

    let q = cross(s, edge1);
    let v = f * dot(ray_dir, q);

    // Outside triangle
    if (v < 0.0 || u + v > 1.0) {
        return uniforms.max_distance;
    }

    // Compute distance along ray
    let t = f * dot(edge2, q);

    if (t > EPSILON) {
        return t;
    }

    return uniforms.max_distance;
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let ray_idx = global_id.x;

    if (ray_idx >= uniforms.num_rays) {
        return;
    }

    let origin = ray_origins[ray_idx];
    let direction = ray_directions[ray_idx];

    var min_distance = uniforms.max_distance;

    // Test against all triangles
    for (var tri_idx = 0u; tri_idx < uniforms.num_triangles; tri_idx++) {
        let base = tri_idx * 3u;
        let v0 = triangles[base];
        let v1 = triangles[base + 1u];
        let v2 = triangles[base + 2u];

        let distance = ray_triangle_intersection(origin, direction, v0, v1, v2);

        if (distance < min_distance) {
            min_distance = distance;
        }
    }

    hit_distances[ray_idx] = min_distance;
}
