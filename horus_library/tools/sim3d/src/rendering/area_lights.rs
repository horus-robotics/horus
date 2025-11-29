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

/// Marker component to track that a Bevy light was created for an area light
#[derive(Component)]
struct AreaLightRendered;

/// Area light plugin
pub struct AreaLightsPlugin;

impl Plugin for AreaLightsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AreaLightSamplingConfig::default())
            .add_systems(
                Update,
                (
                    area_light_spawn_system,
                    area_light_update_system,
                    area_light_gizmo_system,
                ),
            );
    }
}

/// System to spawn Bevy light components for AreaLight entities
/// Since Bevy doesn't have native area lights, we approximate them:
/// - Rectangle/Disk → Multiple point lights arranged in a pattern
/// - Sphere → Single point light with adjusted intensity
/// - Tube → Multiple point lights along the tube axis
fn area_light_spawn_system(
    mut commands: Commands,
    query: Query<(Entity, &AreaLight, &Transform), Without<AreaLightRendered>>,
) {
    for (entity, area_light, transform) in query.iter() {
        // Calculate effective intensity based on shape and area
        let base_intensity = area_light.intensity * area_light.surface_area();

        // Convert area light color to linear RGB for Bevy
        let srgba = area_light.color.to_srgba();
        let color = Color::srgb(srgba.red, srgba.green, srgba.blue);

        match area_light.shape {
            AreaLightShape::Rectangle { width, height } => {
                // Use a grid of point lights to simulate rectangle
                // Number of lights depends on area (max 9 for performance)
                let lights_x = (width / 0.5).ceil().min(3.0) as i32;
                let lights_y = (height / 0.5).ceil().min(3.0) as i32;
                let intensity_per_light = base_intensity / (lights_x * lights_y) as f32;

                commands.entity(entity).with_children(|parent| {
                    for ix in 0..lights_x {
                        for iy in 0..lights_y {
                            let offset_x = if lights_x > 1 {
                                (ix as f32 / (lights_x - 1) as f32 - 0.5) * width
                            } else {
                                0.0
                            };
                            let offset_y = if lights_y > 1 {
                                (iy as f32 / (lights_y - 1) as f32 - 0.5) * height
                            } else {
                                0.0
                            };

                            parent.spawn((
                                PointLight {
                                    color,
                                    intensity: intensity_per_light * 1000.0, // Bevy uses lumens
                                    range: area_light.range,
                                    shadows_enabled: area_light.cast_shadows
                                        && ix == lights_x / 2
                                        && iy == lights_y / 2,
                                    ..default()
                                },
                                Transform::from_xyz(offset_x, offset_y, 0.0),
                            ));
                        }
                    }
                });
            }
            AreaLightShape::Disk { radius } => {
                // Use center light + ring of lights for disk
                let ring_count = (radius / 0.3).ceil().min(4.0) as i32;
                let total_lights = 1 + ring_count * 4; // center + ring
                let intensity_per_light = base_intensity / total_lights as f32;

                commands.entity(entity).with_children(|parent| {
                    // Center light
                    parent.spawn((
                        PointLight {
                            color,
                            intensity: intensity_per_light * 1000.0,
                            range: area_light.range,
                            shadows_enabled: area_light.cast_shadows,
                            ..default()
                        },
                        Transform::IDENTITY,
                    ));

                    // Ring lights
                    for i in 0..(ring_count * 4) {
                        let angle = (i as f32 / (ring_count * 4) as f32) * std::f32::consts::TAU;
                        let r = radius * 0.7;
                        parent.spawn((
                            PointLight {
                                color,
                                intensity: intensity_per_light * 1000.0,
                                range: area_light.range,
                                shadows_enabled: false,
                                ..default()
                            },
                            Transform::from_xyz(angle.cos() * r, angle.sin() * r, 0.0),
                        ));
                    }
                });
            }
            AreaLightShape::Sphere { radius } => {
                // Single point light at center with intensity based on surface area
                // Sphere emits equally in all directions, natural for point light
                commands.entity(entity).with_children(|parent| {
                    parent.spawn((
                        PointLight {
                            color,
                            intensity: base_intensity * 1000.0,
                            range: area_light.range,
                            radius, // Bevy PointLight has a radius parameter
                            shadows_enabled: area_light.cast_shadows,
                            ..default()
                        },
                        Transform::IDENTITY,
                    ));
                });
            }
            AreaLightShape::Tube { length, radius: _ } => {
                // Use a line of point lights along the tube
                let light_count = (length / 0.3).ceil().min(8.0) as i32;
                let intensity_per_light = base_intensity / light_count as f32;

                commands.entity(entity).with_children(|parent| {
                    for i in 0..light_count {
                        let t = if light_count > 1 {
                            i as f32 / (light_count - 1) as f32
                        } else {
                            0.5
                        };
                        let z = (t - 0.5) * length;

                        parent.spawn((
                            PointLight {
                                color,
                                intensity: intensity_per_light * 1000.0,
                                range: area_light.range,
                                shadows_enabled: area_light.cast_shadows && i == light_count / 2,
                                ..default()
                            },
                            Transform::from_xyz(0.0, 0.0, z),
                        ));
                    }
                });
            }
        }

        // Mark as rendered
        commands.entity(entity).insert(AreaLightRendered);

        tracing::debug!(
            "Spawned area light approximation for {:?} shape at {:?}",
            area_light.shape,
            transform.translation
        );
    }
}

