use bevy::prelude::*;

/// Extended PBR material with full texture support
#[derive(Clone, Debug, Default)]
pub struct ExtendedPBRMaterial {
    // Base properties
    pub base_color: Color,
    pub base_color_texture: Option<Handle<Image>>,

    // Metallic-Roughness workflow
    pub metallic: f32,
    pub roughness: f32,
    pub metallic_roughness_texture: Option<Handle<Image>>,

    // Normal mapping
    pub normal_map: Option<Handle<Image>>,
    pub normal_map_strength: f32,

    // Occlusion
    pub occlusion_texture: Option<Handle<Image>>,
    pub occlusion_strength: f32,

    // Emissive
    pub emissive: Color,
    pub emissive_texture: Option<Handle<Image>>,
    pub emissive_strength: f32,

    // Advanced properties
    pub clearcoat: f32,
    pub clearcoat_roughness: f32,
    pub clearcoat_normal_map: Option<Handle<Image>>,

    pub anisotropy: f32,
    pub anisotropy_rotation: f32,

    pub sheen: f32,
    pub sheen_color: Color,
    pub sheen_roughness: f32,

    pub transmission: f32,
    pub ior: f32, // Index of refraction
    pub thickness: f32,

    pub alpha_mode: AlphaMode,
    pub double_sided: bool,

    // Parallax mapping
    pub height_map: Option<Handle<Image>>,
    pub parallax_scale: f32,
    pub parallax_layers: u32,
}

impl ExtendedPBRMaterial {
    /// Create a new material with default values
    pub fn new() -> Self {
        Self {
            base_color: Color::srgb(0.8, 0.8, 0.8),
            metallic: 0.0,
            roughness: 0.5,
            normal_map_strength: 1.0,
            occlusion_strength: 1.0,
            emissive: Color::BLACK,
            emissive_strength: 1.0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.03,
            anisotropy: 0.0,
            anisotropy_rotation: 0.0,
            sheen: 0.0,
            sheen_color: Color::WHITE,
            sheen_roughness: 0.0,
            transmission: 0.0,
            ior: 1.5, // Glass IOR
            thickness: 0.0,
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
            parallax_scale: 0.1,
            parallax_layers: 8,
            ..Default::default()
        }
    }

    /// Convert to Bevy StandardMaterial (with texture support)
    pub fn to_standard_material(&self) -> StandardMaterial {
        StandardMaterial {
            base_color: self.base_color,
            base_color_texture: self.base_color_texture.clone(),
            metallic: self.metallic,
            perceptual_roughness: self.roughness,
            metallic_roughness_texture: self.metallic_roughness_texture.clone(),
            normal_map_texture: self.normal_map.clone(),
            occlusion_texture: self.occlusion_texture.clone(),
            emissive: self.emissive.into(),
            emissive_texture: self.emissive_texture.clone(),
            alpha_mode: self.alpha_mode,
            double_sided: self.double_sided,
            ..Default::default()
        }
    }

    /// Create glass material with transmission
    pub fn glass(color: Color, roughness: f32, ior: f32) -> Self {
        Self {
            base_color: color,
            metallic: 0.0,
            roughness,
            transmission: 1.0,
            ior,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            ..Self::new()
        }
    }

    /// Create car paint material with clearcoat
    pub fn car_paint(base_color: Color, clearcoat_roughness: f32) -> Self {
        Self {
            base_color,
            metallic: 0.8,
            roughness: 0.3,
            clearcoat: 1.0,
            clearcoat_roughness,
            ..Self::new()
        }
    }

    /// Create fabric material with sheen
    pub fn fabric(color: Color, sheen_intensity: f32) -> Self {
        Self {
            base_color: color,
            metallic: 0.0,
            roughness: 0.9,
            sheen: sheen_intensity,
            sheen_color: Color::WHITE,
            sheen_roughness: 0.5,
            ..Self::new()
        }
    }

    /// Create anisotropic metal (brushed metal)
    pub fn brushed_metal(color: Color, anisotropy: f32, rotation: f32) -> Self {
        Self {
            base_color: color,
            metallic: 1.0,
            roughness: 0.3,
            anisotropy,
            anisotropy_rotation: rotation,
            ..Self::new()
        }
    }

    /// Create material with parallax mapping
    pub fn with_parallax(mut self, height_map: Handle<Image>, scale: f32) -> Self {
        self.height_map = Some(height_map);
        self.parallax_scale = scale;
        self
    }

    /// Add normal map
    pub fn with_normal_map(mut self, normal_map: Handle<Image>, strength: f32) -> Self {
        self.normal_map = Some(normal_map);
        self.normal_map_strength = strength;
        self
    }

    /// Add ambient occlusion map
    pub fn with_occlusion(mut self, occlusion: Handle<Image>, strength: f32) -> Self {
        self.occlusion_texture = Some(occlusion);
        self.occlusion_strength = strength;
        self
    }
}

