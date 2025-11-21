//! GPU Acceleration Module
//!
//! Provides GPU-accelerated physics and sensor processing using wgpu compute shaders.
//! Falls back to CPU when GPU is unavailable or when GPU acceleration provides no benefit.

pub mod benchmarks;
pub mod collision;
pub mod integration;
pub mod profiling;
pub mod raycasting;

// Re-export pipeline types
pub use collision::GPUCollisionPipeline;
pub use raycasting::GPURaycastPipeline;

use bevy::prelude::*;
use std::sync::Arc;

/// GPU acceleration configuration
#[derive(Resource, Clone, Debug)]
pub struct GPUAccelerationConfig {
    /// Enable GPU-accelerated collision detection
    pub enable_collision: bool,
    /// Enable GPU-accelerated sensor raycasting
    pub enable_raycasting: bool,
    /// Enable GPU-accelerated rigid body integration
    pub enable_integration: bool,
    /// Minimum number of objects before using GPU (below this, CPU is faster)
    pub min_objects_for_gpu: usize,
    /// GPU device preference (High Performance vs Low Power)
    pub power_preference: wgpu::PowerPreference,
    /// Enable multi-GPU support
    pub enable_multi_gpu: bool,
}

impl Default for GPUAccelerationConfig {
    fn default() -> Self {
        Self {
            enable_collision: true,
            enable_raycasting: true,
            enable_integration: false, // Disabled by default, experimental
            min_objects_for_gpu: 100,  // GPU overhead not worth it for < 100 objects
            power_preference: wgpu::PowerPreference::HighPerformance,
            enable_multi_gpu: false, // Disabled by default
        }
    }
}

/// GPU compute context
#[derive(Resource)]
pub struct GPUComputeContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub adapter_info: wgpu::AdapterInfo,
}

impl GPUComputeContext {
    /// Initialize GPU compute context
    pub async fn new(config: &GPUAccelerationConfig) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: config.power_preference,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find suitable GPU adapter")?;

        let adapter_info = adapter.get_info();
        tracing::info!(
            "Selected GPU: {} ({:?})",
            adapter_info.name,
            adapter_info.backend
        );

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("sim3d_compute_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to create GPU device: {}", e))?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
        })
    }

    /// Check if GPU should be used for given workload size
    pub fn should_use_gpu(&self, num_objects: usize, min_threshold: usize) -> bool {
        num_objects >= min_threshold
    }
}

/// GPU performance metrics
#[derive(Resource, Default, Debug, Clone)]
pub struct GPUMetrics {
    /// Time spent in GPU collision detection (microseconds)
    pub collision_time_us: f64,
    /// Time spent in GPU raycasting (microseconds)
    pub raycasting_time_us: f64,
    /// Time spent in GPU integration (microseconds)
    pub integration_time_us: f64,
    /// Number of GPU dispatches this frame
    pub num_dispatches: u32,
    /// GPU memory usage (bytes)
    pub memory_usage_bytes: u64,
}

impl GPUMetrics {
    pub fn reset(&mut self) {
        self.collision_time_us = 0.0;
        self.raycasting_time_us = 0.0;
        self.integration_time_us = 0.0;
        self.num_dispatches = 0;
    }

    pub fn total_time_us(&self) -> f64 {
        self.collision_time_us + self.raycasting_time_us + self.integration_time_us
    }
}

/// GPU acceleration system plugin
pub struct GPUAccelerationPlugin;

impl Plugin for GPUAccelerationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GPUAccelerationConfig>()
            .init_resource::<GPUMetrics>();

        // Initialize GPU context (async)
        // Note: This happens during app startup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = GPUAccelerationConfig::default();
        assert!(config.enable_collision);
        assert!(config.enable_raycasting);
        assert!(!config.enable_integration);
        assert_eq!(config.min_objects_for_gpu, 100);
    }

    #[test]
    fn test_metrics_reset() {
        let mut metrics = GPUMetrics::default();
        metrics.collision_time_us = 100.0;
        metrics.num_dispatches = 5;

        metrics.reset();

        assert_eq!(metrics.collision_time_us, 0.0);
        assert_eq!(metrics.num_dispatches, 0);
    }
}
