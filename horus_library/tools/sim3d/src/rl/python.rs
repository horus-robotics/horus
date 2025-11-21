//! Python bindings for RL environments using PyO3
//!
//! This module provides Gymnasium-compatible Python bindings for all RL tasks.

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pyo3::types::{PyDict, PyList};

#[cfg(feature = "python")]
use numpy::{PyArray1, PyArray2, PyArrayMethods, ToPyArray};

use bevy::prelude::*;
use std::sync::{Arc, Mutex};

use super::{tasks::*, Action, Observation, RLTask, RLTaskManager, StepResult};

/// Python-exposed RL environment (Gymnasium compatible)
#[cfg(feature = "python")]
#[pyclass(name = "Sim3DEnv")]
pub struct PySim3DEnv {
    task_manager: Arc<Mutex<RLTaskManager>>,
    world: Arc<Mutex<World>>,
    obs_dim: usize,
    action_dim: usize,
    episode_count: usize,
}

#[cfg(feature = "python")]
#[pymethods]
impl PySim3DEnv {
    #[new]
    fn new(task_type: &str, obs_dim: usize, action_dim: usize) -> PyResult<Self> {
        let mut world = World::new();

        // Create task based on type
        let task: Box<dyn RLTask> = match task_type {
            "reaching" => Box::new(ReachingTask::new(obs_dim, action_dim)),
            "balancing" => Box::new(BalancingTask::new(obs_dim, action_dim)),
            "locomotion" => Box::new(LocomotionTask::new(obs_dim, action_dim)),
            "navigation" => Box::new(NavigationTask::new(obs_dim, action_dim)),
            "manipulation" => Box::new(ManipulationTask::new(obs_dim, action_dim)),
            "push" => Box::new(PushTask::new(obs_dim, action_dim)),
            _ => return Err(pyo3::exceptions::PyValueError::new_err(
                format!("Unknown task type: {}. Available: reaching, balancing, locomotion, navigation, manipulation, push", task_type)
            )),
        };

        let mut task_manager = RLTaskManager::new();
        task_manager.set_task(task);

        Ok(Self {
            task_manager: Arc::new(Mutex::new(task_manager)),
            world: Arc::new(Mutex::new(world)),
            obs_dim,
            action_dim,
            episode_count: 0,
        })
    }

    /// Reset the environment (Gym/Gymnasium API)
    fn reset(&mut self, py: Python) -> PyResult<Py<PyArray1<f32>>> {
        let mut task_manager = self.task_manager.lock().unwrap();
        let mut world = self.world.lock().unwrap();

        if let Some(obs) = task_manager.reset(&mut world) {
            self.episode_count += 1;
            let obs_array = obs.data.to_pyarray_bound(py).to_owned();
            Ok(obs_array.unbind())
        } else {
            Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Failed to reset environment",
            ))
        }
    }

    /// Step the environment (Gym/Gymnasium API)
    fn step(
        &mut self,
        py: Python,
        action: Vec<f32>,
    ) -> PyResult<(Py<PyArray1<f32>>, f32, bool, bool, Py<PyDict>)> {
        let mut task_manager = self.task_manager.lock().unwrap();
        let mut world = self.world.lock().unwrap();

        let action_obj = Action::Continuous(action);

        if let Some(result) = task_manager.step(&mut world, &action_obj) {
            let obs_array = result.observation.data.to_pyarray_bound(py).to_owned();

            // Create info dict
            let info = PyDict::new_bound(py);
            info.set_item("total_reward", result.info.total_reward)?;
            info.set_item("steps", result.info.steps)?;
            info.set_item("success", result.info.success)?;
            info.set_item(
                "termination_reason",
                format!("{:?}", result.info.termination_reason),
            )?;

            Ok((
                obs_array.unbind(),
                result.reward,
                result.done,
                result.truncated,
                info.unbind(),
            ))
        } else {
            Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Failed to step environment",
            ))
        }
    }

    /// Get observation space (Gym/Gymnasium API)
    fn observation_space(&self, py: Python) -> PyResult<Py<PyDict>> {
        let space = PyDict::new_bound(py);
        space.set_item("type", "Box")?;
        space.set_item("shape", vec![self.obs_dim])?;
        space.set_item("dtype", "float32")?;
        Ok(space.unbind())
    }

    /// Get action space (Gym/Gymnasium API)
    fn action_space(&self, py: Python) -> PyResult<Py<PyDict>> {
        let space = PyDict::new_bound(py);
        space.set_item("type", "Box")?;
        space.set_item("shape", vec![self.action_dim])?;
        space.set_item("low", -1.0)?;
        space.set_item("high", 1.0)?;
        space.set_item("dtype", "float32")?;
        Ok(space.unbind())
    }

    /// Render the environment (optional)
    fn render(&self, _mode: &str) -> PyResult<()> {
        // Rendering handled by Bevy in separate thread
        // This is a no-op for headless mode
        Ok(())
    }

    /// Close the environment
    fn close(&mut self) -> PyResult<()> {
        // Cleanup if needed
        Ok(())
    }

    /// Get current episode count
    #[getter]
    fn episode_count(&self) -> usize {
        self.episode_count
    }

    /// Get total steps across all episodes
    #[getter]
    fn total_steps(&self) -> usize {
        let task_manager = self.task_manager.lock().unwrap();
        task_manager.total_steps
    }
}