/// Texture set for complete PBR material
#[derive(Clone, Debug, Default)]
pub struct PBRTextureSet {
    pub albedo: Option<Handle<Image>>,
    pub normal: Option<Handle<Image>>,
    pub metallic_roughness: Option<Handle<Image>>,
    pub occlusion: Option<Handle<Image>>,
    pub emissive: Option<Handle<Image>>,
    pub height: Option<Handle<Image>>,
}

impl PBRTextureSet {
    /// Create from file paths
    pub fn from_paths(
        asset_server: &AssetServer,
        albedo: Option<&str>,
        normal: Option<&str>,
        metallic_roughness: Option<&str>,
        occlusion: Option<&str>,
        emissive: Option<&str>,
        height: Option<&str>,
    ) -> Self {
        Self {
            albedo: albedo.map(|p| asset_server.load(p)),
            normal: normal.map(|p| asset_server.load(p)),
            metallic_roughness: metallic_roughness.map(|p| asset_server.load(p)),
            occlusion: occlusion.map(|p| asset_server.load(p)),
            emissive: emissive.map(|p| asset_server.load(p)),
            height: height.map(|p| asset_server.load(p)),
        }
    }

    /// Apply textures to material
    pub fn apply_to_material(&self, material: &mut ExtendedPBRMaterial) {
        if let Some(ref tex) = self.albedo {
            material.base_color_texture = Some(tex.clone());
        }
        if let Some(ref tex) = self.normal {
            material.normal_map = Some(tex.clone());
        }
        if let Some(ref tex) = self.metallic_roughness {
            material.metallic_roughness_texture = Some(tex.clone());
        }
        if let Some(ref tex) = self.occlusion {
            material.occlusion_texture = Some(tex.clone());
        }
        if let Some(ref tex) = self.emissive {
            material.emissive_texture = Some(tex.clone());
        }
        if let Some(ref tex) = self.height {
            material.height_map = Some(tex.clone());
        }
    }
}

/// Advanced material presets using extended PBR
pub struct AdvancedMaterialPresets;

impl AdvancedMaterialPresets {
    /// Chrome (highly reflective metal)
    pub fn chrome() -> ExtendedPBRMaterial {
        ExtendedPBRMaterial {
            base_color: Color::srgb(0.95, 0.95, 0.95),
            metallic: 1.0,
            roughness: 0.05,
            ..ExtendedPBRMaterial::new()
        }
    }

    /// Brushed aluminum with anisotropic reflection
    pub fn brushed_aluminum() -> ExtendedPBRMaterial {
        ExtendedPBRMaterial::brushed_metal(
            Color::srgb(0.91, 0.92, 0.92),
            0.8,
            0.0, // Horizontal brushing
        )
    }

    /// Car paint with clearcoat
    pub fn car_paint_red() -> ExtendedPBRMaterial {
        ExtendedPBRMaterial::car_paint(Color::srgb(0.8, 0.1, 0.1), 0.03)
    }

    /// Velvet fabric with sheen
    pub fn velvet(color: Color) -> ExtendedPBRMaterial {
        ExtendedPBRMaterial::fabric(color, 1.0)
    }

    /// Translucent plastic
    pub fn translucent_plastic(color: Color, thickness: f32) -> ExtendedPBRMaterial {
        ExtendedPBRMaterial {
            base_color: color,
            metallic: 0.0,
            roughness: 0.2,
            transmission: 0.9,
            ior: 1.45,
            thickness,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            ..ExtendedPBRMaterial::new()
        }
    }

    /// Clear glass
    pub fn clear_glass() -> ExtendedPBRMaterial {
        ExtendedPBRMaterial::glass(Color::srgba(1.0, 1.0, 1.0, 0.1), 0.0, 1.52)
    }

    /// Frosted glass
    pub fn frosted_glass() -> ExtendedPBRMaterial {
        ExtendedPBRMaterial::glass(Color::srgba(0.95, 0.95, 0.95, 0.3), 0.4, 1.52)
    }

    /// Carbon fiber (with potential for texture)
    pub fn carbon_fiber() -> ExtendedPBRMaterial {
        ExtendedPBRMaterial {
            base_color: Color::srgb(0.05, 0.05, 0.05),
            metallic: 0.0,
            roughness: 0.2,
            anisotropy: 0.3,
            ..ExtendedPBRMaterial::new()
        }
    }

    /// Ceramic with subtle roughness variation
    pub fn ceramic(color: Color) -> ExtendedPBRMaterial {
        ExtendedPBRMaterial {
            base_color: color,
            metallic: 0.0,
            roughness: 0.15,
            ..ExtendedPBRMaterial::new()
        }
    }

    /// Leather
    pub fn leather(color: Color) -> ExtendedPBRMaterial {
        ExtendedPBRMaterial {
            base_color: color,
            metallic: 0.0,
            roughness: 0.7,
            ..ExtendedPBRMaterial::new()
        }
    }

