//! GPU-accelerated collision detection using compute shaders

use super::GPUComputeContext;
use bevy::prelude::*;
use wgpu::util::DeviceExt;

/// GPU collision detection pipeline
pub struct GPUCollisionPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GPUCollisionPipeline {
    pub fn new(context: &GPUComputeContext) -> Self {
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("collision_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/broad_phase.wgsl").into()),
            });

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("collision_bind_group_layout"),
                    entries: &[
                        // AABB buffer (input)
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
                        // Collision pairs (output)
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Uniforms (num objects, etc.)
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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
                    label: Some("collision_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("collision_pipeline"),
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

    /// Perform broad-phase collision detection on GPU
    pub fn detect_collisions(
        &self,
        context: &GPUComputeContext,
        aabbs: &[[f32; 6]], // [min_x, min_y, min_z, max_x, max_y, max_z]
    ) -> Vec<(u32, u32)> {
        let num_objects = aabbs.len();

        // Create GPU buffers
        let aabb_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("aabb_buffer"),
                contents: bytemuck::cast_slice(aabbs),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Output buffer for collision pairs (worst case: all pairs)
        let max_pairs = (num_objects * (num_objects - 1)) / 2;
        let output_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("collision_pairs_buffer"),
            size: (max_pairs * 8) as u64, // 2 u32s per pair
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Uniforms
        let uniforms = [num_objects as u32, 0, 0, 0]; // Padding for alignment
        let uniform_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("collision_uniforms"),
                contents: bytemuck::cast_slice(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group
        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("collision_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: aabb_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });

        // Create command encoder
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("collision_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("collision_pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch work groups (64 threads per group)
            let workgroup_size = 64;
            let num_workgroups = (num_objects + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(num_workgroups as u32, 1, 1);
        }

        // Read back results
        let staging_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_buffer"),
            size: (max_pairs * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (max_pairs * 8) as u64,
        );

        context.queue.submit(Some(encoder.finish()));

        // Map and read results
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        context.device.poll(wgpu::Maintain::Wait);
        futures::executor::block_on(receiver).unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let pairs: &[u32] = bytemuck::cast_slice(&data);

        // Parse collision pairs (first element is count)
        let num_pairs = pairs[0] as usize;
        let mut result = Vec::with_capacity(num_pairs);
        for i in 0..num_pairs {
            let idx = (i * 2) + 1;
            result.push((pairs[idx], pairs[idx + 1]));
        }

        drop(data);
        staging_buffer.unmap();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_overlap() {
        // Test basic AABB overlap logic
        let aabb1 = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let aabb2 = [0.5, 0.5, 0.5, 1.5, 1.5, 1.5];

        // Should overlap
        assert!(aabbs_overlap(&aabb1, &aabb2));
    }

    fn aabbs_overlap(a: &[f32; 6], b: &[f32; 6]) -> bool {
        a[3] >= b[0] && a[0] <= b[3] && // X overlap
        a[4] >= b[1] && a[1] <= b[4] && // Y overlap
        a[5] >= b[2] && a[2] <= b[5] // Z overlap
    }
}
