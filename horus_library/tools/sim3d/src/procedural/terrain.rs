use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use noise::{NoiseFn, Perlin, Simplex};

/// Terrain generation configuration
#[derive(Clone, Debug, Resource)]
pub struct TerrainConfig {
    pub size: Vec2,
    pub resolution: UVec2,
    pub height_scale: f32,
    pub noise_config: NoiseConfig,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            size: Vec2::new(100.0, 100.0),
            resolution: UVec2::new(100, 100),
            height_scale: 10.0,
            noise_config: NoiseConfig::default(),
        }
    }
}

/// Noise generation configuration
#[derive(Clone, Debug)]
pub struct NoiseConfig {
    pub noise_type: NoiseType,
    pub octaves: u32,
    pub frequency: f64,
    pub lacunarity: f64,
    pub persistence: f64,
    pub seed: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseType {
    Perlin,
    Simplex,
    Combined,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            noise_type: NoiseType::Perlin,
            octaves: 4,
            frequency: 1.0,
            lacunarity: 2.0,
            persistence: 0.5,
            seed: 0,
        }
    }
}

impl NoiseConfig {
    /// Smooth rolling hills
    pub fn hills() -> Self {
        Self {
            octaves: 3,
            frequency: 0.5,
            lacunarity: 2.0,
            persistence: 0.5,
            ..Default::default()
        }
    }

    /// Rugged mountains
    pub fn mountains() -> Self {
        Self {
            octaves: 6,
            frequency: 0.8,
            lacunarity: 2.5,
            persistence: 0.6,
            ..Default::default()
        }
    }

    /// Flat plains with subtle variation
    pub fn plains() -> Self {
        Self {
            octaves: 2,
            frequency: 0.3,
            lacunarity: 2.0,
            persistence: 0.3,
            ..Default::default()
        }
    }

    /// Rocky, detailed terrain
    pub fn rocky() -> Self {
        Self {
            octaves: 8,
            frequency: 1.5,
            lacunarity: 2.3,
            persistence: 0.55,
            ..Default::default()
        }
    }
}

/// Heightmap-based terrain
#[derive(Clone, Debug)]
pub struct Heightmap {
    pub width: u32,
    pub height: u32,
    pub heights: Vec<f32>,
}

impl Heightmap {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            heights: vec![0.0; (width * height) as usize],
        }
    }

    pub fn get(&self, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return 0.0;
        }
        self.heights[(y * self.width + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, height: f32) {
        if x < self.width && y < self.height {
            self.heights[(y * self.width + x) as usize] = height;
        }
    }

    /// Generate heightmap from noise
    pub fn from_noise(config: &TerrainConfig) -> Self {
        let mut heightmap = Self::new(config.resolution.x, config.resolution.y);
        let perlin = Perlin::new(config.noise_config.seed);
        let simplex = Simplex::new(config.noise_config.seed);

        for y in 0..config.resolution.y {
            for x in 0..config.resolution.x {
                let nx = x as f64 / config.resolution.x as f64;
                let ny = y as f64 / config.resolution.y as f64;

                let mut height = 0.0;
                let mut amplitude = 1.0;
                let mut frequency = config.noise_config.frequency;

                for _ in 0..config.noise_config.octaves {
                    let sample_x = nx * frequency;
                    let sample_y = ny * frequency;

                    let noise_value = match config.noise_config.noise_type {
                        NoiseType::Perlin => perlin.get([sample_x, sample_y]),
                        NoiseType::Simplex => simplex.get([sample_x, sample_y]),
                        NoiseType::Combined => {
                            (perlin.get([sample_x, sample_y]) + simplex.get([sample_x, sample_y]))
                                / 2.0
                        }
                    };

                    height += noise_value * amplitude;
                    amplitude *= config.noise_config.persistence;
                    frequency *= config.noise_config.lacunarity;
                }

                heightmap.set(x, y, height as f32 * config.height_scale);
            }
        }

        heightmap
    }

    /// Load from image file
    pub fn from_image(image: &Image) -> Option<Self> {
        // Simplified - would need proper image parsing
        let width = image.texture_descriptor.size.width;
        let height = image.texture_descriptor.size.height;

        let heightmap = Self::new(width, height);
        // Extract heights from image data (grayscale)
        // This is a simplified version
        Some(heightmap)
    }

    /// Apply erosion simulation
    pub fn apply_erosion(&mut self, iterations: u32, strength: f32) {
        for _ in 0..iterations {
            let mut new_heights = self.heights.clone();

            for y in 1..(self.height - 1) {
                for x in 1..(self.width - 1) {
                    let current = self.get(x, y);

                    // Calculate average of neighbors
                    let neighbors = [
                        self.get(x - 1, y),
                        self.get(x + 1, y),
                        self.get(x, y - 1),
                        self.get(x, y + 1),
                    ];

                    let avg = neighbors.iter().sum::<f32>() / 4.0;
                    let diff = avg - current;

                    new_heights[(y * self.width + x) as usize] = current + diff * strength;
                }
            }

            self.heights = new_heights;
        }
    }

    /// Normalize heights to 0-1 range
    pub fn normalize(&mut self) {
        let min = self.heights.iter().copied().fold(f32::INFINITY, f32::min);
        let max = self
            .heights
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let range = max - min;

        if range > 0.0 {
            for height in &mut self.heights {
                *height = (*height - min) / range;
            }
        }
    }
}

