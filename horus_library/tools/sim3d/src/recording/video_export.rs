use bevy::prelude::*;
use std::path::PathBuf;

/// Video recording format
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoFormat {
    /// Image sequence (PNG)
    ImageSequence,
    /// Image sequence (JPEG)
    JpegSequence,
    /// Raw RGB frames (can be encoded externally)
    RawRGB,
}

/// Video recording configuration
#[derive(Resource, Clone, Debug)]
pub struct VideoRecordingConfig {
    pub format: VideoFormat,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub output_path: PathBuf,
    pub quality: u8, // 0-100 for JPEG
}

impl Default for VideoRecordingConfig {
    fn default() -> Self {
        Self {
            format: VideoFormat::ImageSequence,
            width: 1920,
            height: 1080,
            framerate: 30,
            output_path: PathBuf::from("recordings"),
            quality: 90,
        }
    }
}

impl VideoRecordingConfig {
    pub fn new(output_path: PathBuf, width: u32, height: u32) -> Self {
        Self {
            output_path,
            width,
            height,
            ..Self::default()
        }
    }

    pub fn with_framerate(mut self, framerate: u32) -> Self {
        self.framerate = framerate;
        self
    }

    pub fn with_format(mut self, format: VideoFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality.min(100);
        self
    }
}

/// Frame data for video recording
#[derive(Clone, Debug)]
pub struct VideoFrame {
    pub frame_number: u32,
    pub timestamp: f64,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGB8 or RGBA8 format
    pub format: ImageDataFormat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageDataFormat {
    RGB8,
    RGBA8,
}

impl VideoFrame {
    pub fn new(
        frame_number: u32,
        timestamp: f64,
        width: u32,
        height: u32,
        data: Vec<u8>,
        format: ImageDataFormat,
    ) -> Self {
        Self {
            frame_number,
            timestamp,
            width,
            height,
            data,
            format,
        }
    }

    /// Save frame as PNG
    pub fn save_as_png(&self, path: &PathBuf) -> anyhow::Result<()> {
        use image::{ImageBuffer, Rgb, Rgba};

        match self.format {
            ImageDataFormat::RGB8 => {
                let img =
                    ImageBuffer::<Rgb<u8>, _>::from_raw(self.width, self.height, self.data.clone())
                        .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;
                img.save(path)?;
            }
            ImageDataFormat::RGBA8 => {
                let img = ImageBuffer::<Rgba<u8>, _>::from_raw(
                    self.width,
                    self.height,
                    self.data.clone(),
                )
                .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;
                img.save(path)?;
            }
        }
        Ok(())
    }

    /// Save frame as JPEG
    pub fn save_as_jpeg(&self, path: &PathBuf, quality: u8) -> anyhow::Result<()> {
        use image::{codecs::jpeg::JpegEncoder, ImageBuffer, Rgb};

        let img = match self.format {
            ImageDataFormat::RGB8 => {
                ImageBuffer::<Rgb<u8>, _>::from_raw(self.width, self.height, self.data.clone())
                    .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?
            }
            ImageDataFormat::RGBA8 => {
                // Convert RGBA to RGB
                let rgb_data: Vec<u8> = self
                    .data
                    .chunks(4)
                    .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
                    .collect();
                ImageBuffer::<Rgb<u8>, _>::from_raw(self.width, self.height, rgb_data)
                    .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?
            }
        };

        let file = std::fs::File::create(path)?;
        let mut encoder = JpegEncoder::new_with_quality(file, quality);
        encoder.encode(
            img.as_raw(),
            self.width,
            self.height,
            image::ExtendedColorType::Rgb8,
        )?;
        Ok(())
    }
}

/// Video recorder resource
#[derive(Resource)]
pub struct VideoRecorder {
    pub active: bool,
    pub config: VideoRecordingConfig,
    pub frames: Vec<VideoFrame>,
    pub start_time: f64,
    pub frame_count: u32,
    pub last_frame_time: f64,
}

impl VideoRecorder {
    pub fn new(config: VideoRecordingConfig) -> Self {
        Self {
            active: false,
            config,
            frames: Vec::new(),
            start_time: 0.0,
            frame_count: 0,
            last_frame_time: 0.0,
        }
    }

    pub fn start_recording(&mut self, time: f64) {
        self.active = true;
        self.start_time = time;
        self.frame_count = 0;
        self.frames.clear();
        self.last_frame_time = -1.0;
    }

    pub fn stop_recording(&mut self) {
        self.active = false;
    }

    pub fn should_capture_frame(&mut self, current_time: f64) -> bool {
        if !self.active {
            return false;
        }

        let interval = 1.0 / self.config.framerate as f64;
        if current_time - self.last_frame_time >= interval {
            self.last_frame_time = current_time;
            true
        } else {
            false
        }
    }

    pub fn add_frame(&mut self, frame: VideoFrame) {
        if self.active {
            self.frames.push(frame);
            self.frame_count += 1;
        }
    }

