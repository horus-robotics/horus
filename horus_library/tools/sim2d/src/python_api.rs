//! Python API for sim2d
//!
//! This module provides Python bindings for the sim2d simulator using PyO3.
//!
//! Example usage:
//! ```python
//! from horus.sim2d import Sim2D
//!
//! sim = Sim2D()
//! sim.run(duration=10.0)
//! ```

#![allow(clippy::too_many_arguments)]
#![allow(clippy::useless_conversion)]

use crate::{Obstacle, ObstacleShape, RobotConfig, Sim2DBuilder, WorldConfig};
use pyo3::prelude::*;

/// Python wrapper for Sim2D simulator
#[pyclass]
pub struct Sim2D {
    robot_config: RobotConfig,
    world_config: WorldConfig,
    robot_name: String,
    topic_prefix: String,
    headless: bool,
}

#[pymethods]
impl Sim2D {
    /// Create a new Sim2D instance
    ///
    /// Args:
    ///     robot_name (str, optional): Name of the robot. Defaults to "robot".
    ///     topic_prefix (str, optional): Topic prefix for HORUS communication. Defaults to "robot".
    ///     headless (bool, optional): Run without GUI. Defaults to True.
    ///     robot_width (float, optional): Robot width in meters. Defaults to 0.5.
    ///     robot_length (float, optional): Robot length in meters. Defaults to 0.8.
    ///     robot_max_speed (float, optional): Maximum robot speed in m/s. Defaults to 2.0.
    ///     world_width (float, optional): World width in meters. Defaults to 20.0.
    ///     world_height (float, optional): World height in meters. Defaults to 15.0.
    #[new]
    #[pyo3(signature = (robot_name="robot", topic_prefix="robot", headless=true, robot_width=0.5, robot_length=0.8, robot_max_speed=2.0, world_width=20.0, world_height=15.0))]
    fn new(
        robot_name: &str,
        topic_prefix: &str,
        headless: bool,
        robot_width: f32,
        robot_length: f32,
        robot_max_speed: f32,
        world_width: f32,
        world_height: f32,
    ) -> PyResult<Self> {
        let robot_config = RobotConfig {
            name: robot_name.to_string(),
            topic_prefix: topic_prefix.to_string(),
            width: robot_width,
            length: robot_length,
            max_speed: robot_max_speed,
            ..Default::default()
        };

        let world_config = WorldConfig {
            width: world_width,
            height: world_height,
            obstacles: Vec::new(), // Start with empty world
        };

        Ok(Self {
            robot_config,
            world_config,
            robot_name: robot_name.to_string(),
            topic_prefix: topic_prefix.to_string(),
            headless,
        })
    }

    /// Run the simulation for a specified duration (in seconds)
    ///
    /// This creates a temporary simulation instance and runs it for the specified duration.
    ///
    /// Args:
    ///     duration (float): Duration to run in seconds
    fn run(&self, duration: f32) -> PyResult<()> {
        let mut app = Sim2DBuilder::new()
            .with_robot(self.robot_config.clone())
            .with_world(self.world_config.clone())
            .robot_name(&self.robot_name)
            .topic_prefix(&self.topic_prefix)
            .headless(self.headless)
            .build()
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create sim2d: {}",
                    e
                ))
            })?;

        app.run_for(duration);
        Ok(())
    }

    /// Add an obstacle to the world configuration
    ///
    /// Args:
    ///     pos (tuple): Position (x, y) in meters
    ///     size (tuple): Size [width, height] for rectangle or [radius, radius] for circle
    ///     shape (str, optional): Shape type: "rectangle" or "circle". Defaults to "rectangle".
    ///     color (tuple, optional): RGB color (0.0-1.0). Defaults to None (gray).
    #[pyo3(signature = (pos, size, shape="rectangle", color=None))]
    fn add_obstacle(
        &mut self,
        pos: (f64, f64),
        size: (f64, f64),
        shape: &str,
        color: Option<(f64, f64, f64)>,
    ) -> PyResult<()> {
        let obstacle_shape = match shape {
            "rectangle" => ObstacleShape::Rectangle,
            "circle" => ObstacleShape::Circle,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Shape must be 'rectangle' or 'circle'",
                ))
            }
        };

        let color_array = color.map(|(r, g, b)| [r as f32, g as f32, b as f32]);

        let obstacle = Obstacle {
            pos: [pos.0 as f32, pos.1 as f32],
            shape: obstacle_shape,
            size: [size.0 as f32, size.1 as f32],
            color: color_array,
        };

        self.world_config.obstacles.push(obstacle);
        Ok(())
    }

    /// Clear all obstacles from the world configuration
    fn clear_obstacles(&mut self) -> PyResult<()> {
        self.world_config.obstacles.clear();
        Ok(())
    }

    /// Get robot configuration
    ///
    /// Returns:
    ///     dict: Robot configuration parameters
    fn get_robot_config(&self) -> PyResult<RobotConfigPy> {
        Ok(RobotConfigPy {
            name: self.robot_config.name.clone(),
            topic_prefix: self.robot_config.topic_prefix.clone(),
            width: self.robot_config.width,
            length: self.robot_config.length,
            max_speed: self.robot_config.max_speed,
            color: self.robot_config.color.to_vec(),
        })
    }

    /// Get world configuration
    ///
    /// Returns:
    ///     dict: World configuration parameters
    fn get_world_config(&self) -> PyResult<WorldConfigPy> {
        Ok(WorldConfigPy {
            width: self.world_config.width,
            height: self.world_config.height,
            obstacle_count: self.world_config.obstacles.len(),
        })
    }

    /// Set robot position
    ///
    /// Args:
    ///     pos (tuple): Position (x, y) in meters
    fn set_robot_position(&mut self, pos: (f64, f64)) -> PyResult<()> {
        self.robot_config.position = [pos.0 as f32, pos.1 as f32];
        Ok(())
    }

    /// Set robot color
    ///
    /// Args:
    ///     color (tuple): RGB color (0.0-1.0)
    fn set_robot_color(&mut self, color: (f64, f64, f64)) -> PyResult<()> {
        self.robot_config.color = [color.0 as f32, color.1 as f32, color.2 as f32];
        Ok(())
    }

    /// Python representation
    fn __repr__(&self) -> String {
        format!(
            "Sim2D(robot_name='{}', topic_prefix='{}', headless={})",
            self.robot_name, self.topic_prefix, self.headless
        )
    }

    /// String representation
    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Python-friendly robot configuration
#[pyclass]
#[derive(Clone)]
pub struct RobotConfigPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub topic_prefix: String,
    #[pyo3(get)]
    pub width: f32,
    #[pyo3(get)]
    pub length: f32,
    #[pyo3(get)]
    pub max_speed: f32,
    #[pyo3(get)]
    pub color: Vec<f32>,
}

/// Python-friendly world configuration
#[pyclass]
#[derive(Clone)]
pub struct WorldConfigPy {
    #[pyo3(get)]
    pub width: f32,
    #[pyo3(get)]
    pub height: f32,
    #[pyo3(get)]
    pub obstacle_count: usize,
}

/// Python module initialization
#[pymodule]
fn _sim2d(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Sim2D>()?;
    m.add_class::<RobotConfigPy>()?;
    m.add_class::<WorldConfigPy>()?;
    Ok(())
}
