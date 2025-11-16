// Python wrappers for control messages
use horus_library::messages::control;
use pyo3::prelude::*;

/// Python wrapper for MotorCommand
#[pyclass(module = "horus.library._library", name = "MotorCommand")]
#[derive(Clone)]
pub struct PyMotorCommand {
    pub(crate) inner: control::MotorCommand,
}

#[pymethods]
impl PyMotorCommand {
    #[new]
    #[pyo3(signature = (motor_id=0, mode=0, target=0.0))]
    fn new(motor_id: u8, mode: u8, target: f64) -> Self {
        Self {
            inner: control::MotorCommand {
                motor_id,
                mode,
                target,
                ..Default::default()
            },
        }
    }

    #[classattr]
    const MODE_VELOCITY: u8 = control::MotorCommand::MODE_VELOCITY;
    #[classattr]
    const MODE_POSITION: u8 = control::MotorCommand::MODE_POSITION;
    #[classattr]
    const MODE_TORQUE: u8 = control::MotorCommand::MODE_TORQUE;
    #[classattr]
    const MODE_VOLTAGE: u8 = control::MotorCommand::MODE_VOLTAGE;

    #[staticmethod]
    fn velocity(motor_id: u8, velocity: f64) -> Self {
        Self {
            inner: control::MotorCommand::velocity(motor_id, velocity),
        }
    }

    #[staticmethod]
    fn position(motor_id: u8, position: f64, max_velocity: f64) -> Self {
        Self {
            inner: control::MotorCommand::position(motor_id, position, max_velocity),
        }
    }

    #[staticmethod]
    fn stop(motor_id: u8) -> Self {
        Self {
            inner: control::MotorCommand::stop(motor_id),
        }
    }

    #[getter]
    fn motor_id(&self) -> u8 {
        self.inner.motor_id
    }

    #[getter]
    fn mode(&self) -> u8 {
        self.inner.mode
    }

    #[getter]
    fn target(&self) -> f64 {
        self.inner.target
    }

    #[setter]
    fn set_target(&mut self, value: f64) {
        self.inner.target = value;
    }

    #[getter]
    fn enable(&self) -> bool {
        self.inner.enable
    }

    #[setter]
    fn set_enable(&mut self, value: bool) {
        self.inner.enable = value;
    }

    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn __repr__(&self) -> String {
        format!(
            "MotorCommand(id={}, mode={}, target={:.2})",
            self.inner.motor_id, self.inner.mode, self.inner.target
        )
    }
}

/// Python wrapper for DifferentialDriveCommand
#[pyclass(module = "horus.library._library", name = "DifferentialDriveCommand")]
#[derive(Clone)]
pub struct PyDifferentialDriveCommand {
    pub(crate) inner: control::DifferentialDriveCommand,
}

#[pymethods]
impl PyDifferentialDriveCommand {
    #[new]
    #[pyo3(signature = (left_velocity=0.0, right_velocity=0.0))]
    fn new(left_velocity: f64, right_velocity: f64) -> Self {
        Self {
            inner: control::DifferentialDriveCommand::new(left_velocity, right_velocity),
        }
    }

    #[staticmethod]
    fn stop() -> Self {
        Self {
            inner: control::DifferentialDriveCommand::stop(),
        }
    }

    #[staticmethod]
    fn from_twist(linear: f64, angular: f64, wheel_base: f64, wheel_radius: f64) -> Self {
        Self {
            inner: control::DifferentialDriveCommand::from_twist(
                linear,
                angular,
                wheel_base,
                wheel_radius,
            ),
        }
    }

    #[getter]
    fn left_velocity(&self) -> f64 {
        self.inner.left_velocity
    }

    #[setter]
    fn set_left_velocity(&mut self, value: f64) {
        self.inner.left_velocity = value;
    }

    #[getter]
    fn right_velocity(&self) -> f64 {
        self.inner.right_velocity
    }

    #[setter]
    fn set_right_velocity(&mut self, value: f64) {
        self.inner.right_velocity = value;
    }

    #[getter]
    fn enable(&self) -> bool {
        self.inner.enable
    }

    #[setter]
    fn set_enable(&mut self, value: bool) {
        self.inner.enable = value;
    }

    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn __repr__(&self) -> String {
        format!(
            "DifferentialDriveCommand(left={:.2}, right={:.2})",
            self.inner.left_velocity, self.inner.right_velocity
        )
    }
}

