use bevy::prelude::*;

/// Environment map type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentMapType {
    /// Cube map (6 faces)
    CubeMap,
    /// Equirectangular panorama
    Equirectangular,
    /// Procedurally generated sky
    Procedural,
}

/// Skybox/environment configuration
#[derive(Resource, Clone, Debug)]
pub struct EnvironmentConfig {
    pub enabled: bool,
    pub map_type: EnvironmentMapType,
    pub texture: Option<Handle<Image>>,
    pub brightness: f32,
    pub rotation: Quat,
    pub blur_radius: f32, // For reflections
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            map_type: EnvironmentMapType::Procedural,
            texture: None,
            brightness: 1.0,
            rotation: Quat::IDENTITY,
            blur_radius: 0.0,
        }
    }
}

impl EnvironmentConfig {
    pub fn new(map_type: EnvironmentMapType) -> Self {
        Self {
            enabled: true,
            map_type,
            ..Default::default()
        }
    }

    pub fn with_texture(mut self, texture: Handle<Image>) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_brightness(mut self, brightness: f32) -> Self {
        self.brightness = brightness;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }
}

/// Procedural sky configuration
#[derive(Resource, Clone, Debug)]
pub struct ProceduralSkyConfig {
    pub enabled: bool,
    pub sun_position: Vec3,
    pub sun_color: Color,
    pub sun_intensity: f32,
    pub sky_color_zenith: Color,
    pub sky_color_horizon: Color,
    pub ground_color: Color,
    pub atmosphere_density: f32,
    pub rayleigh_scattering: f32,
    pub mie_scattering: f32,
    pub time_of_day: f32, // 0.0 = midnight, 0.5 = noon, 1.0 = midnight
}

impl Default for ProceduralSkyConfig {
    fn default() -> Self {
        Self::noon()
    }
}

impl ProceduralSkyConfig {
    /// Noon sky
    pub fn noon() -> Self {
        Self {
            enabled: true,
            sun_position: Vec3::new(0.0, 1.0, -0.3).normalize(),
            sun_color: Color::srgb(1.0, 1.0, 0.95),
            sun_intensity: 1.0,
            sky_color_zenith: Color::srgb(0.2, 0.5, 0.9),
            sky_color_horizon: Color::srgb(0.6, 0.7, 0.9),
            ground_color: Color::srgb(0.3, 0.4, 0.3),
            atmosphere_density: 1.0,
            rayleigh_scattering: 1.0,
            mie_scattering: 0.1,
            time_of_day: 0.5,
        }
    }

    /// Sunset sky
    pub fn sunset() -> Self {
        Self {
            enabled: true,
            sun_position: Vec3::new(0.8, 0.2, -0.3).normalize(),
            sun_color: Color::srgb(1.0, 0.7, 0.4),
            sun_intensity: 0.8,
            sky_color_zenith: Color::srgb(0.4, 0.3, 0.6),
            sky_color_horizon: Color::srgb(1.0, 0.5, 0.3),
            ground_color: Color::srgb(0.2, 0.2, 0.3),
            atmosphere_density: 1.5,
            rayleigh_scattering: 1.2,
            mie_scattering: 0.3,
            time_of_day: 0.75,
        }
    }

    /// Night sky
    pub fn night() -> Self {
        Self {
            enabled: true,
            sun_position: Vec3::new(0.0, -0.5, 0.0).normalize(),
            sun_color: Color::srgb(0.1, 0.1, 0.2),
            sun_intensity: 0.05,
            sky_color_zenith: Color::srgb(0.01, 0.01, 0.05),
            sky_color_horizon: Color::srgb(0.05, 0.05, 0.1),
            ground_color: Color::srgb(0.02, 0.02, 0.02),
            atmosphere_density: 0.5,
            rayleigh_scattering: 0.5,
            mie_scattering: 0.05,
            time_of_day: 0.0,
        }
    }

    /// Overcast/cloudy sky
    pub fn overcast() -> Self {
        Self {
            enabled: true,
            sun_position: Vec3::new(0.0, 0.7, -0.3).normalize(),
            sun_color: Color::srgb(0.9, 0.9, 0.9),
            sun_intensity: 0.3,
            sky_color_zenith: Color::srgb(0.6, 0.6, 0.65),
            sky_color_horizon: Color::srgb(0.7, 0.7, 0.75),
            ground_color: Color::srgb(0.3, 0.35, 0.3),
            atmosphere_density: 2.0,
            rayleigh_scattering: 0.5,
            mie_scattering: 0.5,
            time_of_day: 0.5,
        }
    }

