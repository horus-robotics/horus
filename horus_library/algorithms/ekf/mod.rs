//! Extended Kalman Filter (EKF) for Robot Localization
//!
//! State estimation using sensor fusion with nonlinear motion and measurement models.
//!
//! # Features
//!
//! - 2D pose estimation (x, y, theta)
//! - Velocity state (vx, vy, omega)
//! - Prediction and update steps
//! - Configurable process and measurement noise
//! - Handles nonlinear motion models
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::ekf::EKF;
//!
//! let mut ekf = EKF::new();
//!
//! // Set initial state
//! ekf.set_state([0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);  // x, y, theta, vx, vy, omega
//!
//! // Prediction step (time update)
//! ekf.predict(0.01);  // dt = 10ms
//!
//! // Measurement update (odometry)
//! ekf.update_odometry([1.0, 0.5, 0.1]);  // x, y, theta measurements
//! ```

/// Extended Kalman Filter for 2D Robot Localization
///
/// State vector: [x, y, theta, vx, vy, omega]
pub struct EKF {
    /// State vector: [x, y, theta, vx, vy, omega]
    state: [f64; 6],

    /// State covariance matrix (6x6)
    covariance: [[f64; 6]; 6],

    /// Process noise covariance (6x6)
    process_noise: [[f64; 6]; 6],

    /// Measurement noise covariance for odometry (3x3)
    odometry_noise: [[f64; 3]; 3],
}

impl EKF {
    /// Create new EKF with default parameters
    pub fn new() -> Self {
        let mut ekf = Self {
            state: [0.0; 6],
            covariance: [[0.0; 6]; 6],
            process_noise: [[0.0; 6]; 6],
            odometry_noise: [[0.0; 3]; 3],
        };

        // Initialize covariance (high initial uncertainty)
        for i in 0..6 {
            ekf.covariance[i][i] = 1.0;
        }

        // Initialize process noise
        ekf.process_noise[0][0] = 0.1; // x
        ekf.process_noise[1][1] = 0.1; // y
        ekf.process_noise[2][2] = 0.05; // theta
        ekf.process_noise[3][3] = 0.2; // vx
        ekf.process_noise[4][4] = 0.2; // vy
        ekf.process_noise[5][5] = 0.1; // omega

        // Initialize odometry noise
        ekf.odometry_noise[0][0] = 0.05; // x measurement
        ekf.odometry_noise[1][1] = 0.05; // y measurement
        ekf.odometry_noise[2][2] = 0.02; // theta measurement

        ekf
    }

    /// Set state vector
    pub fn set_state(&mut self, state: [f64; 6]) {
        self.state = state;
    }

    /// Get state vector
    pub fn get_state(&self) -> [f64; 6] {
        self.state
    }

    /// Get pose (x, y, theta)
    pub fn get_pose(&self) -> (f64, f64, f64) {
        (self.state[0], self.state[1], self.state[2])
    }

    /// Get velocity (vx, vy, omega)
    pub fn get_velocity(&self) -> (f64, f64, f64) {
        (self.state[3], self.state[4], self.state[5])
    }

    /// Get position uncertainty (std dev)
    pub fn get_position_uncertainty(&self) -> f64 {
        (self.covariance[0][0] + self.covariance[1][1]).sqrt()
    }

    /// Set covariance matrix
    pub fn set_covariance(&mut self, covariance: [[f64; 6]; 6]) {
        self.covariance = covariance;
    }

    /// Get covariance matrix
    pub fn get_covariance(&self) -> [[f64; 6]; 6] {
        self.covariance
    }

    /// Set process noise covariance
    pub fn set_process_noise(&mut self, noise: [[f64; 6]; 6]) {
        self.process_noise = noise;
    }

    /// Set odometry measurement noise
    pub fn set_odometry_noise(&mut self, noise: [[f64; 3]; 3]) {
        self.odometry_noise = noise;
    }

    /// Prediction step (time update)
    ///
    /// Uses kinematic motion model:
    /// x(k+1) = x(k) + vx*dt
    /// y(k+1) = y(k) + vy*dt
    /// theta(k+1) = theta(k) + omega*dt
    pub fn predict(&mut self, dt: f64) {
        // Predict state using motion model
        let old_state = self.state;

        self.state[0] += old_state[3] * dt; // x += vx * dt
        self.state[1] += old_state[4] * dt; // y += vy * dt
        self.state[2] += old_state[5] * dt; // theta += omega * dt

        // Normalize angle
        self.state[2] = normalize_angle(self.state[2]);

        // Velocities remain constant
        // state[3], state[4], state[5] unchanged

        // Predict covariance: P = P + Q*dt
        for i in 0..6 {
            for j in 0..6 {
                self.covariance[i][j] += self.process_noise[i][j] * dt;
            }
        }
    }

