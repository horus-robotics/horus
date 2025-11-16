use bevy::prelude::*;
use std::collections::HashMap;

/// PBR material properties
#[derive(Clone, Debug)]
pub struct PBRMaterialProperties {
    /// Base color (albedo)
    pub base_color: Color,
    /// Metallic factor (0.0 = dielectric, 1.0 = metallic)
    pub metallic: f32,
    /// Roughness factor (0.0 = smooth, 1.0 = rough)
    pub roughness: f32,
    /// Emissive color (self-illumination)
    pub emissive: Color,
    /// Alpha mode (opaque, mask, blend)
    pub alpha_mode: AlphaMode,
}

impl Default for PBRMaterialProperties {
    fn default() -> Self {
        Self {
            base_color: Color::srgb(0.8, 0.8, 0.8),
            metallic: 0.0,
            roughness: 0.5,
            emissive: Color::BLACK,
            alpha_mode: AlphaMode::Opaque,
        }
    }
}

impl PBRMaterialProperties {
    /// Create a metallic material
    pub fn metallic(color: Color, roughness: f32) -> Self {
        Self {
            base_color: color,
            metallic: 1.0,
            roughness,
            ..Default::default()
        }
    }

    /// Create a dielectric (non-metallic) material
    pub fn dielectric(color: Color, roughness: f32) -> Self {
        Self {
            base_color: color,
            metallic: 0.0,
            roughness,
            ..Default::default()
        }
    }

    /// Create an emissive material
    pub fn emissive(color: Color, intensity: f32) -> Self {
        Self {
            base_color: Color::BLACK,
            emissive: Color::srgb(
                color.to_srgba().red * intensity,
                color.to_srgba().green * intensity,
                color.to_srgba().blue * intensity,
            ),
            ..Default::default()
        }
    }

    /// Convert to Bevy StandardMaterial
    pub fn to_standard_material(&self) -> StandardMaterial {
        StandardMaterial {
            base_color: self.base_color,
            metallic: self.metallic,
            perceptual_roughness: self.roughness,
            emissive: self.emissive.into(),
            alpha_mode: self.alpha_mode,
            ..Default::default()
        }
    }
}

/// Material presets for common robot parts
pub struct MaterialPresets;

impl MaterialPresets {
    /// Brushed aluminum
    pub fn aluminum() -> PBRMaterialProperties {
        PBRMaterialProperties::metallic(Color::srgb(0.91, 0.92, 0.92), 0.3)
    }

    /// Steel/iron
    pub fn steel() -> PBRMaterialProperties {
        PBRMaterialProperties::metallic(Color::srgb(0.6, 0.6, 0.65), 0.4)
    }

    /// Copper
    pub fn copper() -> PBRMaterialProperties {
        PBRMaterialProperties::metallic(Color::srgb(0.95, 0.64, 0.54), 0.2)
    }

    /// Gold
    pub fn gold() -> PBRMaterialProperties {
        PBRMaterialProperties::metallic(Color::srgb(1.0, 0.85, 0.0), 0.1)
    }

    /// Matte plastic
    pub fn plastic_matte(color: Color) -> PBRMaterialProperties {
        PBRMaterialProperties::dielectric(color, 0.8)
    }

    /// Glossy plastic
    pub fn plastic_glossy(color: Color) -> PBRMaterialProperties {
        PBRMaterialProperties::dielectric(color, 0.2)
    }

    /// Rubber
    pub fn rubber(color: Color) -> PBRMaterialProperties {
        PBRMaterialProperties::dielectric(color, 0.9)
    }

    /// Glass (transparent)
    pub fn glass() -> PBRMaterialProperties {
        PBRMaterialProperties {
            base_color: Color::srgba(0.9, 0.9, 0.95, 0.3),
            metallic: 0.0,
            roughness: 0.0,
            alpha_mode: AlphaMode::Blend,
            ..Default::default()
        }
    }

    /// Carbon fiber
    pub fn carbon_fiber() -> PBRMaterialProperties {
        PBRMaterialProperties::dielectric(Color::srgb(0.1, 0.1, 0.1), 0.3)
    }

