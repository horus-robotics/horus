use bevy::prelude::*;

/// Time control modes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeControlMode {
    /// Normal time progression
    Normal,
    /// Paused
    Paused,
    /// Slow motion
    SlowMotion,
    /// Fast forward
    FastForward,
    /// Frame-by-frame stepping
    FrameStepping,
}

/// Time control resource
#[derive(Resource, Clone, Debug)]
pub struct TimeControl {
    pub mode: TimeControlMode,
    pub time_scale: f32,
    pub paused: bool,
    pub step_frame: bool,
    pub accumulated_time: f64,
    pub frame_count: u64,
}

impl Default for TimeControl {
    fn default() -> Self {
        Self {
            mode: TimeControlMode::Normal,
            time_scale: 1.0,
            paused: false,
            step_frame: false,
            accumulated_time: 0.0,
            frame_count: 0,
        }
    }
}

impl TimeControl {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pause time
    pub fn pause(&mut self) {
        self.paused = true;
        self.mode = TimeControlMode::Paused;
    }

    /// Resume time
    pub fn resume(&mut self) {
        self.paused = false;
        self.mode = TimeControlMode::Normal;
    }

    /// Toggle pause
    pub fn toggle_pause(&mut self) {
        if self.paused {
            self.resume();
        } else {
            self.pause();
        }
    }

    /// Set slow motion (0.1x - 1.0x speed)
    pub fn set_slow_motion(&mut self, scale: f32) {
        self.time_scale = scale.clamp(0.01, 1.0);
        self.paused = false;
        self.mode = TimeControlMode::SlowMotion;
    }

    /// Set fast forward (1.0x - 10.0x speed)
    pub fn set_fast_forward(&mut self, scale: f32) {
        self.time_scale = scale.clamp(1.0, 10.0);
        self.paused = false;
        self.mode = TimeControlMode::FastForward;
    }

    /// Set normal speed
    pub fn set_normal_speed(&mut self) {
        self.time_scale = 1.0;
        self.paused = false;
        self.mode = TimeControlMode::Normal;
    }

    /// Set arbitrary time scale
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0);
        self.paused = false;

        self.mode = if scale < 1.0 {
            TimeControlMode::SlowMotion
        } else if scale > 1.0 {
            TimeControlMode::FastForward
        } else {
            TimeControlMode::Normal
        };
    }

    /// Enable frame stepping mode
    pub fn enable_frame_stepping(&mut self) {
        self.mode = TimeControlMode::FrameStepping;
        self.paused = true;
    }

    /// Step forward one frame
    pub fn step_frame_forward(&mut self) {
        if self.mode == TimeControlMode::FrameStepping || self.paused {
            self.step_frame = true;
        }
    }

    /// Get the effective time delta for this frame
    pub fn get_effective_delta(&self, base_delta: f32) -> f32 {
        if self.paused && !self.step_frame {
            0.0
        } else {
            base_delta * self.time_scale
        }
    }

    /// Update accumulated time
    pub fn update(&mut self, delta: f32) {
        if !self.paused || self.step_frame {
            self.accumulated_time += (delta * self.time_scale) as f64;
            self.frame_count += 1;
            self.step_frame = false;
        }
    }

    /// Reset accumulated time and frame count
    pub fn reset(&mut self) {
        self.accumulated_time = 0.0;
        self.frame_count = 0;
    }

    /// Get current FPS (frames per second)
    pub fn get_fps(&self, base_fps: f32) -> f32 {
        if self.paused && !self.step_frame {
            0.0
        } else {
            base_fps * self.time_scale
        }
    }
}

/// System to apply time control to Bevy's time
pub fn apply_time_control_system(
    mut time_control: ResMut<TimeControl>,
    mut time: ResMut<Time<Virtual>>,
) {
    if time_control.paused && !time_control.step_frame {
        time.pause();
    } else {
        time.unpause();
        time.set_relative_speed(time_control.time_scale);
    }

    // Update time control state
    let delta = time.delta_secs();
    time_control.update(delta);
}

/// Preset time scales
pub struct TimeScalePresets;

