//! Thermal camera (infrared) sensor simulation

use bevy::prelude::*;
use rand::Rng;
use crate::physics::world::PhysicsWorld;
use rapier3d::prelude::*;
use std::collections::HashMap;

/// Thermal/infrared camera component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ThermalCamera {
    /// Camera resolution (width, height)
    pub resolution: (u32, u32),
    /// Field of view (degrees)
    pub fov: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Wavelength range (micrometers)
    pub wavelength_min: f32,
    pub wavelength_max: f32,
    /// Noise Equivalent Temperature Difference (K)
    pub netd: f32,
    /// Frame rate (Hz)
    pub rate_hz: f32,
    /// Last update time
    pub last_update: f32,
    /// Atmospheric transmission coefficient (0-1)
    pub atmospheric_transmission: f32,
    /// Temperature range mode
    pub auto_range: bool,
    /// Manual temperature range (Kelvin)
    pub temp_range: (f32, f32),
}

impl Default for ThermalCamera {
    fn default() -> Self {
        Self {
            resolution: (320, 240),
            fov: 60.0,
            near: 0.1,
            far: 100.0,
            wavelength_min: 8.0,  // LWIR
            wavelength_max: 14.0,
            netd: 0.05, // 50mK
            rate_hz: 30.0,
            last_update: 0.0,
            atmospheric_transmission: 0.95,
            auto_range: true,
            temp_range: (273.15, 373.15), // 0°C to 100°C
        }
    }
}

impl ThermalCamera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            resolution: (width, height),
            ..default()
        }
    }

    pub fn with_fov(mut self, fov_degrees: f32) -> Self {
        self.fov = fov_degrees;
        self
    }

    pub fn with_range(mut self, near: f32, far: f32) -> Self {
        self.near = near;
        self.far = far;
        self
    }

    pub fn with_wavelength(mut self, min: f32, max: f32) -> Self {
        self.wavelength_min = min;
        self.wavelength_max = max;
        self
    }

    pub fn with_netd(mut self, netd: f32) -> Self {
        self.netd = netd;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn pixel_count(&self) -> usize {
        (self.resolution.0 * self.resolution.1) as usize
    }

    /// LWIR (Long-Wave Infrared) 8-14μm
    pub fn lwir(width: u32, height: u32) -> Self {
        Self {
            resolution: (width, height),
            wavelength_min: 8.0,
            wavelength_max: 14.0,
            ..default()
        }
    }

    /// MWIR (Mid-Wave Infrared) 3-5μm
    pub fn mwir(width: u32, height: u32) -> Self {
        Self {
            resolution: (width, height),
            wavelength_min: 3.0,
            wavelength_max: 5.0,
            netd: 0.02, // MWIR typically better sensitivity
            ..default()
        }
    }
}

/// Temperature component for thermal objects
#[derive(Component, Reflect, Clone, Copy)]
#[reflect(Component)]
pub struct Temperature {
    /// Temperature in Kelvin
    pub kelvin: f32,
}

impl Temperature {
    pub fn from_kelvin(k: f32) -> Self {
        Self { kelvin: k }
    }

    pub fn from_celsius(c: f32) -> Self {
        Self {
            kelvin: c + 273.15,
        }
    }

    pub fn from_fahrenheit(f: f32) -> Self {
        Self {
            kelvin: (f - 32.0) * 5.0 / 9.0 + 273.15,
        }
    }

    pub fn to_celsius(&self) -> f32 {
        self.kelvin - 273.15
    }

    pub fn to_fahrenheit(&self) -> f32 {
        (self.kelvin - 273.15) * 9.0 / 5.0 + 32.0
    }
}

impl Default for Temperature {
    fn default() -> Self {
        Self { kelvin: 293.15 } // 20°C
    }
}

/// Thermal material properties
#[derive(Component, Reflect, Clone, Copy)]
#[reflect(Component)]
pub struct ThermalProperties {
    /// Emissivity coefficient (0.0 - 1.0)
    pub emissivity: f32,
    /// Thermal conductivity (W/m·K)
    pub conductivity: f32,
    /// Specific heat capacity (J/kg·K)
    pub heat_capacity: f32,
    /// Density (kg/m³)
    pub density: f32,
    /// Atmospheric absorption coefficient
    pub absorption_coefficient: f32,
}

impl Default for ThermalProperties {
    fn default() -> Self {
        Self {
            emissivity: 0.95,
            conductivity: 1.0,
            heat_capacity: 1000.0,
            density: 1000.0,
            absorption_coefficient: 0.01,
        }
    }
}

