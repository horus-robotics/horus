//! Camera sensor module for sim2d
//!
//! Provides a simulated 2D camera sensor using raycast-based rendering.
//! The camera generates grayscale images by raycasting from the robot's perspective.

use crate::Obstacle;
use serde::{Deserialize, Serialize};

/// Camera sensor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// Enable camera sensor
    pub enabled: bool,

    /// Horizontal field of view in radians
    pub fov_horizontal: f32,

    /// Vertical field of view in radians
    pub fov_vertical: f32,

    /// Image width in pixels
    pub width: usize,

    /// Image height in pixels
    pub height: usize,

    /// Maximum render distance in meters
    pub max_distance: f32,

    /// Topic to publish images on
    pub topic: String,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fov_horizontal: 1.0472, // 60 degrees
            fov_vertical: 0.7854,   // 45 degrees
            width: 320,
            height: 240,
            max_distance: 20.0,
            topic: "camera.image".to_string(),
        }
    }
}

/// Grayscale image data
#[derive(Debug, Clone)]
pub struct GrayscaleImage {
    /// Image width
    pub width: usize,

    /// Image height
    pub height: usize,

    /// Pixel data (row-major, 0-255)
    pub data: Vec<u8>,
}

impl GrayscaleImage {
    /// Create a new grayscale image
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0; width * height],
        }
    }

    /// Set pixel value
    pub fn set_pixel(&mut self, x: usize, y: usize, value: u8) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = value;
        }
    }

    /// Get pixel value
    pub fn get_pixel(&self, x: usize, y: usize) -> u8 {
        if x < self.width && y < self.height {
            self.data[y * self.width + x]
        } else {
            0
        }
    }

    /// Convert to PNG bytes
    pub fn to_png(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use image::ImageBuffer;

        let img = ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_raw(
            self.width as u32,
            self.height as u32,
            self.data.clone(),
        )
        .ok_or("Failed to create image buffer")?;

        let mut png_data = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_data),
            image::ImageOutputFormat::Png,
        )?;

        Ok(png_data)
    }

    /// Fill with background color
    pub fn fill(&mut self, value: u8) {
        self.data.fill(value);
    }
}

/// Camera sensor that generates images via raycasting
pub struct CameraSensor {
    /// Configuration
    config: CameraConfig,

    /// Last rendered image
    last_image: Option<GrayscaleImage>,
}

impl CameraSensor {
    /// Create a new camera sensor
    pub fn new(config: CameraConfig) -> Self {
        Self {
            config,
            last_image: None,
        }
    }

    /// Render an image from the robot's perspective
    pub fn render(
        &mut self,
        robot_position: [f32; 2],
        robot_heading: f32,
        obstacles: &[Obstacle],
        world_width: f32,
        world_height: f32,
    ) -> &GrayscaleImage {
        let mut image = GrayscaleImage::new(self.config.width, self.config.height);

        // Fill with sky color (bright)
        image.fill(220);

        // Render each pixel column using raycasting
        for x in 0..self.config.width {
            // Calculate ray angle for this pixel column
            let normalized_x = (x as f32 / self.config.width as f32) - 0.5;
            let ray_angle = robot_heading + normalized_x * self.config.fov_horizontal;

            // Cast ray and get distance to nearest obstacle
            let hit_distance = self.raycast(
                robot_position,
                ray_angle,
                obstacles,
                world_width,
                world_height,
            );

            // Calculate column height based on distance (perspective projection)
            if let Some(distance) = hit_distance {
                let wall_height = (self.config.height as f32 * 0.5) / distance.max(0.1_f32);
                let wall_height_pixels = wall_height.min(self.config.height as f32) as usize;

                // Calculate wall brightness based on distance
                let brightness =
                    (255.0 * (1.0 - (distance / self.config.max_distance).min(1.0_f32))) as u8;
                let wall_color = brightness.max(30_u8); // Ensure walls aren't too dark

                // Draw vertical line for wall
                let start_y = (self.config.height / 2).saturating_sub(wall_height_pixels / 2);
                let end_y =
                    (self.config.height / 2 + wall_height_pixels / 2).min(self.config.height);

                // Sky is already filled (top, 0..start_y)

                // Fill wall
                for y in start_y..end_y {
                    image.set_pixel(x, y, wall_color);
                }

                // Fill floor (bottom half, darker)
                for y in end_y..self.config.height {
                    image.set_pixel(x, y, 80); // Floor color
                }
            }
        }

        self.last_image = Some(image);
        // Safe to unwrap since we just set it above
        self.last_image
            .as_ref()
            .expect("Just set last_image to Some")
    }