/// Vectorized environment for parallel RL training
#[cfg(feature = "python")]
#[pyclass(name = "VecSim3DEnv")]
pub struct PyVecSim3DEnv {
    envs: Vec<PySim3DEnv>,
    num_envs: usize,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyVecSim3DEnv {
    #[new]
    fn new(task_type: &str, obs_dim: usize, action_dim: usize, num_envs: usize) -> PyResult<Self> {
        let mut envs = Vec::with_capacity(num_envs);
        for _ in 0..num_envs {
            envs.push(PySim3DEnv::new(task_type, obs_dim, action_dim)?);
        }

        Ok(Self { envs, num_envs })
    }

    /// Reset all environments
    fn reset(&mut self, py: Python) -> PyResult<Py<PyArray2<f32>>> {
        let mut observations = Vec::new();

        for env in &mut self.envs {
            let obs = env.reset(py)?;
            let obs_data: Vec<f32> = obs.bind(py).to_vec()?;
            observations.extend(obs_data);
        }

        // Reshape to (num_envs, obs_dim)
        let obs_array = PyArray2::from_vec2_bound(
            py,
            &observations
                .chunks(self.envs[0].obs_dim)
                .map(|chunk| chunk.to_vec())
                .collect::<Vec<_>>(),
        )?;

        Ok(obs_array.unbind())
    }

    /// Step all environments with vectorized actions
    fn step(
        &mut self,
        py: Python,
        actions: Vec<Vec<f32>>,
    ) -> PyResult<(
        Py<PyArray2<f32>>,
        Vec<f32>,
        Vec<bool>,
        Vec<bool>,
        Vec<Py<PyDict>>,
    )> {
        if actions.len() != self.num_envs {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Expected {} actions, got {}",
                self.num_envs,
                actions.len()
            )));
        }

        let mut observations = Vec::new();
        let mut rewards = Vec::new();
        let mut dones = Vec::new();
        let mut truncateds = Vec::new();
        let mut infos = Vec::new();

        for (env, action) in self.envs.iter_mut().zip(actions.iter()) {
            let (obs, reward, done, truncated, info) = env.step(py, action.clone())?;

            let obs_data: Vec<f32> = obs.bind(py).to_vec()?;
            observations.extend(obs_data);
            rewards.push(reward);
            dones.push(done);
            truncateds.push(truncated);
            infos.push(info);

            // Auto-reset if episode is done
            if done || truncated {
                let _ = env.reset(py)?;
            }
        }

        // Reshape observations to (num_envs, obs_dim)
        let obs_array = PyArray2::from_vec2_bound(
            py,
            &observations
                .chunks(self.envs[0].obs_dim)
                .map(|chunk| chunk.to_vec())
                .collect::<Vec<_>>(),
        )?;

        Ok((obs_array.unbind(), rewards, dones, truncateds, infos))
    }

    /// Get number of environments
    #[getter]
    fn num_envs(&self) -> usize {
        self.num_envs
    }

    /// Close all environments
    fn close(&mut self) -> PyResult<()> {
        for env in &mut self.envs {
            env.close()?;
        }
        Ok(())
    }
}

/// Utility function to create environment from config
#[cfg(feature = "python")]
#[pyfunction]
pub fn make_env(
    task_type: &str,
    obs_dim: Option<usize>,
    action_dim: Option<usize>,
) -> PyResult<PySim3DEnv> {
    // Default dimensions based on task type
    let (default_obs, default_act) = match task_type {
        "reaching" => (10, 6),
        "balancing" => (6, 1),
        "locomotion" => (22, 12),
        "navigation" => (21, 2),
        "manipulation" => (25, 4),
        "push" => (30, 2),
        _ => (10, 6),
    };

    PySim3DEnv::new(
        task_type,
        obs_dim.unwrap_or(default_obs),
        action_dim.unwrap_or(default_act),
    )
}

/// Utility function to create vectorized environment
#[cfg(feature = "python")]
#[pyfunction]
pub fn make_vec_env(
    task_type: &str,
    num_envs: usize,
    obs_dim: Option<usize>,
    action_dim: Option<usize>,
) -> PyResult<PyVecSim3DEnv> {
    let (default_obs, default_act) = match task_type {
        "reaching" => (10, 6),
        "balancing" => (6, 1),
        "locomotion" => (22, 12),
        "navigation" => (21, 2),
        "manipulation" => (25, 4),
        "push" => (30, 2),
        _ => (10, 6),
    };

    PyVecSim3DEnv::new(
        task_type,
        obs_dim.unwrap_or(default_obs),
        action_dim.unwrap_or(default_act),
        num_envs,
    )
}

// Note: Python module is defined in lib.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_dimensions() {
        // Verify default dimensions match task implementations
        let reaching_dims = (10, 6);
        let balancing_dims = (6, 1);
        let locomotion_dims = (22, 12);
        let navigation_dims = (21, 2);
        let manipulation_dims = (25, 4);
        let push_dims = (30, 2);

        // These should match the observation sizes in each task's get_observation
        assert_eq!(reaching_dims, (10, 6));
        assert_eq!(balancing_dims, (6, 1));
        assert_eq!(locomotion_dims, (22, 12));
        assert_eq!(navigation_dims, (21, 2));
        assert_eq!(manipulation_dims, (25, 4));
        assert_eq!(push_dims, (30, 2));
    }
}
