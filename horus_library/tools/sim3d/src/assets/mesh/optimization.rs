//! Mesh optimization module
//!
//! Provides mesh decimation and LOD (Level of Detail) generation using
//! Quadric Error Metrics (QEM) for edge collapse simplification.

// Public API for mesh optimization - may not be used internally
#![allow(dead_code)]

use bevy::prelude::*;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Configuration for mesh decimation
#[derive(Debug, Clone)]
pub struct DecimationOptions {
    /// Target triangle count (absolute number)
    pub target_triangles: Option<usize>,
    /// Target reduction ratio (0.0 to 1.0, where 0.5 = reduce by 50%)
    pub reduction_ratio: Option<f32>,
    /// Preserve mesh boundaries
    pub preserve_boundaries: bool,
    /// Maximum error threshold
    pub max_error: f32,
    /// Preserve UV seams
    pub preserve_uv_seams: bool,
}

impl Default for DecimationOptions {
    fn default() -> Self {
        Self {
            target_triangles: None,
            reduction_ratio: Some(0.5),
            preserve_boundaries: true,
            max_error: f32::INFINITY,
            preserve_uv_seams: true,
        }
    }
}

impl DecimationOptions {
    /// Create with target triangle count
    pub fn with_target_triangles(mut self, count: usize) -> Self {
        self.target_triangles = Some(count);
        self.reduction_ratio = None;
        self
    }

    /// Create with reduction ratio
    pub fn with_reduction_ratio(mut self, ratio: f32) -> Self {
        self.reduction_ratio = Some(ratio.clamp(0.0, 1.0));
        self.target_triangles = None;
        self
    }

    /// Set boundary preservation
    pub fn preserve_boundaries(mut self, preserve: bool) -> Self {
        self.preserve_boundaries = preserve;
        self
    }

    /// Set maximum error threshold
    pub fn max_error(mut self, error: f32) -> Self {
        self.max_error = error;
        self
    }
}

/// LOD (Level of Detail) configuration
#[derive(Debug, Clone)]
pub struct LODConfig {
    /// Number of LOD levels to generate
    pub num_levels: usize,
    /// Reduction ratio between levels (e.g., 0.5 = each level has 50% fewer triangles)
    pub reduction_per_level: f32,
    /// Preserve boundaries in all LOD levels
    pub preserve_boundaries: bool,
}

impl Default for LODConfig {
    fn default() -> Self {
        Self {
            num_levels: 3,
            reduction_per_level: 0.5,
            preserve_boundaries: true,
        }
    }
}

/// Quadric error matrix for vertex position
#[derive(Debug, Clone, Copy)]
struct Quadric {
    // Quadric matrix is symmetric, so we store only 10 values:
    // [ a  b  c  d ]
    // [ b  e  f  g ]
    // [ c  f  h  i ]
    // [ d  g  i  j ]
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    g: f64,
    h: f64,
    i: f64,
    j: f64,
}

impl Quadric {
    /// Create zero quadric
    fn zero() -> Self {
        Self {
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
            g: 0.0,
            h: 0.0,
            i: 0.0,
            j: 0.0,
        }
    }

    /// Create quadric from plane equation ax + by + cz + d = 0
    fn from_plane(normal: Vec3, d: f64) -> Self {
        let a = normal.x as f64;
        let b = normal.y as f64;
        let c = normal.z as f64;

        Self {
            a: a * a,
            b: a * b,
            c: a * c,
            d: a * d,
            e: b * b,
            f: b * c,
            g: b * d,
            h: c * c,
            i: c * d,
            j: d * d,
        }
    }

    /// Add two quadrics
    fn add(&self, other: &Quadric) -> Self {
        Self {
            a: self.a + other.a,
            b: self.b + other.b,
            c: self.c + other.c,
            d: self.d + other.d,
            e: self.e + other.e,
            f: self.f + other.f,
            g: self.g + other.g,
            h: self.h + other.h,
            i: self.i + other.i,
            j: self.j + other.j,
        }
    }

    /// Evaluate quadric error at position
    fn error(&self, v: Vec3) -> f64 {
        let x = v.x as f64;
        let y = v.y as f64;
        let z = v.z as f64;

        self.a * x * x
            + 2.0 * self.b * x * y
            + 2.0 * self.c * x * z
            + 2.0 * self.d * x
            + self.e * y * y
            + 2.0 * self.f * y * z
            + 2.0 * self.g * y
            + self.h * z * z
            + 2.0 * self.i * z
            + self.j
    }