    /// Cast a ray and return distance to nearest obstacle
    fn raycast(
        &self,
        origin: [f32; 2],
        angle: f32,
        obstacles: &[Obstacle],
        world_width: f32,
        world_height: f32,
    ) -> Option<f32> {
        let direction = [angle.cos(), angle.sin()];
        let mut min_distance = None;

        // Check world boundaries
        let boundary_distance =
            self.raycast_boundaries(origin, direction, world_width, world_height);
        if let Some(dist) = boundary_distance {
            min_distance = Some(dist);
        }

        // Check all obstacles
        for obstacle in obstacles {
            if let Some(dist) = self.raycast_obstacle(origin, direction, obstacle) {
                if dist < min_distance.unwrap_or(f32::INFINITY) {
                    min_distance = Some(dist);
                }
            }
        }

        min_distance
    }

    /// Raycast against world boundaries
    fn raycast_boundaries(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        width: f32,
        height: f32,
    ) -> Option<f32> {
        let mut min_t = f32::INFINITY;

        // Check each boundary
        // Left wall (x = 0)
        if direction[0] < 0.0 {
            let t = -origin[0] / direction[0];
            let y = origin[1] + t * direction[1];
            if t > 0.0 && y >= 0.0 && y <= height {
                min_t = min_t.min(t);
            }
        }

        // Right wall (x = width)
        if direction[0] > 0.0 {
            let t = (width - origin[0]) / direction[0];
            let y = origin[1] + t * direction[1];
            if t > 0.0 && y >= 0.0 && y <= height {
                min_t = min_t.min(t);
            }
        }

        // Bottom wall (y = 0)
        if direction[1] < 0.0 {
            let t = -origin[1] / direction[1];
            let x = origin[0] + t * direction[0];
            if t > 0.0 && x >= 0.0 && x <= width {
                min_t = min_t.min(t);
            }
        }

        // Top wall (y = height)
        if direction[1] > 0.0 {
            let t = (height - origin[1]) / direction[1];
            let x = origin[0] + t * direction[0];
            if t > 0.0 && x >= 0.0 && x <= width {
                min_t = min_t.min(t);
            }
        }

        if min_t.is_finite() {
            Some(min_t)
        } else {
            None
        }
    }

    /// Raycast against a single obstacle
    fn raycast_obstacle(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        obstacle: &Obstacle,
    ) -> Option<f32> {
        use crate::ObstacleShape;

        match obstacle.shape {
            ObstacleShape::Rectangle => {
                self.raycast_rectangle(origin, direction, obstacle.pos, obstacle.size)
            }
            ObstacleShape::Circle => {
                self.raycast_circle(origin, direction, obstacle.pos, obstacle.size[0] / 2.0)
            }
        }
    }

    /// Raycast against an axis-aligned rectangle
    fn raycast_rectangle(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        center: [f32; 2],
        size: [f32; 2],
    ) -> Option<f32> {
        let half_width = size[0] / 2.0;
        let half_height = size[1] / 2.0;

        let min_x = center[0] - half_width;
        let max_x = center[0] + half_width;
        let min_y = center[1] - half_height;
        let max_y = center[1] + half_height;

        let mut t_min = 0.0_f32;
        let mut t_max = f32::INFINITY;

        // X-axis slabs
        if direction[0].abs() > 1e-6 {
            let tx1 = (min_x - origin[0]) / direction[0];
            let tx2 = (max_x - origin[0]) / direction[0];
            t_min = t_min.max(tx1.min(tx2));
            t_max = t_max.min(tx1.max(tx2));
        } else if origin[0] < min_x || origin[0] > max_x {
            return None;
        }

        // Y-axis slabs
        if direction[1].abs() > 1e-6 {
            let ty1 = (min_y - origin[1]) / direction[1];
            let ty2 = (max_y - origin[1]) / direction[1];
            t_min = t_min.max(ty1.min(ty2));
            t_max = t_max.min(ty1.max(ty2));
        } else if origin[1] < min_y || origin[1] > max_y {
            return None;
        }

        if t_max >= t_min && t_min >= 0.0 {
            Some(t_min)
        } else {
            None
        }
    }

