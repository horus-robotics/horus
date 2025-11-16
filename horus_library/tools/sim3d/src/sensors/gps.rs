use bevy::prelude::*;
use rand::Rng;
use std::collections::VecDeque;

/// Position measurement with timestamp
#[derive(Clone, Debug)]
struct PositionSample {
    timestamp: f32,
    position: Vec3,
}

/// Velocity smoothing method
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VelocitySmoothingMethod {
    /// Simple finite difference (v = Δp / Δt)
    SimpleDifference,
    /// Weighted average over multiple samples (more recent = higher weight)
    WeightedAverage,
    /// Linear regression for smoother estimates
    LinearRegression,
}

/// GPS sensor component
#[derive(Component, Clone)]
pub struct GPS {
    pub rate_hz: f32,
    pub last_update: f32,

    // Noise parameters (in meters)
    pub horizontal_noise_std: f32,
    pub vertical_noise_std: f32,

    // Bias/drift (simulates atmospheric effects, satellite geometry)
    pub horizontal_bias: Vec2,
    pub vertical_bias: f32,

    // Bias drift rate (meters per second)
    pub bias_drift_rate: f32,

    // Accuracy parameters
    pub min_satellites: u8,
    pub current_satellites: u8,
    pub hdop: f32, // Horizontal Dilution of Precision
    pub vdop: f32, // Vertical Dilution of Precision

    // Position history for velocity computation
    position_history: VecDeque<PositionSample>,
    history_size: usize,

    // Velocity computation parameters
    pub velocity_smoothing: VelocitySmoothingMethod,
    pub min_samples_for_velocity: usize,
}

impl Default for GPS {
    fn default() -> Self {
        Self {
            rate_hz: 10.0,
            last_update: 0.0,
            horizontal_noise_std: 2.5, // Typical consumer GPS: 2.5m CEP
            vertical_noise_std: 5.0,   // Vertical typically worse
            horizontal_bias: Vec2::ZERO,
            vertical_bias: 0.0,
            bias_drift_rate: 0.01, // 1cm/s drift
            min_satellites: 4,
            current_satellites: 8,
            hdop: 1.2,
            vdop: 1.8,
            position_history: VecDeque::new(),
            history_size: 10, // Store last 10 samples (1 second at 10Hz)
            velocity_smoothing: VelocitySmoothingMethod::WeightedAverage,
            min_samples_for_velocity: 2,
        }
    }
}

impl GPS {
    pub fn new(rate_hz: f32) -> Self {
        Self {
            rate_hz,
            ..default()
        }
    }

    /// High accuracy GPS (RTK, DGPS)
    pub fn high_accuracy() -> Self {
        Self {
            horizontal_noise_std: 0.02, // 2cm RTK
            vertical_noise_std: 0.03,
            bias_drift_rate: 0.001,
            hdop: 0.8,
            vdop: 1.0,
            ..default()
        }
    }

    /// Consumer-grade GPS
    pub fn consumer_grade() -> Self {
        Self {
            horizontal_noise_std: 5.0, // 5m typical
            vertical_noise_std: 10.0,
            bias_drift_rate: 0.05,
            hdop: 2.0,
            vdop: 3.0,
            ..default()
        }
    }

    /// Low-quality GPS (urban canyon, poor satellite visibility)
    pub fn low_quality() -> Self {
        Self {
            horizontal_noise_std: 15.0,
            vertical_noise_std: 30.0,
            bias_drift_rate: 0.2,
            current_satellites: 4,
            hdop: 4.5,
            vdop: 6.0,
            ..default()
        }
    }

    pub fn with_noise(mut self, horizontal: f32, vertical: f32) -> Self {
        self.horizontal_noise_std = horizontal;
        self.vertical_noise_std = vertical;
        self
    }