    /// Wood
    pub fn wood() -> PBRMaterialProperties {
        PBRMaterialProperties::dielectric(Color::srgb(0.55, 0.4, 0.3), 0.7)
    }

    /// Concrete
    pub fn concrete() -> PBRMaterialProperties {
        PBRMaterialProperties::dielectric(Color::srgb(0.6, 0.6, 0.6), 0.9)
    }

    /// LED light (emissive)
    pub fn led(color: Color, intensity: f32) -> PBRMaterialProperties {
        PBRMaterialProperties::emissive(color, intensity)
    }
}

/// Material library for managing and reusing materials
#[derive(Resource, Default)]
pub struct MaterialLibrary {
    materials: HashMap<String, Handle<StandardMaterial>>,
}

impl MaterialLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a material to the library
    pub fn add(
        &mut self,
        name: impl Into<String>,
        handle: Handle<StandardMaterial>,
    ) {
        self.materials.insert(name.into(), handle);
    }

    /// Get a material by name
    pub fn get(&self, name: &str) -> Option<Handle<StandardMaterial>> {
        self.materials.get(name).cloned()
    }

    /// Remove a material from the library
    pub fn remove(&mut self, name: &str) -> Option<Handle<StandardMaterial>> {
        self.materials.remove(name)
    }

    /// Check if material exists
    pub fn contains(&self, name: &str) -> bool {
        self.materials.contains_key(name)
    }

    /// Get all material names
    pub fn names(&self) -> Vec<String> {
        self.materials.keys().cloned().collect()
    }

    /// Clear all materials
    pub fn clear(&mut self) {
        self.materials.clear();
    }

    /// Get number of materials
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Check if library is empty
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

/// Helper functions for creating and managing materials
pub struct MaterialUtils;

impl MaterialUtils {
    /// Create standard material from properties
    pub fn create_standard(
        properties: &PBRMaterialProperties,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        materials.add(properties.to_standard_material())
    }

    /// Create colored material
    pub fn create_colored(
        color: Color,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        materials.add(StandardMaterial {
            base_color: color,
            ..Default::default()
        })
    }

    /// Create wireframe material
    pub fn create_wireframe(
        color: Color,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..Default::default()
        })
    }

    /// Create unlit material (no lighting calculations)
    pub fn create_unlit(
        color: Color,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..Default::default()
        })
    }

    /// Update material color
    pub fn update_color(
        material: &mut StandardMaterial,
        color: Color,
    ) {
        material.base_color = color;
    }

    /// Update material transparency
    pub fn update_alpha(
        material: &mut StandardMaterial,
        alpha: f32,
        alpha_mode: AlphaMode,
    ) {
        material.base_color = material.base_color.with_alpha(alpha);
        material.alpha_mode = alpha_mode;
    }

    /// Blend two colors
    pub fn blend_colors(a: Color, b: Color, t: f32) -> Color {
        let a_srgba = a.to_srgba();
        let b_srgba = b.to_srgba();

        Color::srgba(
            a_srgba.red * (1.0 - t) + b_srgba.red * t,
            a_srgba.green * (1.0 - t) + b_srgba.green * t,
            a_srgba.blue * (1.0 - t) + b_srgba.blue * t,
            a_srgba.alpha * (1.0 - t) + b_srgba.alpha * t,
        )
    }
}

/// Color palette utilities
pub struct ColorPalette;

impl ColorPalette {
    /// Robot part colors
    pub fn robot_base() -> Color {
        Color::srgb(0.3, 0.3, 0.35)
    }

    pub fn robot_joint() -> Color {
        Color::srgb(0.5, 0.5, 0.55)
    }

    pub fn robot_link() -> Color {
        Color::srgb(0.7, 0.7, 0.75)
    }

    /// Sensor colors
    pub fn sensor_lidar() -> Color {
        Color::srgb(0.2, 0.6, 0.9)
    }

    pub fn sensor_camera() -> Color {
        Color::srgb(0.9, 0.3, 0.3)
    }

