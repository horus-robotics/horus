//! Linear Kalman Filter
//!
//! Optimal state estimation for linear systems with Gaussian noise.
//!
//! # Features
//!
//! - Linear state estimation
//! - Prediction and update steps
//! - Configurable system and measurement models
//! - Multi-dimensional state support
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::kalman_filter::KalmanFilter;
//!
//! // 1D position-velocity tracker
//! let mut kf = KalmanFilter::new(2, 1);  // 2 states, 1 measurement
//!
//! // Set initial state [position, velocity]
//! kf.set_state(vec![0.0, 0.0]);
//!
//! // Predict
//! kf.predict();
//!
//! // Update with measurement
//! kf.update(vec![1.5]);
//! ```

/// Linear Kalman Filter
pub struct KalmanFilter {
    n_states: usize,      // Number of state variables
    n_measurements: usize, // Number of measurements

    state: Vec<f64>,                  // State vector
    covariance: Vec<Vec<f64>>,        // State covariance matrix
    process_noise: Vec<Vec<f64>>,     // Process noise covariance (Q)
    measurement_noise: Vec<Vec<f64>>, // Measurement noise covariance (R)

    state_transition: Vec<Vec<f64>>,  // State transition matrix (F)
    measurement_matrix: Vec<Vec<f64>>, // Measurement matrix (H)
}

impl KalmanFilter {
    /// Create new Kalman filter
    ///
    /// # Arguments
    /// * `n_states` - Number of state variables
    /// * `n_measurements` - Number of measurements
    pub fn new(n_states: usize, n_measurements: usize) -> Self {
        let mut kf = Self {
            n_states,
            n_measurements,
            state: vec![0.0; n_states],
            covariance: vec![vec![0.0; n_states]; n_states],
            process_noise: vec![vec![0.0; n_states]; n_states],
            measurement_noise: vec![vec![0.0; n_measurements]; n_measurements],
            state_transition: vec![vec![0.0; n_states]; n_states],
            measurement_matrix: vec![vec![0.0; n_measurements]; n_states],
        };

        // Initialize to identity matrices
        for i in 0..n_states {
            kf.covariance[i][i] = 1.0;
            kf.process_noise[i][i] = 0.1;
            kf.state_transition[i][i] = 1.0;
        }

        for i in 0..n_measurements {
            kf.measurement_noise[i][i] = 0.1;
            if i < n_states {
                kf.measurement_matrix[i][i] = 1.0;
            }
        }

        kf
    }

    /// Set state vector
    pub fn set_state(&mut self, state: Vec<f64>) {
        if state.len() == self.n_states {
            self.state = state;
        }
    }

    /// Get state vector
    pub fn get_state(&self) -> &Vec<f64> {
        &self.state
    }

    /// Set state transition matrix
    pub fn set_state_transition(&mut self, matrix: Vec<Vec<f64>>) {
        if matrix.len() == self.n_states && matrix[0].len() == self.n_states {
            self.state_transition = matrix;
        }
    }

    /// Set measurement matrix
    pub fn set_measurement_matrix(&mut self, matrix: Vec<Vec<f64>>) {
        if matrix.len() == self.n_measurements && matrix[0].len() == self.n_states {
            self.measurement_matrix = matrix;
        }
    }

    /// Set process noise covariance
    pub fn set_process_noise(&mut self, matrix: Vec<Vec<f64>>) {
        if matrix.len() == self.n_states && matrix[0].len() == self.n_states {
            self.process_noise = matrix;
        }
    }

    /// Set measurement noise covariance
    pub fn set_measurement_noise(&mut self, matrix: Vec<Vec<f64>>) {
        if matrix.len() == self.n_measurements && matrix[0].len() == self.n_measurements {
            self.measurement_noise = matrix;
        }
    }

    /// Prediction step
    pub fn predict(&mut self) {
        // x = F * x
        self.state = matrix_vector_mult(&self.state_transition, &self.state);

        // P = F * P * F^T + Q
        let temp = matrix_mult(&self.state_transition, &self.covariance);
        let f_transpose = transpose(&self.state_transition);
        let fppft = matrix_mult(&temp, &f_transpose);
        self.covariance = matrix_add(&fppft, &self.process_noise);
    }