impl TimeScalePresets {
    pub const VERY_SLOW: f32 = 0.1;
    pub const SLOW: f32 = 0.25;
    pub const HALF_SPEED: f32 = 0.5;
    pub const NORMAL: f32 = 1.0;
    pub const DOUBLE_SPEED: f32 = 2.0;
    pub const QUAD_SPEED: f32 = 4.0;
    pub const MAX_SPEED: f32 = 10.0;
}

/// Keyframe for time control recording
#[derive(Clone, Debug)]
pub struct TimeKeyframe {
    pub time: f64,
    pub time_scale: f32,
    pub mode: TimeControlMode,
}

impl TimeKeyframe {
    pub fn new(time: f64, time_scale: f32, mode: TimeControlMode) -> Self {
        Self {
            time,
            time_scale,
            mode,
        }
    }
}

/// Time control recording (for scripted time manipulation)
#[derive(Resource, Clone, Debug, Default)]
pub struct TimeControlRecording {
    pub keyframes: Vec<TimeKeyframe>,
    pub playing: bool,
    pub current_index: usize,
    pub loop_playback: bool,
}

impl TimeControlRecording {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_keyframe(&mut self, keyframe: TimeKeyframe) {
        self.keyframes.push(keyframe);
        // Sort by time
        self.keyframes
            .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    pub fn play(&mut self) {
        self.playing = true;
        self.current_index = 0;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.current_index = 0;
    }

    pub fn clear(&mut self) {
        self.keyframes.clear();
        self.current_index = 0;
    }

    pub fn update(&mut self, current_time: f64, time_control: &mut TimeControl) {
        if !self.playing || self.keyframes.is_empty() {
            return;
        }

        // Find the appropriate keyframe
        while self.current_index < self.keyframes.len() {
            let keyframe = &self.keyframes[self.current_index];

            if current_time >= keyframe.time {
                // Apply this keyframe
                time_control.time_scale = keyframe.time_scale;
                time_control.mode = keyframe.mode;
                self.current_index += 1;
            } else {
                break;
            }
        }

        // Handle looping
        if self.current_index >= self.keyframes.len() {
            if self.loop_playback {
                self.current_index = 0;
            } else {
                self.playing = false;
            }
        }
    }
}

/// System to update time control recording
pub fn update_time_control_recording_system(
    time: Res<Time<Virtual>>,
    mut time_control: ResMut<TimeControl>,
    mut recording: ResMut<TimeControlRecording>,
) {
    recording.update(time.elapsed_secs_f64(), &mut time_control);
}

/// Time control statistics
#[derive(Clone, Debug)]
pub struct TimeStats {
    pub real_time_elapsed: f64,
    pub simulation_time_elapsed: f64,
    pub frame_count: u64,
    pub average_fps: f32,
    pub current_time_scale: f32,
}

impl TimeStats {
    pub fn from_time_control(time_control: &TimeControl, real_time: f64) -> Self {
        let average_fps = if real_time > 0.0 {
            time_control.frame_count as f32 / real_time as f32
        } else {
            0.0
        };

        Self {
            real_time_elapsed: real_time,
            simulation_time_elapsed: time_control.accumulated_time,
            frame_count: time_control.frame_count,
            average_fps,
            current_time_scale: time_control.time_scale,
        }
    }

    pub fn get_time_ratio(&self) -> f64 {
        if self.real_time_elapsed > 0.0 {
            self.simulation_time_elapsed / self.real_time_elapsed
        } else {
            0.0
        }
    }

