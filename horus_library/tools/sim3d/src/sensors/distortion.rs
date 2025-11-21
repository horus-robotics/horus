//! Lens distortion models for realistic camera simulation

use bevy::prelude::*;

/// Camera lens distortion component
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct LensDistortion {
    /// Distortion model type
    pub model: DistortionModel,
    /// Radial distortion coefficients [k1, k2, k3, k4]
    pub radial_coeffs: [f32; 4],
    /// Tangential distortion coefficients [p1, p2]
    pub tangential_coeffs: [f32; 2],
    /// Vignetting intensity (0.0 = none, 1.0 = full)
    pub vignetting_intensity: f32,
    /// Vignetting falloff power (typically 2-4)
    pub vignetting_falloff: f32,
    /// Chromatic aberration strength
    pub chromatic_aberration: f32,
    /// Optical center offset (normalized coordinates, 0.5 = center)
    pub optical_center: (f32, f32),
}

impl Default for LensDistortion {
    fn default() -> Self {
        Self {
            model: DistortionModel::BrownConrady,
            radial_coeffs: [0.0, 0.0, 0.0, 0.0],
            tangential_coeffs: [0.0, 0.0],
            vignetting_intensity: 0.0,
            vignetting_falloff: 3.0,
            chromatic_aberration: 0.0,
            optical_center: (0.5, 0.5),
        }
    }
}

impl LensDistortion {
    /// No distortion (ideal pinhole camera)
    pub fn none() -> Self {
        Self::default()
    }

    /// Barrel distortion (common in wide-angle lenses)
    pub fn barrel(strength: f32) -> Self {
        Self {
            model: DistortionModel::BrownConrady,
            radial_coeffs: [-strength, 0.0, 0.0, 0.0],
            ..default()
        }
    }

    /// Pincushion distortion (common in telephoto lenses)
    pub fn pincushion(strength: f32) -> Self {
        Self {
            model: DistortionModel::BrownConrady,
            radial_coeffs: [strength, 0.0, 0.0, 0.0],
            ..default()
        }
    }

    /// Fisheye lens (very wide angle with strong barrel distortion)
    pub fn fisheye(strength: f32) -> Self {
        Self {
            model: DistortionModel::FisheyeEquidistant,
            radial_coeffs: [strength, 0.0, 0.0, 0.0],
            vignetting_intensity: 0.3,
            vignetting_falloff: 4.0,
            ..default()
        }
    }

    /// Wide-angle lens (moderate barrel distortion)
    pub fn wide_angle() -> Self {
        Self {
            model: DistortionModel::BrownConrady,
            radial_coeffs: [-0.15, 0.02, 0.0, 0.0],
            vignetting_intensity: 0.15,
            vignetting_falloff: 3.0,
            chromatic_aberration: 0.002,
            ..default()
        }
    }

    /// Standard lens (minimal distortion)
    pub fn standard() -> Self {
        Self {
            model: DistortionModel::BrownConrady,
            radial_coeffs: [-0.02, 0.001, 0.0, 0.0],
            vignetting_intensity: 0.05,
            vignetting_falloff: 3.5,
            chromatic_aberration: 0.0005,
            ..default()
        }
    }

    /// Telephoto lens (slight pincushion)
    pub fn telephoto() -> Self {
        Self {
            model: DistortionModel::BrownConrady,
            radial_coeffs: [0.05, -0.01, 0.0, 0.0],
            vignetting_intensity: 0.1,
            vignetting_falloff: 4.0,
            chromatic_aberration: 0.001,
            ..default()
        }
    }

    /// Low-quality webcam (barrel distortion, strong vignetting)
    pub fn webcam() -> Self {
        Self {
            model: DistortionModel::BrownConrady,
            radial_coeffs: [-0.25, 0.08, 0.0, 0.0],
            tangential_coeffs: [0.001, 0.001],
            vignetting_intensity: 0.3,
            vignetting_falloff: 2.5,
            chromatic_aberration: 0.003,
            ..default()
        }
    }

    /// Apply distortion to normalized image coordinates
    /// Input: (x, y) in range [-1, 1] where (0, 0) is center
    /// Output: distorted (x, y) coordinates
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        // Adjust for optical center offset
        let cx = (self.optical_center.0 - 0.5) * 2.0;
        let cy = (self.optical_center.1 - 0.5) * 2.0;
        let x_centered = x - cx;
        let y_centered = y - cy;

