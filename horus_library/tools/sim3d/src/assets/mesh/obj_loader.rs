//! Wavefront OBJ mesh loader
//!
//! Loads .obj files with support for:
//! - Positions, normals, UVs
//! - Multiple materials
//! - Texture references

use super::{processing::*, LoadedMesh, MaterialInfo, MeshLoadOptions};
use crate::error::{EnhancedError, Result};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::path::{Path, PathBuf};

/// Load an OBJ file
pub fn load_obj(path: &Path, options: &MeshLoadOptions) -> Result<LoadedMesh> {
    tracing::debug!("Loading OBJ file: {}", path.display());

    // Load OBJ file
    let (models, materials) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ignore_points: true,
            ignore_lines: true,
        },
    )
    .map_err(|e| EnhancedError::mesh_load_failed(path, format!("OBJ parsing error: {}", e)))?;

    if models.is_empty() {
        return Err(EnhancedError::mesh_load_failed(
            path,
            "OBJ file contains no models"
        )
        .with_hint("The OBJ file may be empty or corrupted")
        .with_suggestion("Open the file in a 3D viewer (Blender, MeshLab) to verify it contains geometry"));
    }

    tracing::debug!("OBJ file contains {} models", models.len());

    // Use the first model (multi-mesh OBJ files can be split by calling load_obj multiple times)
    let model = &models[0];
    let mesh_data = &model.mesh;

    tracing::debug!(
        "Model '{}': {} vertices, {} indices",
        model.name,
        mesh_data.positions.len() / 3,
        mesh_data.indices.len()
    );

    // Extract and scale positions
    let mut positions: Vec<[f32; 3]> = mesh_data
        .positions
        .chunks(3)
        .map(|p| [p[0], p[1], p[2]])
        .collect();

    if options.scale != Vec3::ONE {
        scale_positions(&mut positions, options.scale);
    }

    // Extract or generate normals
    let normals: Vec<[f32; 3]> = if mesh_data.normals.is_empty() {
        if options.generate_normals {
            tracing::debug!("Generating normals for OBJ mesh");
            generate_normals(&positions, &mesh_data.indices)
        } else {
            tracing::warn!("OBJ mesh has no normals and generation is disabled");
            vec![[0.0, 1.0, 0.0]; positions.len()]
        }
    } else {
        mesh_data
            .normals
            .chunks(3)
            .map(|n| [n[0], n[1], n[2]])
            .collect()
    };

    // Extract UVs
    let mut uvs: Vec<[f32; 2]> = if !mesh_data.texcoords.is_empty() {
        mesh_data
            .texcoords
            .chunks(2)
            .map(|uv| [uv[0], uv[1]])
            .collect()
    } else {
        // Generate default UVs if missing
        vec![[0.0, 0.0]; positions.len()]
    };

    if options.flip_uvs && !mesh_data.texcoords.is_empty() {
        flip_uvs(&mut uvs);
    }

    // Create Bevy mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());

    if !mesh_data.texcoords.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.clone());

        // Generate tangents if requested and we have UVs
        if options.generate_tangents {
            tracing::debug!("Generating tangents for OBJ mesh");
            let tangents = generate_tangents(&positions, &normals, &uvs, &mesh_data.indices);
            mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
        }
    }

    mesh.insert_indices(Indices::U32(mesh_data.indices.clone()));

    // Calculate bounds
    let bounds = calculate_aabb(&positions);

    // Extract material information
    let material_info = extract_materials(path, model, materials.as_ref().ok());

    // Get texture paths
    let texture_paths = material_info
        .iter()
        .filter_map(|m| m.diffuse_texture.clone())
        .chain(
            material_info
                .iter()
                .filter_map(|m| m.normal_texture.clone()),
        )
        .collect();

    Ok(LoadedMesh {
        mesh,
        materials: material_info,
        texture_paths,
        embedded_textures: Vec::new(), // OBJ format doesn't support embedded textures
        bounds,
        triangle_count: mesh_data.indices.len() / 3,
        vertex_count: positions.len(),
    })
}

/// Extract material information from OBJ materials
fn extract_materials(
    obj_path: &Path,
    model: &tobj::Model,
    materials: Option<&Vec<tobj::Material>>,
) -> Vec<MaterialInfo> {
    let mut material_infos = Vec::new();

    if let Some(mat_id) = model.mesh.material_id {
        if let Some(materials) = materials {
            if mat_id < materials.len() {
                let mat = &materials[mat_id];

                let diffuse_texture = mat
                    .diffuse_texture
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .map(|s| resolve_texture_path(obj_path, s));

                let normal_texture = mat
                    .normal_texture
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .map(|s| resolve_texture_path(obj_path, s));

                let diffuse_color = mat
                    .diffuse
                    .as_ref()
                    .filter(|d| d.iter().any(|&x| x > 0.0))
                    .map(|d| Color::srgb(d[0], d[1], d[2]));

                material_infos.push(MaterialInfo {
                    name: mat.name.clone(),
                    diffuse_color,
                    diffuse_texture,
                    normal_texture,
                    metallic: mat.shininess.unwrap_or(0.0) / 1000.0, // Rough conversion
                    roughness: 1.0 - (mat.shininess.unwrap_or(0.0) / 1000.0),
                });
            }
        }
    }

    // If no materials found, use default
    if material_infos.is_empty() {
        material_infos.push(MaterialInfo::default());
    }

    material_infos
}

/// Resolve texture path relative to OBJ file
fn resolve_texture_path(obj_path: &Path, texture_name: &str) -> PathBuf {
    // Get directory containing the OBJ file
    let obj_dir = obj_path.parent().unwrap_or(Path::new("."));

    // Try relative to OBJ file first
    let relative_path = obj_dir.join(texture_name);
    if relative_path.exists() {
        return relative_path;
    }

    // Try as absolute path
    let absolute_path = PathBuf::from(texture_name);
    if absolute_path.exists() {
        return absolute_path;
    }

    // Return relative path even if it doesn't exist (will be handled by texture loader)
    relative_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_options_application() {
        let options = MeshLoadOptions {
            scale: Vec3::new(2.0, 2.0, 2.0),
            generate_normals: true,
            generate_tangents: false,
            flip_uvs: true,
            validate: false,
            max_triangles: None,
        };

        assert_eq!(options.scale, Vec3::new(2.0, 2.0, 2.0));
        assert!(options.generate_normals);
        assert!(options.flip_uvs);
    }

    #[test]
    fn test_resolve_texture_path() {
        let obj_path = PathBuf::from("/models/robot/robot.obj");
        let texture_name = "textures/diffuse.png";

        let resolved = resolve_texture_path(&obj_path, texture_name);
        assert_eq!(
            resolved,
            PathBuf::from("/models/robot/textures/diffuse.png")
        );
    }
}