    pub fn with_bias_drift(mut self, drift_rate: f32) -> Self {
        self.bias_drift_rate = drift_rate;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn update_time(&mut self, current_time: f32) {
        self.last_update = current_time;
    }

    /// Update bias drift over time
    pub fn update_bias(&mut self, dt: f32) {
        let mut rng = rand::thread_rng();

        // Random walk for bias drift
        self.horizontal_bias.x += rng.gen_range(-self.bias_drift_rate..self.bias_drift_rate) * dt;
        self.horizontal_bias.y += rng.gen_range(-self.bias_drift_rate..self.bias_drift_rate) * dt;
        self.vertical_bias += rng.gen_range(-self.bias_drift_rate..self.bias_drift_rate) * dt;

        // Clamp bias to reasonable limits (prevents runaway drift)
        let max_bias = self.horizontal_noise_std * 3.0;
        self.horizontal_bias.x = self.horizontal_bias.x.clamp(-max_bias, max_bias);
        self.horizontal_bias.y = self.horizontal_bias.y.clamp(-max_bias, max_bias);
        self.vertical_bias = self.vertical_bias.clamp(-max_bias * 2.0, max_bias * 2.0);
    }

    /// Simulate satellite availability changes
    pub fn update_satellite_count(&mut self) {
        let mut rng = rand::thread_rng();

        // Slowly vary satellite count (changes every few seconds typically)
        if rng.gen_bool(0.01) {
            let delta = if rng.gen_bool(0.5) { 1 } else { -1 };
            self.current_satellites = (self.current_satellites as i8 + delta)
                .clamp(self.min_satellites as i8, 12) as u8;
        }
    }

    /// Check if GPS has valid fix
    pub fn has_fix(&self) -> bool {
        self.current_satellites >= self.min_satellites
    }

    /// Get position quality indicator (0.0 = poor, 1.0 = excellent)
    pub fn quality(&self) -> f32 {
        if !self.has_fix() {
            return 0.0;
        }

        // Quality based on HDOP and satellite count
        let hdop_quality = (5.0 - self.hdop).max(0.0) / 5.0;
        let sat_quality = (self.current_satellites.saturating_sub(self.min_satellites) as f32) / 8.0;

        (hdop_quality * 0.6 + sat_quality * 0.4).min(1.0)
    }

    /// Add position sample to history
    pub fn add_position_sample(&mut self, timestamp: f32, position: Vec3) {
        self.position_history.push_back(PositionSample {
            timestamp,
            position,
        });

        // Remove old samples beyond history size
        while self.position_history.len() > self.history_size {
            self.position_history.pop_front();
        }
    }

    /// Compute velocity from position history
    pub fn compute_velocity(&self) -> Option<Vec3> {
        if self.position_history.len() < self.min_samples_for_velocity {
            return None;
        }

        match self.velocity_smoothing {
            VelocitySmoothingMethod::SimpleDifference => self.velocity_simple_difference(),
            VelocitySmoothingMethod::WeightedAverage => self.velocity_weighted_average(),
            VelocitySmoothingMethod::LinearRegression => self.velocity_linear_regression(),
        }
    }

    /// Simple finite difference: v = (p_last - p_first) / (t_last - t_first)
    fn velocity_simple_difference(&self) -> Option<Vec3> {
        let first = self.position_history.front()?;
        let last = self.position_history.back()?;

        let dt = last.timestamp - first.timestamp;
        if dt < 1e-6 {
            return None;
        }

        let dp = last.position - first.position;
        Some(dp / dt)
    }

    /// Weighted average of velocity estimates (more recent samples have higher weight)
    fn velocity_weighted_average(&self) -> Option<Vec3> {
        if self.position_history.len() < 2 {
            return None;
        }

        let mut weighted_velocity = Vec3::ZERO;
        let mut total_weight = 0.0;

        // Compute pairwise velocities with exponential weighting
        for i in 1..self.position_history.len() {
            let prev = &self.position_history[i - 1];
            let curr = &self.position_history[i];

            let dt = curr.timestamp - prev.timestamp;
            if dt < 1e-6 {
                continue;
            }

            let velocity = (curr.position - prev.position) / dt;

            // Exponential weight: more recent samples get higher weight
            // weight = exp(k * normalized_time) where normalized_time ∈ [0, 1]
            let normalized_time = i as f32 / (self.position_history.len() - 1) as f32;
            let weight = (2.0 * normalized_time).exp(); // e^0 = 1.0 to e^2 ≈ 7.4

            weighted_velocity += velocity * weight;
            total_weight += weight;
        }

        if total_weight > 1e-6 {
            Some(weighted_velocity / total_weight)
        } else {
            None
        }
    }

    /// Linear regression: fit line to position vs time, slope = velocity
    fn velocity_linear_regression(&self) -> Option<Vec3> {
        let n = self.position_history.len();
        if n < 2 {
            return None;
        }

        // Use first timestamp as reference (improve numerical stability)
        let t0 = self.position_history[0].timestamp;

        // Compute means
        let mut mean_t = 0.0;
        let mut mean_p = Vec3::ZERO;

        for sample in &self.position_history {
            let t_rel = sample.timestamp - t0;
            mean_t += t_rel;
            mean_p += sample.position;
        }
        mean_t /= n as f32;
        mean_p /= n as f32;

        // Compute slope (velocity) for each component using least squares
        // slope = Σ((t - mean_t) * (p - mean_p)) / Σ((t - mean_t)²)
        let mut numerator = Vec3::ZERO;
        let mut denominator = 0.0;

        for sample in &self.position_history {
            let t_rel = sample.timestamp - t0;
            let t_diff = t_rel - mean_t;
            let p_diff = sample.position - mean_p;

            numerator += p_diff * t_diff;
            denominator += t_diff * t_diff;
        }

        if denominator > 1e-6 {
            Some(numerator / denominator)
        } else {
            None
        }
    }

    /// Compute velocity covariance from position covariance
    pub fn compute_velocity_covariance(&self, position_std: f32) -> Vec<f64> {
        if self.position_history.len() < 2 {
            // No velocity estimate possible
            return vec![f64::MAX; 9];
        }

        // Velocity covariance propagation using finite differences
        // For v = Δp / Δt, var(v) ≈ 2 * var(p) / Δt²
        // (factor of 2 because we're differencing two noisy measurements)

        let first = self.position_history.front().unwrap();
        let last = self.position_history.back().unwrap();
        let dt = (last.timestamp - first.timestamp) as f64;

        if dt < 1e-6 {
            return vec![f64::MAX; 9];
        }

        let position_var = (position_std * position_std) as f64;
        let velocity_var = 2.0 * position_var / (dt * dt);

        // Diagonal covariance matrix
        vec![
            velocity_var, 0.0, 0.0,
            0.0, velocity_var, 0.0,
            0.0, 0.0, velocity_var,
        ]
    }

    /// Clear position history (e.g., after GPS dropout)
    pub fn clear_history(&mut self) {
        self.position_history.clear();
    }

    /// Get number of samples in history
    pub fn history_count(&self) -> usize {
        self.position_history.len()
    }
}

/// GPS data output
#[derive(Component, Clone, Debug)]
pub struct GPSData {
    pub timestamp: f32,

