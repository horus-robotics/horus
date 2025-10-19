use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Python wrapper for HORUS Scheduler
///
/// The scheduler manages the execution of multiple nodes,
/// handling their lifecycle and coordinating their execution.
#[pyclass]
pub struct PyScheduler {
    nodes: Arc<Mutex<HashMap<String, PyObject>>>,
    running: Arc<Mutex<bool>>,
    tick_rate_hz: f64,
}

#[pymethods]
impl PyScheduler {
    #[new]
    pub fn new() -> PyResult<Self> {
        Ok(PyScheduler {
            nodes: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
            tick_rate_hz: 100.0, // Default 100Hz
        })
    }

    /// Add a node to the scheduler
    fn add_node(&mut self, py: Python, node: PyObject) -> PyResult<()> {
        // Extract node name
        let name: String = node.getattr(py, "name")?.extract(py)?;

        // Store the node
        let mut nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;
        nodes.insert(name.clone(), node);

        Ok(())
    }

    /// Remove a node from the scheduler
    fn remove_node(&mut self, name: String) -> PyResult<bool> {
        let mut nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;
        Ok(nodes.remove(&name).is_some())
    }

    /// Set the tick rate in Hz
    fn set_tick_rate(&mut self, rate_hz: f64) -> PyResult<()> {
        if rate_hz <= 0.0 || rate_hz > 10000.0 {
            return Err(PyRuntimeError::new_err(
                "Tick rate must be between 0 and 10000 Hz",
            ));
        }
        self.tick_rate_hz = rate_hz;
        Ok(())
    }

    /// Run the scheduler for a specified duration (in seconds)
    fn run_for(&mut self, py: Python, duration_seconds: f64) -> PyResult<()> {
        if duration_seconds <= 0.0 {
            return Err(PyRuntimeError::new_err("Duration must be positive"));
        }

        let tick_duration = Duration::from_secs_f64(1.0 / self.tick_rate_hz);
        let total_ticks = (duration_seconds * self.tick_rate_hz) as usize;

        // Set running flag
        {
            let mut running = self.running.lock().map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to lock running flag: {}", e))
            })?;
            *running = true;
        }

        // Initialize all nodes
        {
            let nodes = self
                .nodes
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

            for (name, node) in nodes.iter() {
                if let Err(e) = node.call_method0(py, "init") {
                    eprintln!("Failed to initialize node '{}': {:?}", name, e);
                }
            }
        }

        // Main execution loop
        for tick in 0..total_ticks {
            let tick_start = std::time::Instant::now();

            // Check if we should stop
            {
                let running = self.running.lock().map_err(|e| {
                    PyRuntimeError::new_err(format!("Failed to lock running flag: {}", e))
                })?;
                if !*running {
                    break;
                }
            }

            // Execute tick for all nodes
            {
                let nodes = self
                    .nodes
                    .lock()
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

                for (name, node) in nodes.iter() {
                    if let Err(e) = node.call_method0(py, "tick") {
                        eprintln!("Error in node '{}' tick: {:?}", name, e);
                    }
                }
            }

            // Sleep for remainder of tick period
            let elapsed = tick_start.elapsed();
            if elapsed < tick_duration {
                thread::sleep(tick_duration - elapsed);
            } else if tick % 100 == 0 {
                // Warn about timing issues every 100 ticks
                eprintln!(
                    "Warning: Tick {} took {:?}, longer than period {:?}",
                    tick, elapsed, tick_duration
                );
            }
        }

        // Shutdown all nodes
        {
            let nodes = self
                .nodes
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

            for (name, node) in nodes.iter() {
                if let Err(e) = node.call_method0(py, "shutdown") {
                    eprintln!("Failed to shutdown node '{}': {:?}", name, e);
                }
            }
        }

        // Clear running flag
        {
            let mut running = self.running.lock().map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to lock running flag: {}", e))
            })?;
            *running = false;
        }

        Ok(())
    }

    /// Run the scheduler indefinitely (until stop() is called)
    fn run(&mut self, py: Python) -> PyResult<()> {
        let tick_duration = Duration::from_secs_f64(1.0 / self.tick_rate_hz);

        // Set running flag
        {
            let mut running = self.running.lock().map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to lock running flag: {}", e))
            })?;
            *running = true;
        }

        // Initialize all nodes
        {
            let nodes = self
                .nodes
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

            for (name, node) in nodes.iter() {
                if let Err(e) = node.call_method0(py, "init") {
                    eprintln!("Failed to initialize node '{}': {:?}", name, e);
                }
            }
        }

        // Main execution loop
        loop {
            let tick_start = std::time::Instant::now();

            // Check if we should stop
            {
                let running = self.running.lock().map_err(|e| {
                    PyRuntimeError::new_err(format!("Failed to lock running flag: {}", e))
                })?;
                if !*running {
                    break;
                }
            }

            // Execute tick for all nodes
            {
                let nodes = self
                    .nodes
                    .lock()
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

                for (name, node) in nodes.iter() {
                    if let Err(e) = node.call_method0(py, "tick") {
                        eprintln!("Error in node '{}' tick: {:?}", name, e);
                    }
                }
            }

            // Sleep for remainder of tick period
            let elapsed = tick_start.elapsed();
            if elapsed < tick_duration {
                thread::sleep(tick_duration - elapsed);
            }
        }

        // Shutdown all nodes
        {
            let nodes = self
                .nodes
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

            for (name, node) in nodes.iter() {
                if let Err(e) = node.call_method0(py, "shutdown") {
                    eprintln!("Failed to shutdown node '{}': {:?}", name, e);
                }
            }
        }

        Ok(())
    }

    /// Stop the scheduler
    fn stop(&mut self) -> PyResult<()> {
        let mut running = self
            .running
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock running flag: {}", e)))?;
        *running = false;
        Ok(())
    }

    /// Check if the scheduler is running
    fn is_running(&self) -> PyResult<bool> {
        let running = self
            .running
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock running flag: {}", e)))?;
        Ok(*running)
    }

    /// Get list of registered node names
    fn get_nodes(&self) -> PyResult<Vec<String>> {
        let nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;
        Ok(nodes.keys().cloned().collect())
    }

    fn __repr__(&self) -> PyResult<String> {
        let nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;
        Ok(format!(
            "Scheduler(nodes={}, tick_rate={}Hz)",
            nodes.len(),
            self.tick_rate_hz
        ))
    }
}
