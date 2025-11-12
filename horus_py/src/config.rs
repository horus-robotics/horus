use horus::scheduling::config::{
    RobotPreset, SchedulerConfig,
};
use pyo3::prelude::*;

/// Robot-specific configuration presets for quick setup
///
/// These presets provide optimized configurations for different types of robots,
/// making it easy to get started with sensible defaults.
#[pyclass(module = "horus._horus", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PyRobotPreset {
    /// Standard industrial robot (default)
    /// - 60 Hz tick rate
    /// - Circuit breaker enabled
    /// - Auto-restart on failures
    Standard,

    /// Safety-critical medical/surgical robot
    /// - Enhanced fault tolerance
    /// - Redundancy enabled
    /// - Conservative timing
    SafetyCritical,

    /// Hard real-time aerospace/defense
    /// - WCET enforcement (Python: soft, ms-level)
    /// - Deadline monitoring
    /// - Watchdog timers
    HardRealTime,

    /// High-performance racing/competition
    /// - Maximum tick rate
    /// - Parallel execution
    /// - Minimal overhead
    HighPerformance,

    /// Educational/research platform
    /// - Detailed logging
    /// - Profiling enabled
    /// - Relaxed constraints
    Educational,

    /// Mobile/field robot
    /// - Power management enabled
    /// - Network-aware
    /// - Battery optimization
    Mobile,

    /// Underwater/marine robot
    /// - Fault tolerance enhanced
    /// - Network delays tolerated
    /// - Checkpointing enabled
    Underwater,

    /// Space/satellite robot
    /// - Maximum reliability
    /// - Radiation hardening (software)
    /// - Formal verification
    Space,

    /// Swarm robotics
    /// - Distributed coordination
    /// - Network time sync
    /// - Scalable monitoring
    Swarm,

    /// Soft robotics
    /// - Adaptive control
    /// - Compliant timing
    /// - Sensor fusion optimized
    SoftRobotics,

    /// Custom configuration
    /// - Start from defaults
    /// - Fully customizable
    Custom,
}

#[pymethods]
impl PyRobotPreset {
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    fn __str__(&self) -> String {
        match self {
            PyRobotPreset::Standard => "Standard",
            PyRobotPreset::SafetyCritical => "SafetyCritical",
            PyRobotPreset::HardRealTime => "HardRealTime",
            PyRobotPreset::HighPerformance => "HighPerformance",
            PyRobotPreset::Educational => "Educational",
            PyRobotPreset::Mobile => "Mobile",
            PyRobotPreset::Underwater => "Underwater",
            PyRobotPreset::Space => "Space",
            PyRobotPreset::Swarm => "Swarm",
            PyRobotPreset::SoftRobotics => "SoftRobotics",
            PyRobotPreset::Custom => "Custom",
        }
        .to_string()
    }
}

impl From<PyRobotPreset> for RobotPreset {
    fn from(preset: PyRobotPreset) -> Self {
        match preset {
            PyRobotPreset::Standard => RobotPreset::Standard,
            PyRobotPreset::SafetyCritical => RobotPreset::SafetyCritical,
            PyRobotPreset::HardRealTime => RobotPreset::HardRealTime,
            PyRobotPreset::HighPerformance => RobotPreset::HighPerformance,
            PyRobotPreset::Educational => RobotPreset::Educational,
            PyRobotPreset::Mobile => RobotPreset::Mobile,
            PyRobotPreset::Underwater => RobotPreset::Underwater,
            PyRobotPreset::Space => RobotPreset::Space,
            PyRobotPreset::Swarm => RobotPreset::Swarm,
            PyRobotPreset::SoftRobotics => RobotPreset::SoftRobotics,
            PyRobotPreset::Custom => RobotPreset::Custom,
        }
    }
}

/// Simplified scheduler configuration for Python
///
/// Provides a subset of Rust's SchedulerConfig with Python-friendly defaults.
/// For advanced users, all fields can be customized after creation.
#[pyclass(module = "horus._horus")]
#[derive(Clone, Debug)]
pub struct PySchedulerConfig {
    #[pyo3(get, set)]
    /// Global tick rate in Hz (default: 60.0)
    pub tick_rate: f64,

    #[pyo3(get, set)]
    /// Enable circuit breaker pattern (default: True)
    pub circuit_breaker: bool,

    #[pyo3(get, set)]
    /// Max failures before circuit opens (default: 5)
    pub max_failures: u32,

    #[pyo3(get, set)]
    /// Enable automatic node restart (default: True)
    pub auto_restart: bool,

    #[pyo3(get, set)]
    /// Enable deadline monitoring (default: False, set True for soft RT)
    pub deadline_monitoring: bool,

    #[pyo3(get, set)]
    /// Enable watchdog timers (default: False)
    pub watchdog_enabled: bool,

    #[pyo3(get, set)]
    /// Watchdog timeout in milliseconds (default: 1000)
    pub watchdog_timeout_ms: u64,

