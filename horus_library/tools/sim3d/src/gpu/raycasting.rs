//! GPU-accelerated sensor raycasting using compute shaders

use super::GPUComputeContext;
use bevy::prelude::*;
use wgpu::util::DeviceExt;

/// Raycast uniforms matching WGSL layout
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct RaycastUniforms {
    num_rays: u32,
    num_triangles: u32,
    max_distance: f32,
    padding: f32,
}

/// GPU raycasting pipeline for LiDAR/depth sensors
pub struct GPURaycastPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GPURaycastPipeline {
    pub fn new(context: &GPUComputeContext) -> Self {
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("raycast_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/raycast.wgsl").into()),
            });

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("raycast_bind_group_layout"),
                    entries: &[
                        // Ray origins (input)
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Ray directions (input)
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Triangle data (input)
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Hit distances (output)
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("raycast_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("raycast_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    /// Perform batch raycasting on GPU
    pub fn cast_rays(
        &self,
        context: &GPUComputeContext,
        rays: &[(Vec3, Vec3)],   // (origin, direction) pairs
        triangles: &[[Vec3; 3]], // Triangle vertices
        max_distance: f32,
    ) -> Vec<f32> {
        let num_rays = rays.len();
        let num_triangles = triangles.len();

        // Flatten ray data
        let mut ray_origins = Vec::with_capacity(num_rays * 3);
        let mut ray_directions = Vec::with_capacity(num_rays * 3);
        for (origin, direction) in rays {
            ray_origins.extend_from_slice(&[origin.x, origin.y, origin.z]);
            ray_directions.extend_from_slice(&[direction.x, direction.y, direction.z]);
        }

        // Flatten triangle data
        let mut triangle_data = Vec::with_capacity(num_triangles * 9);
        for tri in triangles {
            for vertex in tri {
                triangle_data.extend_from_slice(&[vertex.x, vertex.y, vertex.z]);
            }
        }

        // Create GPU buffers
        let origin_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ray_origins"),
                contents: bytemuck::cast_slice(&ray_origins),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let direction_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ray_directions"),
                    contents: bytemuck::cast_slice(&ray_directions),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let triangle_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("triangles"),
                    contents: bytemuck::cast_slice(&triangle_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let output_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hit_distances"),
            size: (num_rays * 4) as u64, // f32 per ray
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Uniforms
        let uniforms = RaycastUniforms {
            num_rays: num_rays as u32,
            num_triangles: num_triangles as u32,
            max_distance,
            padding: 0.0,
        };
        let uniform_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("raycast_uniforms"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group
        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raycast_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: origin_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: direction_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: triangle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });

        // Execute compute shader
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("raycast_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("raycast_pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            let workgroup_size = 64;
            let num_workgroups = (num_rays + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        // Read results
        let staging_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_buffer"),
            size: (num_rays * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, (num_rays * 4) as u64);

        context.queue.submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        context.device.poll(wgpu::Maintain::Wait);
        futures::executor::block_on(receiver).unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let distances: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        staging_buffer.unmap();

        distances
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_triangle_intersection() {
        // Basic ray-triangle intersection test
        let origin = Vec3::new(0.0, 0.0, -5.0);
        let direction = Vec3::new(0.0, 0.0, 1.0);

        let triangle = [
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        // Should hit at distance 5.0
        assert!(ray_hits_triangle(origin, direction, &triangle));
    }

    fn ray_hits_triangle(origin: Vec3, direction: Vec3, tri: &[Vec3; 3]) -> bool {
        // Möller-Trumbore algorithm
        let edge1 = tri[1] - tri[0];
        let edge2 = tri[2] - tri[0];
        let h = direction.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < 1e-8 {
            return false;
        }

        let f = 1.0 / a;
        let s = origin - tri[0];
        let u = f * s.dot(h);

        if u < 0.0 || u > 1.0 {
            return false;
        }

        let q = s.cross(edge1);
        let v = f * direction.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return false;
        }

        let t = f * edge2.dot(q);
        t > 1e-8
    }
}