        // Apply distortion model
        let (x_dist, y_dist) = match self.model {
            DistortionModel::BrownConrady => self.apply_brown_conrady(x_centered, y_centered),
            DistortionModel::FisheyeEquidistant => {
                self.apply_fisheye_equidistant(x_centered, y_centered)
            }
            DistortionModel::FisheyeStereographic => {
                self.apply_fisheye_stereographic(x_centered, y_centered)
            }
            DistortionModel::KannalaBrandt => self.apply_kannala_brandt(x_centered, y_centered),
        };

        (x_dist + cx, y_dist + cy)
    }

    /// Brown-Conrady distortion model (most common, used in OpenCV)
    fn apply_brown_conrady(&self, x: f32, y: f32) -> (f32, f32) {
        let r2 = x * x + y * y;
        let r4 = r2 * r2;
        let r6 = r4 * r2;
        let r8 = r4 * r4;

        // Radial distortion
        let radial_distortion = 1.0
            + self.radial_coeffs[0] * r2
            + self.radial_coeffs[1] * r4
            + self.radial_coeffs[2] * r6
            + self.radial_coeffs[3] * r8;

        // Tangential distortion
        let xy = x * y;
        let p1 = self.tangential_coeffs[0];
        let p2 = self.tangential_coeffs[1];

        let x_tangential = 2.0 * p1 * xy + p2 * (r2 + 2.0 * x * x);
        let y_tangential = p1 * (r2 + 2.0 * y * y) + 2.0 * p2 * xy;

        let x_dist = x * radial_distortion + x_tangential;
        let y_dist = y * radial_distortion + y_tangential;

        (x_dist, y_dist)
    }

    /// Fisheye equidistant projection (f * θ)
    fn apply_fisheye_equidistant(&self, x: f32, y: f32) -> (f32, f32) {
        let r = (x * x + y * y).sqrt();
        if r < 1e-6 {
            return (x, y);
        }

        let theta = r.atan();
        let theta2 = theta * theta;
        let theta4 = theta2 * theta2;
        let theta6 = theta4 * theta2;
        let theta8 = theta4 * theta4;

        let theta_dist = theta
            * (1.0
                + self.radial_coeffs[0] * theta2
                + self.radial_coeffs[1] * theta4
                + self.radial_coeffs[2] * theta6
                + self.radial_coeffs[3] * theta8);

        let scale = theta_dist / r;
        (x * scale, y * scale)
    }

    /// Fisheye stereographic projection
    fn apply_fisheye_stereographic(&self, x: f32, y: f32) -> (f32, f32) {
        let r = (x * x + y * y).sqrt();
        if r < 1e-6 {
            return (x, y);
        }

        let theta = r.atan();
        let r_proj = 2.0 * (theta / 2.0).tan();

        let scale = r_proj / r;
        (x * scale, y * scale)
    }

    /// Kannala-Brandt fisheye model
    fn apply_kannala_brandt(&self, x: f32, y: f32) -> (f32, f32) {
        let r = (x * x + y * y).sqrt();
        if r < 1e-6 {
            return (x, y);
        }

        let theta = r.atan();
        let theta2 = theta * theta;
        let theta3 = theta2 * theta;
        let theta5 = theta3 * theta2;
        let theta7 = theta5 * theta2;
        let theta9 = theta7 * theta2;

        let r_dist = theta
            + self.radial_coeffs[0] * theta3
            + self.radial_coeffs[1] * theta5
            + self.radial_coeffs[2] * theta7
            + self.radial_coeffs[3] * theta9;

        let scale = r_dist / r;
        (x * scale, y * scale)
    }

    /// Calculate vignetting factor at normalized coordinates
    /// Returns multiplier in range [0, 1] where 1 = no vignetting
    pub fn vignetting_factor(&self, x: f32, y: f32) -> f32 {
        if self.vignetting_intensity <= 0.0 {
            return 1.0;
        }

        let r = (x * x + y * y).sqrt();
        let falloff = 1.0 - (r.powf(self.vignetting_falloff) * self.vignetting_intensity);
        falloff.clamp(0.0, 1.0)
    }

    /// Calculate chromatic aberration offset for RGB channels
    /// Returns (r_offset, g_offset, b_offset) for normalized coordinates
    pub fn chromatic_aberration_offsets(&self, x: f32, y: f32) -> [(f32, f32); 3] {
        if self.chromatic_aberration <= 0.0 {
            return [(x, y), (x, y), (x, y)];
        }

        let r = (x * x + y * y).sqrt();
        let strength = self.chromatic_aberration;

        // Red channel: expand outward
        let scale_r = 1.0 + strength * r;
        // Green channel: no offset (reference)
        let scale_g = 1.0;
        // Blue channel: contract inward
        let scale_b = 1.0 - strength * r;

        [
            (x * scale_r, y * scale_r), // Red
            (x * scale_g, y * scale_g), // Green
            (x * scale_b, y * scale_b), // Blue
        ]
    }
}

