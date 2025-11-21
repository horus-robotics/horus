//! Thermal camera (infrared) sensor simulation

use bevy::prelude::*;
use rand::Rng;

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
    /// Update rate (Hz)
    pub rate_hz: f32,
    /// Last update time
    pub last_update: f32,
    /// Wavelength band (micrometers) - typical 8-14μm for LWIR
    pub wavelength_min: f32,
    pub wavelength_max: f32,
    /// Noise equivalent temperature difference (NETD) in Kelvin
    pub netd: f32,
    /// Atmospheric transmission coefficient (0.0-1.0)
    pub atmospheric_transmission: f32,
}

impl Default for ThermalCamera {
    fn default() -> Self {
        Self {
            resolution: (320, 240),
            fov: 60.0,
            near: 0.1,
            far: 100.0,
            rate_hz: 30.0,
            last_update: 0.0,
            wavelength_min: 8.0, // Long-wave infrared (LWIR)
            wavelength_max: 14.0,
            netd: 0.05, // 50mK - high quality sensor
            atmospheric_transmission: 0.95,
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

    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn with_wavelength_band(mut self, min_um: f32, max_um: f32) -> Self {
        self.wavelength_min = min_um;
        self.wavelength_max = max_um;
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
    pub fn new(kelvin: f32) -> Self {
        Self { kelvin }
    }

    pub fn from_celsius(celsius: f32) -> Self {
        Self {
            kelvin: celsius + 273.15,
        }
    }

    pub fn from_fahrenheit(fahrenheit: f32) -> Self {
        Self {
            kelvin: (fahrenheit - 32.0) * 5.0 / 9.0 + 273.15,
        }
    }

    pub fn celsius(&self) -> f32 {
        self.kelvin - 273.15
    }

    pub fn fahrenheit(&self) -> f32 {
        (self.kelvin - 273.15) * 9.0 / 5.0 + 32.0
    }
}

impl Default for Temperature {
    fn default() -> Self {
        Self::from_celsius(20.0) // Room temperature
    }
}

/// Thermal material properties
#[derive(Component, Reflect, Clone, Copy)]
#[reflect(Component)]
pub struct ThermalProperties {
    /// Emissivity (0.0-1.0) - ability to emit thermal radiation
    pub emissivity: f32,
    /// Reflectivity (0.0-1.0) - for reflected temperature
    pub reflectivity: f32,
}

impl Default for ThermalProperties {
    fn default() -> Self {
        Self {
            emissivity: 0.95, // Most objects ~0.9-0.98
            reflectivity: 0.05,
        }
    }
}

impl ThermalProperties {
    pub fn new(emissivity: f32) -> Self {
        Self {
            emissivity: emissivity.clamp(0.0, 1.0),
            reflectivity: (1.0 - emissivity).clamp(0.0, 1.0),
        }
    }

    /// Shiny metal (low emissivity)
    pub fn metal() -> Self {
        Self::new(0.1)
    }

    /// Matte surface (high emissivity)
    pub fn matte() -> Self {
        Self::new(0.95)
    }

    /// Glass (medium emissivity, wavelength dependent)
    pub fn glass() -> Self {
        Self::new(0.85)
    }

    /// Black body (perfect emitter)
    pub fn blackbody() -> Self {
        Self::new(1.0)
    }
}

/// Thermal image data
#[derive(Component, Clone)]
pub struct ThermalImage {
    /// Temperature values in Kelvin for each pixel
    pub temperatures: Vec<f32>,
    /// Image dimensions (width, height)
    pub dimensions: (u32, u32),
    /// Timestamp
    pub timestamp: f32,
    /// Min/max temperature in frame (for auto-scaling)
    pub temp_range: (f32, f32),
}

impl ThermalImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            temperatures: vec![273.15; (width * height) as usize], // 0°C default
            dimensions: (width, height),
            timestamp: 0.0,
            temp_range: (273.15, 273.15),
        }
    }

    pub fn get_temperature(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.dimensions.0 || y >= self.dimensions.1 {
            return None;
        }
        let index = (y * self.dimensions.0 + x) as usize;
        self.temperatures.get(index).copied()
    }

    pub fn set_temperature(&mut self, x: u32, y: u32, temp_kelvin: f32) {
        if x >= self.dimensions.0 || y >= self.dimensions.1 {
            return;
        }
        let index = (y * self.dimensions.0 + x) as usize;
        if let Some(pixel) = self.temperatures.get_mut(index) {
            *pixel = temp_kelvin;
        }
    }