    /// Set time of day (0.0-1.0) and update sky accordingly
    pub fn set_time_of_day(&mut self, time: f32) {
        self.time_of_day = time.clamp(0.0, 1.0);

        // Update sun position based on time
        let angle = time * std::f32::consts::TAU;
        self.sun_position = Vec3::new(angle.cos() * 0.3, angle.sin(), -0.3).normalize();

        // Update colors based on time
        if !(0.25..=0.75).contains(&time) {
            // Night
            self.sky_color_zenith = Color::srgb(0.01, 0.01, 0.05);
            self.sun_intensity = 0.05;
        } else if (0.25..0.3).contains(&time) || (0.7..0.75).contains(&time) {
            // Sunrise/sunset
            self.sky_color_zenith = Color::srgb(0.4, 0.3, 0.6);
            self.sky_color_horizon = Color::srgb(1.0, 0.5, 0.3);
            self.sun_intensity = 0.6;
        } else {
            // Day
            self.sky_color_zenith = Color::srgb(0.2, 0.5, 0.9);
            self.sky_color_horizon = Color::srgb(0.6, 0.7, 0.9);
            self.sun_intensity = 1.0;
        }
    }
}

/// Stars configuration for night sky
#[derive(Resource, Clone, Debug)]
pub struct StarsConfig {
    pub enabled: bool,
    pub count: u32,
    pub brightness: f32,
    pub size: f32,
    pub twinkle: bool,
}

impl Default for StarsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            count: 5000,
            brightness: 1.0,
            size: 1.0,
            twinkle: true,
        }
    }
}

/// Clouds configuration
#[derive(Resource, Clone, Debug)]
pub struct CloudsConfig {
    pub enabled: bool,
    pub density: f32,
    pub coverage: f32,
    pub speed: Vec2,
    pub altitude: f32,
    pub thickness: f32,
}

impl Default for CloudsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            density: 0.5,
            coverage: 0.5,
            speed: Vec2::new(0.01, 0.005),
            altitude: 500.0,
            thickness: 100.0,
        }
    }
}

/// Environment plugin
pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnvironmentConfig::default())
            .insert_resource(ProceduralSkyConfig::default())
            .insert_resource(StarsConfig::default())
            .insert_resource(CloudsConfig::default());
    }
}

/// Helper for loading environment maps
pub struct EnvironmentLoader;

impl EnvironmentLoader {
    /// Load cube map from 6 images
    pub fn load_cubemap(
        asset_server: &AssetServer,
        path_prefix: &str,
        extension: &str,
    ) -> Vec<Handle<Image>> {
        let faces = ["px", "nx", "py", "ny", "pz", "nz"];
        faces
            .iter()
            .map(|face| {
                let path = format!("{}_{}.{}", path_prefix, face, extension);
                asset_server.load(path)
            })
            .collect()
    }

    /// Load equirectangular panorama
    pub fn load_equirectangular(asset_server: &AssetServer, path: &str) -> Handle<Image> {
        asset_server.load(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_config() {
        let env = EnvironmentConfig::new(EnvironmentMapType::CubeMap)
            .with_brightness(1.5)
            .with_rotation(Quat::from_rotation_y(1.0));

        assert!(env.enabled);
        assert_eq!(env.map_type, EnvironmentMapType::CubeMap);
        assert_eq!(env.brightness, 1.5);
    }

    #[test]
    fn test_procedural_sky_presets() {
        let noon = ProceduralSkyConfig::noon();
        assert!(noon.enabled);
        assert_eq!(noon.time_of_day, 0.5);

        let sunset = ProceduralSkyConfig::sunset();
        assert_eq!(sunset.time_of_day, 0.75);

        let night = ProceduralSkyConfig::night();
        assert_eq!(night.time_of_day, 0.0);
    }

    #[test]
    fn test_time_of_day_update() {
        let mut sky = ProceduralSkyConfig::noon();
        sky.set_time_of_day(0.1); // Early morning/night

        assert_eq!(sky.time_of_day, 0.1);
        assert!(sky.sun_intensity < 0.5); // Should be dim
    }

    #[test]
    fn test_stars_config() {
        let stars = StarsConfig::default();
        assert!(!stars.enabled);
        assert_eq!(stars.count, 5000);
        assert!(stars.twinkle);
    }

    #[test]
    fn test_clouds_config() {
        let clouds = CloudsConfig::default();
        assert!(!clouds.enabled);
        assert_eq!(clouds.density, 0.5);
        assert_eq!(clouds.coverage, 0.5);
    }
}