    /// Update step with odometry measurement
    ///
    /// Measurement: [x, y, theta]
    pub fn update_odometry(&mut self, measurement: [f64; 3]) {
        // Measurement prediction (expected measurement from state)
        let predicted = [self.state[0], self.state[1], self.state[2]];

        // Innovation (measurement residual)
        let mut innovation = [0.0; 3];
        for i in 0..3 {
            innovation[i] = measurement[i] - predicted[i];
        }

        // Normalize angle innovation
        innovation[2] = normalize_angle(innovation[2]);

        // Measurement Jacobian H (observation model derivative)
        // For direct position measurement: H = [I_3x3 | 0_3x3]
        let mut h = [[0.0; 6]; 3];
        h[0][0] = 1.0;
        h[1][1] = 1.0;
        h[2][2] = 1.0;

        // Innovation covariance: S = H*P*H' + R
        let mut s = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                s[i][j] = self.odometry_noise[i][j];
                for k in 0..6 {
                    s[i][j] += h[i][k] * self.covariance[k][j];
                }
            }
        }

        // Kalman gain: K = P*H' * inv(S)
        let s_inv = invert_3x3(s);
        let mut kalman_gain = [[0.0; 3]; 6];
        for i in 0..6 {
            for j in 0..3 {
                for k in 0..3 {
                    kalman_gain[i][j] += self.covariance[i][k] * h[k][j] * s_inv[k][j];
                }
            }
        }

        // State update: x = x + K*innovation
        for i in 0..6 {
            for j in 0..3 {
                self.state[i] += kalman_gain[i][j] * innovation[j];
            }
        }

        // Normalize angle
        self.state[2] = normalize_angle(self.state[2]);

        // Covariance update: P = (I - K*H) * P
        let mut i_kh = [[0.0; 6]; 6];
        for i in 0..6 {
            i_kh[i][i] = 1.0;
            for j in 0..6 {
                for k in 0..3 {
                    i_kh[i][j] -= kalman_gain[i][k] * h[k][j];
                }
            }
        }

        let old_cov = self.covariance;
        for i in 0..6 {
            for j in 0..6 {
                self.covariance[i][j] = 0.0;
                for k in 0..6 {
                    self.covariance[i][j] += i_kh[i][k] * old_cov[k][j];
                }
            }
        }
    }

    /// Reset EKF to initial state
    pub fn reset(&mut self) {
        self.state = [0.0; 6];

        for i in 0..6 {
            for j in 0..6 {
                self.covariance[i][j] = if i == j { 1.0 } else { 0.0 };
            }
        }
    }
}

impl Default for EKF {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize angle to [-π, π]
fn normalize_angle(angle: f64) -> f64 {
    let mut a = angle;
    while a > std::f64::consts::PI {
        a -= 2.0 * std::f64::consts::PI;
    }
    while a < -std::f64::consts::PI {
        a += 2.0 * std::f64::consts::PI;
    }
    a
}

/// Invert 3x3 matrix (simplified for covariance matrices)
fn invert_3x3(m: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-10 {
        // Return identity if not invertible
        return [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    }

    let inv_det = 1.0 / det;

    [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let ekf = EKF::new();
        let state = ekf.get_state();

        // Initial state should be zero
        for &s in &state {
            assert_eq!(s, 0.0);
        }
    }

    #[test]
    fn test_prediction() {
        let mut ekf = EKF::new();
        ekf.set_state([0.0, 0.0, 0.0, 1.0, 0.0, 0.0]); // vx = 1.0 m/s

        ekf.predict(1.0); // 1 second

        let (x, y, _theta) = ekf.get_pose();

        // Should move 1 meter in x direction
        assert!((x - 1.0).abs() < 0.01);
        assert!(y.abs() < 0.01);
    }

    #[test]
    fn test_odometry_update() {
        let mut ekf = EKF::new();

        // Predict some movement
        ekf.set_state([0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        ekf.predict(1.0);

        // Update with measurement
        ekf.update_odometry([1.0, 0.0, 0.0]);

        let (x, y, theta) = ekf.get_pose();

        // Should be close to measurement
        assert!((x - 1.0).abs() < 0.1);
        assert!(y.abs() < 0.1);
        assert!(theta.abs() < 0.1);
    }

    #[test]
    fn test_uncertainty_growth() {
        let mut ekf = EKF::new();

        let initial_uncertainty = ekf.get_position_uncertainty();

        // Predict without measurement
        for _ in 0..10 {
            ekf.predict(0.1);
        }

        let final_uncertainty = ekf.get_position_uncertainty();

        // Uncertainty should increase
        assert!(final_uncertainty > initial_uncertainty);
    }

    #[test]
    fn test_measurement_reduces_uncertainty() {
        let mut ekf = EKF::new();

        // Build up uncertainty
        for _ in 0..10 {
            ekf.predict(0.1);
        }

        let uncertainty_before = ekf.get_position_uncertainty();

        // Measurement should reduce uncertainty
        ekf.update_odometry([0.5, 0.5, 0.0]);

        let uncertainty_after = ekf.get_position_uncertainty();

        assert!(uncertainty_after < uncertainty_before);
    }

    #[test]
    fn test_angle_normalization() {
        assert!(
            (normalize_angle(3.5 * std::f64::consts::PI) + std::f64::consts::PI / 2.0).abs() < 0.01
        );
        assert!(
            (normalize_angle(-3.5 * std::f64::consts::PI) - std::f64::consts::PI / 2.0).abs()
                < 0.01
        );
    }

    #[test]
    fn test_reset() {
        let mut ekf = EKF::new();

        ekf.set_state([1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        ekf.predict(1.0);

        ekf.reset();

        let state = ekf.get_state();
        for &s in &state {
            assert_eq!(s, 0.0);
        }
    }

    #[test]
    fn test_rotation() {
        let mut ekf = EKF::new();
        ekf.set_state([0.0, 0.0, 0.0, 0.0, 0.0, 1.0]); // omega = 1 rad/s

        ekf.predict(std::f64::consts::PI / 2.0); // 90 degrees

        let (_x, _y, theta) = ekf.get_pose();

        assert!((theta - std::f64::consts::PI / 2.0).abs() < 0.01);
    }
}
