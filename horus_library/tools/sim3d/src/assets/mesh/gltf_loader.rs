//! glTF/GLB mesh loader
//!
//! Loads glTF 2.0 and GLB (binary glTF) files
//! glTF is the modern standard for 3D asset interchange

use super::{processing::*, EmbeddedTexture, LoadedMesh, MaterialInfo, MeshLoadOptions};
use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Load a glTF or GLB file
pub fn load_gltf(path: &Path, options: &MeshLoadOptions) -> Result<LoadedMesh> {
    tracing::debug!("Loading glTF file: {}", path.display());

    // Load glTF document with images for embedded texture extraction
    let (document, buffers, images) = gltf::import(path)
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

    // Extract embedded textures from the document
    let embedded_textures = extract_embedded_textures(&document, &images);

    // Build a map of image index to embedded texture ID for path resolution
    let embedded_texture_map: HashMap<usize, String> = embedded_textures
        .iter()
        .filter_map(|tex| {
            // Parse the image index from the texture ID (format: "embedded_{index}")
            tex.id
                .strip_prefix("embedded_")
                .and_then(|idx| idx.parse::<usize>().ok())
                .map(|idx| (idx, tex.id.clone()))
        })
        .collect();

    // Extract material information with embedded texture support
    let materials = extract_gltf_materials_with_embedded(&primitive, path, &embedded_texture_map);

    // Extract texture paths with embedded texture support
    let texture_paths =
        extract_texture_paths_with_embedded(&primitive, path, &embedded_texture_map);

    tracing::debug!(
        "Extracted {} embedded textures from glTF file",
        embedded_textures.len()
    );

    Ok(LoadedMesh {
        mesh,
        materials,
        texture_paths,
        embedded_textures,
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
            // Embedded texture - return synthetic path with image index
            PathBuf::from(format!("embedded_{}", image.index()))
        }
    }
}

/// Resolve texture from image source with embedded texture map support
fn resolve_texture_from_image_with_embedded(
    gltf_path: &Path,
    image: &gltf::Image,
    embedded_map: &HashMap<usize, String>,
) -> PathBuf {
    match image.source() {
        gltf::image::Source::Uri { uri, .. } => {
            // Resolve relative to glTF file
            let gltf_dir = gltf_path.parent().unwrap_or(Path::new("."));
            gltf_dir.join(uri)
        }
        gltf::image::Source::View { .. } => {
            // Embedded texture - use the mapped ID if available
            if let Some(texture_id) = embedded_map.get(&image.index()) {
                PathBuf::from(texture_id)
            } else {
                PathBuf::from(format!("embedded_{}", image.index()))
            }
        }
    }
}

/// Extract embedded textures from glTF document
fn extract_embedded_textures(
    document: &gltf::Document,
    images: &[gltf::image::Data],
) -> Vec<EmbeddedTexture> {
    let mut embedded_textures = Vec::new();

    for (idx, image) in document.images().enumerate() {
        match image.source() {
            gltf::image::Source::View { mime_type, .. } => {
                // This is an embedded texture - extract its data
                if idx < images.len() {
                    let image_data = &images[idx];

                    // Convert the decoded image data back to a standard format
                    let (data, mime) = encode_image_data(image_data, mime_type);

                    embedded_textures.push(EmbeddedTexture {
                        id: format!("embedded_{}", idx),
                        mime_type: mime,
                        data,
                    });

                    tracing::debug!(
                        "Extracted embedded texture {} ({} bytes, {})",
                        idx,
                        embedded_textures.last().map(|t| t.data.len()).unwrap_or(0),
                        mime_type
                    );
                }
            }
            gltf::image::Source::Uri { .. } => {
                // External texture - skip (not embedded)
            }
        }
    }

    embedded_textures
}