    /// Update temperature range for the frame
    pub fn update_range(&mut self) {
        if self.temperatures.is_empty() {
            return;
        }
        let min = self
            .temperatures
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        let max = self
            .temperatures
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        self.temp_range = (min, max);
    }

    /// Convert to RGB color image using iron color map (common for thermal)
    pub fn to_color_image(&self, color_map: ThermalColorMap) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.temperatures.len() * 3);
        let (min_temp, max_temp) = self.temp_range;
        let range = (max_temp - min_temp).max(1.0);

        for &temp in &self.temperatures {
            let normalized = ((temp - min_temp) / range).clamp(0.0, 1.0);
            let color = color_map.map(normalized);
            rgb.push((color.0 * 255.0) as u8);
            rgb.push((color.1 * 255.0) as u8);
            rgb.push((color.2 * 255.0) as u8);
        }

        rgb
    }

    /// Get temperature statistics
    pub fn get_statistics(&self) -> ThermalStatistics {
        if self.temperatures.is_empty() {
            return ThermalStatistics::default();
        }

        let count = self.temperatures.len() as f32;
        let sum: f32 = self.temperatures.iter().sum();
        let mean = sum / count;

        let variance: f32 = self
            .temperatures
            .iter()
            .map(|&t| (t - mean).powi(2))
            .sum::<f32>()
            / count;

        ThermalStatistics {
            min: self.temp_range.0,
            max: self.temp_range.1,
            mean,
            std_dev: variance.sqrt(),
        }
    }
}

/// Thermal color mapping schemes
#[derive(Clone, Copy, Debug)]
pub enum ThermalColorMap {
    /// Iron/hot metal color map (black-red-yellow-white)
    Iron,
    /// Rainbow color map (blue-green-yellow-red)
    Rainbow,
    /// Grayscale (cold=black, hot=white)
    Grayscale,
    /// White hot (inverted grayscale)
    WhiteHot,
    /// Black hot
    BlackHot,
}

impl ThermalColorMap {
    /// Map normalized temperature (0.0-1.0) to RGB color (0.0-1.0)
    pub fn map(&self, normalized: f32) -> (f32, f32, f32) {
        match self {
            Self::Iron => Self::iron_map(normalized),
            Self::Rainbow => Self::rainbow_map(normalized),
            Self::Grayscale => (normalized, normalized, normalized),
            Self::WhiteHot => (normalized, normalized, normalized),
            Self::BlackHot => {
                let inv = 1.0 - normalized;
                (inv, inv, inv)
            }
        }
    }

    fn iron_map(t: f32) -> (f32, f32, f32) {
        let r = (t * 3.0 - 1.0).clamp(0.0, 1.0);
        let g = (t * 3.0 - 2.0).clamp(0.0, 1.0);
        let b = (t * 4.0 - 3.0).clamp(0.0, 1.0);
        (r, g, b)
    }

    fn rainbow_map(t: f32) -> (f32, f32, f32) {
        let r = (4.0 * t - 1.5).abs().min(1.5).min(1.0);
        let g = (4.0 * t - 0.5).abs().min(1.5).min(1.0);
        let b = (4.0 * t + 0.5).abs().min(1.5).min(1.0);
        (r, g, b)
    }
}

/// Thermal image statistics
#[derive(Clone, Debug, Default)]
pub struct ThermalStatistics {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub std_dev: f32,
}