    #[pyo3(get, set)]
    /// Enable profiling (default: False)
    pub profiling: bool,

    #[pyo3(get, set)]
    /// Enable power management (default: False)
    pub power_management: bool,

    #[pyo3(get, set)]
    /// Robot preset
    pub preset: PyRobotPreset,
}

#[pymethods]
impl PySchedulerConfig {
    #[new]
    #[pyo3(signature = (preset=PyRobotPreset::Standard))]
    pub fn new(preset: PyRobotPreset) -> Self {
        let rust_preset: RobotPreset = preset.into();
        let rust_config = Self::preset_to_config(rust_preset);

        PySchedulerConfig {
            tick_rate: rust_config.timing.global_rate_hz,
            circuit_breaker: rust_config.fault.circuit_breaker_enabled,
            max_failures: rust_config.fault.max_failures,
            auto_restart: rust_config.fault.auto_restart,
            deadline_monitoring: rust_config.realtime.deadline_monitoring,
            watchdog_enabled: rust_config.realtime.watchdog_enabled,
            watchdog_timeout_ms: rust_config.realtime.watchdog_timeout_ms,
            profiling: rust_config.monitoring.profiling_enabled,
            power_management: rust_config.resources.power_management,
            preset,
        }
    }

    /// Create configuration from a robot preset
    ///
    /// Example:
    ///     config = horus.SchedulerConfig.from_preset(horus.RobotPreset.Mobile)
    #[staticmethod]
    pub fn from_preset(preset: PyRobotPreset) -> Self {
        Self::new(preset)
    }

    /// Create standard configuration (default)
    #[staticmethod]
    pub fn standard() -> Self {
        Self::new(PyRobotPreset::Standard)
    }

    /// Create safety-critical configuration
    #[staticmethod]
    pub fn safety_critical() -> Self {
        Self::new(PyRobotPreset::SafetyCritical)
    }

    /// Create mobile robot configuration
    #[staticmethod]
    pub fn mobile() -> Self {
        Self::new(PyRobotPreset::Mobile)
    }

    /// Create educational configuration with detailed logging
    #[staticmethod]
    pub fn educational() -> Self {
        Self::new(PyRobotPreset::Educational)
    }

    fn __repr__(&self) -> String {
        format!(
            "SchedulerConfig(preset={}, tick_rate={:.1}Hz, circuit_breaker={}, auto_restart={})",
            self.preset.__str__(),
            self.tick_rate,
            self.circuit_breaker,
            self.auto_restart
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

// Helper implementation (not exposed to Python)
impl PySchedulerConfig {
    /// Helper function to convert RobotPreset to SchedulerConfig
    fn preset_to_config(preset: RobotPreset) -> SchedulerConfig {
        match preset {
            RobotPreset::Standard => SchedulerConfig::standard(),
            RobotPreset::SafetyCritical => SchedulerConfig::safety_critical(),
            RobotPreset::HardRealTime => SchedulerConfig::hard_realtime(),
            RobotPreset::HighPerformance => SchedulerConfig::high_performance(),
            RobotPreset::Space => SchedulerConfig::space(),
            RobotPreset::Swarm => SchedulerConfig::swarm(),
            RobotPreset::SoftRobotics => SchedulerConfig::soft_robotics(),
            // Presets not in core config - use standard() as base
            RobotPreset::Educational => SchedulerConfig::standard(),
            RobotPreset::Mobile => SchedulerConfig::standard(),
            RobotPreset::Underwater => SchedulerConfig::standard(),
            RobotPreset::Custom => SchedulerConfig::standard(),
            _ => SchedulerConfig::standard(), // Fallback
        }
    }

    /// Convert to Rust SchedulerConfig
    #[allow(dead_code)]
    pub(crate) fn to_rust_config(&self) -> SchedulerConfig {
        let rust_preset: RobotPreset = self.preset.into();
        let mut config = Self::preset_to_config(rust_preset);

        // Apply Python-specific overrides
        config.timing.global_rate_hz = self.tick_rate;
        config.fault.circuit_breaker_enabled = self.circuit_breaker;
        config.fault.max_failures = self.max_failures;
        config.fault.auto_restart = self.auto_restart;
        config.realtime.deadline_monitoring = self.deadline_monitoring;
        config.realtime.watchdog_enabled = self.watchdog_enabled;
        config.realtime.watchdog_timeout_ms = self.watchdog_timeout_ms;
        config.monitoring.profiling_enabled = self.profiling;
        config.resources.power_management = self.power_management;

        // Python-specific adjustments (soften hard RT constraints)
        config.realtime.wcet_enforcement = false; // Python can't guarantee WCET
        config.realtime.rt_scheduling_class = false; // Don't use SCHED_FIFO in Python
        config.realtime.memory_locking = false; // mlockall not helpful for Python

        config
    }
}