    // Position in world coordinates (or could be lat/lon if needed)
    pub position: Vec3,

    // Position covariance (diagonal: [x, y, z])
    pub position_covariance: Vec<f64>,

    // GPS metadata
    pub satellites_visible: u8,
    pub hdop: f32,
    pub vdop: f32,
    pub fix_quality: u8, // 0=no fix, 1=GPS, 2=DGPS, 3=PPS, 4=RTK, 5=Float RTK

    // Velocity (computed from position history)
    pub velocity: Option<Vec3>,

    // Velocity covariance (diagonal covariance matrix)
    pub velocity_covariance: Vec<f64>,
}

impl Default for GPSData {
    fn default() -> Self {
        Self {
            timestamp: 0.0,
            position: Vec3::ZERO,
            position_covariance: vec![0.0; 9],
            satellites_visible: 0,
            hdop: 99.9,
            vdop: 99.9,
            fix_quality: 0,
            velocity: None,
            velocity_covariance: vec![0.0; 9],
        }
    }
}

impl GPSData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_valid_fix(&self) -> bool {
        self.fix_quality > 0
    }
}

/// System to update GPS sensors
pub fn gps_update_system(
    time: Res<Time>,
    mut query: Query<(&mut GPS, &mut GPSData, &GlobalTransform)>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    for (mut gps, mut gps_data, transform) in query.iter_mut() {
        if !gps.should_update(current_time) {
            continue;
        }

        gps.update_time(current_time);
        gps.update_bias(dt);
        gps.update_satellite_count();

        // Get true position
        let true_position = transform.translation();

        if !gps.has_fix() {
            // No fix - no position update
            gps_data.fix_quality = 0;
            gps_data.satellites_visible = gps.current_satellites;
            gps_data.hdop = 99.9;
            gps_data.vdop = 99.9;
            gps_data.velocity = None;
            gps_data.velocity_covariance = vec![f64::MAX; 9];

            // Clear position history on fix loss
            gps.clear_history();
            continue;
        }

        // Add noise and bias
        let mut rng = rand::thread_rng();

        let noise_x = rng.gen_range(-gps.horizontal_noise_std..gps.horizontal_noise_std);
        let noise_y = rng.gen_range(-gps.horizontal_noise_std..gps.horizontal_noise_std);
        let noise_z = rng.gen_range(-gps.vertical_noise_std..gps.vertical_noise_std);

        gps_data.position = Vec3::new(
            true_position.x + noise_x + gps.horizontal_bias.x,
            true_position.y + noise_z + gps.vertical_bias,
            true_position.z + noise_y + gps.horizontal_bias.y,
        );

        // Set covariance matrix (diagonal)
        gps_data.position_covariance = vec![
            (gps.horizontal_noise_std * gps.horizontal_noise_std) as f64, 0.0, 0.0,
            0.0, (gps.vertical_noise_std * gps.vertical_noise_std) as f64, 0.0,
            0.0, 0.0, (gps.horizontal_noise_std * gps.horizontal_noise_std) as f64,
        ];

        // Update metadata
        gps_data.satellites_visible = gps.current_satellites;
        gps_data.hdop = gps.hdop;
        gps_data.vdop = gps.vdop;
        gps_data.timestamp = current_time;

        // Determine fix quality
        gps_data.fix_quality = if gps.horizontal_noise_std < 0.1 {
            4 // RTK
        } else if gps.horizontal_noise_std < 1.0 {
            2 // DGPS
        } else {
            1 // Standard GPS
        };

        // Add current position to history for velocity computation
        gps.add_position_sample(current_time, gps_data.position);

        // Compute velocity from position history
        gps_data.velocity = gps.compute_velocity();

        // Compute velocity covariance if velocity is available
        if gps_data.velocity.is_some() {
            // Use horizontal noise std for covariance computation
            gps_data.velocity_covariance = gps.compute_velocity_covariance(gps.horizontal_noise_std);
        } else {
            // No velocity estimate - set large covariance
            gps_data.velocity_covariance = vec![f64::MAX; 9];
        }
    }
}

