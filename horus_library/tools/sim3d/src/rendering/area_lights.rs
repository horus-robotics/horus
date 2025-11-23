use bevy::prelude::*;

/// Area light shape
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AreaLightShape {
    Rectangle { width: f32, height: f32 },
    Disk { radius: f32 },
    Sphere { radius: f32 },
    Tube { length: f32, radius: f32 },
}

/// Area light component
#[derive(Component, Clone, Debug)]
pub struct AreaLight {
    pub shape: AreaLightShape,
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub two_sided: bool,
    pub cast_shadows: bool,
    pub falloff: LightFalloff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightFalloff {
    /// Physically accurate inverse square falloff
    InverseSquare,
    /// Linear falloff
    Linear,
    /// Custom exponent
    Custom,
}

impl Default for AreaLight {
    fn default() -> Self {
        Self {
            shape: AreaLightShape::Rectangle {
                width: 1.0,
                height: 1.0,
            },
            color: Color::WHITE,
            intensity: 10.0,
            range: 10.0,
            two_sided: false,
            cast_shadows: true,
            falloff: LightFalloff::InverseSquare,
        }
    }
}

impl AreaLight {
    /// Create rectangular area light
    pub fn rectangle(width: f32, height: f32, intensity: f32) -> Self {
        Self {
            shape: AreaLightShape::Rectangle { width, height },
            intensity,
            ..Default::default()
        }
    }

    /// Create disk area light
    pub fn disk(radius: f32, intensity: f32) -> Self {
        Self {
            shape: AreaLightShape::Disk { radius },
            intensity,
            ..Default::default()
        }
    }

    /// Create sphere area light
    pub fn sphere(radius: f32, intensity: f32) -> Self {
        Self {
            shape: AreaLightShape::Sphere { radius },
            intensity,
            ..Default::default()
        }
    }

    /// Create tube area light
    pub fn tube(length: f32, radius: f32, intensity: f32) -> Self {
        Self {
            shape: AreaLightShape::Tube { length, radius },
            intensity,
            ..Default::default()
        }
    }

    /// Set light color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set light range
    pub fn with_range(mut self, range: f32) -> Self {
        self.range = range;
        self
    }

    /// Enable two-sided emission
    pub fn two_sided(mut self) -> Self {
        self.two_sided = true;
        self
    }

    /// Calculate surface area
    pub fn surface_area(&self) -> f32 {
        match self.shape {
            AreaLightShape::Rectangle { width, height } => width * height,
            AreaLightShape::Disk { radius } => std::f32::consts::PI * radius * radius,
            AreaLightShape::Sphere { radius } => 4.0 * std::f32::consts::PI * radius * radius,
            AreaLightShape::Tube { length, radius } => 2.0 * std::f32::consts::PI * radius * length,
        }
    }

    /// Calculate power (intensity * area)
    pub fn power(&self) -> f32 {
        self.intensity * self.surface_area()
    }

    /// Sample a point on the light surface for soft shadows
    pub fn sample_point(&self, u: f32, v: f32, transform: &Transform) -> Vec3 {
        let local_point = match self.shape {
            AreaLightShape::Rectangle { width, height } => {
                Vec3::new((u - 0.5) * width, (v - 0.5) * height, 0.0)
            }
            AreaLightShape::Disk { radius } => {
                let angle = u * 2.0 * std::f32::consts::PI;
                let r = v.sqrt() * radius;
                Vec3::new(angle.cos() * r, angle.sin() * r, 0.0)
            }
            AreaLightShape::Sphere { radius } => {
                // Uniform sphere sampling
                let theta = u * 2.0 * std::f32::consts::PI;
                let phi = (2.0 * v - 1.0).acos();
                Vec3::new(
                    radius * phi.sin() * theta.cos(),
                    radius * phi.sin() * theta.sin(),
                    radius * phi.cos(),
                )
            }
            AreaLightShape::Tube { length, radius } => {
                let angle = u * 2.0 * std::f32::consts::PI;
                let z = (v - 0.5) * length;
                Vec3::new(angle.cos() * radius, angle.sin() * radius, z)
            }
        };

        transform.transform_point(local_point)
    }

    /// Calculate illumination at a point
    pub fn illumination_at(&self, point: Vec3, normal: Vec3, light_transform: &Transform) -> Color {
        let to_light = light_transform.translation - point;
        let distance = to_light.length();

        if distance > self.range {
            return Color::BLACK;
        }

        let light_dir = to_light / distance;
        let ndotl = normal.dot(light_dir).max(0.0);

        // Calculate falloff
        let attenuation = match self.falloff {
            LightFalloff::InverseSquare => 1.0 / (distance * distance + 1.0),
            LightFalloff::Linear => (1.0 - (distance / self.range)).max(0.0),
            LightFalloff::Custom => (1.0 - (distance / self.range).powi(2)).max(0.0),
        };

        let intensity = self.intensity * ndotl * attenuation;

        Color::srgb(
            self.color.to_srgba().red * intensity,
            self.color.to_srgba().green * intensity,
            self.color.to_srgba().blue * intensity,
        )
    }
}

/// Area light sampling configuration
#[derive(Resource, Clone, Debug)]
pub struct AreaLightSamplingConfig {
    /// Number of samples for soft shadows
    pub soft_shadow_samples: u32,

    /// Enable importance sampling
    pub importance_sampling: bool,

