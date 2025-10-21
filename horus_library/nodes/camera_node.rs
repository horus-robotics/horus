use crate::vision::ImageEncoding;
use crate::{CameraInfo, CompressedImage, Image};
use horus_core::error::HorusResult;
use horus_core::{Hub, Node, NodeInfo};
use std::time::{SystemTime, UNIX_EPOCH};

/// Camera Node - Generic camera interface for vision input
///
/// Captures images from various camera sources and publishes Image/CompressedImage messages.
/// Supports multiple backends (OpenCV, V4L2) and configurable image parameters.
pub struct CameraNode {
    publisher: Hub<Image>,
    compressed_publisher: Hub<CompressedImage>,
    info_publisher: Hub<CameraInfo>,

    // Configuration
    device_id: u32,
    width: u32,
    height: u32,
    fps: f32,
    encoding: ImageEncoding,
    compress_images: bool,
    quality: u8,

    // State
    is_initialized: bool,
    frame_count: u64,
    last_frame_time: u64,

    #[cfg(feature = "opencv-backend")]
    capture: Option<opencv::videoio::VideoCapture>,
}

impl CameraNode {
    /// Create a new camera node with default topic "camera/image"
    pub fn new() -> HorusResult<Self> {
        Self::new_with_topic("camera")
    }

    /// Create a new camera node with custom topic prefix
    pub fn new_with_topic(topic_prefix: &str) -> HorusResult<Self> {
        let image_topic = format!("{}/image", topic_prefix);
        let compressed_topic = format!("{}/image/compressed", topic_prefix);
        let info_topic = format!("{}/camera_info", topic_prefix);

        Ok(Self {
            publisher: Hub::new(&image_topic)?,
            compressed_publisher: Hub::new(&compressed_topic)?,
            info_publisher: Hub::new(&info_topic)?,

            device_id: 0,
            width: 640,
            height: 480,
            fps: 30.0,
            encoding: ImageEncoding::Bgr8,
            compress_images: false,
            quality: 90,

            is_initialized: false,
            frame_count: 0,
            last_frame_time: 0,

            #[cfg(feature = "opencv-backend")]
            capture: None,
        })
    }

    /// Set camera device ID (0 for default camera)
    pub fn set_device_id(&mut self, device_id: u32) {
        self.device_id = device_id;
    }

    /// Set image resolution
    pub fn set_resolution(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Set capture framerate
    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps.max(1.0).min(120.0);
    }

    /// Set image encoding format
    pub fn set_encoding(&mut self, encoding: ImageEncoding) {
        self.encoding = encoding;
    }

    /// Enable/disable image compression
    pub fn set_compression(&mut self, enabled: bool, quality: u8) {
        self.compress_images = enabled;
        self.quality = quality.min(100);
    }

