use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use horus::NodeInfo as CoreNodeInfo;
use crate::node::PyNodeInfo;

/// Registered node with priority, logging, and per-node rate control
struct RegisteredNode {
    node: PyObject,
    name: String,
    priority: u32,
    logging_enabled: bool,
    context: Arc<Mutex<CoreNodeInfo>>,
    cached_info: Option<Py<PyNodeInfo>>, // Cache PyNodeInfo to avoid creating new ones every tick
    rate_hz: f64,  // Phase 1: Per-node rate control
    last_tick: Instant, // Phase 1: Track last execution time
}

/// Python wrapper for HORUS Scheduler with per-node rate control
///
/// The scheduler manages the execution of multiple nodes,
/// handling their lifecycle and coordinating their execution.
/// Supports per-node rate control for flexible scheduling.
#[pyclass]
pub struct PyScheduler {
    nodes: Arc<Mutex<Vec<RegisteredNode>>>,
    running: Arc<Mutex<bool>>,
    tick_rate_hz: f64, // Global scheduler tick rate
}

#[pymethods]
impl PyScheduler {
    #[new]
    pub fn new() -> PyResult<Self> {
        Ok(PyScheduler {
            nodes: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(Mutex::new(false)),
            tick_rate_hz: 100.0, // Default 100Hz
        })
    }

    /// Register a node with priority, logging, and optional rate control
    #[pyo3(signature = (node, priority, logging_enabled, rate_hz=None))]
    fn register(&mut self, py: Python, node: PyObject, priority: u32, logging_enabled: bool, rate_hz: Option<f64>) -> PyResult<()> {
        // Extract node name
        let name: String = node.getattr(py, "name")?.extract(py)?;

        // Create NodeInfo context for this node
        let context = Arc::new(Mutex::new(CoreNodeInfo::new(name.clone(), logging_enabled)));

        // Use provided rate or default to global scheduler rate
        let node_rate = rate_hz.unwrap_or(self.tick_rate_hz);

        // Store the registered node
        let mut nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

        nodes.push(RegisteredNode {
            node,
            name: name.clone(),
            priority,
            logging_enabled,
            context,
            cached_info: None, // Will be created on first use
            rate_hz: node_rate, // Phase 1: Per-node rate
            last_tick: Instant::now(), // Phase 1: Initialize timestamp
        });

        println!(
            "Registered node '{}' with priority {} (logging: {}, rate: {}Hz)",
            name, priority, logging_enabled, node_rate
        );

        Ok(())
    }

    /// Add a node to the scheduler (backward compatibility - uses default priority)
    fn add_node(&mut self, py: Python, node: PyObject) -> PyResult<()> {
        // Use current count as priority to maintain insertion order
        let priority = {
            let nodes = self
                .nodes
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;
            nodes.len() as u32
        };

        self.register(py, node, priority, false, None)
    }

    /// Phase 1: Set per-node rate control
    fn set_node_rate(&mut self, node_name: String, rate_hz: f64) -> PyResult<()> {
        if rate_hz <= 0.0 || rate_hz > 10000.0 {
            return Err(PyRuntimeError::new_err(
                "Rate must be between 0 and 10000 Hz",
            ));
        }

        let mut nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

        for registered in nodes.iter_mut() {
            if registered.name == node_name {
                registered.rate_hz = rate_hz;
                println!("Set node '{}' rate to {}Hz", node_name, rate_hz);
                return Ok(());
            }
        }

        Err(PyRuntimeError::new_err(format!(
            "Node '{}' not found",
            node_name
        )))
    }

    /// Phase 1: Get node statistics
    fn get_node_stats(&self, py: Python, node_name: String) -> PyResult<PyObject> {
        let nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

        for registered in nodes.iter() {
            if registered.name == node_name {
                let dict = PyDict::new_bound(py);
                dict.set_item("name", &registered.name)?;
                dict.set_item("priority", registered.priority)?;
                dict.set_item("rate_hz", registered.rate_hz)?;
                dict.set_item("logging_enabled", registered.logging_enabled)?;

                // Get metrics from NodeInfo
                if let Ok(ctx) = registered.context.lock() {
                    let metrics = ctx.metrics();
                    dict.set_item("total_ticks", metrics.total_ticks)?;
                    dict.set_item("errors_count", metrics.errors_count)?;
                }

                return Ok(dict.into());
            }
        }

        Err(PyRuntimeError::new_err(format!(
            "Node '{}' not found",
            node_name
        )))
    }