    /// Export all frames to disk
    pub fn export_frames(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.config.output_path)?;

        for frame in &self.frames {
            let filename = match self.config.format {
                VideoFormat::ImageSequence => {
                    format!("frame_{:06}.png", frame.frame_number)
                }
                VideoFormat::JpegSequence => {
                    format!("frame_{:06}.jpg", frame.frame_number)
                }
                VideoFormat::RawRGB => {
                    format!("frame_{:06}.raw", frame.frame_number)
                }
            };

            let path = self.config.output_path.join(filename);

            match self.config.format {
                VideoFormat::ImageSequence => {
                    frame.save_as_png(&path)?;
                }
                VideoFormat::JpegSequence => {
                    frame.save_as_jpeg(&path, self.config.quality)?;
                }
                VideoFormat::RawRGB => {
                    std::fs::write(&path, &frame.data)?;
                }
            }
        }

        // Write metadata file
        let metadata = VideoMetadata {
            width: self.config.width,
            height: self.config.height,
            framerate: self.config.framerate,
            frame_count: self.frame_count,
            duration: self.frames.last().map(|f| f.timestamp).unwrap_or(0.0) - self.start_time,
            format: self.config.format,
        };
        let metadata_path = self.config.output_path.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(metadata_path, metadata_json)?;

        Ok(())
    }

    pub fn get_duration(&self) -> f64 {
        if let Some(last_frame) = self.frames.last() {
            last_frame.timestamp - self.start_time
        } else {
            0.0
        }
    }

    pub fn get_estimated_size_mb(&self) -> f64 {
        if self.frames.is_empty() {
            return 0.0;
        }

        let bytes_per_frame = match self.config.format {
            VideoFormat::ImageSequence => {
                // PNG compression is roughly 30-50% of raw
                (self.config.width * self.config.height * 3) as f64 * 0.4
            }
            VideoFormat::JpegSequence => {
                // JPEG compression varies with quality
                let compression_ratio = 1.0 - (self.config.quality as f64 / 100.0) * 0.9;
                (self.config.width * self.config.height * 3) as f64 * compression_ratio
            }
            VideoFormat::RawRGB => (self.config.width * self.config.height * 3) as f64,
        };

        (bytes_per_frame * self.frames.len() as f64) / (1024.0 * 1024.0)
    }
}

/// Video metadata for export
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub frame_count: u32,
    pub duration: f64,
    pub format: VideoFormat,
}

impl serde::Serialize for VideoFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            VideoFormat::ImageSequence => serializer.serialize_str("ImageSequence"),
            VideoFormat::JpegSequence => serializer.serialize_str("JpegSequence"),
            VideoFormat::RawRGB => serializer.serialize_str("RawRGB"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for VideoFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "ImageSequence" => Ok(VideoFormat::ImageSequence),
            "JpegSequence" => Ok(VideoFormat::JpegSequence),
            "RawRGB" => Ok(VideoFormat::RawRGB),
            _ => Err(serde::de::Error::custom("Invalid video format")),
        }
    }
}

/// Screenshot capture component
#[derive(Component, Clone, Debug)]
pub struct ScreenshotCapture {
    pub enabled: bool,
    pub save_path: Option<PathBuf>,
}

impl Default for ScreenshotCapture {
    fn default() -> Self {
        Self {
            enabled: false,
            save_path: None,
        }
    }
}

impl ScreenshotCapture {
    pub fn new(save_path: PathBuf) -> Self {
        Self {
            enabled: true,
            save_path: Some(save_path),
        }
    }

    pub fn take_screenshot(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let path = self
            .save_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No save path set"))?;

        use image::{ImageBuffer, Rgb};
        let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, image_data.to_vec())
            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;

        std::fs::create_dir_all(path.parent().unwrap())?;
        img.save(path)?;

        self.enabled = false; // One-shot screenshot
        Ok(())
    }
}

/// Helper to convert frame data for different uses
pub struct FrameConverter;

impl FrameConverter {
    /// Convert RGBA to RGB
    pub fn rgba_to_rgb(rgba_data: &[u8]) -> Vec<u8> {
        rgba_data
            .chunks(4)
            .flat_map(|rgba| [rgba[0], rgba[1], rgba[2]])
            .collect()
    }