/// Distortion model types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum DistortionModel {
    /// Brown-Conrady (plumb-bob) model - most common, used in OpenCV
    BrownConrady,
    /// Fisheye equidistant projection (f * θ)
    FisheyeEquidistant,
    /// Fisheye stereographic projection
    FisheyeStereographic,
    /// Kannala-Brandt fisheye model
    KannalaBrandt,
}

/// Utility to apply distortion to an image buffer
pub fn apply_distortion_to_image(
    input: &[u8],
    output: &mut [u8],
    width: u32,
    height: u32,
    distortion: &LensDistortion,
    channels: usize,
) {
    let aspect = width as f32 / height as f32;

    for y in 0..height {
        for x in 0..width {
            // Normalize to [-1, 1]
            let nx = (x as f32 / width as f32 - 0.5) * 2.0;
            let ny = (y as f32 / height as f32 - 0.5) * 2.0 / aspect;

            // Apply vignetting
            let vignette = distortion.vignetting_factor(nx, ny);

            // Get chromatic aberration offsets
            let ca_offsets = distortion.chromatic_aberration_offsets(nx, ny);

            // Sample each channel with its offset
            for c in 0..channels {
                let (offset_x, offset_y) = if c < 3 {
                    ca_offsets[c]
                } else {
                    (nx, ny) // Alpha channel: no offset
                };

                // Apply distortion
                let (dist_x, dist_y) = distortion.apply(offset_x, offset_y);

                // Convert back to pixel coordinates
                let src_x = ((dist_x / 2.0 + 0.5) * width as f32).clamp(0.0, width as f32 - 1.0);
                let src_y =
                    ((dist_y * aspect / 2.0 + 0.5) * height as f32).clamp(0.0, height as f32 - 1.0);

                // Bilinear interpolation
                let value = bilinear_sample(input, width, height, channels, src_x, src_y, c);

                // Apply vignetting
                let vignetted = (value as f32 * vignette) as u8;

                let dst_idx = ((y * width + x) * channels as u32 + c as u32) as usize;
                if dst_idx < output.len() {
                    output[dst_idx] = vignetted;
                }
            }
        }
    }
}

