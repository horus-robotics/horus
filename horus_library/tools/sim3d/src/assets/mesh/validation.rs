//! Mesh validation utilities
//!
//! Validates mesh geometry for common issues:
//! - Missing or invalid attributes
//! - Degenerate triangles
//! - Non-manifold geometry
//! - Invalid normals

// Public API for mesh validation - may not all be used internally
#![allow(dead_code)]

use anyhow::Result;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, Mesh, VertexAttributeValues};

/// Validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

/// Validate mesh geometry
pub fn validate_mesh(mesh: &Mesh) -> Result<()> {
    let mut report = ValidationReport::new();

    // Check required attributes
    validate_positions(mesh, &mut report);
    validate_normals(mesh, &mut report);
    validate_indices(mesh, &mut report);

    // Check for degenerate triangles
    check_degenerate_triangles(mesh, &mut report);

    // Check for NaN or infinite values
    check_for_nan(mesh, &mut report);

    if !report.is_valid() {
        let error_msg = format!("Mesh validation failed:\n{}", report.errors.join("\n"));
        anyhow::bail!(error_msg);
    }

    if !report.warnings.is_empty() {
        tracing::warn!("Mesh validation warnings:\n{}", report.warnings.join("\n"));
    }

    Ok(())
}

/// Validate positions attribute
fn validate_positions(mesh: &Mesh, report: &mut ValidationReport) {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(positions)) => {
            if positions.is_empty() {
                report.add_error("Mesh has no vertices".to_string());
            } else {
                tracing::debug!("Mesh has {} vertices", positions.len());
            }
        }
        Some(_) => {
            report
                .add_error("Position attribute has wrong format (expected Float32x3)".to_string());
        }
        None => {
            report.add_error("Mesh is missing position attribute".to_string());
        }
    }
}

/// Validate normals attribute
fn validate_normals(mesh: &Mesh, report: &mut ValidationReport) {
    match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(VertexAttributeValues::Float32x3(normals)) => {
            // Check if normal count matches vertex count
            if let Some(VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                if normals.len() != positions.len() {
                    report.add_error(format!(
                        "Normal count ({}) doesn't match vertex count ({})",
                        normals.len(),
                        positions.len()
                    ));
                }
            }

            // Check for zero-length normals
            let mut zero_count = 0;
            for normal in normals {
                let len_sq = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
                if len_sq < 0.001 {
                    zero_count += 1;
                }
            }

            if zero_count > 0 {
                report.add_warning(format!("Found {} zero-length normals", zero_count));
            }
        }
        Some(_) => {
            report
                .add_warning("Normal attribute has wrong format (expected Float32x3)".to_string());
        }
        None => {
            report.add_warning("Mesh is missing normals (can be generated)".to_string());
        }
    }
}

/// Validate indices
fn validate_indices(mesh: &Mesh, report: &mut ValidationReport) {
    let vertex_count = if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        positions.len()
    } else {
        return;
    };

    match mesh.indices() {
        Some(Indices::U16(indices)) => {
            let vertex_count_u16 = if vertex_count > u16::MAX as usize {
                report.add_error(format!(
                    "Vertex count {} exceeds u16::MAX, but index buffer is U16",
                    vertex_count
                ));
                return;
            } else {
                vertex_count as u16
            };
            validate_index_buffer_u16(indices, vertex_count_u16, report);
        }
        Some(Indices::U32(indices)) => {
            validate_index_buffer_u32(indices, vertex_count, report);
        }
        None => {
            report.add_warning("Mesh has no index buffer (non-indexed mesh)".to_string());
        }
    }
}

/// Validate U16 index buffer values
fn validate_index_buffer_u16(indices: &[u16], vertex_count: u16, report: &mut ValidationReport) {
    if indices.is_empty() {
        report.add_error("Index buffer is empty".to_string());
        return;
    }

    if indices.len() % 3 != 0 {
        report.add_error(format!(
            "Index count ({}) is not divisible by 3",
            indices.len()
        ));
    }

    // Check for out-of-bounds indices
    for (i, &idx) in indices.iter().enumerate() {
        if idx >= vertex_count {
            report.add_error(format!(
                "Index {} at position {} is out of bounds (vertex count: {})",
                idx, i, vertex_count
            ));
        }
    }
}