/// GPS visualization system (draws GPS uncertainty circle)
pub fn visualize_gps_system(
    mut gizmos: Gizmos,
    query: Query<(&GPS, &GPSData, &GlobalTransform)>,
) {
    for (gps, gps_data, transform) in query.iter() {
        if !gps_data.has_valid_fix() {
            continue;
        }

        let pos = transform.translation();

        // Draw uncertainty circle (2-sigma = 95% confidence)
        let radius = gps.horizontal_noise_std * 2.0;
        let color = if gps.quality() > 0.7 {
            Color::srgb(0.0, 1.0, 0.0) // Good quality = green
        } else if gps.quality() > 0.4 {
            Color::srgb(1.0, 1.0, 0.0) // Medium = yellow
        } else {
            Color::srgb(1.0, 0.0, 0.0) // Poor = red
        };

        // Draw horizontal uncertainty circle (lying flat in XZ plane)
        use bevy::math::Isometry3d;
        let isometry = Isometry3d::new(pos, Quat::IDENTITY);
        gizmos.circle(isometry, radius, color);

        // Draw vertical uncertainty line
        let vradius = gps.vertical_noise_std * 2.0;
        gizmos.line(
            pos + Vec3::Y * vradius,
            pos - Vec3::Y * vradius,
            color,
        );

        // Draw measured position
        gizmos.sphere(gps_data.position, 0.1, color);

        // Draw line from true to measured
        gizmos.line(pos, gps_data.position, Color::srgba(1.0, 1.0, 1.0, 0.3));
    }
}

/// Coordinate conversion utilities (if needed for lat/lon)
pub mod coordinate_conversion {
    use super::*;