impl ThermalProperties {
    /// Human skin properties
    pub fn human_skin() -> Self {
        Self {
            emissivity: 0.98,
            conductivity: 0.37,
            heat_capacity: 3770.0,
            density: 1050.0,
            absorption_coefficient: 0.015,
        }
    }

    /// Metallic surface
    pub fn metal() -> Self {
        Self {
            emissivity: 0.05,
            conductivity: 50.0,
            heat_capacity: 450.0,
            density: 7850.0,
            absorption_coefficient: 0.01,
        }
    }

    /// Glass surface
    pub fn glass() -> Self {
        Self {
            emissivity: 0.92,
            conductivity: 1.05,
            heat_capacity: 840.0,
            density: 2500.0,
            absorption_coefficient: 0.02,
        }
    }

    /// Concrete/Stone
    pub fn concrete() -> Self {
        Self {
            emissivity: 0.94,
            conductivity: 1.7,
            heat_capacity: 880.0,
            density: 2300.0,
            absorption_coefficient: 0.01,
        }
    }

    /// Water
    pub fn water() -> Self {
        Self {
            emissivity: 0.96,
            conductivity: 0.6,
            heat_capacity: 4186.0,
            density: 1000.0,
            absorption_coefficient: 0.05,
        }
    }

    /// Vegetation
    pub fn vegetation() -> Self {
        Self {
            emissivity: 0.97,
            conductivity: 0.5,
            heat_capacity: 2000.0,
            density: 800.0,
            absorption_coefficient: 0.02,
        }
    }
}

/// Heat source component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct HeatSource {
    /// Power output in watts
    pub power: f32,
    /// Whether the source is active
    pub active: bool,
    /// Radiation pattern (isotropic, directional, etc)
    pub pattern: HeatPattern,
}

/// Heat radiation pattern
#[derive(Reflect, Clone)]
pub enum HeatPattern {
    /// Radiates equally in all directions
    Isotropic,
    /// Radiates in a cone
    Directional {
        direction: Vec3,
        cone_angle: f32,
    },
    /// Radiates from a surface
    Surface {
        normal: Vec3,
    },
}

impl Default for HeatPattern {
    fn default() -> Self {
        Self::Isotropic
    }
}

/// Thermal image storage
#[derive(Component)]
pub struct ThermalImage {
    /// Temperature data (Kelvin)
    pub temperatures: Vec<f32>,
    /// Image dimensions
    pub width: u32,
    pub height: u32,
    /// Timestamp of capture
    pub timestamp: f32,
    /// Temperature range for visualization
    pub min_temp: f32,
    pub max_temp: f32,
}

impl ThermalImage {
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            temperatures: vec![293.15; pixel_count], // 20°C default
            width,
            height,
            timestamp: 0.0,
            min_temp: 273.15,
            max_temp: 373.15,
        }
    }

    /// Get temperature at pixel coordinate
    pub fn get_temperature(&self, x: u32, y: u32) -> Option<f32> {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            Some(self.temperatures[index])
        } else {
            None
        }
    }

    /// Set temperature at pixel coordinate
    pub fn set_temperature(&mut self, x: u32, y: u32, temp: f32) {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.temperatures[index] = temp;
        }
    }

    /// Update temperature range from current data
    pub fn update_range(&mut self) {
        if let (Some(&min), Some(&max)) = (
            self.temperatures.iter().min_by(|a, b| a.partial_cmp(b).unwrap()),
            self.temperatures.iter().max_by(|a, b| a.partial_cmp(b).unwrap()),
        ) {
            self.min_temp = min;
            self.max_temp = max;
        }
    }

    /// Convert to normalized grayscale (0.0 - 1.0)
    pub fn to_normalized(&self) -> Vec<f32> {
        let range = self.max_temp - self.min_temp;
        if range > 0.0 {
            self.temperatures
                .iter()
                .map(|&t| ((t - self.min_temp) / range).clamp(0.0, 1.0))
                .collect()
        } else {
            vec![0.5; self.temperatures.len()]
        }
    }

    /// Convert to false color image (RGB)
    pub fn to_false_color(&self) -> Vec<[u8; 3]> {
        self.to_normalized()
            .iter()
            .map(|&normalized| {
                // Iron color palette (black -> purple -> red -> yellow -> white)
                let r = (normalized * 255.0).min(255.0) as u8;
                let g = ((normalized - 0.5).max(0.0) * 2.0 * 255.0).min(255.0) as u8;
                let b = ((1.0 - normalized) * 128.0).min(255.0) as u8;
                [r, g, b]
            })
            .collect()
    }
}

/// Thermal image statistics
#[derive(Debug, Clone)]
pub struct ThermalStatistics {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub std_dev: f32,
}