    /// Raycast against a circle
    fn raycast_circle(
        &self,
        origin: [f32; 2],
        direction: [f32; 2],
        center: [f32; 2],
        radius: f32,
    ) -> Option<f32> {
        let oc = [origin[0] - center[0], origin[1] - center[1]];

        let a = direction[0] * direction[0] + direction[1] * direction[1];
        let b = 2.0 * (oc[0] * direction[0] + oc[1] * direction[1]);
        let c = oc[0] * oc[0] + oc[1] * oc[1] - radius * radius;

        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return None;
        }

        let t1 = (-b - discriminant.sqrt()) / (2.0 * a);
        let t2 = (-b + discriminant.sqrt()) / (2.0 * a);

        if t1 >= 0.0 {
            Some(t1)
        } else if t2 >= 0.0 {
            Some(t2)
        } else {
            None
        }
    }

    /// Get the last rendered image
    pub fn get_last_image(&self) -> Option<&GrayscaleImage> {
        self.last_image.as_ref()
    }

    /// Get configuration
    pub fn config(&self) -> &CameraConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObstacleShape;

    #[test]
    fn test_grayscale_image() {
        let mut img = GrayscaleImage::new(100, 100);
        img.set_pixel(50, 50, 255);
        assert_eq!(img.get_pixel(50, 50), 255);
        assert_eq!(img.get_pixel(0, 0), 0);
    }

    #[test]
    fn test_camera_render_empty() {
        let config = CameraConfig::default();
        let mut camera = CameraSensor::new(config);

        let image = camera.render([10.0, 7.5], 0.0, &[], 20.0, 15.0);

        assert_eq!(image.width, 320);
        assert_eq!(image.height, 240);
    }

    #[test]
    fn test_camera_render_with_obstacle() {
        let config = CameraConfig::default();
        let mut camera = CameraSensor::new(config);

        let obstacles = vec![Obstacle {
            pos: [15.0, 7.5],
            shape: ObstacleShape::Rectangle,
            size: [2.0, 2.0],
            color: None,
        }];

        let image = camera.render([10.0, 7.5], 0.0, &obstacles, 20.0, 15.0);

        assert_eq!(image.width, 320);
        assert_eq!(image.height, 240);

        // Center pixel should show the wall (not sky or floor)
        let center_pixel = image.get_pixel(160, 120);
        assert!(
            center_pixel < 220,
            "Center pixel should be darker than sky (220), but got {}",
            center_pixel
        );
        assert!(
            center_pixel > 80,
            "Center pixel should be brighter than floor (80), but got {}",
            center_pixel
        );
    }

    #[test]
    fn test_raycast_rectangle() {
        let config = CameraConfig::default();
        let camera = CameraSensor::new(config);

        // Ray pointing right, rectangle ahead
        let distance = camera.raycast_rectangle([0.0, 0.0], [1.0, 0.0], [5.0, 0.0], [2.0, 2.0]);

        assert!(distance.is_some());
        assert!((distance.unwrap() - 4.0).abs() < 0.1);
    }

    #[test]
    fn test_raycast_circle() {
        let config = CameraConfig::default();
        let camera = CameraSensor::new(config);

        // Ray pointing right, circle ahead
        let distance = camera.raycast_circle(
            [0.0, 0.0],
            [1.0, 0.0],
            [5.0, 0.0],
            1.0, // radius
        );

        assert!(distance.is_some());
        assert!((distance.unwrap() - 4.0).abs() < 0.1);
    }

    #[test]
    fn test_png_export() {
        let mut img = GrayscaleImage::new(10, 10);
        img.fill(128);

        let png_data = img.to_png().unwrap();
        assert!(!png_data.is_empty());
        // PNG signature
        assert_eq!(&png_data[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