    pub fn sensor_imu() -> Color {
        Color::srgb(0.3, 0.9, 0.3)
    }

    pub fn sensor_gps() -> Color {
        Color::srgb(0.9, 0.7, 0.2)
    }

    /// Environment colors
    pub fn ground() -> Color {
        Color::srgb(0.4, 0.5, 0.4)
    }

    pub fn sky() -> Color {
        Color::srgb(0.53, 0.81, 0.92)
    }

    pub fn obstacle() -> Color {
        Color::srgb(0.8, 0.4, 0.2)
    }

    /// Status colors
    pub fn success() -> Color {
        Color::srgb(0.2, 0.8, 0.2)
    }

    pub fn warning() -> Color {
        Color::srgb(0.9, 0.7, 0.0)
    }

    pub fn error() -> Color {
        Color::srgb(0.9, 0.2, 0.2)
    }

    pub fn neutral() -> Color {
        Color::srgb(0.5, 0.5, 0.5)
    }

    /// Generate color from HSV
    pub fn from_hsv(hue: f32, saturation: f32, value: f32) -> Color {
        let c = value * saturation;
        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = value - c;

        let (r, g, b) = if hue < 60.0 {
            (c, x, 0.0)
        } else if hue < 120.0 {
            (x, c, 0.0)
        } else if hue < 180.0 {
            (0.0, c, x)
        } else if hue < 240.0 {
            (0.0, x, c)
        } else if hue < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color::srgb(r + m, g + m, b + m)
    }

    /// Generate rainbow colors for visualization
    pub fn rainbow(count: usize) -> Vec<Color> {
        (0..count)
            .map(|i| {
                let hue = (i as f32 / count as f32) * 360.0;
                Self::from_hsv(hue, 0.8, 0.9)
            })
            .collect()
    }
}

/// Plugin to register material systems
pub struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MaterialLibrary>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbr_material_creation() {
        let metallic = PBRMaterialProperties::metallic(Color::WHITE, 0.5);
        assert_eq!(metallic.metallic, 1.0);
        assert_eq!(metallic.roughness, 0.5);

        let dielectric = PBRMaterialProperties::dielectric(Color::WHITE, 0.5);
        assert_eq!(dielectric.metallic, 0.0);
    }

    #[test]
    fn test_material_library() {
        let mut library = MaterialLibrary::new();
        assert!(library.is_empty());

        let handle: Handle<StandardMaterial> = Handle::default();
        library.add("test_material", handle.clone());

        assert_eq!(library.len(), 1);
        assert!(library.contains("test_material"));
        assert!(library.get("test_material").is_some());

        library.remove("test_material");
        assert!(library.is_empty());
    }

    #[test]
    fn test_color_blend() {
        let a = Color::srgb(1.0, 0.0, 0.0);
        let b = Color::srgb(0.0, 0.0, 1.0);
        let mid = MaterialUtils::blend_colors(a, b, 0.5);

        let mid_srgba = mid.to_srgba();
        assert!((mid_srgba.red - 0.5).abs() < 0.01);
        assert!((mid_srgba.blue - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_material_presets() {
        let aluminum = MaterialPresets::aluminum();
        assert_eq!(aluminum.metallic, 1.0);

        let plastic = MaterialPresets::plastic_matte(Color::WHITE);
        assert_eq!(plastic.metallic, 0.0);
        assert!(plastic.roughness > 0.5);
    }

    #[test]
    fn test_color_palette() {
        let ground = ColorPalette::ground();
        assert!(ground.to_srgba().red > 0.0);

        let rainbow = ColorPalette::rainbow(5);
        assert_eq!(rainbow.len(), 5);
    }

    #[test]
    fn test_hsv_conversion() {
        let red = ColorPalette::from_hsv(0.0, 1.0, 1.0);
        assert!((red.to_srgba().red - 1.0).abs() < 0.01);

        let green = ColorPalette::from_hsv(120.0, 1.0, 1.0);
        assert!((green.to_srgba().green - 1.0).abs() < 0.01);
    }
}