    pub fn print_summary(&self) {
        println!("Time Statistics:");
        println!("  Real Time Elapsed: {:.2}s", self.real_time_elapsed);
        println!(
            "  Simulation Time Elapsed: {:.2}s",
            self.simulation_time_elapsed
        );
        println!("  Frame Count: {}", self.frame_count);
        println!("  Average FPS: {:.2}", self.average_fps);
        println!("  Current Time Scale: {:.2}x", self.current_time_scale);
        println!("  Time Ratio: {:.2}x", self.get_time_ratio());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_control_creation() {
        let tc = TimeControl::new();
        assert_eq!(tc.time_scale, 1.0);
        assert!(!tc.paused);
        assert_eq!(tc.mode, TimeControlMode::Normal);
    }

    #[test]
    fn test_pause_resume() {
        let mut tc = TimeControl::new();

        tc.pause();
        assert!(tc.paused);
        assert_eq!(tc.mode, TimeControlMode::Paused);

        tc.resume();
        assert!(!tc.paused);
        assert_eq!(tc.mode, TimeControlMode::Normal);
    }

    #[test]
    fn test_toggle_pause() {
        let mut tc = TimeControl::new();

        tc.toggle_pause();
        assert!(tc.paused);

        tc.toggle_pause();
        assert!(!tc.paused);
    }

    #[test]
    fn test_slow_motion() {
        let mut tc = TimeControl::new();

        tc.set_slow_motion(0.5);
        assert_eq!(tc.time_scale, 0.5);
        assert_eq!(tc.mode, TimeControlMode::SlowMotion);
        assert!(!tc.paused);
    }

    #[test]
    fn test_fast_forward() {
        let mut tc = TimeControl::new();

        tc.set_fast_forward(2.0);
        assert_eq!(tc.time_scale, 2.0);
        assert_eq!(tc.mode, TimeControlMode::FastForward);
        assert!(!tc.paused);
    }

    #[test]
    fn test_time_scale_clamping() {
        let mut tc = TimeControl::new();

        tc.set_slow_motion(0.001); // Too slow
        assert_eq!(tc.time_scale, 0.01); // Clamped to minimum

        tc.set_fast_forward(100.0); // Too fast
        assert_eq!(tc.time_scale, 10.0); // Clamped to maximum
    }

    #[test]
    fn test_effective_delta() {
        let mut tc = TimeControl::new();

        tc.set_time_scale(2.0);
        assert_eq!(tc.get_effective_delta(1.0), 2.0);

        tc.pause();
        assert_eq!(tc.get_effective_delta(1.0), 0.0);
    }

    #[test]
    fn test_frame_stepping() {
        let mut tc = TimeControl::new();

        tc.enable_frame_stepping();
        assert_eq!(tc.mode, TimeControlMode::FrameStepping);
        assert!(tc.paused);

        assert_eq!(tc.get_effective_delta(1.0), 0.0);

        tc.step_frame_forward();
        assert!(tc.step_frame);

        // After update, step_frame should be reset
        tc.update(1.0);
        assert!(!tc.step_frame);
    }

    #[test]
    fn test_accumulated_time() {
        let mut tc = TimeControl::new();

        tc.update(1.0);
        assert_eq!(tc.accumulated_time, 1.0);

        tc.set_time_scale(2.0);
        tc.update(1.0);
        assert_eq!(tc.accumulated_time, 3.0); // 1.0 + (1.0 * 2.0)
    }

    #[test]
    fn test_time_control_recording() {
        let mut recording = TimeControlRecording::new();

        recording.add_keyframe(TimeKeyframe::new(0.0, 1.0, TimeControlMode::Normal));
        recording.add_keyframe(TimeKeyframe::new(2.0, 0.5, TimeControlMode::SlowMotion));
        recording.add_keyframe(TimeKeyframe::new(5.0, 2.0, TimeControlMode::FastForward));

        assert_eq!(recording.keyframes.len(), 3);

        let mut tc = TimeControl::new();

        recording.play();
        recording.update(0.0, &mut tc);
        assert_eq!(tc.time_scale, 1.0);

        recording.update(2.5, &mut tc);
        assert_eq!(tc.time_scale, 0.5);

        recording.update(6.0, &mut tc);
        assert_eq!(tc.time_scale, 2.0);
    }

    #[test]
    fn test_time_stats() {
        let mut tc = TimeControl::new();

        tc.set_time_scale(2.0);
        tc.update(1.0);
        tc.update(1.0);
        tc.update(1.0);

        let stats = TimeStats::from_time_control(&tc, 3.0);

        assert_eq!(stats.simulation_time_elapsed, 6.0); // 3 updates * 1.0 * 2.0
        assert_eq!(stats.real_time_elapsed, 3.0);
        assert_eq!(stats.frame_count, 3);
        assert_eq!(stats.get_time_ratio(), 2.0);
    }

    #[test]
    fn test_reset() {
        let mut tc = TimeControl::new();

        tc.update(1.0);
        tc.update(1.0);

        assert_eq!(tc.frame_count, 2);
        assert_eq!(tc.accumulated_time, 2.0);

        tc.reset();

        assert_eq!(tc.frame_count, 0);
        assert_eq!(tc.accumulated_time, 0.0);
    }
}
