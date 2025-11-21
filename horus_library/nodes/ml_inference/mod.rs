// Machine Learning Inference Nodes
//
// Production-ready ML inference nodes for HORUS

#[cfg(feature = "onnx")]
pub mod onnx_inference;

#[cfg(feature = "tflite-inference")]
pub mod tflite_inference;

#[cfg(feature = "onnx")]
pub use onnx_inference::{InferenceConfig, ONNXInferenceNode};

#[cfg(feature = "tflite-inference")]
pub use tflite_inference::{TFLiteConfig, TFLiteInferenceNode};
