use bevy::prelude::*;
use rand::thread_rng;

use crate::sensors::noise::{GaussianNoise, NoiseModel};

/// Depth camera sensor component
#[derive(Component, Clone)]
pub struct DepthCamera {
    /// Horizontal resolution (pixels)
    pub width: u32,
    /// Vertical resolution (pixels)
    pub height: u32,
    /// Horizontal field of view (radians)
    pub fov_horizontal: f32,
    /// Vertical field of view (radians)
    pub fov_vertical: f32,
    /// Minimum detection range (meters)
    pub min_range: f32,
    /// Maximum detection range (meters)
    pub max_range: f32,
    /// Update rate (Hz)
    pub rate_hz: f32,
    /// Time since last update
    pub time_since_update: f32,
    /// Current depth image (row-major order)
    pub depth_image: Vec<f32>,
    /// Noise model for depth measurements
    pub noise_std_dev: f32,
}

impl DepthCamera {
    /// Create a new depth camera with default parameters
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            fov_horizontal: 1.0472, // ~60 degrees
            fov_vertical: 0.7854,   // ~45 degrees
            min_range: 0.1,
            max_range: 10.0,
            rate_hz: 30.0,
            time_since_update: 0.0,
            depth_image: vec![0.0; (width * height) as usize],
            noise_std_dev: 0.01, // 1cm standard deviation
        }
    }

    /// Create a high resolution depth camera
    pub fn high_res() -> Self {
        Self::new(640, 480)
    }

    /// Create a low resolution depth camera (for performance)
    pub fn low_res() -> Self {
        Self::new(160, 120)
    }

    /// Get depth value at pixel coordinates
    pub fn get_depth(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y * self.width + x) as usize;
        Some(self.depth_image[index])
    }

    /// Set depth value at pixel coordinates
    pub fn set_depth(&mut self, x: u32, y: u32, depth: f32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = (y * self.width + x) as usize;
        self.depth_image[index] = depth;
    }

    /// Convert pixel coordinates to ray direction in camera frame
    pub fn pixel_to_ray(&self, x: u32, y: u32) -> Vec3 {
        let u = (x as f32 + 0.5) / self.width as f32;
        let v = (y as f32 + 0.5) / self.height as f32;

        // Convert to normalized device coordinates [-1, 1]
        let ndc_x = 2.0 * u - 1.0;
        let ndc_y = 1.0 - 2.0 * v; // Flip Y axis

        // Calculate ray direction
        let tan_half_fov_h = (self.fov_horizontal / 2.0).tan();
        let tan_half_fov_v = (self.fov_vertical / 2.0).tan();

        let ray_x = ndc_x * tan_half_fov_h;
        let ray_y = ndc_y * tan_half_fov_v;
        let ray_z = 1.0;

        Vec3::new(ray_x, ray_y, ray_z).normalize()
    }

    /// Convert depth at pixel to 3D point in camera frame
    pub fn pixel_to_point(&self, x: u32, y: u32) -> Option<Vec3> {
        let depth = self.get_depth(x, y)?;
        if depth < self.min_range || depth > self.max_range {
            return None;
        }

        let ray = self.pixel_to_ray(x, y);
        Some(ray * depth)
    }

    /// Get all valid 3D points from the depth image
    pub fn get_point_cloud(&self) -> Vec<Vec3> {
        let mut points = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(point) = self.pixel_to_point(x, y) {
                    points.push(point);
                }
            }
        }

        points
    }

    /// Check if sensor needs update based on rate
    pub fn should_update(&self, _delta_time: f32) -> bool {
        if self.rate_hz <= 0.0 {
            return false;
        }
        self.time_since_update >= 1.0 / self.rate_hz
    }

    /// Apply noise to depth measurements
    pub fn apply_noise(&mut self) {
        if self.noise_std_dev == 0.0 {
            return;
        }

        let mut rng = thread_rng();
        let _noise = GaussianNoise::zero_mean(self.noise_std_dev);

        for depth in &mut self.depth_image {
            if *depth >= self.min_range && *depth <= self.max_range {
                // Noise increases with distance (quadratic model)
                let distance_factor = 1.0 + 0.01 * depth.powi(2);
                let depth_noise = GaussianNoise::zero_mean(self.noise_std_dev * distance_factor);
                *depth = depth_noise
                    .apply(*depth, &mut rng)
                    .clamp(self.min_range, self.max_range);
            }
        }
    }

    /// Clear depth image (set all to max range)
    pub fn clear(&mut self) {
        self.depth_image.fill(self.max_range);
    }
}

/// System to update depth cameras
pub fn depth_camera_update_system(
    mut cameras: Query<(&mut DepthCamera, &Transform)>,
    time: Res<Time>,
) {
    let delta_time = time.delta_secs();

    for (mut camera, _transform) in cameras.iter_mut() {
        camera.time_since_update += delta_time;

        if camera.should_update(delta_time) {
            camera.time_since_update = 0.0;
            // Apply noise after depth measurements are taken
            camera.apply_noise();
        }
    }
}

/// Plugin to register depth camera systems
pub struct DepthCameraPlugin;

impl Plugin for DepthCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, depth_camera_update_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_camera_creation() {
        let camera = DepthCamera::new(320, 240);
        assert_eq!(camera.width, 320);
        assert_eq!(camera.height, 240);
        assert_eq!(camera.depth_image.len(), 320 * 240);
    }

    #[test]
    fn test_high_res_camera() {
        let camera = DepthCamera::high_res();
        assert_eq!(camera.width, 640);
        assert_eq!(camera.height, 480);
    }

    #[test]
    fn test_depth_access() {
        let mut camera = DepthCamera::new(10, 10);
        camera.set_depth(5, 5, 3.5);
        assert_eq!(camera.get_depth(5, 5), Some(3.5));
        assert_eq!(camera.get_depth(100, 100), None);
    }

    #[test]
    fn test_pixel_to_ray() {
        let camera = DepthCamera::new(640, 480);

        // Center pixel should point forward (mostly Z)
        let ray = camera.pixel_to_ray(320, 240);
        assert!(ray.z > 0.9);
        assert!(ray.x.abs() < 0.1);
        assert!(ray.y.abs() < 0.1);
    }

    #[test]
    fn test_pixel_to_point() {
        let mut camera = DepthCamera::new(10, 10);
        camera.set_depth(5, 5, 2.0);

        let point = camera.pixel_to_point(5, 5);
        assert!(point.is_some());

        let point = point.unwrap();
        assert!((point.length() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_should_update() {
        let mut camera = DepthCamera::new(10, 10);
        camera.rate_hz = 10.0;
        camera.time_since_update = 0.05;

        assert!(!camera.should_update(0.01));

        camera.time_since_update = 0.15;
        assert!(camera.should_update(0.01));
    }

    #[test]
    fn test_point_cloud() {
        let mut camera = DepthCamera::new(5, 5);

        // Set some valid depths
        for y in 0..5 {
            for x in 0..5 {
                camera.set_depth(x, y, 1.0);
            }
        }

        let cloud = camera.get_point_cloud();
        assert_eq!(cloud.len(), 25);
    }

    #[test]
    fn test_clear() {
        let mut camera = DepthCamera::new(10, 10);
        camera.set_depth(5, 5, 1.0);

        camera.clear();

        let depth = camera.get_depth(5, 5).unwrap();
        assert_eq!(depth, camera.max_range);
    }
}
