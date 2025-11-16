use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct PIDController {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    pub integral: f32,
    pub last_error: f32,
    pub min_output: f32,
    pub max_output: f32,
}

impl PIDController {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            last_error: 0.0,
            min_output: f32::NEG_INFINITY,
            max_output: f32::INFINITY,
        }
    }

    pub fn with_limits(mut self, min: f32, max: f32) -> Self {
        self.min_output = min;
        self.max_output = max;
        self
    }

    pub fn update(&mut self, error: f32, dt: f32) -> f32 {
        self.integral += error * dt;

        self.integral = self.integral.clamp(
            self.min_output / self.ki.max(0.001),
            self.max_output / self.ki.max(0.001),
        );

        let derivative = (error - self.last_error) / dt.max(0.001);
        self.last_error = error;

        let output = self.kp * error + self.ki * self.integral + self.kd * derivative;

        output.clamp(self.min_output, self.max_output)
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.last_error = 0.0;
    }
}

#[derive(Component, Debug, Clone)]
pub struct JointController {
    pub target_position: f32,
    pub target_velocity: f32,
    pub pid: PIDController,
    pub mode: ControlMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlMode {
    Position,
    Velocity,
    Torque,
}

impl JointController {
    pub fn position_control(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            target_position: 0.0,
            target_velocity: 0.0,
            pid: PIDController::new(kp, ki, kd),
            mode: ControlMode::Position,
        }
    }

    pub fn velocity_control(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            target_position: 0.0,
            target_velocity: 0.0,
            pid: PIDController::new(kp, ki, kd),
            mode: ControlMode::Velocity,
        }
    }

    pub fn compute_torque(&mut self, current_position: f32, current_velocity: f32, dt: f32) -> f32 {
        match self.mode {
            ControlMode::Position => {
                let position_error = self.target_position - current_position;
                self.pid.update(position_error, dt)
            }
            ControlMode::Velocity => {
                let velocity_error = self.target_velocity - current_velocity;
                self.pid.update(velocity_error, dt)
            }
            ControlMode::Torque => self.target_velocity,
        }
    }
}