/// System to update Bevy lights when AreaLight properties change
fn area_light_update_system(
    query: Query<(&AreaLight, &Children), (With<AreaLightRendered>, Changed<AreaLight>)>,
    mut light_query: Query<&mut PointLight>,
) {
    for (area_light, children) in query.iter() {
        let base_intensity = area_light.intensity * area_light.surface_area();
        let srgba = area_light.color.to_srgba();
        let color = Color::srgb(srgba.red, srgba.green, srgba.blue);

        let child_count = children.len().max(1) as f32;
        let intensity_per_light = (base_intensity / child_count) * 1000.0;

        for &child in children.iter() {
            if let Ok(mut point_light) = light_query.get_mut(child) {
                point_light.color = color;
                point_light.intensity = intensity_per_light;
                point_light.range = area_light.range;
            }
        }
    }
}

/// System to draw gizmos for area lights (debug visualization)
fn area_light_gizmo_system(
    mut gizmos: Gizmos,
    query: Query<(&AreaLight, &Transform, &GlobalTransform)>,
    config: Res<AreaLightSamplingConfig>,
) {
    // Only draw if we have temporal filtering enabled (used as debug flag)
    if !config.temporal_filter {
        return;
    }

    for (area_light, _local_transform, global_transform) in query.iter() {
        let transform = global_transform.compute_transform();
        let srgba = area_light.color.to_srgba();
        let gizmo_color = Color::srgba(srgba.red, srgba.green, srgba.blue, 0.5);

        match area_light.shape {
            AreaLightShape::Rectangle { width, height } => {
                // Draw rectangle outline
                let half_w = width / 2.0;
                let half_h = height / 2.0;
                let corners = [
                    transform.transform_point(Vec3::new(-half_w, -half_h, 0.0)),
                    transform.transform_point(Vec3::new(half_w, -half_h, 0.0)),
                    transform.transform_point(Vec3::new(half_w, half_h, 0.0)),
                    transform.transform_point(Vec3::new(-half_w, half_h, 0.0)),
                ];

                gizmos.line(corners[0], corners[1], gizmo_color);
                gizmos.line(corners[1], corners[2], gizmo_color);
                gizmos.line(corners[2], corners[3], gizmo_color);
                gizmos.line(corners[3], corners[0], gizmo_color);

                // Draw light direction indicator
                let center = transform.translation;
                let forward = transform.forward();
                gizmos.line(center, center + forward.as_vec3() * 0.5, gizmo_color);
            }
            AreaLightShape::Disk { radius } => {
                // Draw circle outline
                let segments = 16;
                for i in 0..segments {
                    let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
                    let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
                    let p1 = transform.transform_point(Vec3::new(
                        angle1.cos() * radius,
                        angle1.sin() * radius,
                        0.0,
                    ));
                    let p2 = transform.transform_point(Vec3::new(
                        angle2.cos() * radius,
                        angle2.sin() * radius,
                        0.0,
                    ));
                    gizmos.line(p1, p2, gizmo_color);
                }
            }
            AreaLightShape::Sphere { radius } => {
                // Draw sphere gizmo
                gizmos.sphere(transform.translation, radius, gizmo_color);
            }
            AreaLightShape::Tube { length, radius } => {
                // Draw tube outline (cylinder)
                let half_len = length / 2.0;
                let segments = 8;

                // Draw end caps
                for end in [-half_len, half_len] {
                    for i in 0..segments {
                        let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
                        let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
                        let p1 = transform.transform_point(Vec3::new(
                            angle1.cos() * radius,
                            angle1.sin() * radius,
                            end,
                        ));
                        let p2 = transform.transform_point(Vec3::new(
                            angle2.cos() * radius,
                            angle2.sin() * radius,
                            end,
                        ));
                        gizmos.line(p1, p2, gizmo_color);
                    }
                }

                // Draw connecting lines
                for i in 0..4 {
                    let angle = (i as f32 / 4.0) * std::f32::consts::TAU;
                    let p1 = transform.transform_point(Vec3::new(
                        angle.cos() * radius,
                        angle.sin() * radius,
                        -half_len,
                    ));
                    let p2 = transform.transform_point(Vec3::new(
                        angle.cos() * radius,
                        angle.sin() * radius,
                        half_len,
                    ));
                    gizmos.line(p1, p2, gizmo_color);
                }
            }
        }
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