    /// Wet surface (enhanced reflectivity)
    pub fn wet_surface(base_color: Color) -> ExtendedPBRMaterial {
        ExtendedPBRMaterial {
            base_color,
            metallic: 0.0,
            roughness: 0.1,
            clearcoat: 0.5,
            clearcoat_roughness: 0.0,
            ..ExtendedPBRMaterial::new()
        }
    }

    /// Iridescent material
    pub fn iridescent(base_color: Color) -> ExtendedPBRMaterial {
        ExtendedPBRMaterial {
            base_color,
            metallic: 0.8,
            roughness: 0.2,
            clearcoat: 0.8,
            clearcoat_roughness: 0.1,
            ..ExtendedPBRMaterial::new()
        }
    }
}

/// Material layer for layered materials
#[derive(Clone, Debug)]
pub struct MaterialLayer {
    pub material: ExtendedPBRMaterial,
    pub blend_mode: LayerBlendMode,
    pub opacity: f32,
    pub mask: Option<Handle<Image>>,
}

#[derive(Clone, Debug)]
pub enum LayerBlendMode {
    Mix,
    Add,
    Multiply,
    Screen,
    Overlay,
}

/// Complex material with multiple layers
#[derive(Clone, Debug)]
pub struct LayeredMaterial {
    pub base_layer: ExtendedPBRMaterial,
    pub additional_layers: Vec<MaterialLayer>,
}

impl LayeredMaterial {
    pub fn new(base: ExtendedPBRMaterial) -> Self {
        Self {
            base_layer: base,
            additional_layers: Vec::new(),
        }
    }

    pub fn add_layer(
        &mut self,
        material: ExtendedPBRMaterial,
        blend_mode: LayerBlendMode,
        opacity: f32,
    ) {
        self.additional_layers.push(MaterialLayer {
            material,
            blend_mode,
            opacity,
            mask: None,
        });
    }

    pub fn add_layer_with_mask(
        &mut self,
        material: ExtendedPBRMaterial,
        blend_mode: LayerBlendMode,
        opacity: f32,
        mask: Handle<Image>,
    ) {
        self.additional_layers.push(MaterialLayer {
            material,
            blend_mode,
            opacity,
            mask: Some(mask),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_pbr_creation() {
        let mat = ExtendedPBRMaterial::new();
        assert_eq!(mat.metallic, 0.0);
        assert_eq!(mat.roughness, 0.5);
        assert_eq!(mat.ior, 1.5);
    }

    #[test]
    fn test_glass_material() {
        let glass = ExtendedPBRMaterial::glass(Color::WHITE, 0.0, 1.52);
        assert_eq!(glass.transmission, 1.0);
        assert_eq!(glass.ior, 1.52);
        assert!(matches!(glass.alpha_mode, AlphaMode::Blend));
    }

    #[test]
    fn test_car_paint() {
        let paint = ExtendedPBRMaterial::car_paint(Color::srgb(0.8, 0.1, 0.1), 0.03);
        assert_eq!(paint.clearcoat, 1.0);
        assert_eq!(paint.clearcoat_roughness, 0.03);
    }

    #[test]
    fn test_fabric() {
        let fabric = ExtendedPBRMaterial::fabric(Color::srgb(0.3, 0.3, 0.8), 0.8);
        assert_eq!(fabric.sheen, 0.8);
        assert!(fabric.roughness > 0.5);
    }

    #[test]
    fn test_brushed_metal() {
        let metal = ExtendedPBRMaterial::brushed_metal(Color::srgb(0.9, 0.9, 0.9), 0.7, 45.0);
        assert_eq!(metal.metallic, 1.0);
        assert_eq!(metal.anisotropy, 0.7);
        assert_eq!(metal.anisotropy_rotation, 45.0);
    }

    #[test]
    fn test_advanced_presets() {
        let chrome = AdvancedMaterialPresets::chrome();
        assert_eq!(chrome.metallic, 1.0);
        assert!(chrome.roughness < 0.1);

        let glass = AdvancedMaterialPresets::clear_glass();
        assert_eq!(glass.ior, 1.52);
        assert_eq!(glass.transmission, 1.0);
    }

    #[test]
    fn test_layered_material() {
        let base = ExtendedPBRMaterial::new();
        let mut layered = LayeredMaterial::new(base);

        let layer_mat = AdvancedMaterialPresets::chrome();
        layered.add_layer(layer_mat, LayerBlendMode::Mix, 0.5);

        assert_eq!(layered.additional_layers.len(), 1);
        assert_eq!(layered.additional_layers[0].opacity, 0.5);
    }

    #[test]
    fn test_texture_set() {
        let textures = PBRTextureSet::default();
        assert!(textures.albedo.is_none());
        assert!(textures.normal.is_none());
    }
}
