use bevy::prelude::*;
use rand::thread_rng;

use crate::sensors::depth::DepthCamera;
use crate::sensors::noise::{GaussianNoise, NoiseModel};

/// RGB-D camera sensor (combines color and depth like Kinect/RealSense)
#[derive(Component, Clone)]
pub struct RGBDCamera {
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
    /// RGB image data (row-major, RGB format)
    pub color_image: Vec<u8>,
    /// Depth image data (row-major order, in meters)
    pub depth_image: Vec<f32>,
    /// Depth noise standard deviation
    pub depth_noise_std_dev: f32,
    /// Color noise standard deviation (0-255 scale)
    pub color_noise_std_dev: f32,
}

impl RGBDCamera {
    /// Create a new RGB-D camera with default parameters
    pub fn new(width: u32, height: u32) -> Self {
        let num_pixels = (width * height) as usize;
        Self {
            width,
            height,
            fov_horizontal: 1.0472, // ~60 degrees
            fov_vertical: 0.7854,   // ~45 degrees
            min_range: 0.1,
            max_range: 10.0,
            rate_hz: 30.0,
            time_since_update: 0.0,
            color_image: vec![0; num_pixels * 3], // RGB = 3 bytes per pixel
            depth_image: vec![0.0; num_pixels],
            depth_noise_std_dev: 0.01, // 1cm standard deviation
            color_noise_std_dev: 2.0,  // Small color noise
        }
    }

    /// Create a high resolution RGB-D camera (like RealSense D435)
    pub fn high_res() -> Self {
        Self::new(640, 480)
    }

    /// Create a low resolution RGB-D camera (for performance)
    pub fn low_res() -> Self {
        Self::new(320, 240)
    }

    /// Get RGB color at pixel coordinates
    pub fn get_color(&self, x: u32, y: u32) -> Option<[u8; 3]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = ((y * self.width + x) * 3) as usize;
        Some([
            self.color_image[index],
            self.color_image[index + 1],
            self.color_image[index + 2],
        ])
    }

    /// Set RGB color at pixel coordinates
    pub fn set_color(&mut self, x: u32, y: u32, color: [u8; 3]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = ((y * self.width + x) * 3) as usize;
        self.color_image[index] = color[0];
        self.color_image[index + 1] = color[1];
        self.color_image[index + 2] = color[2];
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

    /// Get colored point cloud
    pub fn get_colored_point_cloud(&self) -> Vec<(Vec3, Color)> {
        let mut points = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(point) = self.pixel_to_point(x, y) {
                    if let Some(rgb) = self.get_color(x, y) {
                        let color = Color::srgb(
                            rgb[0] as f32 / 255.0,
                            rgb[1] as f32 / 255.0,
                            rgb[2] as f32 / 255.0,
                        );
                        points.push((point, color));
                    }
                }
            }
        }

        points
    }

    /// Get depth-only point cloud (no color)
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

    /// Convert to standalone DepthCamera
    pub fn to_depth_camera(&self) -> DepthCamera {
        let mut depth_cam = DepthCamera::new(self.width, self.height);
        depth_cam.fov_horizontal = self.fov_horizontal;
        depth_cam.fov_vertical = self.fov_vertical;
        depth_cam.min_range = self.min_range;
        depth_cam.max_range = self.max_range;
        depth_cam.rate_hz = self.rate_hz;
        depth_cam.depth_image = self.depth_image.clone();
        depth_cam.noise_std_dev = self.depth_noise_std_dev;
        depth_cam
    }

    /// Check if sensor needs update based on rate
    pub fn should_update(&self, _delta_time: f32) -> bool {
        if self.rate_hz <= 0.0 {
            return false;
        }
        self.time_since_update >= 1.0 / self.rate_hz
    }

    /// Apply noise to depth and color measurements
    pub fn apply_noise(&mut self) {
        let mut rng = thread_rng();

        // Apply depth noise
        if self.depth_noise_std_dev > 0.0 {
            for depth in &mut self.depth_image {
                if *depth >= self.min_range && *depth <= self.max_range {
                    // Noise increases with distance (quadratic model)
                    let distance_factor = 1.0 + 0.01 * depth.powi(2);
                    let depth_noise =
                        GaussianNoise::zero_mean(self.depth_noise_std_dev * distance_factor);
                    *depth = depth_noise
                        .apply(*depth, &mut rng)
                        .clamp(self.min_range, self.max_range);
                }
            }
        }

        // Apply color noise
        if self.color_noise_std_dev > 0.0 {
            let color_noise = GaussianNoise::zero_mean(self.color_noise_std_dev);
            for pixel in &mut self.color_image {
                let noisy = color_noise.apply(*pixel as f32, &mut rng);
                *pixel = noisy.clamp(0.0, 255.0) as u8;
            }
        }
    }

    /// Clear both depth and color images
    pub fn clear(&mut self) {
        self.depth_image.fill(self.max_range);
        self.color_image.fill(0);
    }
}