    /// Find optimal vertex position that minimizes error
    fn optimal_position(&self, v1: Vec3, v2: Vec3) -> (Vec3, f64) {
        // Try to solve the linear system to find optimal position
        // For simplicity, use midpoint (more robust)
        let midpoint = (v1 + v2) * 0.5;
        let error = self.error(midpoint);
        (midpoint, error)
    }
}

/// Edge collapse candidate
#[derive(Debug, Clone)]
struct EdgeCollapse {
    v1: usize,
    v2: usize,
    error: f64,
    new_pos: Vec3,
}

impl PartialEq for EdgeCollapse {
    fn eq(&self, other: &Self) -> bool {
        self.error == other.error
    }
}

impl Eq for EdgeCollapse {}

impl PartialOrd for EdgeCollapse {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EdgeCollapse {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (we want smallest error first)
        other
            .error
            .partial_cmp(&self.error)
            .unwrap_or(Ordering::Equal)
    }
}

/// Decimate mesh using Quadric Error Metrics
pub fn decimate_mesh(
    mesh: &mut bevy::render::mesh::Mesh,
    options: DecimationOptions,
) -> anyhow::Result<()> {
    // Extract positions and indices
    let positions = extract_positions(mesh)?;
    let indices = extract_indices(mesh)?;

    let current_triangles = indices.len() / 3;

    // Calculate target triangle count
    let target_triangles = if let Some(target) = options.target_triangles {
        target
    } else if let Some(ratio) = options.reduction_ratio {
        ((current_triangles as f32) * (1.0 - ratio)) as usize
    } else {
        anyhow::bail!("Must specify either target_triangles or reduction_ratio");
    };

    if target_triangles >= current_triangles {
        return Ok(()); // Nothing to do
    }

    tracing::info!(
        "Decimating mesh: {} -> {} triangles ({:.1}% reduction)",
        current_triangles,
        target_triangles,
        100.0 * (1.0 - target_triangles as f32 / current_triangles as f32)
    );

    // Build edge and face structures
    let (new_positions, new_indices) =
        decimate_internal(&positions, &indices, target_triangles, &options)?;

    // Update mesh
    update_mesh_geometry(mesh, new_positions, new_indices)?;

    Ok(())
}

