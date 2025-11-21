//! Mesh processing utilities
//!
//! Functions for normal generation, tangent generation, and bounding box calculation

use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::render::primitives::Aabb;

/// Generate smooth normals for a mesh
pub fn generate_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];

    // Accumulate face normals
    for triangle in indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        if i0 >= positions.len() || i1 >= positions.len() || i2 >= positions.len() {
            continue;
        }

        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let normal = edge1.cross(edge2).normalize_or_zero();

        // Accumulate normals at each vertex
        normals[i0] = (Vec3::from(normals[i0]) + normal).to_array();
        normals[i1] = (Vec3::from(normals[i1]) + normal).to_array();
        normals[i2] = (Vec3::from(normals[i2]) + normal).to_array();
    }

    // Normalize accumulated normals
    for normal in &mut normals {
        let n = Vec3::from(*normal).normalize_or_zero();
        *normal = n.to_array();
    }

    normals
}

/// Generate flat normals (one per triangle, not smoothed)
pub fn generate_flat_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = Vec::with_capacity(positions.len());

    for triangle in indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        if i0 >= positions.len() || i1 >= positions.len() || i2 >= positions.len() {
            continue;
        }

        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let normal = edge1.cross(edge2).normalize_or_zero();
        let normal_array = normal.to_array();

        // All three vertices of the triangle get the same normal
        normals.push(normal_array);
        normals.push(normal_array);
        normals.push(normal_array);
    }

    normals
}

/// Calculate axis-aligned bounding box for a set of positions
pub fn calculate_aabb(positions: &[[f32; 3]]) -> Aabb {
    if positions.is_empty() {
        return Aabb {
            center: Vec3::ZERO.into(),
            half_extents: Vec3::ZERO.into(),
        };
    }

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);

    for pos in positions {
        let p = Vec3::from(*pos);
        min = min.min(p);
        max = max.max(p);
    }

    let center = (min + max) * 0.5;
    let half_extents = (max - min) * 0.5;

    Aabb {
        center: center.into(),
        half_extents: half_extents.into(),
    }
}

/// Apply scale transform to positions
pub fn scale_positions(positions: &mut [[f32; 3]], scale: Vec3) {
    for pos in positions {
        pos[0] *= scale.x;
        pos[1] *= scale.y;
        pos[2] *= scale.z;
    }
}

/// Flip UV coordinates vertically
pub fn flip_uvs(uvs: &mut [[f32; 2]]) {
    for uv in uvs {
        uv[1] = 1.0 - uv[1];
    }
}

/// Count triangles in index buffer
pub fn count_triangles(indices: Option<&Indices>) -> usize {
    match indices {
        Some(Indices::U16(idx)) => idx.len() / 3,
        Some(Indices::U32(idx)) => idx.len() / 3,
        None => 0,
    }
}

/// Count vertices in mesh
pub fn count_vertices(mesh: &bevy::render::mesh::Mesh) -> usize {
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION)
    {
        positions.len()
    } else {
        0
    }
}

/// Generate tangents for normal mapping (requires positions, normals, and UVs)
pub fn generate_tangents(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    indices: &[u32],
) -> Vec<[f32; 4]> {
    let mut tangents = vec![[0.0f32; 4]; positions.len()];
    let mut bitangents = vec![Vec3::ZERO; positions.len()];

    // Calculate tangents and bitangents per triangle
    for triangle in indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;

        if i0 >= positions.len() || i1 >= positions.len() || i2 >= positions.len() {
            continue;
        }

        let v0 = Vec3::from(positions[i0]);
        let v1 = Vec3::from(positions[i1]);
        let v2 = Vec3::from(positions[i2]);

        let uv0 = Vec2::from(uvs[i0]);
        let uv1 = Vec2::from(uvs[i1]);
        let uv2 = Vec2::from(uvs[i2]);

        let delta_pos1 = v1 - v0;
        let delta_pos2 = v2 - v0;

        let delta_uv1 = uv1 - uv0;
        let delta_uv2 = uv2 - uv0;

        let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);

        let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
        let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * r;

        // Accumulate tangents and bitangents
        for &idx in triangle {
            let idx = idx as usize;
            let t = Vec3::from([tangents[idx][0], tangents[idx][1], tangents[idx][2]]);
            let new_t = (t + tangent).to_array();
            tangents[idx][0] = new_t[0];
            tangents[idx][1] = new_t[1];
            tangents[idx][2] = new_t[2];

            bitangents[idx] += bitangent;
        }
    }

    // Orthogonalize and normalize
    for i in 0..positions.len() {
        let n = Vec3::from(normals[i]);
        let t = Vec3::from([tangents[i][0], tangents[i][1], tangents[i][2]]);
        let b = bitangents[i];

        // Gram-Schmidt orthogonalize
        let t = (t - n * n.dot(t)).normalize_or_zero();

        // Calculate handedness
        let handedness = if n.cross(t).dot(b) < 0.0 { -1.0 } else { 1.0 };

        tangents[i] = [t.x, t.y, t.z, handedness];
    }

    tangents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_normals() {
        // Simple triangle
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let indices = vec![0, 1, 2];

        let normals = generate_normals(&positions, &indices);

        assert_eq!(normals.len(), 3);
        // Should point in +Z direction
        for normal in normals {
            assert!(normal[2] > 0.99); // Approximately 1.0
        }
    }

    #[test]
    fn test_calculate_aabb() {
        let positions = vec![[-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]];

        let aabb = calculate_aabb(&positions);

        assert_eq!(Vec3::from(aabb.center), Vec3::ZERO);
        assert_eq!(Vec3::from(aabb.half_extents), Vec3::ONE);
    }

    #[test]
    fn test_scale_positions() {
        let mut positions = vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]];

        scale_positions(&mut positions, Vec3::new(2.0, 2.0, 2.0));

        assert_eq!(positions[0], [2.0, 4.0, 6.0]);
        assert_eq!(positions[1], [8.0, 10.0, 12.0]);
    }

    #[test]
    fn test_flip_uvs() {
        let mut uvs = vec![[0.0, 0.0], [1.0, 1.0], [0.5, 0.5]];

        flip_uvs(&mut uvs);

        assert_eq!(uvs[0], [0.0, 1.0]);
        assert_eq!(uvs[1], [1.0, 0.0]);
        assert_eq!(uvs[2], [0.5, 0.5]);
    }

    #[test]
    fn test_empty_aabb() {
        let positions: Vec<[f32; 3]> = vec![];
        let aabb = calculate_aabb(&positions);
        assert_eq!(Vec3::from(aabb.center), Vec3::ZERO);
        assert_eq!(Vec3::from(aabb.half_extents), Vec3::ZERO);
    }
}
