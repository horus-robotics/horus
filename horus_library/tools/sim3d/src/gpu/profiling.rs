//! GPU Performance Profiling and Memory Optimization

use super::GPUMetrics;
use bevy::prelude::*;
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

/// Memory pool for reusing GPU buffers
pub struct GPUBufferPool {
    // TODO: Implement buffer pooling to reduce allocation overhead
    _marker: std::marker::PhantomData<()>,
}

impl Default for GPUBufferPool {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl GPUBufferPool {
    pub fn new() -> Self {
        Self::default()
    }

    // TODO: Add buffer allocation/deallocation methods
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