/// System to update thermal cameras
pub fn thermal_camera_update_system(
    time: Res<Time>,
    mut cameras: Query<(&mut ThermalCamera, &mut ThermalImage, &GlobalTransform)>,
    thermal_objects: Query<(&Temperature, &ThermalProperties, &GlobalTransform)>,
) {
    let current_time = time.elapsed_secs();
    let mut rng = rand::thread_rng();

    for (mut camera, mut image, camera_transform) in cameras.iter_mut() {
        if !camera.should_update(current_time) {
            continue;
        }

        camera.last_update = current_time;
        image.timestamp = current_time;

        // Reset image to ambient temperature
        let ambient_temp = 273.15 + 20.0; // 20°C
        for pixel in image.temperatures.iter_mut() {
            *pixel = ambient_temp;
        }

        let camera_pos = camera_transform.translation();
        let camera_rot = camera_transform.to_scale_rotation_translation().1;

        // Simple placeholder: project thermal objects onto image plane
        // In production, this would use GPU raycasting or rasterization
        for (temperature, properties, object_transform) in thermal_objects.iter() {
            let object_pos = object_transform.translation();
            let relative_pos = object_pos - camera_pos;
            let distance = relative_pos.length();

            if distance < camera.near || distance > camera.far {
                continue;
            }

            // Transform to camera local space
            let local_pos = camera_rot.inverse() * relative_pos;

            // Simple perspective projection
            if local_pos.z > 0.0 {
                let fov_rad = camera.fov.to_radians();
                let aspect = camera.resolution.0 as f32 / camera.resolution.1 as f32;

                let ndc_x = local_pos.x / (local_pos.z * (fov_rad / 2.0).tan());
                let ndc_y = local_pos.y / (local_pos.z * (fov_rad / 2.0).tan() / aspect);

                if ndc_x.abs() <= 1.0 && ndc_y.abs() <= 1.0 {
                    let pixel_x = ((ndc_x + 1.0) * 0.5 * camera.resolution.0 as f32) as u32;
                    let pixel_y = ((1.0 - ndc_y) * 0.5 * camera.resolution.1 as f32) as u32;

                    if pixel_x < camera.resolution.0 && pixel_y < camera.resolution.1 {
                        // Calculate apparent temperature with atmospheric effects
                        let attenuation = camera.atmospheric_transmission.powf(distance / 100.0);
                        let apparent_temp = ambient_temp
                            + (temperature.kelvin - ambient_temp)
                                * properties.emissivity
                                * attenuation;

                        // Add sensor noise (NETD)
                        let noise = rng.gen_range(-camera.netd..camera.netd);
                        let measured_temp = apparent_temp + noise;

                        image.set_temperature(pixel_x, pixel_y, measured_temp);
                    }
                }
            }
        }

        image.update_range();
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
        let temp = Temperature::from_celsius(20.0);
        assert!((temp.kelvin - 293.15).abs() < 0.01);
        assert!((temp.celsius() - 20.0).abs() < 0.01);
        assert!((temp.fahrenheit() - 68.0).abs() < 0.1);
    }

    #[test]
    fn test_temperature_fahrenheit() {
        let temp = Temperature::from_fahrenheit(32.0); // Freezing point
        assert!((temp.celsius() - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_thermal_properties() {
        let metal = ThermalProperties::metal();
        let matte = ThermalProperties::matte();

        assert!(metal.emissivity < matte.emissivity);
        assert!(metal.reflectivity > matte.reflectivity);
    }

    #[test]
    fn test_thermal_image() {
        let mut image = ThermalImage::new(10, 10);

        image.set_temperature(5, 5, 350.0);
        assert_eq!(image.get_temperature(5, 5), Some(350.0));
        assert_eq!(image.get_temperature(15, 15), None);

        image.update_range();
        assert!(image.temp_range.1 >= 350.0);
    }

    #[test]
    fn test_thermal_color_maps() {
        let iron = ThermalColorMap::Iron;
        let rainbow = ThermalColorMap::Rainbow;

        let cold = iron.map(0.0);
        let hot = iron.map(1.0);

        // Cold should be darker than hot
        let cold_brightness = cold.0 + cold.1 + cold.2;
        let hot_brightness = hot.0 + hot.1 + hot.2;
        assert!(hot_brightness > cold_brightness);

        // Test rainbow produces different colors
        let color1 = rainbow.map(0.0);
        let color2 = rainbow.map(0.5);
        assert_ne!(color1, color2);
    }

    #[test]
    fn test_thermal_statistics() {
        let mut image = ThermalImage::new(10, 10);

        for i in 0..100 {
            image.temperatures[i] = 273.15 + (i as f32);
        }

        image.update_range();
        let stats = image.get_statistics();

        assert_eq!(stats.min, 273.15);
        assert_eq!(stats.max, 273.15 + 99.0);
        assert!((stats.mean - (273.15 + 49.5)).abs() < 0.1);
    }

    #[test]
    fn test_color_image_conversion() {
        let mut image = ThermalImage::new(2, 2);
        image.temperatures = vec![273.15, 300.0, 350.0, 400.0];
        image.update_range();

        let rgb = image.to_color_image(ThermalColorMap::Grayscale);
        assert_eq!(rgb.len(), 12); // 2x2 pixels * 3 channels

        // First pixel should be darkest (coldest)
        assert!(rgb[0] <= rgb[6]); // Compare R channel of first and third pixel
    }

    #[test]
    fn test_blackbody_emissivity() {
        let blackbody = ThermalProperties::blackbody();
        assert_eq!(blackbody.emissivity, 1.0);
        assert_eq!(blackbody.reflectivity, 0.0);
    }
}
