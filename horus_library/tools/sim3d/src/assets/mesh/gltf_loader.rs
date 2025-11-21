//! glTF/GLB mesh loader
//!
//! Loads glTF 2.0 and GLB (binary glTF) files
//! glTF is the modern standard for 3D asset interchange

use super::{processing::*, LoadedMesh, MaterialInfo, MeshLoadOptions};
use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::path::{Path, PathBuf};

/// Load a glTF or GLB file
pub fn load_gltf(path: &Path, options: &MeshLoadOptions) -> Result<LoadedMesh> {
    tracing::debug!("Loading glTF file: {}", path.display());

    // Load glTF document
    let (document, buffers, _images) = gltf::import(path)
        .with_context(|| format!("Failed to load glTF file: {}", path.display()))?;

    tracing::debug!(
        "glTF file contains {} meshes, {} materials",
        document.meshes().count(),
        document.materials().count()
    );

    // Get the first mesh (most glTF files for robots have one main mesh)
    let mesh_data = document
        .meshes()
        .next()
        .context("glTF file contains no meshes")?;

    tracing::debug!("Loading mesh '{:?}'", mesh_data.name());

    // Get the first primitive from the mesh
    let primitive = mesh_data
        .primitives()
        .next()
        .context("Mesh has no primitives")?;

    // Extract positions
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

    let positions_iter = reader.read_positions().context("Mesh has no positions")?;

    let mut positions: Vec<[f32; 3]> = positions_iter.collect();

    if options.scale != Vec3::ONE {
        scale_positions(&mut positions, options.scale);
    }

    tracing::debug!("Loaded {} vertices", positions.len());

    // Extract normals or generate them
    let normals: Vec<[f32; 3]> = if let Some(normals_iter) = reader.read_normals() {
        normals_iter.collect()
    } else if options.generate_normals {
        tracing::debug!("Generating normals for glTF mesh");
        // We need indices to generate normals, extract them first
        let indices = extract_indices(&primitive, &buffers)?;
        generate_normals(&positions, &indices)
    } else {
        vec![[0.0, 1.0, 0.0]; positions.len()]
    };

    // Extract UVs (texture coordinates)
    let mut uvs: Vec<[f32; 2]> = if let Some(uvs_iter) = reader.read_tex_coords(0) {
        uvs_iter.into_f32().collect()
    } else {
        vec![[0.0, 0.0]; positions.len()]
    };

    if options.flip_uvs && reader.read_tex_coords(0).is_some() {
        flip_uvs(&mut uvs);
    }

    // Extract indices
    let indices = extract_indices(&primitive, &buffers)?;

    tracing::debug!("Loaded {} triangles", indices.len() / 3);

    // Create Bevy mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.clone());

    if reader.read_tex_coords(0).is_some() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.clone());

        // Generate tangents if requested
        if options.generate_tangents {
            // Check if glTF already has tangents
            if let Some(tangents_iter) = reader.read_tangents() {
                let tangents: Vec<[f32; 4]> = tangents_iter.collect();
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
            } else {
                tracing::debug!("Generating tangents for glTF mesh");
                let tangents = generate_tangents(&positions, &normals, &uvs, &indices);
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
            }
        }
    }

    mesh.insert_indices(Indices::U32(indices.clone()));

    // Calculate bounds
    let bounds = calculate_aabb(&positions);

    // Extract material information
    let materials = extract_gltf_materials(&primitive, path);

    // Extract texture paths
    let texture_paths = extract_texture_paths(&primitive, path);

    Ok(LoadedMesh {
        mesh,
        materials,
        texture_paths,
        bounds,
        triangle_count: indices.len() / 3,
        vertex_count: positions.len(),
    })
}

/// Extract indices from glTF primitive
fn extract_indices(
    primitive: &gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Result<Vec<u32>> {
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

    if let Some(indices_reader) = reader.read_indices() {
        let indices: Vec<u32> = indices_reader.into_u32().collect();
        Ok(indices)
    } else {
        anyhow::bail!("glTF primitive has no indices");
    }
}

/// Extract material information from glTF
fn extract_gltf_materials(primitive: &gltf::Primitive, gltf_path: &Path) -> Vec<MaterialInfo> {
    let mut materials = Vec::new();

    let material = primitive.material();
    let pbr = material.pbr_metallic_roughness();

    let base_color = pbr.base_color_factor();
    let diffuse_color = Color::srgba(base_color[0], base_color[1], base_color[2], base_color[3]);

    let diffuse_texture = pbr
        .base_color_texture()
        .map(|info| resolve_gltf_texture_path(gltf_path, &info));

    let normal_texture = material
        .normal_texture()
        .map(|normal_tex| resolve_gltf_texture_from_texture(gltf_path, &normal_tex.texture()));

    materials.push(MaterialInfo {
        name: material.name().unwrap_or("gltf_material").to_string(),
        diffuse_color: Some(diffuse_color),
        diffuse_texture,
        normal_texture,
        metallic: pbr.metallic_factor(),
        roughness: pbr.roughness_factor(),
    });

    materials
}

/// Extract texture paths from glTF materials
fn extract_texture_paths(primitive: &gltf::Primitive, gltf_path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let material = primitive.material();
    let pbr = material.pbr_metallic_roughness();

    if let Some(tex_info) = pbr.base_color_texture() {
        paths.push(resolve_gltf_texture_path(gltf_path, &tex_info));
    }

    if let Some(normal_tex) = material.normal_texture() {
        paths.push(resolve_gltf_texture_from_texture(
            gltf_path,
            &normal_tex.texture(),
        ));
    }

    paths
}

/// Resolve glTF texture path from Info
fn resolve_gltf_texture_path(gltf_path: &Path, tex_info: &gltf::texture::Info) -> PathBuf {
    let texture = tex_info.texture();
    resolve_texture_from_image(gltf_path, &texture.source())
}

/// Resolve glTF texture path from Texture directly
fn resolve_gltf_texture_from_texture(gltf_path: &Path, texture: &gltf::Texture) -> PathBuf {
    resolve_texture_from_image(gltf_path, &texture.source())
}

/// Resolve texture from image source
fn resolve_texture_from_image(gltf_path: &Path, image: &gltf::Image) -> PathBuf {
    match image.source() {
        gltf::image::Source::Uri { uri, .. } => {
            // Resolve relative to glTF file
            let gltf_dir = gltf_path.parent().unwrap_or(Path::new("."));
            gltf_dir.join(uri)
        }
        gltf::image::Source::View { .. } => {
            // Embedded texture - return synthetic path (buffer extraction not yet implemented)
            PathBuf::from("embedded_texture")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gltf_load_options() {
        let options = MeshLoadOptions::default()
            .with_scale(Vec3::splat(0.01))
            .generate_tangents(true);

        assert_eq!(options.scale, Vec3::splat(0.01));
        assert!(options.generate_tangents);
    }
}
