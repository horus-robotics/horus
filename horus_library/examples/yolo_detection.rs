// YOLO Object Detection Example
//
// Demonstrates real-time object detection using YOLOv8 models.
// This example shows how to:
// - Set up a YOLO detector node
// - Process camera images
// - Receive and display detection results
//
// Usage:
//   cargo run --example yolo_detection --features ml-inference

use horus_core::{Hub, Node, NodeInfo, Scheduler};
use horus_library::messages::{DetectionArray, Image, ImageEncoding};
use horus_library::nodes::{YOLOConfig, YOLOv8DetectorNode};

/// Simple camera simulator node for testing
struct CameraSimNode {
    image_pub: Hub<Image>,
    frame_count: u64,
}

impl CameraSimNode {
    fn new() -> horus_core::HorusResult<Self> {
        Ok(Self {
            image_pub: Hub::new("camera/raw")?,
            frame_count: 0,
        })
    }
}

impl Node for CameraSimNode {
    fn name(&self) -> &'static str {
        "CameraSimNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        // Simulate 640x480 RGB image
        let width = 640;
        let height = 480;
        let channels = 3;
        let data = vec![128u8; width * height * channels];

        let image = Image {
            width: width as u32,
            height: height as u32,
            encoding: ImageEncoding::Rgb8,
            step: (width * channels) as u32,
            data,
            frame_id: [0u8; 32],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
        };

        let _ = self.image_pub.send(image, &mut ctx);
        self.frame_count += 1;

        if self.frame_count % 30 == 0 {
            println!("Camera: Sent {} frames", self.frame_count);
        }
    }
}

/// Detection display node
struct DetectionDisplayNode {
    detection_sub: Hub<DetectionArray>,
}

impl DetectionDisplayNode {
    fn new() -> horus_core::HorusResult<Self> {
        Ok(Self {
            detection_sub: Hub::new("vision/detections")?,
        })
    }
}

impl Node for DetectionDisplayNode {
    fn name(&self) -> &'static str {
        "DetectionDisplayNode"
    }

    fn tick(&mut self, mut ctx: Option<&mut NodeInfo>) {
        while let Some(detections) = self.detection_sub.recv(&mut ctx) {
            println!("\n=== Detections ===");
            println!("Found {} objects:", detections.count);

            for i in 0..(detections.count as usize) {
                let det = &detections.detections[i];

                // Convert class_name from [u8; 32] to String
                let class_name = String::from_utf8_lossy(&det.class_name)
                    .trim_end_matches('\0')
                    .to_string();

                println!(
                    "  [{}] {} - {:.2}% confidence at ({}, {}) size {}x{}",
                    i + 1,
                    class_name,
                    det.confidence * 100.0,
                    det.bbox.x_offset,
                    det.bbox.y_offset,
                    det.bbox.width,
                    det.bbox.height
                );
            }
            println!("==================\n");
        }
    }
}

fn main() -> horus_core::HorusResult<()> {
    println!("HORUS YOLOv8 Object Detection Example");
    println!("======================================\n");

    // Check if model file exists
    let model_path = "models/yolov8n.onnx";
    if !std::path::Path::new(model_path).exists() {
        println!("ERROR: Model file not found: {}", model_path);
        println!("\nTo download YOLOv8n ONNX model:");
        println!("  mkdir -p models");
        println!("  wget https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov8n.onnx -O models/yolov8n.onnx");
        println!(
            "\nOr use any YOLOv8 ONNX model from https://github.com/ultralytics/ultralytics\n"
        );
        return Ok(());
    }

    println!("Initializing nodes...");

    // Create nodes
    let camera = CameraSimNode::new()?;

    let yolo_config = YOLOConfig {
        conf_threshold: 0.25,
        iou_threshold: 0.45,
        input_size: 640,
        use_gpu: false,
        num_threads: 4,
    };

    let detector =
        YOLOv8DetectorNode::new(model_path, "camera/raw", "vision/detections", yolo_config)?;

    let display = DetectionDisplayNode::new()?;

    println!("Starting scheduler...\n");

    // Create scheduler and add nodes
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(camera), 0, Some(false));
    scheduler.add(Box::new(detector), 1, Some(true));
    scheduler.add(Box::new(display), 2, Some(false));

    // Run for 10 seconds
    println!("Running detection for 10 seconds...\n");
    scheduler.run_for(std::time::Duration::from_secs(10))?;

    println!("\nExample completed successfully!");
    Ok(())
}
