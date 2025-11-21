//! GPU/CPU Hybrid Acceleration for Physics and Sensors
//!
//! Provides automatic fallback between GPU and CPU based on workload size
//! and hardware availability.

use crate::gpu::{
    GPUAccelerationConfig, GPUCollisionPipeline, GPUComputeContext, GPUMetrics, GPURaycastPipeline,
};
use bevy::prelude::*;

/// GPU-accelerated physics adapter (with CPU fallback)
#[derive(Resource)]
pub struct GPUPhysicsAdapter {
    context: Option<GPUComputeContext>,
    collision_pipeline: Option<GPUCollisionPipeline>,
    raycast_pipeline: Option<GPURaycastPipeline>,
    config: GPUAccelerationConfig,
    initialization_failed: bool,
}

impl Default for GPUPhysicsAdapter {
    fn default() -> Self {
        Self {
            context: None,
            collision_pipeline: None,
            raycast_pipeline: None,
            config: GPUAccelerationConfig::default(),
            initialization_failed: false,
        }
    }
}

impl GPUPhysicsAdapter {
    /// Initialize GPU acceleration (async)
    pub async fn initialize(config: GPUAccelerationConfig) -> Self {
        match GPUComputeContext::new(&config).await {
            Ok(context) => {
                tracing::info!("GPU acceleration initialized successfully");

                let collision_pipeline = if config.enable_collision {
                    Some(GPUCollisionPipeline::new(&context))
                } else {
                    None
                };

                let raycast_pipeline = if config.enable_raycasting {
                    Some(GPURaycastPipeline::new(&context))
                } else {
                    None
                };

                Self {
                    context: Some(context),
                    collision_pipeline,
                    raycast_pipeline,
                    config,
                    initialization_failed: false,
                }
            }
            Err(e) => {
                tracing::warn!(
                    "GPU acceleration initialization failed: {}. Falling back to CPU.",
                    e
                );
                Self {
                    context: None,
                    collision_pipeline: None,
                    raycast_pipeline: None,
                    config,
                    initialization_failed: true,
                }
            }
        }
    }

    /// Check if GPU is available and should be used
    pub fn should_use_gpu(&self, num_objects: usize) -> bool {
        if self.initialization_failed || self.context.is_none() {
            return false;
        }

        num_objects >= self.config.min_objects_for_gpu
    }

    /// Perform raycast (GPU if beneficial, otherwise CPU fallback)
    pub fn cast_rays(
        &self,
        rays: &[(Vec3, Vec3)],
        triangles: &[[Vec3; 3]],
        max_distance: f32,
        metrics: &mut GPUMetrics,
    ) -> Vec<f32> {
        let num_rays = rays.len();

        // Use GPU if available and beneficial
        if self.should_use_gpu(num_rays) {
            if let (Some(context), Some(pipeline)) = (&self.context, &self.raycast_pipeline) {
                let start = std::time::Instant::now();
                let result = pipeline.cast_rays(context, rays, triangles, max_distance);
                let elapsed_us = start.elapsed().as_micros() as f64;

                metrics.raycasting_time_us += elapsed_us;
                metrics.num_dispatches += 1;

                tracing::trace!(
                    "GPU raycast: {} rays in {:.2}µs ({:.0} rays/ms)",
                    num_rays,
                    elapsed_us,
                    (num_rays as f64) / (elapsed_us / 1000.0)
                );

                return result;
            }
        }

        // CPU fallback (current behavior)
        tracing::trace!("Using CPU raycast fallback for {} rays", num_rays);
        cpu_raycast_fallback(rays, triangles, max_distance)
    }

    /// Get GPU info string for debugging
    pub fn get_info(&self) -> String {
        if let Some(ref context) = self.context {
            format!(
                "GPU: {} ({:?})",
                context.adapter_info.name, context.adapter_info.backend
            )
        } else if self.initialization_failed {
            "GPU: Not available (initialization failed)".to_string()
        } else {
            "GPU: Not initialized".to_string()
        }
    }
}

/// CPU fallback for raycasting
fn cpu_raycast_fallback(
    rays: &[(Vec3, Vec3)],
    triangles: &[[Vec3; 3]],
    max_distance: f32,
) -> Vec<f32> {
    rays.iter()
        .map(|(origin, direction)| {
            let mut min_dist = max_distance;
            for tri in triangles {
                if let Some(dist) = ray_triangle_intersect(*origin, *direction, tri) {
                    if dist < min_dist {
                        min_dist = dist;
                    }
                }
            }
            min_dist
        })
        .collect()
}

/// Möller-Trumbore ray-triangle intersection
fn ray_triangle_intersect(origin: Vec3, direction: Vec3, tri: &[Vec3; 3]) -> Option<f32> {
    let epsilon = 1e-8;
    let edge1 = tri[1] - tri[0];
    let edge2 = tri[2] - tri[0];
    let h = direction.cross(edge2);
    let a = edge1.dot(h);

    if a.abs() < epsilon {
        return None;
    }

    let f = 1.0 / a;
    let s = origin - tri[0];
    let u = f * s.dot(h);

    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * direction.dot(q);

    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);

    if t > epsilon {
        Some(t)
    } else {
        None
    }
}

/// System to initialize GPU acceleration
pub fn setup_gpu_acceleration(mut commands: Commands, config: Res<GPUAccelerationConfig>) {
    tracing::info!("Setting up GPU acceleration...");

    // Note: We can't use async in Bevy systems directly,
    // so we initialize synchronously or use a task pool
    let adapter = pollster::block_on(GPUPhysicsAdapter::initialize((*config).clone()));

    tracing::info!("{}", adapter.get_info());
    commands.insert_resource(adapter);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_raycast_fallback() {
        let rays = vec![(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 1.0))];

        let triangles = vec![[
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]];

        let results = cpu_raycast_fallback(&rays, &triangles, 100.0);
        assert_eq!(results.len(), 1);
        assert!((results[0] - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_ray_triangle_intersect() {
        let origin = Vec3::new(0.0, 0.0, -5.0);
        let direction = Vec3::new(0.0, 0.0, 1.0);
        let triangle = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        let result = ray_triangle_intersect(origin, direction, &triangle);
        assert!(result.is_some());
        assert!((result.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_ray_miss() {
        let origin = Vec3::new(10.0, 0.0, -5.0);
        let direction = Vec3::new(0.0, 0.0, 1.0);
        let triangle = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        let result = ray_triangle_intersect(origin, direction, &triangle);
        assert!(result.is_none());
    }
}