/// Terrain mesh generator
pub struct TerrainMeshGenerator;

impl TerrainMeshGenerator {
    /// Generate terrain mesh from heightmap
    pub fn generate_mesh(heightmap: &Heightmap, world_size: Vec2) -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let width = heightmap.width;
        let height = heightmap.height;

        let x_scale = world_size.x / (width - 1) as f32;
        let z_scale = world_size.y / (height - 1) as f32;

        // Generate vertices
        for z in 0..height {
            for x in 0..width {
                let h = heightmap.get(x, z);
                let world_x = x as f32 * x_scale - world_size.x / 2.0;
                let world_z = z as f32 * z_scale - world_size.y / 2.0;

                positions.push([world_x, h, world_z]);
                uvs.push([
                    x as f32 / (width - 1) as f32,
                    z as f32 / (height - 1) as f32,
                ]);
            }
        }

        // Calculate normals
        for z in 0..height {
            for x in 0..width {
                let normal = Self::calculate_normal(heightmap, x, z, x_scale, z_scale);
                normals.push(normal);
            }
        }

        // Generate indices
        for z in 0..(height - 1) {
            for x in 0..(width - 1) {
                let top_left = z * width + x;
                let top_right = top_left + 1;
                let bottom_left = (z + 1) * width + x;
                let bottom_right = bottom_left + 1;

                // Two triangles per quad
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));

        mesh
    }

    fn calculate_normal(
        heightmap: &Heightmap,
        x: u32,
        z: u32,
        x_scale: f32,
        z_scale: f32,
    ) -> [f32; 3] {
        let h_l = heightmap.get(x.saturating_sub(1), z);
        let h_r = heightmap.get((x + 1).min(heightmap.width - 1), z);
        let h_d = heightmap.get(x, z.saturating_sub(1));
        let h_u = heightmap.get(x, (z + 1).min(heightmap.height - 1));

        let normal = Vec3::new(
            (h_l - h_r) / (2.0 * x_scale),
            2.0,
            (h_d - h_u) / (2.0 * z_scale),
        )
        .normalize();

        normal.to_array()
    }
}

/// Vegetation placement configuration
#[derive(Clone, Debug, Resource)]
pub struct VegetationConfig {
    pub density: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub min_slope: f32,
    pub max_slope: f32,
    pub seed: u32,
}

impl Default for VegetationConfig {
    fn default() -> Self {
        Self {
            density: 0.1,
            min_height: 0.0,
            max_height: 50.0,
            min_slope: 0.0,
            max_slope: 0.5,
            seed: 0,
        }
    }
}

/// Vegetation placement point
#[derive(Clone, Debug)]
pub struct VegetationPoint {
    pub position: Vec3,
    pub normal: Vec3,
    pub scale: f32,
    pub rotation: f32,
}

/// Vegetation placer
pub struct VegetationPlacer;