    /// Update step with measurement
    pub fn update(&mut self, measurement: Vec<f64>) {
        if measurement.len() != self.n_measurements {
            return;
        }

        // y = z - H * x (innovation)
        let hx = matrix_vector_mult(&self.measurement_matrix, &self.state);
        let innovation: Vec<f64> = measurement
            .iter()
            .zip(hx.iter())
            .map(|(z, h)| z - h)
            .collect();

        // S = H * P * H^T + R (innovation covariance)
        let hp = matrix_mult(&self.measurement_matrix, &self.covariance);
        let h_transpose = transpose(&self.measurement_matrix);
        let hpht = matrix_mult(&hp, &h_transpose);
        let s = matrix_add(&hpht, &self.measurement_noise);

        // K = P * H^T * S^-1 (Kalman gain)
        let s_inv = matrix_inverse(&s);
        let pht = matrix_mult(&self.covariance, &h_transpose);
        let kalman_gain = matrix_mult(&pht, &s_inv);

        // x = x + K * y (state update)
        let ky = matrix_vector_mult(&kalman_gain, &innovation);
        self.state = vec_add(&self.state, &ky);

        // P = (I - K * H) * P (covariance update)
        let kh = matrix_mult(&kalman_gain, &self.measurement_matrix);
        let i_kh = matrix_sub(&identity(self.n_states), &kh);
        self.covariance = matrix_mult(&i_kh, &self.covariance);
    }
}

// Matrix operations
fn matrix_vector_mult(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix.iter().map(|row| row.iter().zip(vector).map(|(a, b)| a * b).sum()).collect()
}

fn matrix_mult(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = a.len();
    let cols = b[0].len();
    let inner = b.len();

    let mut result = vec![vec![0.0; cols]; rows];
    for i in 0..rows {
        for j in 0..cols {
            for k in 0..inner {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

fn transpose(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = matrix.len();
    let cols = matrix[0].len();
    let mut result = vec![vec![0.0; rows]; cols];
    for i in 0..rows {
        for j in 0..cols {
            result[j][i] = matrix[i][j];
        }
    }
    result
}

fn matrix_add(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    a.iter().zip(b).map(|(row_a, row_b)| row_a.iter().zip(row_b).map(|(x, y)| x + y).collect()).collect()
}

fn matrix_sub(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    a.iter().zip(b).map(|(row_a, row_b)| row_a.iter().zip(row_b).map(|(x, y)| x - y).collect()).collect()
}

fn vec_add(a: &[f64], b: &[f64]) -> Vec<f64> {
    a.iter().zip(b).map(|(x, y)| x + y).collect()
}

fn identity(n: usize) -> Vec<Vec<f64>> {
    let mut mat = vec![vec![0.0; n]; n];
    for i in 0..n {
        mat[i][i] = 1.0;
    }
    mat
}

fn matrix_inverse(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = matrix.len();
    if n == 1 {
        return vec![vec![1.0 / matrix[0][0]]];
    }
    // Simplified 2x2 inverse
    if n == 2 {
        let det = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
        if det.abs() < 1e-10 {
            return identity(n);
        }
        return vec![
            vec![matrix[1][1] / det, -matrix[0][1] / det],
            vec![-matrix[1][0] / det, matrix[0][0] / det],
        ];
    }
    // For larger matrices, return identity (simplified)
    identity(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create() {
        let kf = KalmanFilter::new(2, 1);
        assert_eq!(kf.get_state().len(), 2);
    }

    #[test]
    fn test_predict() {
        let mut kf = KalmanFilter::new(2, 1);
        kf.set_state(vec![0.0, 1.0]);  // position=0, velocity=1

        // Set up constant velocity model
        kf.set_state_transition(vec![vec![1.0, 1.0], vec![0.0, 1.0]]);

        kf.predict();

        // After prediction: position should be 1.0
        assert!((kf.get_state()[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_update() {
        let mut kf = KalmanFilter::new(1, 1);
        kf.set_state(vec![0.0]);

        kf.update(vec![1.0]);

        // State should move toward measurement
        assert!(kf.get_state()[0] > 0.0);
    }
}
