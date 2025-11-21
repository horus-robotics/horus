//! Control algorithms for robot tasks
//!
//! Provides baseline controllers (PID, PD, etc.) for comparison with RL agents.

use std::collections::VecDeque;

/// PID Controller for continuous control
#[derive(Clone, Debug)]
pub struct PIDController {
    /// Proportional gain
    pub kp: f32,
    /// Integral gain
    pub ki: f32,
    /// Derivative gain
    pub kd: f32,

    /// Previous error
    prev_error: f32,
    /// Integral sum
    integral: f32,
    /// Error history for derivative (moving average)
    error_history: VecDeque<f32>,
    /// Maximum integral value (anti-windup)
    pub integral_max: f32,
    /// Output limits
    pub output_min: f32,
    pub output_max: f32,

    /// Sample time (seconds)
    pub dt: f32,
    /// Derivative smoothing window size
    derivative_window: usize,
}

impl PIDController {
    pub fn new(kp: f32, ki: f32, kd: f32, dt: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            prev_error: 0.0,
            integral: 0.0,
            error_history: VecDeque::new(),
            integral_max: 100.0,
            output_min: -1.0,
            output_max: 1.0,
            dt,
            derivative_window: 5,
        }
    }

    /// Create PID controller with gains only (dt = 0.01)
    pub fn with_gains(kp: f32, ki: f32, kd: f32) -> Self {
        Self::new(kp, ki, kd, 0.01)
    }

    /// Set output limits
    pub fn with_limits(mut self, min: f32, max: f32) -> Self {
        self.output_min = min;
        self.output_max = max;
        self
    }

    /// Set integral limit (anti-windup)
    pub fn with_integral_limit(mut self, limit: f32) -> Self {
        self.integral_max = limit;
        self
    }

    /// Set derivative smoothing window
    pub fn with_derivative_window(mut self, window: usize) -> Self {
        self.derivative_window = window;
        self
    }

    /// Compute control output
    pub fn update(&mut self, setpoint: f32, measurement: f32) -> f32 {
        let error = setpoint - measurement;

        // Proportional term
        let p = self.kp * error;

        // Integral term with anti-windup
        self.integral += error * self.dt;
        self.integral = self.integral.clamp(-self.integral_max, self.integral_max);
        let i = self.ki * self.integral;

        // Derivative term with smoothing
        self.error_history.push_back(error);
        if self.error_history.len() > self.derivative_window {
            self.error_history.pop_front();
        }

        let derivative = if self.error_history.len() >= 2 {
            // Use moving average for derivative
            let recent_error = self.error_history.iter().rev().next().unwrap();
            let old_error = self.error_history.iter().next().unwrap();
            (recent_error - old_error) / (self.dt * self.error_history.len() as f32)
        } else {
            0.0
        };

        let d = self.kd * derivative;

        // Compute output
        let output = p + i + d;

        // Clamp output
        output.clamp(self.output_min, self.output_max)
    }

    /// Reset controller state
    pub fn reset(&mut self) {
        self.prev_error = 0.0;
        self.integral = 0.0;
        self.error_history.clear();
    }

    /// Get current error
    pub fn error(&self) -> f32 {
        self.prev_error
    }

    /// Get current integral term
    pub fn integral_term(&self) -> f32 {
        self.ki * self.integral
    }
}

/// PD Controller (simplified PID without integral term)
#[derive(Clone, Debug)]
pub struct PDController {
    pid: PIDController,
}

impl PDController {
    pub fn new(kp: f32, kd: f32, dt: f32) -> Self {
        Self {
            pid: PIDController::new(kp, 0.0, kd, dt),
        }
    }

    pub fn with_gains(kp: f32, kd: f32) -> Self {
        Self::new(kp, kd, 0.01)
    }

    pub fn with_limits(mut self, min: f32, max: f32) -> Self {
        self.pid = self.pid.with_limits(min, max);
        self
    }

    pub fn update(&mut self, setpoint: f32, measurement: f32) -> f32 {
        self.pid.update(setpoint, measurement)
    }

    pub fn reset(&mut self) {
        self.pid.reset();
    }
}

/// Multi-dimensional PID controller
#[derive(Clone, Debug)]
pub struct MultiPIDController {
    controllers: Vec<PIDController>,
}

impl MultiPIDController {
    /// Create multi-dimensional PID with same gains for all dimensions
    pub fn new(num_dims: usize, kp: f32, ki: f32, kd: f32, dt: f32) -> Self {
        Self {
            controllers: (0..num_dims)
                .map(|_| PIDController::new(kp, ki, kd, dt))
                .collect(),
        }
    }

    /// Create with individual gains per dimension
    pub fn with_individual_gains(gains: Vec<(f32, f32, f32)>, dt: f32) -> Self {
        Self {
            controllers: gains
                .into_iter()
                .map(|(kp, ki, kd)| PIDController::new(kp, ki, kd, dt))
                .collect(),
        }
    }

