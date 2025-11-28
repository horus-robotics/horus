//! GPU Performance Profiling and Memory Optimization

use super::GPUMetrics;
use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// GPU profiling session for tracking performance over time
#[derive(Resource, Default)]
pub struct GPUProfilingSession {
    /// Rolling average of GPU frame time (microseconds)
    pub avg_gpu_time_us: f64,
    /// Rolling average of dispatch count
    pub avg_dispatches: f32,
    /// Peak GPU memory usage observed (bytes)
    pub peak_memory_bytes: u64,
    /// Number of frames profiled
    pub frames_profiled: u32,
    /// GPU vs CPU decision history (for adaptive thresholds)
    pub gpu_usage_ratio: f32,
}

impl GPUProfilingSession {
    /// Update profiling stats with latest frame metrics
    pub fn update(&mut self, metrics: &GPUMetrics) {
        const ROLLING_ALPHA: f64 = 0.1; // EMA smoothing factor

        let total_time = metrics.total_time_us();

        // Update rolling averages
        if self.frames_profiled == 0 {
            self.avg_gpu_time_us = total_time;
            self.avg_dispatches = metrics.num_dispatches as f32;
        } else {
            self.avg_gpu_time_us =
                self.avg_gpu_time_us * (1.0 - ROLLING_ALPHA) + total_time * ROLLING_ALPHA;
            self.avg_dispatches = self.avg_dispatches * (1.0 - ROLLING_ALPHA as f32)
                + metrics.num_dispatches as f32 * ROLLING_ALPHA as f32;
        }

        // Track peak memory
        if metrics.memory_usage_bytes > self.peak_memory_bytes {
            self.peak_memory_bytes = metrics.memory_usage_bytes;
        }

        // Update usage ratio
        let gpu_active = if metrics.num_dispatches > 0 { 1.0 } else { 0.0 };
        self.gpu_usage_ratio = self.gpu_usage_ratio * 0.95 + gpu_active * 0.05;

        self.frames_profiled += 1;
    }

    /// Get a summary report for debugging
    pub fn get_report(&self) -> String {
        format!(
            "GPU Profiling Report:\n\
             - Frames Profiled: {}\n\
             - Avg GPU Time: {:.2}µs\n\
             - Avg Dispatches: {:.1}/frame\n\
             - Peak Memory: {:.2}MB\n\
             - GPU Usage: {:.1}%",
            self.frames_profiled,
            self.avg_gpu_time_us,
            self.avg_dispatches,
            (self.peak_memory_bytes as f64) / (1024.0 * 1024.0),
            self.gpu_usage_ratio * 100.0
        )
    }

    /// Check if GPU is being underutilized
    pub fn is_gpu_underutilized(&self) -> bool {
        self.frames_profiled > 100 && self.gpu_usage_ratio < 0.1
    }

    /// Check if GPU is showing performance benefits
    pub fn is_gpu_beneficial(&self) -> bool {
        self.frames_profiled > 100 && self.avg_dispatches > 0.5
    }
}

/// System to update GPU profiling every frame
pub fn update_gpu_profiling_system(
    metrics: Res<GPUMetrics>,
    mut profiling: ResMut<GPUProfilingSession>,
) {
    profiling.update(&metrics);
}

/// System to print GPU profiling report periodically
pub fn report_gpu_profiling_system(
    time: Res<Time>,
    profiling: Res<GPUProfilingSession>,
    mut last_report: Local<f32>,
) {
    const REPORT_INTERVAL_SECS: f32 = 10.0;

    let current_time = time.elapsed_secs();
    if current_time - *last_report >= REPORT_INTERVAL_SECS {
        *last_report = current_time;

        if profiling.frames_profiled > 0 {
            tracing::info!("{}", profiling.get_report());

            if profiling.is_gpu_underutilized() {
                tracing::warn!(
                    "GPU acceleration is underutilized ({}% usage). Consider disabling for this workload.",
                    profiling.gpu_usage_ratio * 100.0
                );
            }
        }
    }
}

/// GPU buffer descriptor for pool management
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BufferDescriptor {
    /// Size of the buffer in bytes
    pub size: usize,
    /// Usage flags (vertex, index, uniform, storage)
    pub usage: BufferUsage,
    /// Alignment requirements
    pub alignment: usize,
}

/// Buffer usage flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferUsage {
    Vertex,
    Index,
    Uniform,
    Storage,
    Staging,
}

