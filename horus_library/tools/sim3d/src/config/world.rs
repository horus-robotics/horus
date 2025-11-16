use bevy::prelude::*;

use crate::physics::MaterialPreset;

/// World configuration for simulation environment
#[derive(Resource, Clone, Debug)]
pub struct WorldConfig {
    /// World name
    pub name: String,
    /// Enable ground plane
    pub enable_ground: bool,
    /// Ground size (x, z)
    pub ground_size: (f32, f32),
    /// Ground material
    pub ground_material: MaterialPreset,
    /// Ground color
    pub ground_color: Color,
    /// Ambient light intensity
    pub ambient_light: f32,
    /// Directional light intensity
    pub directional_light: f32,
    /// Directional light direction
    pub light_direction: Vec3,
    /// Sky color (for simple sky)
    pub sky_color: Color,
    /// Enable fog
    pub enable_fog: bool,
    /// Fog color
    pub fog_color: Color,
    /// Fog density
    pub fog_density: f32,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            name: "default_world".to_string(),
            enable_ground: true,
            ground_size: (100.0, 100.0),
            ground_material: MaterialPreset::concrete(),
            ground_color: Color::srgb(0.5, 0.5, 0.5),
            ambient_light: 0.3,
            directional_light: 0.7,
            light_direction: Vec3::new(-0.5, -1.0, -0.5).normalize(),
            sky_color: Color::srgb(0.53, 0.81, 0.92),
            enable_fog: false,
            fog_color: Color::srgb(0.7, 0.7, 0.7),
            fog_density: 0.01,
        }
    }
}

impl WorldConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..default()
        }
    }

    pub fn with_ground(mut self, enable: bool) -> Self {
        self.enable_ground = enable;
        self
    }

    pub fn with_ground_size(mut self, size_x: f32, size_z: f32) -> Self {
        self.ground_size = (size_x, size_z);
        self
    }

    pub fn with_ground_material(mut self, material: MaterialPreset) -> Self {
        self.ground_material = material;
        self
    }

    pub fn with_ground_color(mut self, color: Color) -> Self {
        self.ground_color = color;
        self
    }

    pub fn with_lighting(mut self, ambient: f32, directional: f32) -> Self {
        self.ambient_light = ambient;
        self.directional_light = directional;
        self
    }

    pub fn with_light_direction(mut self, direction: Vec3) -> Self {
        self.light_direction = direction.normalize();
        self
    }

    pub fn with_sky_color(mut self, color: Color) -> Self {
        self.sky_color = color;
        self
    }

    pub fn with_fog(mut self, enable: bool, color: Color, density: f32) -> Self {
        self.enable_fog = enable;
        self.fog_color = color;
        self.fog_density = density;
        self
    }
}

/// World presets for different environments
pub struct WorldPresets;

impl WorldPresets {
    /// Indoor warehouse environment
    pub fn warehouse() -> WorldConfig {
        WorldConfig::new("warehouse")
            .with_ground_size(50.0, 50.0)
            .with_ground_material(MaterialPreset::concrete())
            .with_ground_color(Color::srgb(0.6, 0.6, 0.6))
            .with_lighting(0.5, 0.5) // More ambient light indoors
            .with_sky_color(Color::srgb(0.9, 0.9, 0.9))
    }

    /// Outdoor environment with grass
    pub fn outdoor() -> WorldConfig {
        WorldConfig::new("outdoor")
            .with_ground_size(200.0, 200.0)
            .with_ground_material(MaterialPreset::wood()) // Grass-like friction
            .with_ground_color(Color::srgb(0.3, 0.6, 0.3)) // Green grass
            .with_lighting(0.3, 0.7) // Stronger directional light
            .with_light_direction(Vec3::new(-0.3, -1.0, -0.4).normalize())
            .with_sky_color(Color::srgb(0.53, 0.81, 0.92))
    }

    /// Laboratory environment
    pub fn laboratory() -> WorldConfig {
        WorldConfig::new("laboratory")
            .with_ground_size(20.0, 20.0)
            .with_ground_material(MaterialPreset::plastic())
            .with_ground_color(Color::srgb(0.95, 0.95, 0.95)) // Clean white floor
            .with_lighting(0.6, 0.4) // Bright ambient lighting
            .with_sky_color(Color::srgb(1.0, 1.0, 1.0))
    }

    /// Factory floor
    pub fn factory() -> WorldConfig {
        WorldConfig::new("factory")
            .with_ground_size(100.0, 100.0)
            .with_ground_material(MaterialPreset::steel())
            .with_ground_color(Color::srgb(0.4, 0.4, 0.45)) // Metallic gray
            .with_lighting(0.4, 0.6)
            .with_sky_color(Color::srgb(0.8, 0.8, 0.8))
    }

    /// Desert environment
    pub fn desert() -> WorldConfig {
        WorldConfig::new("desert")
            .with_ground_size(500.0, 500.0)
            .with_ground_material(MaterialPreset::concrete()) // Sandy
            .with_ground_color(Color::srgb(0.93, 0.79, 0.69)) // Sand color
            .with_lighting(0.2, 0.8) // Strong sun
            .with_light_direction(Vec3::new(-0.2, -1.0, -0.3).normalize())
            .with_sky_color(Color::srgb(0.53, 0.8, 0.98))
            .with_fog(true, Color::srgb(0.93, 0.89, 0.85), 0.005) // Heat haze
    }

    /// Space environment (no ground, no atmosphere)
    pub fn space() -> WorldConfig {
        WorldConfig::new("space")
            .with_ground(false)
            .with_lighting(0.0, 1.0) // Only directional light (sun)
            .with_light_direction(Vec3::new(-1.0, -0.2, 0.0).normalize())
            .with_sky_color(Color::srgb(0.0, 0.0, 0.0)) // Black
    }