impl VegetationPlacer {
    /// Generate vegetation placement points
    pub fn generate_points(
        heightmap: &Heightmap,
        world_size: Vec2,
        config: &VegetationConfig,
    ) -> Vec<VegetationPoint> {
        let mut points = Vec::new();
        let mut rng = fastrand::Rng::with_seed(config.seed as u64);

        let total_samples = (world_size.x * world_size.y * config.density) as u32;

        for _ in 0..total_samples {
            let x = rng.u32(0..heightmap.width);
            let z = rng.u32(0..heightmap.height);

            let height = heightmap.get(x, z);

            // Check height constraints
            if height < config.min_height || height > config.max_height {
                continue;
            }

            // Calculate slope
            let slope = Self::calculate_slope(heightmap, x, z);
            if slope < config.min_slope || slope > config.max_slope {
                continue;
            }

            // Calculate world position
            let x_scale = world_size.x / (heightmap.width - 1) as f32;
            let z_scale = world_size.y / (heightmap.height - 1) as f32;

            let world_x = x as f32 * x_scale - world_size.x / 2.0;
            let world_z = z as f32 * z_scale - world_size.y / 2.0;

            points.push(VegetationPoint {
                position: Vec3::new(world_x, height, world_z),
                normal: Vec3::Y, // Simplified
                scale: rng.f32() * 0.5 + 0.75,
                rotation: rng.f32() * std::f32::consts::TAU,
            });
        }

        points
    }

    fn calculate_slope(heightmap: &Heightmap, x: u32, z: u32) -> f32 {
        let _h = heightmap.get(x, z);
        let h_l = heightmap.get(x.saturating_sub(1), z);
        let h_r = heightmap.get((x + 1).min(heightmap.width - 1), z);
        let h_d = heightmap.get(x, z.saturating_sub(1));
        let h_u = heightmap.get(x, (z + 1).min(heightmap.height - 1));

        let dx = (h_r - h_l).abs();
        let dz = (h_u - h_d).abs();

        (dx * dx + dz * dz).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heightmap_creation() {
        let heightmap = Heightmap::new(10, 10);
        assert_eq!(heightmap.width, 10);
        assert_eq!(heightmap.height, 10);
        assert_eq!(heightmap.heights.len(), 100);
    }

    #[test]
    fn test_heightmap_get_set() {
        let mut heightmap = Heightmap::new(10, 10);
        heightmap.set(5, 5, 10.0);
        assert_eq!(heightmap.get(5, 5), 10.0);
    }

    #[test]
    fn test_noise_config_presets() {
        let hills = NoiseConfig::hills();
        assert_eq!(hills.octaves, 3);

        let mountains = NoiseConfig::mountains();
        assert_eq!(mountains.octaves, 6);

        let plains = NoiseConfig::plains();
        assert_eq!(plains.octaves, 2);
    }

    #[test]
    fn test_heightmap_from_noise() {
        let config = TerrainConfig {
            resolution: UVec2::new(50, 50),
            noise_config: NoiseConfig::hills(),
            ..Default::default()
        };

        let heightmap = Heightmap::from_noise(&config);
        assert_eq!(heightmap.width, 50);
        assert_eq!(heightmap.height, 50);

        // Check that heights are generated (not all zero)
        let non_zero = heightmap.heights.iter().any(|&h| h != 0.0);
        assert!(non_zero);
    }

    #[test]
    fn test_heightmap_normalize() {
        let mut heightmap = Heightmap::new(5, 5);
        heightmap.set(0, 0, -10.0);
        heightmap.set(4, 4, 20.0);

        heightmap.normalize();

        let min = heightmap
            .heights
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        let max = heightmap
            .heights
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);

        assert!((min - 0.0).abs() < 0.01);
        assert!((max - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_terrain_mesh_generation() {
        let mut heightmap = Heightmap::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                heightmap.set(x, y, (x + y) as f32);
            }
        }

        let mesh = TerrainMeshGenerator::generate_mesh(&heightmap, Vec2::new(100.0, 100.0));

        // Mesh should have vertices
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
        assert!(mesh.indices().is_some());
    }

    #[test]
    fn test_vegetation_placement() {
        let heightmap = Heightmap::from_noise(&TerrainConfig::default());
        let config = VegetationConfig {
            density: 0.05,
            ..Default::default()
        };

        let points =
            VegetationPlacer::generate_points(&heightmap, Vec2::new(100.0, 100.0), &config);

        // Should generate some points
        assert!(!points.is_empty());

        // Points should have valid properties
        for point in &points {
            assert!(point.scale > 0.0);
            assert!(point.rotation >= 0.0 && point.rotation <= std::f32::consts::TAU);
        }
    }
}