    /// Convert world position to GPS coordinates (simplified)
    /// In a real system, you'd use a proper geodetic conversion
    pub fn world_to_gps(world_pos: Vec3, origin_lat: f64, origin_lon: f64) -> (f64, f64, f64) {
        // Very simplified conversion (flat earth approximation)
        // 1 degree latitude ≈ 111 km
        // 1 degree longitude ≈ 111 km * cos(latitude)

        let meters_per_degree_lat = 111_000.0;
        let meters_per_degree_lon = 111_000.0 * origin_lat.to_radians().cos();

        let lat = origin_lat + (world_pos.z as f64) / meters_per_degree_lat;
        let lon = origin_lon + (world_pos.x as f64) / meters_per_degree_lon;
        let alt = world_pos.y as f64;

        (lat, lon, alt)
    }

    /// Convert GPS coordinates to world position
    pub fn gps_to_world(lat: f64, lon: f64, alt: f64, origin_lat: f64, origin_lon: f64) -> Vec3 {
        let meters_per_degree_lat = 111_000.0;
        let meters_per_degree_lon = 111_000.0 * origin_lat.to_radians().cos();

        let x = ((lon - origin_lon) * meters_per_degree_lon) as f32;
        let y = alt as f32;
        let z = ((lat - origin_lat) * meters_per_degree_lat) as f32;

        Vec3::new(x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gps_quality() {
        let gps = GPS::high_accuracy();
        assert!(gps.quality() > 0.7);

        let gps = GPS::low_quality();
        assert!(gps.quality() < 0.5);
    }

    #[test]
    fn test_gps_fix() {
        let mut gps = GPS::default();
        assert!(gps.has_fix());

        gps.current_satellites = 3;
        assert!(!gps.has_fix());
    }

    #[test]
    fn test_coordinate_conversion() {
        let origin = (37.7749, -122.4194); // San Francisco
        let world_pos = Vec3::new(1000.0, 10.0, 1000.0);

        let (lat, lon, alt) = coordinate_conversion::world_to_gps(
            world_pos,
            origin.0,
            origin.1,
        );

        let back = coordinate_conversion::gps_to_world(lat, lon, alt, origin.0, origin.1);

        assert!((back - world_pos).length() < 1.0); // Within 1m error
    }

    #[test]
    fn test_velocity_simple_difference() {
        let mut gps = GPS::default();
        gps.velocity_smoothing = VelocitySmoothingMethod::SimpleDifference;

        // Add samples with constant velocity (1 m/s in X direction)
        gps.add_position_sample(0.0, Vec3::new(0.0, 0.0, 0.0));
        gps.add_position_sample(1.0, Vec3::new(1.0, 0.0, 0.0));
        gps.add_position_sample(2.0, Vec3::new(2.0, 0.0, 0.0));

        let velocity = gps.compute_velocity().unwrap();
        assert!((velocity.x - 1.0).abs() < 0.01);
        assert!(velocity.y.abs() < 0.01);
        assert!(velocity.z.abs() < 0.01);
    }

    #[test]
    fn test_velocity_weighted_average() {
        let mut gps = GPS::default();
        gps.velocity_smoothing = VelocitySmoothingMethod::WeightedAverage;

        // Add samples with constant velocity
        for i in 0..5 {
            let t = i as f32 * 0.1;
            let pos = Vec3::new(t * 2.0, 0.0, 0.0); // 2 m/s
            gps.add_position_sample(t, pos);
        }

        let velocity = gps.compute_velocity().unwrap();
        // Should be close to 2.0 m/s
        assert!((velocity.x - 2.0).abs() < 0.2);
    }

    #[test]
    fn test_velocity_linear_regression() {
        let mut gps = GPS::default();
        gps.velocity_smoothing = VelocitySmoothingMethod::LinearRegression;

        // Add samples with constant velocity plus noise
        for i in 0..10 {
            let t = i as f32 * 0.1;
            let pos = Vec3::new(t * 1.5, 0.0, 0.0); // 1.5 m/s
            gps.add_position_sample(t, pos);
        }

        let velocity = gps.compute_velocity().unwrap();
        // Linear regression should give accurate estimate
        assert!((velocity.x - 1.5).abs() < 0.05);
    }

    #[test]
    fn test_velocity_insufficient_samples() {
        let mut gps = GPS::default();
        gps.min_samples_for_velocity = 3;

        // Add only 2 samples
        gps.add_position_sample(0.0, Vec3::ZERO);
        gps.add_position_sample(1.0, Vec3::X);

        // Should return None (insufficient samples)
        assert!(gps.compute_velocity().is_none());
    }

    #[test]
    fn test_velocity_covariance() {
        let mut gps = GPS::default();
        gps.add_position_sample(0.0, Vec3::ZERO);
        gps.add_position_sample(1.0, Vec3::X);

        let cov = gps.compute_velocity_covariance(2.5);

        // Should have 9 elements (3x3 matrix)
        assert_eq!(cov.len(), 9);

        // Diagonal elements should be positive
        assert!(cov[0] > 0.0);
        assert!(cov[4] > 0.0);
        assert!(cov[8] > 0.0);

        // Off-diagonal should be zero (uncorrelated)
        assert_eq!(cov[1], 0.0);
        assert_eq!(cov[2], 0.0);
    }

    #[test]
    fn test_history_management() {
        let mut gps = GPS::default();
        gps.history_size = 3;

        // Add more samples than history size
        for i in 0..5 {
            gps.add_position_sample(i as f32, Vec3::new(i as f32, 0.0, 0.0));
        }

        // Should only keep last 3
        assert_eq!(gps.history_count(), 3);

        // Clear history
        gps.clear_history();
        assert_eq!(gps.history_count(), 0);
    }

    #[test]
    fn test_velocity_with_acceleration() {
        let mut gps = GPS::default();
        gps.velocity_smoothing = VelocitySmoothingMethod::LinearRegression;

        // Add samples with constant acceleration (x = 0.5 * a * t²)
        // a = 2 m/s², so x = t²
        for i in 0..10 {
            let t = i as f32 * 0.1;
            let pos = Vec3::new(t * t, 0.0, 0.0);
            gps.add_position_sample(t, pos);
        }

        let velocity = gps.compute_velocity().unwrap();

        // At t=0.9s (midpoint of 0.0-0.9), velocity should be ~1.8 m/s
        // Linear regression averages, so we expect something in that range
        assert!(velocity.x > 0.5 && velocity.x < 2.5);
    }

    #[test]
    fn test_velocity_stationary() {
        let mut gps = GPS::default();

        // Add samples at same position (stationary)
        for i in 0..5 {
            gps.add_position_sample(i as f32 * 0.1, Vec3::new(10.0, 5.0, 3.0));
        }

        let velocity = gps.compute_velocity().unwrap();

        // Velocity should be near zero
        assert!(velocity.length() < 0.01);
    }

    #[test]
    fn test_velocity_zero_time_delta() {
        let mut gps = GPS::default();

        // Add samples with same timestamp (invalid)
        gps.add_position_sample(1.0, Vec3::ZERO);
        gps.add_position_sample(1.0, Vec3::X);

        // Should return None (zero time delta)
        assert!(gps.compute_velocity().is_none());
    }

    #[test]
    fn test_velocity_methods_consistency() {
        let mut gps = GPS::default();

        // Add samples with constant velocity
        for i in 0..10 {
            let t = i as f32 * 0.1;
            gps.add_position_sample(t, Vec3::new(t * 3.0, 0.0, 0.0));
        }

        // All methods should give similar results for constant velocity
        gps.velocity_smoothing = VelocitySmoothingMethod::SimpleDifference;
        let v1 = gps.compute_velocity().unwrap();

        gps.velocity_smoothing = VelocitySmoothingMethod::WeightedAverage;
        let v2 = gps.compute_velocity().unwrap();

        gps.velocity_smoothing = VelocitySmoothingMethod::LinearRegression;
        let v3 = gps.compute_velocity().unwrap();

        // All should be close to 3.0 m/s
        assert!((v1.x - 3.0).abs() < 0.1);
        assert!((v2.x - 3.0).abs() < 0.1);
        assert!((v3.x - 3.0).abs() < 0.1);
    }
}