/// Python wrapper for ServoCommand
#[pyclass(module = "horus.library._library", name = "ServoCommand")]
#[derive(Clone)]
pub struct PyServoCommand {
    pub(crate) inner: control::ServoCommand,
}

#[pymethods]
impl PyServoCommand {
    #[new]
    #[pyo3(signature = (servo_id=0, position=0.0))]
    fn new(servo_id: u8, position: f32) -> Self {
        Self {
            inner: control::ServoCommand::new(servo_id, position),
        }
    }

    #[staticmethod]
    fn with_speed(servo_id: u8, position: f32, speed: f32) -> Self {
        Self {
            inner: control::ServoCommand::with_speed(servo_id, position, speed),
        }
    }

    #[staticmethod]
    fn disable(servo_id: u8) -> Self {
        Self {
            inner: control::ServoCommand::disable(servo_id),
        }
    }

    #[staticmethod]
    fn from_degrees(servo_id: u8, degrees: f32) -> Self {
        Self {
            inner: control::ServoCommand::from_degrees(servo_id, degrees),
        }
    }

    #[getter]
    fn servo_id(&self) -> u8 {
        self.inner.servo_id
    }

    #[getter]
    fn position(&self) -> f32 {
        self.inner.position
    }

    #[setter]
    fn set_position(&mut self, value: f32) {
        self.inner.position = value;
    }

    #[getter]
    fn speed(&self) -> f32 {
        self.inner.speed
    }

    #[setter]
    fn set_speed(&mut self, value: f32) {
        self.inner.speed = value;
    }

    #[getter]
    fn enable(&self) -> bool {
        self.inner.enable
    }

    #[setter]
    fn set_enable(&mut self, value: bool) {
        self.inner.enable = value;
    }

    fn __repr__(&self) -> String {
        format!(
            "ServoCommand(id={}, pos={:.2}, speed={:.2})",
            self.inner.servo_id, self.inner.position, self.inner.speed
        )
    }
}

/// Python wrapper for PwmCommand
#[pyclass(module = "horus.library._library", name = "PwmCommand")]
#[derive(Clone)]
pub struct PyPwmCommand {
    pub(crate) inner: control::PwmCommand,
}

#[pymethods]
impl PyPwmCommand {
    #[new]
    #[pyo3(signature = (channel_id=0, duty_cycle=0.0))]
    fn new(channel_id: u8, duty_cycle: f32) -> Self {
        Self {
            inner: control::PwmCommand::new(channel_id, duty_cycle),
        }
    }

    #[staticmethod]
    fn forward(channel: u8, speed: f32) -> Self {
        Self {
            inner: control::PwmCommand::forward(channel, speed),
        }
    }

    #[staticmethod]
    fn reverse(channel: u8, speed: f32) -> Self {
        Self {
            inner: control::PwmCommand::reverse(channel, speed),
        }
    }

    #[staticmethod]
    fn coast(channel: u8) -> Self {
        Self {
            inner: control::PwmCommand::coast(channel),
        }
    }

    #[staticmethod]
    fn brake(channel: u8) -> Self {
        Self {
            inner: control::PwmCommand::brake(channel),
        }
    }

    #[getter]
    fn channel_id(&self) -> u8 {
        self.inner.channel_id
    }

    #[getter]
    fn duty_cycle(&self) -> f32 {
        self.inner.duty_cycle
    }

    #[setter]
    fn set_duty_cycle(&mut self, value: f32) {
        self.inner.duty_cycle = value;
    }

    #[getter]
    fn frequency(&self) -> u32 {
        self.inner.frequency
    }

    #[setter]
    fn set_frequency(&mut self, value: u32) {
        self.inner.frequency = value;
    }

    #[getter]
    fn enable(&self) -> bool {
        self.inner.enable
    }

    #[setter]
    fn set_enable(&mut self, value: bool) {
        self.inner.enable = value;
    }

    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn speed(&self) -> f32 {
        self.inner.speed()
    }

    fn is_forward(&self) -> bool {
        self.inner.is_forward()
    }

    fn __repr__(&self) -> String {
        format!(
            "PwmCommand(ch={}, duty={:.1}%, freq={}Hz)",
            self.inner.channel_id,
            self.inner.duty_cycle * 100.0,
            self.inner.frequency
        )
    }
}