/// Pooled GPU buffer handle
pub struct PooledBuffer {
    /// Unique identifier
    id: usize,
    /// Buffer descriptor
    descriptor: BufferDescriptor,
    /// Actual buffer data (simulated as Vec<u8> for CPU-side pooling)
    data: Vec<u8>,
    /// Last accessed timestamp
    last_accessed: Instant,
}

/// Memory pool for reusing GPU buffers
pub struct GPUBufferPool {
    /// Available buffers organized by descriptor
    available_buffers: Arc<Mutex<Vec<PooledBuffer>>>,
    /// Currently allocated buffers
    allocated_buffers: Arc<Mutex<Vec<PooledBuffer>>>,
    /// Next buffer ID
    next_id: Arc<Mutex<usize>>,
    /// Maximum pool size in bytes
    max_pool_size: usize,
    /// Current pool size in bytes
    current_size: Arc<Mutex<usize>>,
    /// Allocation statistics
    stats: Arc<Mutex<AllocationStats>>,
}

/// Buffer allocation statistics
#[derive(Default, Debug)]
struct AllocationStats {
    allocations: usize,
    deallocations: usize,
    pool_hits: usize,
    pool_misses: usize,
    bytes_allocated: usize,
    bytes_reused: usize,
}

impl Default for GPUBufferPool {
    fn default() -> Self {
        Self::with_max_size(256 * 1024 * 1024) // 256 MB default pool size
    }
}

impl GPUBufferPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            available_buffers: Arc::new(Mutex::new(Vec::new())),
            allocated_buffers: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(0)),
            max_pool_size: max_size,
            current_size: Arc::new(Mutex::new(0)),
            stats: Arc::new(Mutex::new(AllocationStats::default())),
        }
    }

    /// Allocate a buffer from the pool or create a new one
    pub fn allocate(&self, descriptor: BufferDescriptor) -> Result<usize, String> {
        let mut available = self.available_buffers.lock().unwrap();
        let mut allocated = self.allocated_buffers.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Try to find a matching buffer in the pool
        let reused_index = available.iter().position(|buffer| {
            buffer.descriptor.size >= descriptor.size
                && buffer.descriptor.usage == descriptor.usage
                && buffer.descriptor.alignment == descriptor.alignment
        });

        let buffer = if let Some(index) = reused_index {
            // Reuse existing buffer from pool
            let mut buffer = available.remove(index);
            buffer.last_accessed = Instant::now();
            stats.pool_hits += 1;
            stats.bytes_reused += buffer.descriptor.size;
            buffer
        } else {
            // Check if we have space for a new allocation
            let current_size = *self.current_size.lock().unwrap();
            if current_size + descriptor.size > self.max_pool_size {
                // Try to evict oldest unused buffers
                self.evict_oldest_buffers(descriptor.size)?;
            }

            // Create new buffer
            let mut next_id = self.next_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;

            let buffer = PooledBuffer {
                id,
                descriptor: descriptor.clone(),
                data: vec![0u8; descriptor.size],
                last_accessed: Instant::now(),
            };

            *self.current_size.lock().unwrap() += descriptor.size;
            stats.pool_misses += 1;
            stats.bytes_allocated += descriptor.size;
            buffer
        };

        let buffer_id = buffer.id;
        allocated.push(buffer);
        stats.allocations += 1;

        Ok(buffer_id)
    }

    /// Release a buffer back to the pool
    pub fn deallocate(&self, buffer_id: usize) -> Result<(), String> {
        let mut allocated = self.allocated_buffers.lock().unwrap();
        let mut available = self.available_buffers.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Find the buffer in allocated list
        let buffer_index = allocated
            .iter()
            .position(|b| b.id == buffer_id)
            .ok_or_else(|| format!("Buffer {} not found in allocated pool", buffer_id))?;

        let mut buffer = allocated.remove(buffer_index);
        buffer.last_accessed = Instant::now();

        // Clear buffer data for security
        buffer.data.fill(0);

        // Return to available pool
        available.push(buffer);
        stats.deallocations += 1;

        Ok(())
    }

    /// Evict oldest buffers to make space
    fn evict_oldest_buffers(&self, required_space: usize) -> Result<(), String> {
        let mut available = self.available_buffers.lock().unwrap();
        let mut current_size = self.current_size.lock().unwrap();

        // Sort by last accessed time (oldest first)
        available.sort_by_key(|b| b.last_accessed);

        let mut freed_space = 0;
        while freed_space < required_space && !available.is_empty() {
            let buffer = available.remove(0);
            freed_space += buffer.descriptor.size;
            *current_size -= buffer.descriptor.size;
        }

        if freed_space < required_space {
            Err(format!(
                "Cannot allocate {} bytes: pool exhausted (max: {} bytes)",
                required_space, self.max_pool_size
            ))
        } else {
            Ok(())
        }
    }

    /// Clear all unused buffers from the pool
    pub fn clear_unused(&self) {
        let mut available = self.available_buffers.lock().unwrap();
        let mut current_size = self.current_size.lock().unwrap();

        for buffer in available.drain(..) {
            *current_size -= buffer.descriptor.size;
        }
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> String {
        let stats = self.stats.lock().unwrap();
        let available_count = self.available_buffers.lock().unwrap().len();
        let allocated_count = self.allocated_buffers.lock().unwrap().len();
        let current_size = *self.current_size.lock().unwrap();

        format!(
            "GPUBufferPool Stats:\n\
             - Pool Size: {:.2} MB / {:.2} MB\n\
             - Buffers: {} allocated, {} available\n\
             - Allocations: {} total, {} hits ({:.1}% hit rate)\n\
             - Memory: {:.2} MB allocated, {:.2} MB reused",
            current_size as f64 / (1024.0 * 1024.0),
            self.max_pool_size as f64 / (1024.0 * 1024.0),
            allocated_count,
            available_count,
            stats.allocations,
            stats.pool_hits,
            if stats.allocations > 0 {
                (stats.pool_hits as f64 / stats.allocations as f64) * 100.0
            } else {
                0.0
            },
            stats.bytes_allocated as f64 / (1024.0 * 1024.0),
            stats.bytes_reused as f64 / (1024.0 * 1024.0),
        )
    }

    /// Defragment the pool by consolidating free space
    pub fn defragment(&self) {
        let mut available = self.available_buffers.lock().unwrap();

        // Sort buffers by size to improve allocation efficiency
        available.sort_by_key(|b| b.descriptor.size);

        // Merge adjacent buffers with same usage if possible
        let mut i = 0;
        while i < available.len().saturating_sub(1) {
            if available[i].descriptor.usage == available[i + 1].descriptor.usage
                && available[i].descriptor.alignment == available[i + 1].descriptor.alignment
            {
                // Merge buffers
                let next_buffer = available.remove(i + 1);
                available[i].descriptor.size += next_buffer.descriptor.size;
                available[i].data.extend(next_buffer.data);
            } else {
                i += 1;
            }
        }
    }
}

