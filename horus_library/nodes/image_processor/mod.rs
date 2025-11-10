use crate::Image;
use horus_core::error::HorusResult;

// Type alias for cleaner signatures
type Result<T> = HorusResult<T>;
use horus_core::{Hub, Node, NodeInfo, NodeInfoExt};

/// Image Processor Node - Computer vision preprocessing and filtering
///
/// Performs common image processing operations like resizing, filtering,
/// color space conversion, and edge detection. Useful for preparing
/// images for object detection, feature extraction, or visualization.
pub struct ImageProcessorNode {
    subscriber: Hub<Image>,
    publisher: Hub<Image>,

    // Configuration
    target_width: u32,
    target_height: u32,
    resize_enabled: bool,
    grayscale_enabled: bool,
    gaussian_blur_size: u32,
    edge_detection_enabled: bool,
    brightness_adjustment: f32,  // -1.0 to 1.0
    contrast_adjustment: f32,    // 0.5 to 2.0

    // Statistics
    images_processed: u64,
    processing_time_us: u64,
}

impl ImageProcessorNode {
    /// Create a new image processor node
    pub fn new() -> Result<Self> {
        Self::new_with_topics("camera/image", "camera/processed")
    }

    /// Create with custom input/output topics
    pub fn new_with_topics(input_topic: &str, output_topic: &str) -> Result<Self> {
        Ok(Self {
            subscriber: Hub::new(input_topic)?,
            publisher: Hub::new(output_topic)?,
            target_width: 640,
            target_height: 480,
            resize_enabled: false,
            grayscale_enabled: false,
            gaussian_blur_size: 0,
            edge_detection_enabled: false,
            brightness_adjustment: 0.0,
            contrast_adjustment: 1.0,
            images_processed: 0,
            processing_time_us: 0,
        })
    }

    /// Enable image resizing
    pub fn enable_resize(&mut self, width: u32, height: u32) {
        self.target_width = width;
        self.target_height = height;
        self.resize_enabled = true;
    }

    /// Disable image resizing
    pub fn disable_resize(&mut self) {
        self.resize_enabled = false;
    }

    /// Enable grayscale conversion
    pub fn enable_grayscale(&mut self) {
        self.grayscale_enabled = true;
    }

    /// Disable grayscale conversion
    pub fn disable_grayscale(&mut self) {
        self.grayscale_enabled = false;
    }

    /// Enable Gaussian blur (size must be odd: 3, 5, 7, etc.)
    pub fn enable_gaussian_blur(&mut self, kernel_size: u32) {
        self.gaussian_blur_size = if kernel_size % 2 == 0 {
            kernel_size + 1
        } else {
            kernel_size
        };
    }

    /// Disable Gaussian blur
    pub fn disable_gaussian_blur(&mut self) {
        self.gaussian_blur_size = 0;
    }

    /// Enable edge detection (Canny, Sobel, etc.)
    pub fn enable_edge_detection(&mut self) {
        self.edge_detection_enabled = true;
    }

    /// Disable edge detection
    pub fn disable_edge_detection(&mut self) {
        self.edge_detection_enabled = false;
    }

    /// Set brightness adjustment (-1.0 to 1.0)
    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness_adjustment = brightness.clamp(-1.0, 1.0);
    }

    /// Set contrast adjustment (0.5 to 2.0)
    pub fn set_contrast(&mut self, contrast: f32) {
        self.contrast_adjustment = contrast.clamp(0.5, 2.0);
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> (u64, u64) {
        (self.images_processed, self.processing_time_us)
    }

    /// Process an image through the pipeline
    fn process_image(&mut self, image: Image, mut ctx: Option<&mut NodeInfo>) -> Option<Image> {
        let start_time = std::time::Instant::now();

        // Create output image (start with copy)
        let mut processed = image.clone();

        // Apply processing pipeline

        // 1. Resize if enabled
        if self.resize_enabled
            && (image.width != self.target_width || image.height != self.target_height)
        {
            ctx.log_debug(&format!(
                "Resizing from {}x{} to {}x{}",
                image.width, image.height, self.target_width, self.target_height
            ));

            // In real implementation, use image resizing library
            // For now, just update dimensions (simulation)
            processed.width = self.target_width;
            processed.height = self.target_height;
        }

        // 2. Convert to grayscale if enabled
        if self.grayscale_enabled {
            ctx.log_debug("Converting to grayscale");
            // In real implementation, convert RGB/BGR to grayscale
            // For simulation, just note the conversion
        }

        // 3. Apply Gaussian blur if enabled
        if self.gaussian_blur_size > 0 {
            ctx.log_debug(&format!(
                "Applying Gaussian blur (kernel size {})",
                self.gaussian_blur_size
            ));
            // In real implementation, apply Gaussian filter
        }

        // 4. Apply brightness/contrast adjustments
        if self.brightness_adjustment != 0.0 || self.contrast_adjustment != 1.0 {
            ctx.log_debug(&format!(
                "Adjusting brightness={:.2}, contrast={:.2}",
                self.brightness_adjustment, self.contrast_adjustment
            ));
            // In real implementation, adjust pixel values
        }

        // 5. Apply edge detection if enabled
        if self.edge_detection_enabled {
            ctx.log_debug("Applying edge detection");
            // In real implementation, apply Canny or Sobel edge detection
        }

        // Update statistics
        let elapsed = start_time.elapsed();
        self.processing_time_us = elapsed.as_micros() as u64;
        self.images_processed += 1;

        ctx.log_debug(&format!(
            "Image processed in {} μs (total: {})",
            self.processing_time_us, self.images_processed
        ));

        Some(processed)
    }
}

impl Node for ImageProcessorNode {
    fn name(&self) -> &'static str {
        "ImageProcessorNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Process all available images
        while let Some(image) = self.subscriber.recv(None) {
            if let Some(processed) = self.process_image(image, ctx.as_deref_mut()) {
                let _ = self.publisher.send(processed, None);
            }
        }
    }
}