/// Python wrapper for StepperCommand
#[pyclass(module = "horus.library._library", name = "StepperCommand")]
#[derive(Clone)]
pub struct PyStepperCommand {
    pub(crate) inner: control::StepperCommand,
}

#[pymethods]
impl PyStepperCommand {
    #[new]
    fn new() -> Self {
        Self {
            inner: control::StepperCommand::default(),
        }
    }

    #[classattr]
    const MODE_STEPS: u8 = control::StepperCommand::MODE_STEPS;
    #[classattr]
    const MODE_POSITION: u8 = control::StepperCommand::MODE_POSITION;
    #[classattr]
    const MODE_VELOCITY: u8 = control::StepperCommand::MODE_VELOCITY;
    #[classattr]
    const MODE_HOMING: u8 = control::StepperCommand::MODE_HOMING;

    #[staticmethod]
    fn steps(motor_id: u8, steps: i64) -> Self {
        Self {
            inner: control::StepperCommand::steps(motor_id, steps),
        }
    }

    #[staticmethod]
    fn position(motor_id: u8, position: f64, max_velocity: f64) -> Self {
        Self {
            inner: control::StepperCommand::position(motor_id, position, max_velocity),
        }
    }

    #[staticmethod]
    fn velocity(motor_id: u8, velocity: f64) -> Self {
        Self {
            inner: control::StepperCommand::velocity(motor_id, velocity),
        }
    }

    #[staticmethod]
    fn home(motor_id: u8, homing_velocity: f64) -> Self {
        Self {
            inner: control::StepperCommand::home(motor_id, homing_velocity),
        }
    }

    #[staticmethod]
    fn disable(motor_id: u8) -> Self {
        Self {
            inner: control::StepperCommand::disable(motor_id),
        }
    }

    #[getter]
    fn motor_id(&self) -> u8 {
        self.inner.motor_id
    }

    #[getter]
    fn mode(&self) -> u8 {
        self.inner.mode
    }

    #[getter]
    fn target(&self) -> f64 {
        self.inner.target
    }

    #[getter]
    fn microsteps(&self) -> u16 {
        self.inner.microsteps
    }

    #[setter]
    fn set_microsteps(&mut self, value: u16) {
        self.inner.microsteps = value;
    }

    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn __repr__(&self) -> String {
        format!(
            "StepperCommand(id={}, mode={}, target={:.2})",
            self.inner.motor_id, self.inner.mode, self.inner.target
        )
    }
}

/// Python wrapper for PidConfig
#[pyclass(module = "horus.library._library", name = "PidConfig")]
#[derive(Clone)]
pub struct PyPidConfig {
    pub(crate) inner: control::PidConfig,
}

#[pymethods]
impl PyPidConfig {
    #[new]
    #[pyo3(signature = (kp=0.0, ki=0.0, kd=0.0))]
    fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            inner: control::PidConfig::new(kp, ki, kd),
        }
    }

    #[staticmethod]
    fn proportional(kp: f64) -> Self {
        Self {
            inner: control::PidConfig::proportional(kp),
        }
    }

    #[staticmethod]
    fn pi(kp: f64, ki: f64) -> Self {
        Self {
            inner: control::PidConfig::pi(kp, ki),
        }
    }

    #[staticmethod]
    fn pd(kp: f64, kd: f64) -> Self {
        Self {
            inner: control::PidConfig::pd(kp, kd),
        }
    }

    #[getter]
    fn kp(&self) -> f64 {
        self.inner.kp
    }

    #[setter]
    fn set_kp(&mut self, value: f64) {
        self.inner.kp = value;
    }

    #[getter]
    fn ki(&self) -> f64 {
        self.inner.ki
    }

    #[setter]
    fn set_ki(&mut self, value: f64) {
        self.inner.ki = value;
    }

    #[getter]
    fn kd(&self) -> f64 {
        self.inner.kd
    }

    #[setter]
    fn set_kd(&mut self, value: f64) {
        self.inner.kd = value;
    }

    fn is_valid(&self) -> bool {
        self.inner.is_valid()
    }

    fn __repr__(&self) -> String {
        format!(
            "PidConfig(kp={:.3}, ki={:.3}, kd={:.3})",
            self.inner.kp, self.inner.ki, self.inner.kd
        )
    }
}
