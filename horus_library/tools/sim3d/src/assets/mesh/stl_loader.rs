//! STL (STereoLithography) mesh loader
//!
//! Loads .stl files in both ASCII and binary formats.
//! STL files contain only triangle geometry with normals,
//! no materials or UV coordinates.

use super::{processing::*, LoadedMesh, MaterialInfo, MeshLoadOptions};
use crate::error::{EnhancedError, Result};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::path::Path;

/// Load an STL file (automatically detects ASCII or binary format)
pub fn load_stl(path: &Path, options: &MeshLoadOptions) -> Result<LoadedMesh> {
    tracing::debug!("Loading STL file: {}", path.display());

    // Read STL file (stl_io handles both ASCII and binary automatically)
    let mut file = std::fs::File::open(path).map_err(|e| EnhancedError::file_not_found(path))?;

    let stl = stl_io::read_stl(&mut file)
        .map_err(|e| EnhancedError::mesh_load_failed(
            path,
            format!("STL parsing error: {}", e)
        )
        .with_hint("STL file may be corrupted or in an unsupported format")
        .with_suggestion("Try opening the STL in a 3D viewer to verify it's valid. STL must be either valid ASCII or binary format."))?;

    tracing::debug!("STL file contains {} vertices", stl.vertices.len());

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // STL IndexedMesh has vertices, normals, and faces
    // Apply scale to vertices
    for vertex in &stl.vertices {
        positions.push([
            vertex[0] * options.scale.x,
            vertex[1] * options.scale.y,
            vertex[2] * options.scale.z,
        ]);
    }

    // Extract face indices (convert to u32)
    for face in &stl.faces {
        indices.push(face.vertices[0] as u32);
        indices.push(face.vertices[1] as u32);
        indices.push(face.vertices[2] as u32);
    }

    // Generate normals from geometry
    // STL IndexedMesh doesn't store normals separately in the same way
    // Always generate smooth normals or use flat shading
    tracing::debug!("Generating normals for STL mesh");
    normals = generate_normals(&positions, &indices);

    tracing::debug!(
        "Created mesh with {} vertices, {} triangles",
        positions.len(),
        indices.len() / 3
    );

    // Create Bevy mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices.clone()));

    // STL files don't have UV coordinates, create default ones
    let uvs = vec![[0.0, 0.0]; positions.len()];
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    // Calculate bounds
    let bounds = calculate_aabb(&positions);

    // STL files don't have materials or header in IndexedMesh
    let materials = vec![MaterialInfo {
        name: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("stl_mesh")
            .to_string(),
        diffuse_color: Some(Color::srgb(0.8, 0.8, 0.8)),
        diffuse_texture: None,
        normal_texture: None,
        metallic: 0.0,
        roughness: 0.5,
    }];

    Ok(LoadedMesh {
        mesh,
        materials,
        texture_paths: Vec::new(),
        bounds,
        triangle_count: indices.len() / 3,
        vertex_count: positions.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_stl_loading_ascii() {
        // Create a simple ASCII STL file
        let temp_dir = std::env::temp_dir();
        let stl_path = temp_dir.join("test_triangle.stl");

        let stl_content = r#"solid test
facet normal 0.0 0.0 1.0
  outer loop
    vertex 0.0 0.0 0.0
    vertex 1.0 0.0 0.0
    vertex 0.0 1.0 0.0
  endloop
endfacet
endsolid test
"#;

        std::fs::write(&stl_path, stl_content).unwrap();

        let options = MeshLoadOptions::default();
        let result = load_stl(&stl_path, &options);

        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.vertex_count, 3);
        assert_eq!(loaded.triangle_count, 1);

        std::fs::remove_file(stl_path).ok();
    }

    #[test]
    fn test_stl_with_scale() {
        let temp_dir = std::env::temp_dir();
        let stl_path = temp_dir.join("test_scale.stl");

        let stl_content = r#"solid test
facet normal 0.0 0.0 1.0
  outer loop
    vertex 1.0 1.0 1.0
    vertex 2.0 1.0 1.0
    vertex 1.0 2.0 1.0
  endloop
endfacet
endsolid test
"#;

        std::fs::write(&stl_path, stl_content).unwrap();

        let options = MeshLoadOptions::default().with_scale(Vec3::new(2.0, 2.0, 2.0));

        let result = load_stl(&stl_path, &options);
        assert!(result.is_ok());

        std::fs::remove_file(stl_path).ok();
    }
}