/// System to update RGB-D cameras
pub fn rgbd_camera_update_system(
    mut cameras: Query<(&mut RGBDCamera, &Transform)>,
    time: Res<Time>,
) {
    let delta_time = time.delta_secs();

    for (mut camera, _transform) in cameras.iter_mut() {
        camera.time_since_update += delta_time;

        if camera.should_update(delta_time) {
            camera.time_since_update = 0.0;
            // Apply noise after measurements are taken
            camera.apply_noise();
        }
    }
}

/// Plugin to register RGB-D camera systems
pub struct RGBDCameraPlugin;

impl Plugin for RGBDCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, rgbd_camera_update_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgbd_camera_creation() {
        let camera = RGBDCamera::new(320, 240);
        assert_eq!(camera.width, 320);
        assert_eq!(camera.height, 240);
        assert_eq!(camera.depth_image.len(), 320 * 240);
        assert_eq!(camera.color_image.len(), 320 * 240 * 3);
    }

    #[test]
    fn test_high_res_camera() {
        let camera = RGBDCamera::high_res();
        assert_eq!(camera.width, 640);
        assert_eq!(camera.height, 480);
    }

    #[test]
    fn test_color_access() {
        let mut camera = RGBDCamera::new(10, 10);
        camera.set_color(5, 5, [255, 128, 64]);
        assert_eq!(camera.get_color(5, 5), Some([255, 128, 64]));
        assert_eq!(camera.get_color(100, 100), None);
    }

    #[test]
    fn test_depth_access() {
        let mut camera = RGBDCamera::new(10, 10);
        camera.set_depth(5, 5, 3.5);
        assert_eq!(camera.get_depth(5, 5), Some(3.5));
        assert_eq!(camera.get_depth(100, 100), None);
    }

    #[test]
    fn test_pixel_to_ray() {
        let camera = RGBDCamera::new(640, 480);

        // Center pixel should point forward (mostly Z)
        let ray = camera.pixel_to_ray(320, 240);
        assert!(ray.z > 0.9);
        assert!(ray.x.abs() < 0.1);
        assert!(ray.y.abs() < 0.1);
    }

    #[test]
    fn test_pixel_to_point() {
        let mut camera = RGBDCamera::new(10, 10);
        camera.set_depth(5, 5, 2.0);

        let point = camera.pixel_to_point(5, 5);
        assert!(point.is_some());

        let point = point.unwrap();
        assert!((point.length() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_colored_point_cloud() {
        let mut camera = RGBDCamera::new(5, 5);

        // Set some valid depths and colors
        for y in 0..5 {
            for x in 0..5 {
                camera.set_depth(x, y, 1.0);
                camera.set_color(x, y, [255, 0, 0]);
            }
        }

        let cloud = camera.get_colored_point_cloud();
        assert_eq!(cloud.len(), 25);

        // Check first point has color
        let (_, color) = cloud[0];
        assert_eq!(color, Color::srgb(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_to_depth_camera() {
        let rgbd = RGBDCamera::new(320, 240);
        let depth = rgbd.to_depth_camera();

        assert_eq!(depth.width, 320);
        assert_eq!(depth.height, 240);
        assert_eq!(depth.fov_horizontal, rgbd.fov_horizontal);
    }

    #[test]
    fn test_should_update() {
        let mut camera = RGBDCamera::new(10, 10);
        camera.rate_hz = 10.0;
        camera.time_since_update = 0.05;

        assert!(!camera.should_update(0.01));

        camera.time_since_update = 0.15;
        assert!(camera.should_update(0.01));
    }

    #[test]
    fn test_clear() {
        let mut camera = RGBDCamera::new(10, 10);
        camera.set_depth(5, 5, 1.0);
        camera.set_color(5, 5, [255, 255, 255]);

        camera.clear();

        let depth = camera.get_depth(5, 5).unwrap();
        assert_eq!(depth, camera.max_range);

        let color = camera.get_color(5, 5).unwrap();
        assert_eq!(color, [0, 0, 0]);
    }
}