/// Validate U32 index buffer values
fn validate_index_buffer_u32(indices: &[u32], vertex_count: usize, report: &mut ValidationReport) {
    if indices.is_empty() {
        report.add_error("Index buffer is empty".to_string());
        return;
    }

    if indices.len() % 3 != 0 {
        report.add_error(format!(
            "Index count ({}) is not divisible by 3",
            indices.len()
        ));
    }

    // Check for out-of-bounds indices
    for (i, &idx) in indices.iter().enumerate() {
        if idx as usize >= vertex_count {
            report.add_error(format!(
                "Index {} at position {} is out of bounds (vertex count: {})",
                idx, i, vertex_count
            ));
        }
    }
}

/// Check for degenerate triangles
fn check_degenerate_triangles(mesh: &Mesh, report: &mut ValidationReport) {
    let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(pos)) => pos,
        _ => return,
    };

    let indices = match mesh.indices() {
        Some(Indices::U32(idx)) => idx.as_slice(),
        Some(Indices::U16(_)) => {
            // U16 indices not supported for degenerate check, skip
            return;
        }
        None => return,
    };

    let mut degenerate_count = 0;
    const EPSILON: f32 = 1e-6;

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

        // Check if triangle is degenerate (area ~= 0)
        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let cross = edge1.cross(edge2);
        let area = cross.length() * 0.5;

        if area < EPSILON {
            degenerate_count += 1;
        }
    }

    if degenerate_count > 0 {
        report.add_warning(format!(
            "Found {} degenerate triangles (near-zero area)",
            degenerate_count
        ));
    }
}

/// Check for NaN or infinite values in vertex attributes
fn check_for_nan(mesh: &Mesh, report: &mut ValidationReport) {
    // Check positions
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        for (i, pos) in positions.iter().enumerate() {
            for (j, &val) in pos.iter().enumerate() {
                if !val.is_finite() {
                    report.add_error(format!(
                        "Position vertex {} component {} has invalid value: {}",
                        i, j, val
                    ));
                }
            }
        }
    }

    // Check normals
    if let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    {
        for (i, normal) in normals.iter().enumerate() {
            for (j, &val) in normal.iter().enumerate() {
                if !val.is_finite() {
                    report.add_error(format!(
                        "Normal vertex {} component {} has invalid value: {}",
                        i, j, val
                    ));
                }
            }
        }
    }

    // Check UVs
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        for (i, uv) in uvs.iter().enumerate() {
            for (j, &val) in uv.iter().enumerate() {
                if !val.is_finite() {
                    report.add_error(format!(
                        "UV vertex {} component {} has invalid value: {}",
                        i, j, val
                    ));
                }
            }
        }
    }
}

/// Check if mesh is watertight (all edges are shared by exactly 2 triangles)
pub fn check_watertight(mesh: &Mesh) -> bool {
    use std::collections::HashMap;

    let indices = match mesh.indices() {
        Some(Indices::U32(idx)) => idx,
        _ => return false,
    };

    // Count edge usage
    let mut edge_count: HashMap<(u32, u32), usize> = HashMap::new();

    for triangle in indices.chunks(3) {
        if triangle.len() != 3 {
            continue;
        }

        // Add the three edges (order them so (a,b) == (b,a))
        for i in 0..3 {
            let v1 = triangle[i];
            let v2 = triangle[(i + 1) % 3];
            let edge = if v1 < v2 { (v1, v2) } else { (v2, v1) };
            *edge_count.entry(edge).or_insert(0) += 1;
        }
    }

    // Check if all edges are shared by exactly 2 triangles
    edge_count.values().all(|&count| count == 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::PrimitiveTopology;

    #[test]
    fn test_valid_triangle() {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());

        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let indices = vec![0u32, 1, 2];

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_indices(Indices::U32(indices));

        assert!(validate_mesh(&mesh).is_ok());
    }

    #[test]
    fn test_missing_positions() {
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());
        assert!(validate_mesh(&mesh).is_err());
    }

    #[test]
    fn test_out_of_bounds_index() {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());

        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let indices = vec![0u32, 1, 5]; // Index 5 is out of bounds

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_indices(Indices::U32(indices));

        assert!(validate_mesh(&mesh).is_err());
    }
}