/// Bilinear interpolation for image sampling
fn bilinear_sample(
    image: &[u8],
    width: u32,
    height: u32,
    channels: usize,
    x: f32,
    y: f32,
    channel: usize,
) -> u8 {
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);

    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let get_pixel = |px: u32, py: u32| -> f32 {
        let idx = ((py * width + px) * channels as u32 + channel as u32) as usize;
        if idx < image.len() {
            image[idx] as f32
        } else {
            0.0
        }
    };

    let p00 = get_pixel(x0, y0);
    let p10 = get_pixel(x1, y0);
    let p01 = get_pixel(x0, y1);
    let p11 = get_pixel(x1, y1);

    let p0 = p00 * (1.0 - fx) + p10 * fx;
    let p1 = p01 * (1.0 - fx) + p11 * fx;
    let result = p0 * (1.0 - fy) + p1 * fy;

    result.clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_distortion() {
        let distortion = LensDistortion::none();
        let (x, y) = distortion.apply(0.5, 0.5);
        assert!((x - 0.5).abs() < 0.001);
        assert!((y - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_barrel_distortion() {
        let distortion = LensDistortion::barrel(0.2);
        let (x, y) = distortion.apply(0.8, 0.0);

        // Barrel distortion should compress radially
        assert!(x.abs() < 0.8);
    }

    #[test]
    fn test_pincushion_distortion() {
        let distortion = LensDistortion::pincushion(0.2);
        let (x, y) = distortion.apply(0.8, 0.0);

        // Pincushion should expand radially
        assert!(x.abs() > 0.8);
    }

    #[test]
    fn test_vignetting() {
        let mut distortion = LensDistortion::none();
        distortion.vignetting_intensity = 0.5;
        distortion.vignetting_falloff = 2.0;

        let center = distortion.vignetting_factor(0.0, 0.0);
        let edge = distortion.vignetting_factor(1.0, 0.0);

        assert!(center > edge); // Center should be brighter
        assert_eq!(center, 1.0); // Center should be unaffected
        assert!(edge < 1.0); // Edge should be darkened
    }

    #[test]
    fn test_chromatic_aberration() {
        let mut distortion = LensDistortion::none();
        distortion.chromatic_aberration = 0.01;

        let offsets = distortion.chromatic_aberration_offsets(0.5, 0.0);
        let (r_x, _) = offsets[0]; // Red
        let (g_x, _) = offsets[1]; // Green
        let (b_x, _) = offsets[2]; // Blue

        // Red should be more expanded than green, blue more contracted
        assert!(r_x > g_x);
        assert!(b_x < g_x);
    }

    #[test]
    fn test_fisheye_distortion() {
        let distortion = LensDistortion::fisheye(1.0);
        assert_eq!(distortion.model, DistortionModel::FisheyeEquidistant);

        let (x, y) = distortion.apply(0.5, 0.5);
        // Fisheye should apply nonlinear transformation
        assert_ne!(x, 0.5);
        assert_ne!(y, 0.5);
    }

    #[test]
    fn test_wide_angle_preset() {
        let distortion = LensDistortion::wide_angle();
        assert!(distortion.radial_coeffs[0] < 0.0); // Barrel distortion
        assert!(distortion.vignetting_intensity > 0.0);
        assert!(distortion.chromatic_aberration > 0.0);
    }

    #[test]
    fn test_telephoto_preset() {
        let distortion = LensDistortion::telephoto();
        assert!(distortion.radial_coeffs[0] > 0.0); // Pincushion distortion
    }

    #[test]
    fn test_webcam_preset() {
        let distortion = LensDistortion::webcam();
        assert!(distortion.radial_coeffs[0] < 0.0); // Strong barrel
        assert!(distortion.vignetting_intensity > 0.2); // Strong vignetting
        assert!(distortion.chromatic_aberration > 0.002); // Noticeable CA
    }

    #[test]
    fn test_optical_center_offset() {
        let mut distortion = LensDistortion::barrel(0.2);
        distortion.optical_center = (0.6, 0.5); // Offset right

        let (x1, _) = distortion.apply(0.0, 0.0);
        // With offset optical center and distortion, origin should be affected
        let centered_distortion = LensDistortion::barrel(0.2);
        let (x2, _) = centered_distortion.apply(0.0, 0.0);

        // Result should differ from centered distortion
        assert_ne!(x1, x2);
    }

    #[test]
    fn test_distortion_models() {
        let models = [
            DistortionModel::BrownConrady,
            DistortionModel::FisheyeEquidistant,
            DistortionModel::FisheyeStereographic,
            DistortionModel::KannalaBrandt,
        ];

        for model in models {
            let mut distortion = LensDistortion::none();
            distortion.model = model;
            distortion.radial_coeffs[0] = 0.1;

            // All models should produce valid output
            let (x, y) = distortion.apply(0.5, 0.5);
            assert!(x.is_finite());
            assert!(y.is_finite());
        }
    }

    #[test]
    fn test_center_no_distortion() {
        let distortion = LensDistortion::barrel(0.5);
        let (x, y) = distortion.apply(0.0, 0.0);

        // Center should remain at center
        assert!((x - 0.0).abs() < 0.001);
        assert!((y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_bilinear_sample() {
        let image = vec![0u8, 100, 200, 255];
        let value = bilinear_sample(&image, 2, 2, 1, 0.5, 0.5, 0);

        // Should interpolate between pixels
        assert!(value > 0);
        assert!(value < 255);
    }
}