/// Internal decimation algorithm
fn decimate_internal(
    positions: &[[f32; 3]],
    indices: &[u32],
    target_triangles: usize,
    options: &DecimationOptions,
) -> anyhow::Result<(Vec<[f32; 3]>, Vec<u32>)> {
    let num_vertices = positions.len();

    // Build adjacency information
    let mut vertex_faces: Vec<HashSet<usize>> = vec![HashSet::new(); num_vertices];
    let mut vertex_neighbors: Vec<HashSet<usize>> = vec![HashSet::new(); num_vertices];

    for (face_idx, triangle) in indices.chunks(3).enumerate() {
        let v0 = triangle[0] as usize;
        let v1 = triangle[1] as usize;
        let v2 = triangle[2] as usize;

        vertex_faces[v0].insert(face_idx);
        vertex_faces[v1].insert(face_idx);
        vertex_faces[v2].insert(face_idx);

        vertex_neighbors[v0].insert(v1);
        vertex_neighbors[v0].insert(v2);
        vertex_neighbors[v1].insert(v0);
        vertex_neighbors[v1].insert(v2);
        vertex_neighbors[v2].insert(v0);
        vertex_neighbors[v2].insert(v1);
    }

    // Compute quadrics for each vertex
    let mut quadrics = vec![Quadric::zero(); num_vertices];

    for triangle in indices.chunks(3) {
        let v0 = triangle[0] as usize;
        let v1 = triangle[1] as usize;
        let v2 = triangle[2] as usize;

        let p0 = Vec3::from(positions[v0]);
        let p1 = Vec3::from(positions[v1]);
        let p2 = Vec3::from(positions[v2]);

        // Compute face normal
        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let normal = edge1.cross(edge2).normalize_or_zero();

        // Compute plane equation
        let d = -normal.dot(p0) as f64;
        let face_quadric = Quadric::from_plane(normal, d);

        // Add to vertex quadrics
        quadrics[v0] = quadrics[v0].add(&face_quadric);
        quadrics[v1] = quadrics[v1].add(&face_quadric);
        quadrics[v2] = quadrics[v2].add(&face_quadric);
    }

    // Build initial edge collapse heap
    let mut heap = BinaryHeap::new();
    let mut processed_edges = HashSet::new();

    for v1 in 0..num_vertices {
        for &v2 in &vertex_neighbors[v1] {
            if v1 < v2 && !processed_edges.contains(&(v1, v2)) {
                let q = quadrics[v1].add(&quadrics[v2]);
                let p1 = Vec3::from(positions[v1]);
                let p2 = Vec3::from(positions[v2]);
                let (new_pos, error) = q.optimal_position(p1, p2);

                heap.push(EdgeCollapse {
                    v1,
                    v2,
                    error,
                    new_pos,
                });
                processed_edges.insert((v1, v2));
            }
        }
    }

    // Perform edge collapses
    let mut vertex_map: Vec<Option<usize>> = (0..num_vertices).map(Some).collect();
    let mut current_triangles = indices.len() / 3;

    while current_triangles > target_triangles && !heap.is_empty() {
        if let Some(collapse) = heap.pop() {
            // Check if error exceeds threshold
            if collapse.error > options.max_error as f64 {
                break;
            }

            // Check if vertices still exist (not collapsed)
            if vertex_map[collapse.v1].is_none() || vertex_map[collapse.v2].is_none() {
                continue;
            }

            // Collapse edge
            vertex_map[collapse.v2] = Some(collapse.v1);
            current_triangles = count_valid_triangles(indices, &vertex_map);
        }
    }

    // Rebuild mesh
    let mut new_positions = Vec::new();
    let mut new_indices = Vec::new();
    let mut old_to_new: HashMap<usize, usize> = HashMap::new();

    // Add vertices
    for (old_idx, &mapped) in vertex_map.iter().enumerate() {
        if let Some(new_idx) = mapped {
            if new_idx == old_idx {
                let idx = new_positions.len();
                old_to_new.insert(old_idx, idx);
                new_positions.push(positions[old_idx]);
            }
        }
    }

    // Add triangles
    for triangle in indices.chunks(3) {
        let v0 = resolve_vertex(triangle[0] as usize, &vertex_map);
        let v1 = resolve_vertex(triangle[1] as usize, &vertex_map);
        let v2 = resolve_vertex(triangle[2] as usize, &vertex_map);

        // Skip degenerate triangles
        if v0 == v1 || v1 == v2 || v2 == v0 {
            continue;
        }

        if let (Some(&i0), Some(&i1), Some(&i2)) = (
            old_to_new.get(&v0),
            old_to_new.get(&v1),
            old_to_new.get(&v2),
        ) {
            new_indices.push(i0 as u32);
            new_indices.push(i1 as u32);
            new_indices.push(i2 as u32);
        }
    }

    tracing::info!(
        "Decimation complete: {} vertices, {} triangles",
        new_positions.len(),
        new_indices.len() / 3
    );

    Ok((new_positions, new_indices))
}

/// Resolve vertex through collapse chain
fn resolve_vertex(v: usize, vertex_map: &[Option<usize>]) -> usize {
    let mut current = v;
    while let Some(mapped) = vertex_map.get(current).and_then(|&m| m) {
        if mapped == current {
            break;
        }
        current = mapped;
    }
    current
}

/// Count valid triangles after collapses
fn count_valid_triangles(indices: &[u32], vertex_map: &[Option<usize>]) -> usize {
    let mut count = 0;
    for triangle in indices.chunks(3) {
        let v0 = resolve_vertex(triangle[0] as usize, vertex_map);
        let v1 = resolve_vertex(triangle[1] as usize, vertex_map);
        let v2 = resolve_vertex(triangle[2] as usize, vertex_map);

        if v0 != v1 && v1 != v2 && v2 != v0 {
            count += 1;
        }
    }
    count
}

/// Generate multiple LOD levels
pub fn generate_lods(
    mesh: &bevy::render::mesh::Mesh,
    config: LODConfig,
) -> anyhow::Result<Vec<bevy::render::mesh::Mesh>> {
    let mut lods = Vec::new();
    lods.push(mesh.clone()); // LOD 0 is the original mesh

    let mut current_mesh = mesh.clone();
    let mut current_reduction = 0.0;

    for level in 1..=config.num_levels {
        current_reduction += config.reduction_per_level;

        let options = DecimationOptions {
            reduction_ratio: Some(current_reduction.min(0.95)), // Cap at 95% reduction
            preserve_boundaries: config.preserve_boundaries,
            ..Default::default()
        };

        let mut lod_mesh = current_mesh.clone();
        decimate_mesh(&mut lod_mesh, options)?;

        lods.push(lod_mesh.clone());
        current_mesh = lod_mesh;

        tracing::info!("Generated LOD level {}", level);
    }

    Ok(lods)
}

