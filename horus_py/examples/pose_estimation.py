#!/usr/bin/env python3
"""
HORUS Pose Estimation Example (Python)

Demonstrates human pose estimation using MediaPipe/MoveNet models.
Shows how to use Python ML utilities with HORUS.

Usage:
    python3 examples/ml_inference/pose_estimation.py
"""

import sys
import numpy as np
from horus import Node, run
from horus.ml_utils import ONNXInferenceNode, PerformanceMonitor

class PoseEstimationExample(ONNXInferenceNode):
    """
    Pose estimation node using MoveNet or MediaPipe models.
    """

    def __init__(self, model_path="models/movenet_lightning.onnx"):
        super().__init__(
            model_path=model_path,
            input_topic="camera/raw",
            output_topic="poses",
            device="cpu"
        )
        self.monitor = PerformanceMonitor(window_size=30)
        self.frame_count = 0

    def load_model(self):
        """Load MoveNet/MediaPipe ONNX model"""
        try:
            import onnxruntime as ort
        except ImportError:
            print("ERROR: onnxruntime not installed")
            print("Install with: pip install onnxruntime")
            sys.exit(1)

        providers = ['CPUExecutionProvider']
        self.session = ort.InferenceSession(self.model_path, providers=providers)
        self.input_name = self.session.get_inputs()[0].name

        print(f"Model loaded: {self.model_path}")
        print(f"Input: {self.input_name}")

    def preprocess(self, image_data):
        """Preprocess image for pose model (192x192 or 256x256)"""
        # Simulate image data for demo
        # In real use, this would be actual camera image
        img = np.random.randint(0, 255, (192, 192, 3), dtype=np.uint8)

        # Normalize to [-1, 1]
        img = (img.astype(np.float32) / 127.5) - 1.0

        # Add batch dimension and transpose to NCHW
        img = np.transpose(img, (2, 0, 1))
        img = np.expand_dims(img, 0)

        return img

    def infer(self, preprocessed):
        """Run pose estimation inference"""
        outputs = self.session.run(None, {self.input_name: preprocessed})
        return outputs[0]

    def postprocess(self, output):
        """Parse pose keypoints from model output"""
        # MoveNet output format: [1, 17, 3] (keypoints, [y, x, confidence])
        keypoints = []

        num_keypoints = output.shape[1] if len(output.shape) > 1 else 0

        for i in range(min(num_keypoints, 17)):
            y, x, conf = output[0, i, 0], output[0, i, 1], output[0, i, 2]

            if conf > 0.3:  # Confidence threshold
                keypoints.append({
                    'id': i,
                    'x': float(x) * 640,  # Scale to image width
                    'y': float(y) * 480,  # Scale to image height
                    'confidence': float(conf)
                })

        return keypoints


class CameraSimulator(Node):
    """Simulates camera input for testing"""

    def __init__(self):
        super().__init__(
            name="camera_sim",
            pubs=["camera/raw"],
            tick=self.simulate_frame,
            rate=30
        )
        self.frame_num = 0

    def simulate_frame(self, node):
        """Generate simulated frame"""
        # Simulate sending image data
        image_data = np.random.randint(0, 255, (480, 640, 3), dtype=np.uint8)
        node.send("camera/raw", image_data.tobytes())

        self.frame_num += 1
        if self.frame_num % 30 == 0:
            print(f"Camera: Sent {self.frame_num} frames")


class PoseDisplay(Node):
    """Display pose estimation results"""

    def __init__(self):
        super().__init__(
            name="pose_display",
            subs=["poses"],
            tick=self.display_pose,
            rate=10
        )
        self.pose_count = 0

    def display_pose(self, node):
        """Display detected poses"""
        if node.has_msg("poses"):
            keypoints = node.get("poses")
            self.pose_count += 1

            if isinstance(keypoints, list) and len(keypoints) > 0:
                print(f"\n=== Pose {self.pose_count} ===")
                print(f"Detected {len(keypoints)} keypoints:")

                for kp in keypoints[:5]:  # Show first 5
                    print(f"  Keypoint {kp['id']}: "
                          f"({kp['x']:.0f}, {kp['y']:.0f}) "
                          f"conf={kp['confidence']:.2f}")

                if len(keypoints) > 5:
                    print(f"  ... and {len(keypoints) - 5} more")
                print("==================\n")


def main():
    """Main entry point"""
    print("HORUS Pose Estimation Example (Python)")
    print("=" * 50)
    print()

    # Check if model exists
    model_path = "models/movenet_lightning.onnx"
    import os
    if not os.path.exists(model_path):
        print(f"ERROR: Model not found: {model_path}")
        print("\nTo download MoveNet:")
        print("  mkdir -p models")
        print("  wget <movenet_url> -O models/movenet_lightning.onnx")
        print("\nOr use any pose estimation ONNX model")
        print("\nRunning with simulated model for demonstration...\n")

    print("Initializing nodes...")

    # Create nodes
    try:
        camera = CameraSimulator()
        pose_estimator = PoseEstimationExample(model_path)
        display = PoseDisplay()

        print("Starting pose estimation...")
        print()

        # Run for 10 seconds
        run(camera, pose_estimator, display, duration=10, logging=False)

        print("\n\nExample completed successfully!")
        print("\nPerformance Statistics:")
        pose_estimator.monitor.print_stats()

    except Exception as e:
        print(f"ERROR: {e}")
        import traceback
        traceback.print_exc()
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