    /// Enable temporal filtering
    pub temporal_filter: bool,
}

impl Default for AreaLightSamplingConfig {
    fn default() -> Self {
        Self {
            soft_shadow_samples: 16,
            importance_sampling: true,
            temporal_filter: true,
        }
    }
}

/// Common area light setups
pub struct AreaLightPresets;

impl AreaLightPresets {
    /// Softbox light (photography)
    pub fn softbox() -> AreaLight {
        AreaLight::rectangle(2.0, 1.5, 50.0)
            .with_color(Color::srgb(1.0, 1.0, 0.95))
            .with_range(15.0)
    }

    /// Panel light (ceiling)
    pub fn ceiling_panel() -> AreaLight {
        AreaLight::rectangle(1.2, 0.6, 30.0)
            .with_color(Color::srgb(1.0, 1.0, 0.98))
            .with_range(10.0)
            .two_sided()
    }

    /// Neon tube
    pub fn neon_tube(color: Color) -> AreaLight {
        AreaLight::tube(2.0, 0.05, 15.0)
            .with_color(color)
            .with_range(8.0)
    }

    /// Light bulb
    pub fn bulb() -> AreaLight {
        AreaLight::sphere(0.1, 25.0)
            .with_color(Color::srgb(1.0, 0.95, 0.8))
            .with_range(12.0)
    }

    /// Window light (sunlit)
    pub fn window() -> AreaLight {
        AreaLight::rectangle(2.0, 3.0, 100.0)
            .with_color(Color::srgb(0.95, 0.95, 1.0))
            .with_range(20.0)
    }

    /// Monitor/screen light
    pub fn monitor() -> AreaLight {
        AreaLight::rectangle(0.6, 0.4, 5.0)
            .with_color(Color::srgb(0.8, 0.85, 1.0))
            .with_range(3.0)
    }

    /// Portal/sci-fi light
    pub fn portal(color: Color) -> AreaLight {
        AreaLight::disk(1.0, 80.0)
            .with_color(color)
            .with_range(15.0)
            .two_sided()
    }
}

/// Area light plugin
pub struct AreaLightsPlugin;

impl Plugin for AreaLightsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AreaLightSamplingConfig::default());
        // Note: Actual rendering would be implemented in the render graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area_light_creation() {
        let rect = AreaLight::rectangle(2.0, 1.0, 10.0);
        assert!(matches!(rect.shape, AreaLightShape::Rectangle { .. }));
        assert_eq!(rect.intensity, 10.0);

        let disk = AreaLight::disk(0.5, 5.0);
        assert!(matches!(disk.shape, AreaLightShape::Disk { .. }));

        let sphere = AreaLight::sphere(0.3, 20.0);
        assert!(matches!(sphere.shape, AreaLightShape::Sphere { .. }));

        let tube = AreaLight::tube(1.5, 0.1, 15.0);
        assert!(matches!(tube.shape, AreaLightShape::Tube { .. }));
    }

    #[test]
    fn test_surface_area_calculation() {
        let rect = AreaLight::rectangle(2.0, 3.0, 10.0);
        assert_eq!(rect.surface_area(), 6.0);

        let disk = AreaLight::disk(1.0, 10.0);
        assert!((disk.surface_area() - std::f32::consts::PI).abs() < 0.1);

        let sphere = AreaLight::sphere(1.0, 10.0);
        let expected = 4.0 * std::f32::consts::PI;
        assert!((sphere.surface_area() - expected).abs() < 0.1);
    }

    #[test]
    fn test_power_calculation() {
        let light = AreaLight::rectangle(2.0, 1.0, 10.0);
        let power = light.power();
        assert_eq!(power, 20.0); // 2*1*10
    }

    #[test]
    fn test_light_with_modifiers() {
        let light = AreaLight::rectangle(1.0, 1.0, 10.0)
            .with_color(Color::srgb(1.0, 0.0, 0.0))
            .with_range(15.0)
            .two_sided();

        assert_eq!(light.color, Color::srgb(1.0, 0.0, 0.0));
        assert_eq!(light.range, 15.0);
        assert!(light.two_sided);
    }

    #[test]
    fn test_point_sampling() {
        let light = AreaLight::rectangle(2.0, 1.0, 10.0);
        let transform = Transform::from_xyz(0.0, 5.0, 0.0);

        let point = light.sample_point(0.5, 0.5, &transform);
        // Center of rectangle should be at light position
        assert!((point - transform.translation).length() < 0.01);
    }

    #[test]
    fn test_presets() {
        let softbox = AreaLightPresets::softbox();
        assert!(matches!(softbox.shape, AreaLightShape::Rectangle { .. }));

        let bulb = AreaLightPresets::bulb();
        assert!(matches!(bulb.shape, AreaLightShape::Sphere { .. }));

        let tube = AreaLightPresets::neon_tube(Color::srgb(0.0, 1.0, 1.0));
        assert!(matches!(tube.shape, AreaLightShape::Tube { .. }));
    }

    #[test]
    fn test_illumination_calculation() {
        let light = AreaLight::rectangle(1.0, 1.0, 100.0).with_range(10.0);
        let light_transform = Transform::from_xyz(0.0, 5.0, 0.0);

        // Point directly below the light
        let point = Vec3::new(0.0, 0.0, 0.0);
        let normal = Vec3::Y;

        let illumination = light.illumination_at(point, normal, &light_transform);
        assert!(illumination != Color::BLACK); // Should be illuminated

        // Point outside range
        let far_point = Vec3::new(0.0, -20.0, 0.0);
        let far_illumination = light.illumination_at(far_point, normal, &light_transform);
        assert_eq!(far_illumination, Color::BLACK); // Outside range
    }

    #[test]
    fn test_sampling_config() {
        let config = AreaLightSamplingConfig::default();
        assert!(config.soft_shadow_samples > 0);
        assert!(config.importance_sampling);
    }
}