    /// Remove a node from the scheduler
    fn remove_node(&mut self, name: String) -> PyResult<bool> {
        let mut nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

        let original_len = nodes.len();
        nodes.retain(|n| n.name != name);
        Ok(nodes.len() < original_len)
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

            for registered in nodes.iter() {
                let py_info = Py::new(py, PyNodeInfo {
                    inner: registered.context.clone(),
                })?;

                // Try calling with NodeInfo parameter first, fallback to no-arg version
                let result = registered.node.call_method1(py, "init", (py_info,))
                    .or_else(|_| registered.node.call_method0(py, "init"));

                if let Err(e) = result {
                    eprintln!("Failed to initialize node '{}': {:?}", registered.name, e);
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

            // Execute tick for all nodes in priority order
            {
                let mut nodes = self
                    .nodes
                    .lock()
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

                // Sort by priority (lower number = higher priority)
                nodes.sort_by_key(|r| r.priority);

                for registered in nodes.iter_mut() {
                    // Phase 1: Check if enough time has elapsed for this node's rate
                    let now = Instant::now();
                    let elapsed_secs = (now - registered.last_tick).as_secs_f64();
                    let period_secs = 1.0 / registered.rate_hz;

                    // Skip this node if not enough time has passed
                    if elapsed_secs < period_secs {
                        continue;
                    }

                    // Update last tick time
                    registered.last_tick = now;

                    // Start tick timing
                    if let Ok(mut ctx) = registered.context.lock() {
                        ctx.start_tick();
                    }

                    // Get or create cached PyNodeInfo
                    let py_info = if let Some(ref cached) = registered.cached_info {
                        cached.clone_ref(py)
                    } else {
                        let new_info = Py::new(py, PyNodeInfo {
                            inner: registered.context.clone(),
                        })?;
                        registered.cached_info = Some(new_info.clone_ref(py));
                        new_info
                    };

                    // Try calling with NodeInfo parameter first, fallback to no-arg version
                    let result = registered.node.call_method1(py, "tick", (py_info,))
                        .or_else(|_| registered.node.call_method0(py, "tick"));

                    if let Err(e) = result {
                        eprintln!("Error in node '{}' tick: {:?}", registered.name, e);
                    }

                    // Record tick completion
                    if let Ok(mut ctx) = registered.context.lock() {
                        ctx.record_tick();
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

            for registered in nodes.iter() {
                let py_info = Py::new(py, PyNodeInfo {
                    inner: registered.context.clone(),
                })?;

                // Try calling with NodeInfo parameter first, fallback to no-arg version
                let result = registered.node.call_method1(py, "shutdown", (py_info,))
                    .or_else(|_| registered.node.call_method0(py, "shutdown"));

                if let Err(e) = result {
                    eprintln!("Failed to shutdown node '{}': {:?}", registered.name, e);
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

            for registered in nodes.iter() {
                let py_info = Py::new(py, PyNodeInfo {
                    inner: registered.context.clone(),
                })?;

                // Try calling with NodeInfo parameter first, fallback to no-arg version
                let result = registered.node.call_method1(py, "init", (py_info,))
                    .or_else(|_| registered.node.call_method0(py, "init"));

                if let Err(e) = result {
                    eprintln!("Failed to initialize node '{}': {:?}", registered.name, e);
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

            // Execute tick for all nodes in priority order
            {
                let mut nodes = self
                    .nodes
                    .lock()
                    .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

                // Sort by priority (lower number = higher priority)
                nodes.sort_by_key(|r| r.priority);

                for registered in nodes.iter_mut() {
                    // Phase 1: Check if enough time has elapsed for this node's rate
                    let now = Instant::now();
                    let elapsed_secs = (now - registered.last_tick).as_secs_f64();
                    let period_secs = 1.0 / registered.rate_hz;

                    // Skip this node if not enough time has passed
                    if elapsed_secs < period_secs {
                        continue;
                    }

                    // Update last tick time
                    registered.last_tick = now;

                    // Start tick timing
                    if let Ok(mut ctx) = registered.context.lock() {
                        ctx.start_tick();
                    }

                    // Get or create cached PyNodeInfo
                    let py_info = if let Some(ref cached) = registered.cached_info {
                        cached.clone_ref(py)
                    } else {
                        let new_info = Py::new(py, PyNodeInfo {
                            inner: registered.context.clone(),
                        })?;
                        registered.cached_info = Some(new_info.clone_ref(py));
                        new_info
                    };

                    // Try calling with NodeInfo parameter first, fallback to no-arg version
                    let result = registered.node.call_method1(py, "tick", (py_info,))
                        .or_else(|_| registered.node.call_method0(py, "tick"));

                    if let Err(e) = result {
                        eprintln!("Error in node '{}' tick: {:?}", registered.name, e);
                    }

                    // Record tick completion
                    if let Ok(mut ctx) = registered.context.lock() {
                        ctx.record_tick();
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

            for registered in nodes.iter() {
                let py_info = Py::new(py, PyNodeInfo {
                    inner: registered.context.clone(),
                })?;

                // Try calling with NodeInfo parameter first, fallback to no-arg version
                let result = registered.node.call_method1(py, "shutdown", (py_info,))
                    .or_else(|_| registered.node.call_method0(py, "shutdown"));

                if let Err(e) = result {
                    eprintln!("Failed to shutdown node '{}': {:?}", registered.name, e);
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
        Ok(nodes.iter().map(|n| n.name.clone()).collect())
    }

    /// Get node information including priority and logging settings
    fn get_node_info(&self, name: String) -> PyResult<Option<(u32, bool)>> {
        let nodes = self
            .nodes
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to lock nodes: {}", e)))?;

        for registered in nodes.iter() {
            if registered.name == name {
                return Ok(Some((registered.priority, registered.logging_enabled)));
            }
        }
        Ok(None)
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