/// Encode image data to a standard format (PNG or JPEG)
fn encode_image_data(image_data: &gltf::image::Data, original_mime: &str) -> (Vec<u8>, String) {
    use std::io::Cursor;

    // The gltf crate decodes images to raw pixels, so we need to re-encode them
    let width = image_data.width;
    let height = image_data.height;
    let pixels = &image_data.pixels;

    // Determine the image format based on original MIME type or pixel format
    let color_type = match image_data.format {
        gltf::image::Format::R8 => image::ColorType::L8,
        gltf::image::Format::R8G8 => image::ColorType::La8,
        gltf::image::Format::R8G8B8 => image::ColorType::Rgb8,
        gltf::image::Format::R8G8B8A8 => image::ColorType::Rgba8,
        gltf::image::Format::R16 => image::ColorType::L16,
        gltf::image::Format::R16G16 => image::ColorType::La16,
        gltf::image::Format::R16G16B16 => image::ColorType::Rgb16,
        gltf::image::Format::R16G16B16A16 => image::ColorType::Rgba16,
        gltf::image::Format::R32G32B32FLOAT => {
            // Convert float to u8 for PNG encoding
            let rgb8: Vec<u8> = pixels
                .chunks(12)
                .flat_map(|chunk| {
                    // Each channel is 4 bytes (f32)
                    let r = (f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                        .clamp(0.0, 1.0)
                        * 255.0) as u8;
                    let g = (f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]])
                        .clamp(0.0, 1.0)
                        * 255.0) as u8;
                    let b = (f32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]])
                        .clamp(0.0, 1.0)
                        * 255.0) as u8;
                    [r, g, b]
                })
                .collect();

            let mut cursor = Cursor::new(Vec::new());
            if let Ok(()) = image::write_buffer_with_format(
                &mut cursor,
                &rgb8,
                width,
                height,
                image::ColorType::Rgb8,
                image::ImageFormat::Png,
            ) {
                return (cursor.into_inner(), "image/png".to_string());
            }
            // Fallback: return raw data with original mime type
            return (pixels.clone(), original_mime.to_string());
        }
        gltf::image::Format::R32G32B32A32FLOAT => {
            // Convert float to u8 for PNG encoding
            let rgba8: Vec<u8> = pixels
                .chunks(16)
                .flat_map(|chunk| {
                    let r = (f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                        .clamp(0.0, 1.0)
                        * 255.0) as u8;
                    let g = (f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]])
                        .clamp(0.0, 1.0)
                        * 255.0) as u8;
                    let b = (f32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]])
                        .clamp(0.0, 1.0)
                        * 255.0) as u8;
                    let a = (f32::from_le_bytes([chunk[12], chunk[13], chunk[14], chunk[15]])
                        .clamp(0.0, 1.0)
                        * 255.0) as u8;
                    [r, g, b, a]
                })
                .collect();

            let mut cursor = Cursor::new(Vec::new());
            if let Ok(()) = image::write_buffer_with_format(
                &mut cursor,
                &rgba8,
                width,
                height,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            ) {
                return (cursor.into_inner(), "image/png".to_string());
            }
            // Fallback: return raw data with original mime type
            return (pixels.clone(), original_mime.to_string());
        }
    };

    // Encode to PNG (lossless and widely supported)
    let mut cursor = Cursor::new(Vec::new());
    if let Ok(()) = image::write_buffer_with_format(
        &mut cursor,
        pixels,
        width,
        height,
        color_type,
        image::ImageFormat::Png,
    ) {
        (cursor.into_inner(), "image/png".to_string())
    } else {
        // Fallback: return raw pixel data
        (pixels.clone(), original_mime.to_string())
    }
}

/// Extract material information from glTF with embedded texture support
fn extract_gltf_materials_with_embedded(
    primitive: &gltf::Primitive,
    gltf_path: &Path,
    embedded_map: &HashMap<usize, String>,
) -> Vec<MaterialInfo> {
    let mut materials = Vec::new();

    let material = primitive.material();
    let pbr = material.pbr_metallic_roughness();

    let base_color = pbr.base_color_factor();
    let diffuse_color = Color::srgba(base_color[0], base_color[1], base_color[2], base_color[3]);

    let diffuse_texture = pbr.base_color_texture().map(|info| {
        resolve_texture_from_image_with_embedded(gltf_path, &info.texture().source(), embedded_map)
    });

    let normal_texture = material.normal_texture().map(|normal_tex| {
        resolve_texture_from_image_with_embedded(
            gltf_path,
            &normal_tex.texture().source(),
            embedded_map,
        )
    });

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

/// Extract texture paths from glTF materials with embedded texture support
fn extract_texture_paths_with_embedded(
    primitive: &gltf::Primitive,
    gltf_path: &Path,
    embedded_map: &HashMap<usize, String>,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let material = primitive.material();
    let pbr = material.pbr_metallic_roughness();

    if let Some(tex_info) = pbr.base_color_texture() {
        paths.push(resolve_texture_from_image_with_embedded(
            gltf_path,
            &tex_info.texture().source(),
            embedded_map,
        ));
    }

    if let Some(normal_tex) = material.normal_texture() {
        paths.push(resolve_texture_from_image_with_embedded(
            gltf_path,
            &normal_tex.texture().source(),
            embedded_map,
        ));
    }

    paths
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