    /// Convert RGB to RGBA (add alpha channel)
    pub fn rgb_to_rgba(rgb_data: &[u8], alpha: u8) -> Vec<u8> {
        rgb_data
            .chunks(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], alpha])
            .collect()
    }

    /// Flip image vertically (OpenGL convention)
    pub fn flip_vertical(data: &[u8], width: u32, height: u32, channels: u32) -> Vec<u8> {
        let row_size = (width * channels) as usize;
        let mut flipped = vec![0u8; data.len()];

        for y in 0..height {
            let src_offset = (y * width * channels) as usize;
            let dst_offset = ((height - 1 - y) * width * channels) as usize;
            flipped[dst_offset..dst_offset + row_size]
                .copy_from_slice(&data[src_offset..src_offset + row_size]);
        }

        flipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_config_creation() {
        let config = VideoRecordingConfig::default();
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.framerate, 30);
    }

    #[test]
    fn test_video_config_builder() {
        let config = VideoRecordingConfig::new(PathBuf::from("test"), 1280, 720)
            .with_framerate(60)
            .with_format(VideoFormat::JpegSequence)
            .with_quality(85);

        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
        assert_eq!(config.framerate, 60);
        assert_eq!(config.quality, 85);
        assert_eq!(config.format, VideoFormat::JpegSequence);
    }

    #[test]
    fn test_video_recorder_creation() {
        let config = VideoRecordingConfig::default();
        let recorder = VideoRecorder::new(config);
        assert!(!recorder.active);
        assert_eq!(recorder.frame_count, 0);
    }

    #[test]
    fn test_video_recorder_start_stop() {
        let config = VideoRecordingConfig::default();
        let mut recorder = VideoRecorder::new(config);

        recorder.start_recording(0.0);
        assert!(recorder.active);
        assert_eq!(recorder.start_time, 0.0);

        recorder.stop_recording();
        assert!(!recorder.active);
    }

    #[test]
    fn test_should_capture_frame() {
        let config = VideoRecordingConfig::default().with_framerate(30);
        let mut recorder = VideoRecorder::new(config);

        recorder.start_recording(0.0);
        assert!(recorder.should_capture_frame(0.0));
        assert!(!recorder.should_capture_frame(0.01)); // Too soon
        assert!(recorder.should_capture_frame(0.034)); // >1/30 seconds passed
    }

    #[test]
    fn test_add_frame() {
        let config = VideoRecordingConfig::default();
        let mut recorder = VideoRecorder::new(config);

        recorder.start_recording(0.0);

        let frame = VideoFrame::new(0, 0.0, 100, 100, vec![0; 30000], ImageDataFormat::RGB8);
        recorder.add_frame(frame);

        assert_eq!(recorder.frame_count, 1);
        assert_eq!(recorder.frames.len(), 1);
    }

    #[test]
    fn test_get_duration() {
        let config = VideoRecordingConfig::default();
        let mut recorder = VideoRecorder::new(config);

        recorder.start_recording(0.0);

        let frame1 = VideoFrame::new(0, 0.0, 100, 100, vec![0; 30000], ImageDataFormat::RGB8);
        let frame2 = VideoFrame::new(1, 1.0, 100, 100, vec![0; 30000], ImageDataFormat::RGB8);

        recorder.add_frame(frame1);
        recorder.add_frame(frame2);

        assert_eq!(recorder.get_duration(), 1.0);
    }

    #[test]
    fn test_frame_converter_rgba_to_rgb() {
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255]; // Red and Green with alpha
        let rgb = FrameConverter::rgba_to_rgb(&rgba);
        assert_eq!(rgb, vec![255, 0, 0, 0, 255, 0]);
    }

    #[test]
    fn test_frame_converter_rgb_to_rgba() {
        let rgb = vec![255, 0, 0, 0, 255, 0]; // Red and Green
        let rgba = FrameConverter::rgb_to_rgba(&rgb, 255);
        assert_eq!(rgba, vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn test_frame_converter_flip_vertical() {
        // 2x2 RGB image
        let data = vec![
            255, 0, 0, 0, 255, 0, // Row 0: Red, Green
            0, 0, 255, 255, 255, 0, // Row 1: Blue, Yellow
        ];
        let flipped = FrameConverter::flip_vertical(&data, 2, 2, 3);
        assert_eq!(
            flipped,
            vec![
                0, 0, 255, 255, 255, 0, // Row 1 becomes Row 0
                255, 0, 0, 0, 255, 0, // Row 0 becomes Row 1
            ]
        );
    }

    #[test]
    fn test_screenshot_capture() {
        let mut capture = ScreenshotCapture::default();
        assert!(!capture.enabled);

        capture.enabled = true;
        capture.save_path = Some(PathBuf::from("/tmp/test_screenshot.png"));
        assert!(capture.enabled);
    }

    #[test]
    fn test_estimated_size() {
        let config = VideoRecordingConfig::default();
        let mut recorder = VideoRecorder::new(config);

        recorder.start_recording(0.0);

        for i in 0..30 {
            let frame = VideoFrame::new(
                i,
                i as f64 / 30.0,
                1920,
                1080,
                vec![0; 1920 * 1080 * 3],
                ImageDataFormat::RGB8,
            );
            recorder.add_frame(frame);
        }

        let size_mb = recorder.get_estimated_size_mb();
        assert!(size_mb > 0.0);
        assert!(size_mb < 1000.0); // Reasonable upper bound
    }
}