    /// Underwater environment
    pub fn underwater() -> WorldConfig {
        WorldConfig::new("underwater")
            .with_ground_size(100.0, 100.0)
            .with_ground_material(MaterialPreset::concrete())
            .with_ground_color(Color::srgb(0.4, 0.5, 0.45)) // Ocean floor
            .with_lighting(0.4, 0.3) // Dimmer
            .with_sky_color(Color::srgb(0.1, 0.3, 0.5)) // Deep blue
            .with_fog(true, Color::srgb(0.1, 0.3, 0.5), 0.05) // Water turbidity
    }

    /// Ice/Arctic environment
    pub fn arctic() -> WorldConfig {
        WorldConfig::new("arctic")
            .with_ground_size(200.0, 200.0)
            .with_ground_material(MaterialPreset::ice())
            .with_ground_color(Color::srgb(0.9, 0.95, 1.0)) // Ice white-blue
            .with_lighting(0.5, 0.5) // Overcast
            .with_light_direction(Vec3::new(0.0, -1.0, -0.1).normalize())
            .with_sky_color(Color::srgb(0.85, 0.9, 0.95))
            .with_fog(true, Color::srgb(0.95, 0.95, 1.0), 0.01) // Snow
    }

    /// Minimal environment (empty void)
    pub fn minimal() -> WorldConfig {
        WorldConfig::new("minimal")
            .with_ground(false)
            .with_lighting(0.5, 0.5)
            .with_sky_color(Color::srgb(0.2, 0.2, 0.2))
    }
}

/// Obstacle configurations for procedural world generation
#[derive(Clone, Debug)]
pub struct ObstacleConfig {
    pub num_obstacles: usize,
    pub min_size: Vec3,
    pub max_size: Vec3,
    pub spawn_area: (f32, f32), // (width, depth)
    pub material: MaterialPreset,
}

impl Default for ObstacleConfig {
    fn default() -> Self {
        Self {
            num_obstacles: 10,
            min_size: Vec3::new(0.2, 0.5, 0.2),
            max_size: Vec3::new(1.0, 2.0, 1.0),
            spawn_area: (20.0, 20.0),
            material: MaterialPreset::wood(),
        }
    }
}

impl ObstacleConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_count(mut self, count: usize) -> Self {
        self.num_obstacles = count;
        self
    }

    pub fn with_size_range(mut self, min: Vec3, max: Vec3) -> Self {
        self.min_size = min;
        self.max_size = max;
        self
    }

    pub fn with_spawn_area(mut self, width: f32, depth: f32) -> Self {
        self.spawn_area = (width, depth);
        self
    }

    pub fn with_material(mut self, material: MaterialPreset) -> Self {
        self.material = material;
        self
    }
}

/// Predefined obstacle scenarios
pub struct ObstaclePresets;

impl ObstaclePresets {
    /// Sparse obstacles (few large boxes)
    pub fn sparse() -> ObstacleConfig {
        ObstacleConfig::new()
            .with_count(5)
            .with_size_range(Vec3::new(0.5, 1.0, 0.5), Vec3::new(2.0, 2.0, 2.0))
            .with_spawn_area(30.0, 30.0)
    }

    /// Dense obstacles (many small boxes)
    pub fn dense() -> ObstacleConfig {
        ObstacleConfig::new()
            .with_count(50)
            .with_size_range(Vec3::new(0.1, 0.2, 0.1), Vec3::new(0.5, 1.0, 0.5))
            .with_spawn_area(20.0, 20.0)
    }

    /// Corridor environment
    pub fn corridor() -> ObstacleConfig {
        ObstacleConfig::new()
            .with_count(20)
            .with_size_range(Vec3::new(0.3, 1.5, 0.3), Vec3::new(0.5, 2.0, 0.5))
            .with_spawn_area(15.0, 40.0)
    }

    /// Warehouse racks
    pub fn warehouse_racks() -> ObstacleConfig {
        ObstacleConfig::new()
            .with_count(15)
            .with_size_range(Vec3::new(1.0, 2.0, 0.5), Vec3::new(2.0, 3.0, 0.8))
            .with_spawn_area(40.0, 40.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_config() {
        let config = WorldConfig::new("test_world")
            .with_ground_size(50.0, 50.0)
            .with_lighting(0.5, 0.5);

        assert_eq!(config.name, "test_world");
        assert_eq!(config.ground_size, (50.0, 50.0));
        assert_eq!(config.ambient_light, 0.5);
    }

    #[test]
    fn test_world_presets() {
        let warehouse = WorldPresets::warehouse();
        assert_eq!(warehouse.name, "warehouse");
        assert!(warehouse.enable_ground);

        let space = WorldPresets::space();
        assert!(!space.enable_ground); // No ground in space

        let arctic = WorldPresets::arctic();
        assert!(arctic.enable_fog); // Arctic has fog
    }

    #[test]
    fn test_obstacle_config() {
        let config = ObstacleConfig::new()
            .with_count(20)
            .with_spawn_area(30.0, 30.0);

        assert_eq!(config.num_obstacles, 20);
        assert_eq!(config.spawn_area, (30.0, 30.0));
    }

    #[test]
    fn test_obstacle_presets() {
        let sparse = ObstaclePresets::sparse();
        assert!(sparse.num_obstacles < 10); // Few obstacles

        let dense = ObstaclePresets::dense();
        assert!(dense.num_obstacles > 40); // Many obstacles
    }
}
