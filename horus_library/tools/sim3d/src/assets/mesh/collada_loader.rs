//! COLLADA (.dae) mesh loader
//!
//! Loads COLLADA Digital Asset Exchange files (.dae)
//! Common format for robot models from ROS packages

use super::{processing::*, LoadedMesh, MaterialInfo, MeshLoadOptions};
use crate::error::{EnhancedError, Result};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::path::Path;

/// Load a COLLADA file
pub fn load_collada(path: &Path, options: &MeshLoadOptions) -> Result<LoadedMesh> {
    tracing::debug!("Loading COLLADA file: {}", path.display());

    // For now, COLLADA support is limited - the collada crate has a complex API
    // We'll implement basic support by parsing the file and extracting geometry
    // A full implementation would require deep integration with the collada crate

    // Read and parse COLLADA file
    let content = std::fs::read_to_string(path).map_err(|e| EnhancedError::file_not_found(path))?;

    let document = collada::document::ColladaDocument::from_str(&content)
        .map_err(|e| EnhancedError::mesh_load_failed(
            path,
            format!("COLLADA XML parsing error: {:?}", e)
        )
        .with_hint("COLLADA file must be valid XML conforming to the COLLADA 1.4/1.5 specification")
        .with_suggestion("Validate COLLADA XML structure and ensure it contains proper <library_geometries> elements"))?;

    // Get object set
    let obj_set = document.get_obj_set().ok_or_else(|| {
        EnhancedError::mesh_load_failed(path, "Failed to extract geometry from COLLADA document")
            .with_hint("COLLADA file may be missing geometry library or have unsupported features")
    })?;

    if obj_set.objects.is_empty() {
        return Err(EnhancedError::mesh_load_failed(
            path,
            "COLLADA file contains no geometry"
        )
        .with_hint("The file may be a valid COLLADA document but contains no mesh data")
        .with_suggestion("Check that the COLLADA file has <library_geometries> with actual mesh definitions"));
    }

    tracing::debug!("COLLADA file contains {} objects", obj_set.objects.len());

    // Use the first object (most COLLADA files have one main mesh)
    let obj = &obj_set.objects[0];

    tracing::debug!("Object '{}': {} vertices", obj.name, obj.vertices.len());

    // Extract and scale positions
    let mut positions: Vec<[f32; 3]> = obj
        .vertices
        .iter()
        .map(|v| [v.x as f32, v.y as f32, v.z as f32])
        .collect();

    if options.scale != Vec3::ONE {
        scale_positions(&mut positions, options.scale);
    }

    // Extract indices from geometry
    // Note: COLLADA loader has limited support - primarily handles triangulated meshes
    let mut indices: Vec<u32> = Vec::new();

    // The collada crate stores geometry as Vec<Geometry>, each with a mesh field containing PrimitiveElements
    for geometry in &obj.geometry {
        // Access the mesh field which contains the primitive elements
        for element in &geometry.mesh {
            match element {
                collada::PrimitiveElement::Triangles(tri) => {
                    // Triangles are tuples of (vertex_idx, normal_idx, texcoord_idx)
                    for &(v_idx, _n_idx, _t_idx) in &tri.vertices {
                        indices.push(v_idx as u32);
                    }
                }
                collada::PrimitiveElement::Polylist(_poly) => {
                    // Polylist support is limited in this version
                    // The collada crate API for Polylist is complex and varies by version
                    // For production use, consider using a COLLADA->glTF converter
                    tracing::warn!("Polylist geometry found but not fully supported - skipping");
                    tracing::warn!("Consider converting COLLADA to glTF for better support");
                }
                _ => {
                    tracing::warn!("Unsupported COLLADA primitive type, skipping");
                }
            }
        }
    }

    if indices.is_empty() {
        return Err(EnhancedError::mesh_load_failed(
            path,
            "No valid triangle geometry found in COLLADA file"
        )
        .with_hint("The COLLADA file contains geometry but no triangulated meshes")
        .with_suggestion("Check that the file contains triangle primitives, or convert from polylist/polygons to triangles"));
    }

    tracing::debug!(
        "Extracted {} triangles from COLLADA mesh",
        indices.len() / 3
    );

    // Extract or generate normals
    let normals: Vec<[f32; 3]> = if !obj.normals.is_empty() {
        obj.normals
            .iter()
            .map(|n| [n.x as f32, n.y as f32, n.z as f32])
            .collect()
    } else if options.generate_normals {
        tracing::debug!("Generating normals for COLLADA mesh");
        generate_normals(&positions, &indices)
    } else {
        vec![[0.0, 1.0, 0.0]; positions.len()]
    };

    // Extract UVs
    let mut uvs: Vec<[f32; 2]> = if !obj.tex_vertices.is_empty() {
        obj.tex_vertices
            .iter()
            .map(|uv| [uv.x as f32, uv.y as f32])
            .collect()
    } else {
        vec![[0.0, 0.0]; positions.len()]
    };

    if options.flip_uvs && !obj.tex_vertices.is_empty() {
        flip_uvs(&mut uvs);
    }

    // Create Bevy mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());

    if !obj.tex_vertices.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.clone());

        if options.generate_tangents {
            tracing::debug!("Generating tangents for COLLADA mesh");
            let tangents = generate_tangents(&positions, &normals, &uvs, &indices);
            mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
        }
    }

    mesh.insert_indices(Indices::U32(indices.clone()));

    // Calculate bounds
    let bounds = calculate_aabb(&positions);

    // Simple material (COLLADA material parsing is complex)
    let materials = vec![MaterialInfo {
        name: obj.name.clone(),
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

    #[test]
    fn test_collada_load_options() {
        let options = MeshLoadOptions::default();
        assert!(options.generate_normals);
        assert_eq!(options.scale, Vec3::ONE);
    }
}