/// Performance timer for GPU operations
pub struct GPUTimer {
    start: Instant,
    label: String,
}

impl GPUTimer {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            label: label.into(),
        }
    }

    pub fn elapsed_us(&self) -> f64 {
        self.start.elapsed().as_micros() as f64
    }

    pub fn log_elapsed(&self) {
        tracing::debug!("{}: {:.2}µs", self.label, self.elapsed_us());
    }
}

impl Drop for GPUTimer {
    fn drop(&mut self) {
        self.log_elapsed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiling_session_update() {
        let mut session = GPUProfilingSession::default();
        let mut metrics = GPUMetrics::default();

        metrics.raycasting_time_us = 100.0;
        metrics.num_dispatches = 5;
        metrics.memory_usage_bytes = 1024 * 1024;

        session.update(&metrics);

        assert_eq!(session.frames_profiled, 1);
        assert_eq!(session.avg_gpu_time_us, 100.0);
        assert_eq!(session.peak_memory_bytes, 1024 * 1024);
    }

    #[test]
    fn test_rolling_average() {
        let mut session = GPUProfilingSession::default();
        let mut metrics = GPUMetrics::default();

        // First frame
        metrics.raycasting_time_us = 100.0;
        session.update(&metrics);
        assert_eq!(session.avg_gpu_time_us, 100.0);

        // Second frame (should be smoothed)
        metrics.raycasting_time_us = 200.0;
        session.update(&metrics);
        assert!(session.avg_gpu_time_us > 100.0 && session.avg_gpu_time_us < 200.0);
    }

    #[test]
    fn test_gpu_timer() {
        let timer = GPUTimer::new("test_operation");
        std::thread::sleep(std::time::Duration::from_micros(100));
        assert!(timer.elapsed_us() >= 100.0);
    }
}