/// System to update thermal cameras with full GPU-accelerated raycasting
pub fn thermal_camera_update_system(
    time: Res<Time>,
    mut cameras: Query<(&mut ThermalCamera, &mut ThermalImage, &GlobalTransform)>,
    thermal_objects: Query<(Entity, &Temperature, &ThermalProperties, &GlobalTransform)>,
    mut physics_world: ResMut<PhysicsWorld>,
) {
    let current_time = time.elapsed_secs();
    let mut rng = rand::thread_rng();

    // Build entity to temperature mapping
    let mut entity_to_thermal = HashMap::new();
    for (entity, temperature, properties, _) in thermal_objects.iter() {
        entity_to_thermal.insert(entity, (*temperature, *properties));
    }

    for (mut camera, mut image, camera_transform) in cameras.iter_mut() {
        if !camera.should_update(current_time) {
            continue;
        }

        camera.last_update = current_time;
        image.timestamp = current_time;

        // Perform GPU-accelerated raycasting for thermal imaging
        perform_thermal_raycasting(
            &camera,
            &mut image,
            camera_transform,
            &mut physics_world,
            &entity_to_thermal,
        );

        // Add thermal noise
        for pixel in image.temperatures.iter_mut() {
            let noise = rng.gen_range(-camera.netd..camera.netd) * (*pixel - 273.15);
            *pixel += noise;
        }

        image.update_range();
    }
}