    /// Get current frame rate (frames per second)
    pub fn get_actual_fps(&self) -> f32 {
        if self.frame_count < 2 {
            return 0.0;
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let time_diff = current_time - self.last_frame_time;
        if time_diff > 0 {
            1000.0 / time_diff as f32
        } else {
            0.0
        }
    }

    /// Get total frames captured
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    #[cfg(feature = "opencv-backend")]
    fn initialize_opencv(&mut self) -> bool {
        use opencv::videoio::VideoCaptureProperties::{
            CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FRAME_WIDTH,
        };
        use opencv::videoio::{VideoCapture, CAP_ANY};

        match VideoCapture::new(self.device_id as i32, CAP_ANY) {
            Ok(mut cap) => {
                // Set camera properties
                let _ = cap.set(CAP_PROP_FRAME_WIDTH, self.width as f64);
                let _ = cap.set(CAP_PROP_FRAME_HEIGHT, self.height as f64);
                let _ = cap.set(CAP_PROP_FPS, self.fps as f64);

                if cap.is_opened().unwrap_or(false) {
                    self.capture = Some(cap);
                    self.publish_camera_info();
                    return true;
                }
            }
            Err(_) => {}
        }
        false
    }

    #[cfg(not(feature = "opencv-backend"))]
    fn initialize_opencv(&mut self) -> bool {
        false
    }

    #[cfg(feature = "v4l2-backend")]
    fn initialize_v4l2(&mut self) -> bool {
        // V4L2 implementation would go here
        false
    }

    #[cfg(not(feature = "v4l2-backend"))]
    fn initialize_v4l2(&mut self) -> bool {
        false
    }

    fn initialize_camera(&mut self) -> bool {
        if self.is_initialized {
            return true;
        }

        // Try different backends
        #[cfg(feature = "opencv-backend")]
        if self.initialize_opencv() {
            self.is_initialized = true;
            return true;
        }

        #[cfg(feature = "v4l2-backend")]
        if self.initialize_v4l2() {
            self.is_initialized = true;
            return true;
        }

        false
    }

    #[cfg(feature = "opencv-backend")]
    fn capture_opencv_frame(&mut self) -> Option<Vec<u8>> {
        use opencv::core::Mat;

        if let Some(ref mut cap) = self.capture {
            let mut frame = Mat::default();
            if cap.read(&mut frame).unwrap_or(false) && !frame.empty() {
                // Convert Mat to Vec<u8>
                if let Some(bytes) = frame.data_bytes() {
                    return Some(bytes.to_vec());
                }
            }
        }
        None
    }

    #[cfg(not(feature = "opencv-backend"))]
    fn capture_opencv_frame(&mut self) -> Option<Vec<u8>> {
        None
    }

    fn capture_frame(&mut self) -> Option<Vec<u8>> {
        #[cfg(feature = "opencv-backend")]
        if let Some(data) = self.capture_opencv_frame() {
            return Some(data);
        }

        // Fallback: Generate test pattern
        self.generate_test_pattern()
    }

    fn generate_test_pattern(&self) -> Option<Vec<u8>> {
        // Generate a simple test pattern (alternating colors)
        let bytes_per_pixel = match self.encoding {
            ImageEncoding::Mono8 => 1,
            ImageEncoding::Rgb8 | ImageEncoding::Bgr8 => 3,
            ImageEncoding::Rgba8 | ImageEncoding::Bgra8 => 4,
            _ => 3,
        };

        let total_bytes = (self.width * self.height * bytes_per_pixel as u32) as usize;
        let mut data = vec![0u8; total_bytes];

        // Create a simple gradient pattern
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = ((y * self.width + x) * bytes_per_pixel as u32) as usize;
                let intensity = ((x + y + self.frame_count as u32) % 256) as u8;

                match self.encoding {
                    ImageEncoding::Mono8 => {
                        data[idx] = intensity;
                    }
                    ImageEncoding::Rgb8 => {
                        data[idx] = intensity; // R
                        data[idx + 1] = 255 - intensity; // G
                        data[idx + 2] = intensity / 2; // B
                    }
                    ImageEncoding::Bgr8 => {
                        data[idx] = intensity / 2; // B
                        data[idx + 1] = 255 - intensity; // G
                        data[idx + 2] = intensity; // R
                    }
                    _ => {
                        data[idx] = intensity;
                        data[idx + 1] = 255 - intensity;
                        data[idx + 2] = 128;
                    }
                }
            }
        }

        Some(data)
    }

    fn publish_image(&self, data: Vec<u8>) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let image = Image::new(self.width, self.height, self.encoding, data);

        let _ = self.publisher.send(image, None);
    }

    fn publish_camera_info(&self) {
        let camera_info = CameraInfo::new(
            self.width,
            self.height,
            800.0,                    // fx
            800.0,                    // fy
            self.width as f64 / 2.0,  // cx
            self.height as f64 / 2.0, // cy
        );
        let _ = self.info_publisher.send(camera_info, None);
    }
}

impl Node for CameraNode {
    fn name(&self) -> &'static str {
        "CameraNode"
    }

    fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
        // Initialize camera on first tick
        if !self.is_initialized && !self.initialize_camera() {
            return; // Skip if initialization failed
        }

        // Capture and publish frame
        if let Some(data) = self.capture_frame() {
            self.frame_count += 1;
            self.last_frame_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

            self.publish_image(data);

            // Publish camera info periodically
            if self.frame_count.is_multiple_of(30) {
                self.publish_camera_info();
            }
        }
    }
}

// Default impl removed - use CameraNode::new() instead which returns HorusResult