    /// Set output limits for all controllers
    pub fn with_limits(mut self, min: f32, max: f32) -> Self {
        for controller in &mut self.controllers {
            controller.output_min = min;
            controller.output_max = max;
        }
        self
    }

    /// Update all controllers
    pub fn update(&mut self, setpoints: &[f32], measurements: &[f32]) -> Vec<f32> {
        assert_eq!(setpoints.len(), self.controllers.len());
        assert_eq!(measurements.len(), self.controllers.len());

        setpoints
            .iter()
            .zip(measurements.iter())
            .zip(self.controllers.iter_mut())
            .map(|((&sp, &m), controller)| controller.update(sp, m))
            .collect()
    }

    /// Reset all controllers
    pub fn reset(&mut self) {
        for controller in &mut self.controllers {
            controller.reset();
        }
    }

    /// Get number of dimensions
    pub fn num_dims(&self) -> usize {
        self.controllers.len()
    }
}

/// Lead-lag compensator for improved stability
#[derive(Clone, Debug)]
pub struct LeadLagController {
    /// Lead time constant
    pub lead_time: f32,
    /// Lag time constant
    pub lag_time: f32,
    /// Gain
    pub gain: f32,

    prev_input: f32,
    prev_output: f32,
    dt: f32,
}

impl LeadLagController {
    pub fn new(gain: f32, lead_time: f32, lag_time: f32, dt: f32) -> Self {
        Self {
            gain,
            lead_time,
            lag_time,
            prev_input: 0.0,
            prev_output: 0.0,
            dt,
        }
    }

    pub fn update(&mut self, input: f32) -> f32 {
        // Discrete lead-lag compensator using bilinear transform
        let a0 = self.lag_time + 2.0 * self.dt;
        let a1 = self.lag_time - 2.0 * self.dt;
        let b0 = self.lead_time + 2.0 * self.dt;
        let b1 = self.lead_time - 2.0 * self.dt;

        let output = (b0 * input + b1 * self.prev_input - a1 * self.prev_output) / a0;

        self.prev_input = input;
        self.prev_output = output;

        self.gain * output
    }

    pub fn reset(&mut self) {
        self.prev_input = 0.0;
        self.prev_output = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_proportional() {
        let mut pid = PIDController::new(1.0, 0.0, 0.0, 0.01);
        let output = pid.update(10.0, 5.0); // Error = 5.0
        assert_eq!(output, 1.0); // P = 1.0 * 5.0 = 5.0, but clamped to 1.0
    }

    #[test]
    fn test_pid_integral() {
        let mut pid = PIDController::new(0.0, 1.0, 0.0, 0.01);

        // Accumulate error over time
        for _ in 0..10 {
            pid.update(10.0, 5.0); // Error = 5.0
        }

        // Integral should accumulate
        assert!(pid.integral > 0.0);
    }

    #[test]
    fn test_pid_reset() {
        let mut pid = PIDController::new(1.0, 1.0, 1.0, 0.01);

        pid.update(10.0, 5.0);
        assert_ne!(pid.integral, 0.0);

        pid.reset();
        assert_eq!(pid.integral, 0.0);
        assert_eq!(pid.prev_error, 0.0);
    }

    #[test]
    fn test_pd_controller() {
        let mut pd = PDController::new(1.0, 0.5, 0.01);
        let output = pd.update(10.0, 5.0);
        assert!(output >= -1.0 && output <= 1.0);
    }

    #[test]
    fn test_multi_pid() {
        let mut multi_pid = MultiPIDController::new(3, 1.0, 0.1, 0.05, 0.01);

        let setpoints = vec![10.0, 20.0, 30.0];
        let measurements = vec![5.0, 15.0, 25.0];

        let outputs = multi_pid.update(&setpoints, &measurements);
        assert_eq!(outputs.len(), 3);
    }

    #[test]
    fn test_output_limits() {
        let mut pid = PIDController::new(10.0, 0.0, 0.0, 0.01).with_limits(-0.5, 0.5);

        let output = pid.update(100.0, 0.0); // Very large error
        assert!(output >= -0.5 && output <= 0.5);
    }

    #[test]
    fn test_integral_anti_windup() {
        let mut pid = PIDController::new(0.0, 1.0, 0.0, 0.01).with_integral_limit(10.0);

        // Accumulate error to trigger anti-windup
        for _ in 0..1000 {
            pid.update(100.0, 0.0);
        }

        assert!(pid.integral.abs() <= 10.0);
    }

    #[test]
    fn test_lead_lag() {
        let mut lead_lag = LeadLagController::new(1.0, 0.1, 0.5, 0.01);

        let output = lead_lag.update(1.0);
        assert!(output.is_finite());

        lead_lag.reset();
        assert_eq!(lead_lag.prev_input, 0.0);
    }
}