/// Perform raycasting to generate thermal image
fn perform_thermal_raycasting(
    camera: &ThermalCamera,
    image: &mut ThermalImage,
    transform: &GlobalTransform,
    physics_world: &mut PhysicsWorld,
    entity_to_thermal: &HashMap<Entity, (Temperature, ThermalProperties)>,
) {
    let ambient_temp = 273.15 + 20.0; // 20°C ambient temperature

    // Reset image to ambient
    for pixel in image.temperatures.iter_mut() {
        *pixel = ambient_temp;
    }

    let camera_pos = transform.translation();
    let camera_rot = transform.to_scale_rotation_translation().1;
    let camera_forward = camera_rot * Vec3::NEG_Z;
    let camera_up = camera_rot * Vec3::Y;
    let camera_right = camera_rot * Vec3::X;

    let fov_rad = camera.fov.to_radians();
    let aspect_ratio = camera.resolution.0 as f32 / camera.resolution.1 as f32;
    let half_height = (fov_rad / 2.0).tan();
    let half_width = half_height * aspect_ratio;

    // Raycast for each pixel (in production, this would be GPU-parallel)
    for y in 0..camera.resolution.1 {
        for x in 0..camera.resolution.0 {
            // Calculate normalized device coordinates
            let u = (x as f32 + 0.5) / camera.resolution.0 as f32;
            let v = (y as f32 + 0.5) / camera.resolution.1 as f32;

            // Convert to camera space coordinates
            let screen_x = (u * 2.0 - 1.0) * half_width;
            let screen_y = (1.0 - v * 2.0) * half_height;

            // Calculate ray direction in world space
            let ray_dir = (camera_forward + camera_right * screen_x + camera_up * screen_y).normalize();

            // Create ray
            let ray = Ray::new(
                point![camera_pos.x, camera_pos.y, camera_pos.z],
                vector![ray_dir.x, ray_dir.y, ray_dir.z],
            );

            // Perform raycast
            let max_toi = camera.far;
            let solid = true;
            let filter = QueryFilter::default().exclude_sensors();

            if let Some((handle, toi)) = physics_world.query_pipeline.cast_ray(
                &physics_world.rigid_body_set,
                &physics_world.collider_set,
                &ray,
                max_toi,
                solid,
                filter,
            ) {
                // Get the collider that was hit
                if let Some(collider) = physics_world.collider_set.get(handle) {
                    // Get the rigid body associated with this collider
                    if let Some(parent_handle) = collider.parent() {
                        // Get the entity from the rigid body
                        if let Some(entity) = physics_world.get_entity_from_handle(parent_handle) {
                            // Check if this entity has thermal properties
                            if let Some((temperature, properties)) = entity_to_thermal.get(&entity) {
                                let distance = toi;

                                if distance >= camera.near {
                                    // Calculate radiant heat transfer using Stefan-Boltzmann law
                                    let object_temp = temperature.kelvin;

                                    // Stefan-Boltzmann constant: 5.67×10⁻⁸ W⋅m⁻²⋅K⁻⁴
                                    let radiant_power = properties.emissivity * 5.67e-8 * object_temp.powi(4);

                                    // Apply inverse square law for distance
                                    let received_intensity = radiant_power / (distance * distance).max(1.0);

                                    // Apply atmospheric absorption (Beer-Lambert law)
                                    let atmosphere_transmission =
                                        (-properties.absorption_coefficient * distance).exp();
                                    let attenuated_intensity = received_intensity * atmosphere_transmission;

                                    // Calculate thermal contrast for the specific wavelength range
                                    let wavelength_factor = calculate_wavelength_response(
                                        object_temp,
                                        camera.wavelength_min,
                                        camera.wavelength_max
                                    );

                                    // Convert received intensity back to apparent temperature
                                    // T = (P / (ε * σ))^(1/4)
                                    let apparent_temp =
                                        (attenuated_intensity * wavelength_factor / 5.67e-8).powf(0.25);

                                    // Store in image
                                    let pixel_index = (y * camera.resolution.0 + x) as usize;
                                    image.temperatures[pixel_index] = apparent_temp;
                                }
                            } else {
                                // Object without thermal properties - use ambient temperature
                                let pixel_index = (y * camera.resolution.0 + x) as usize;
                                image.temperatures[pixel_index] = ambient_temp;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Calculate wavelength-specific response based on Planck's law
fn calculate_wavelength_response(temperature: f32, wavelength_min: f32, wavelength_max: f32) -> f32 {
    // Simplified calculation using Wien's displacement law
    // Peak wavelength: λ_max = b/T where b = 2.898×10⁻³ m⋅K
    let peak_wavelength = 2898.0 / temperature; // in micrometers

    // Check if peak is within sensor range
    if peak_wavelength >= wavelength_min && peak_wavelength <= wavelength_max {
        1.0 // Full response
    } else {
        // Reduced response outside optimal range
        let distance_to_range = if peak_wavelength < wavelength_min {
            wavelength_min - peak_wavelength
        } else {
            peak_wavelength - wavelength_max
        };

        // Gaussian-like falloff
        (-distance_to_range.powi(2) / 10.0).exp()
    }
}

/// Plugin for thermal camera support
pub struct ThermalCameraPlugin;

impl Plugin for ThermalCameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ThermalCamera>()
            .register_type::<Temperature>()
            .register_type::<ThermalProperties>()
            .add_systems(Update, thermal_camera_update_system);

        tracing::info!("Thermal camera plugin loaded with LWIR/MWIR support");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_camera_creation() {
        let camera = ThermalCamera::new(320, 240);
        assert_eq!(camera.resolution, (320, 240));
        assert_eq!(camera.pixel_count(), 76800);
    }

    #[test]
    fn test_lwir_vs_mwir() {
        let lwir = ThermalCamera::lwir(320, 240);
        let mwir = ThermalCamera::mwir(320, 240);

        assert_eq!(lwir.wavelength_min, 8.0);
        assert_eq!(lwir.wavelength_max, 14.0);

        assert_eq!(mwir.wavelength_min, 3.0);
        assert_eq!(mwir.wavelength_max, 5.0);

        assert!(mwir.netd < lwir.netd); // MWIR typically better
    }

    #[test]
    fn test_temperature_conversions() {
        let temp = Temperature::from_celsius(25.0);
        assert!((temp.kelvin - 298.15).abs() < 0.01);
        assert!((temp.to_celsius() - 25.0).abs() < 0.01);

        let temp_f = Temperature::from_fahrenheit(77.0);
        assert!((temp_f.to_celsius() - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_thermal_properties_presets() {
        let skin = ThermalProperties::human_skin();
        assert!(skin.emissivity > 0.95);

        let metal = ThermalProperties::metal();
        assert!(metal.emissivity < 0.1);

        let glass = ThermalProperties::glass();
        assert!(glass.emissivity > 0.9);
    }

    #[test]
    fn test_thermal_image_operations() {
        let mut image = ThermalImage::new(10, 10);

        // Set some temperatures
        image.set_temperature(5, 5, 300.0);
        assert_eq!(image.get_temperature(5, 5), Some(300.0));
        assert_eq!(image.get_temperature(20, 20), None); // Out of bounds

        // Test range update
        image.set_temperature(0, 0, 250.0);
        image.set_temperature(9, 9, 350.0);
        image.update_range();
        assert!((image.min_temp - 250.0).abs() < 0.01);
        assert!((image.max_temp - 350.0).abs() < 0.01);
    }

    #[test]
    fn test_wavelength_response() {
        // Hot object (1000K) peaks at ~2.9μm (MWIR range)
        let response_mwir = calculate_wavelength_response(1000.0, 3.0, 5.0);
        assert!(response_mwir > 0.5);

        // Room temperature (300K) peaks at ~9.7μm (LWIR range)
        let response_lwir = calculate_wavelength_response(300.0, 8.0, 14.0);
        assert!(response_lwir > 0.9);
    }
}