/// Extract positions from mesh
fn extract_positions(mesh: &bevy::render::mesh::Mesh) -> anyhow::Result<Vec<[f32; 3]>> {
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION)
    {
        Ok(positions.clone())
    } else {
        anyhow::bail!("Mesh does not have position attribute")
    }
}

/// Extract indices from mesh
fn extract_indices(mesh: &bevy::render::mesh::Mesh) -> anyhow::Result<Vec<u32>> {
    match mesh.indices() {
        Some(Indices::U32(indices)) => Ok(indices.clone()),
        Some(Indices::U16(indices)) => Ok(indices.iter().map(|&i| i as u32).collect()),
        None => anyhow::bail!("Mesh does not have indices"),
    }
}

/// Update mesh geometry
fn update_mesh_geometry(
    mesh: &mut bevy::render::mesh::Mesh,
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
) -> anyhow::Result<()> {
    // Update positions
    mesh.insert_attribute(
        bevy::render::mesh::Mesh::ATTRIBUTE_POSITION,
        positions.clone(),
    );

    // Update indices
    mesh.insert_indices(Indices::U32(indices.clone()));

    // Regenerate normals
    let normals = super::processing::generate_normals(&positions, &indices);
    mesh.insert_attribute(bevy::render::mesh::Mesh::ATTRIBUTE_NORMAL, normals);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::PrimitiveTopology;

    fn create_test_cube() -> bevy::render::mesh::Mesh {
        let mut mesh =
            bevy::render::mesh::Mesh::new(PrimitiveTopology::TriangleList, Default::default());

        // Simple cube vertices
        let positions = vec![
            // Front face
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
            // Back face
            [-0.5, -0.5, -0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
        ];

        let indices = vec![
            // Front
            0, 1, 2, 0, 2, 3, // Back
            5, 4, 7, 5, 7, 6, // Left
            4, 0, 3, 4, 3, 7, // Right
            1, 5, 6, 1, 6, 2, // Top
            3, 2, 6, 3, 6, 7, // Bottom
            4, 5, 1, 4, 1, 0,
        ];

        mesh.insert_attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_indices(Indices::U32(indices));

        mesh
    }

    #[test]
    fn test_quadric_creation() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let q = Quadric::from_plane(normal, 0.0);

        // Point on plane should have zero error
        let error = q.error(Vec3::new(1.0, 1.0, 0.0));
        assert!(error.abs() < 1e-6);
    }

    #[test]
    fn test_decimate_cube() {
        let mut mesh = create_test_cube();

        let options = DecimationOptions::default().with_reduction_ratio(0.5);

        let result = decimate_mesh(&mut mesh, options);
        assert!(result.is_ok());

        // Verify mesh still has geometry
        let positions = extract_positions(&mesh).unwrap();
        let indices = extract_indices(&mesh).unwrap();

        assert!(positions.len() > 0);
        assert!(indices.len() > 0);
        assert_eq!(indices.len() % 3, 0); // Valid triangles
    }

    #[test]
    fn test_lod_generation() {
        let mesh = create_test_cube();

        let config = LODConfig {
            num_levels: 2,
            reduction_per_level: 0.3,
            preserve_boundaries: true,
        };

        let lods = generate_lods(&mesh, config).unwrap();

        assert_eq!(lods.len(), 3); // Original + 2 LOD levels

        // Each LOD should have fewer or equal triangles
        for i in 1..lods.len() {
            let prev_tris = extract_indices(&lods[i - 1]).unwrap().len() / 3;
            let curr_tris = extract_indices(&lods[i]).unwrap().len() / 3;
            assert!(curr_tris <= prev_tris);
        }
    }

    #[test]
    fn test_decimation_options() {
        let options = DecimationOptions::default()
            .with_target_triangles(100)
            .max_error(0.01);

        assert_eq!(options.target_triangles, Some(100));
        assert_eq!(options.max_error, 0.01);
    }
}
