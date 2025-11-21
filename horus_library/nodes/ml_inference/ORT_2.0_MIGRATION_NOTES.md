# ORT 2.0 API Migration Notes

## Status

All ML infrastructure code is implemented and structured correctly. The remaining work is adapting to ort 2.0.0-rc.10 API changes.

## Completed

✅ All node structures implemented (ONNXInferenceNode, YOLOv8DetectorNode, TFLiteInferenceNode, SemanticSegmentationNode, PoseEstimationNode, CloudLLMNode)
✅ All ML message types with LogSummary implementations
✅ Model management infrastructure (ModelLoader, ModelRegistry)
✅ Python ML utilities (PyTorch, TensorFlow, ONNX bridges)
✅ Example applications (yolo_detection.rs, llm_chat.rs, pose_estimation.py)
✅ Import paths updated to ort 2.0 API
✅ Session builder API updated
✅ Message type compatibility fixes

## Remaining Work: ORT 2.0 API Adaptation

### 1. Value::from_array() API Change

**Current code:**
```rust
let input_tensor = Value::from_array(input)
    .map_err(|e| HorusError::config(format!("Failed to create input tensor: {}", e)))?;
```

**Issue:** `Value::from_array()` in ort 2.0 requires different tensor trait bounds.

**Solution:** Use ort 2.0's tensor creation API:
```rust
use ort::tensor::OrtOwnedTensor;
let input_tensor = Value::from_array(session.allocator(), &input)?;
```

### 2. Session.run() Input Format

**Current code:**
```rust
let outputs = self.session
    .run(ort::inputs!["input" => input_tensor])?;
```

**Issue:** `inputs!` macro and input format changed in ort 2.0.

**Solution:** Use new input format:
```rust
let outputs = session.run(ort::inputs![input_tensor])?;
// or with named inputs:
let outputs = session.run(ort::inputs!{"input" => input_tensor})?;
```

### 3. Output Extraction API

**Current code:**
```rust
let output = outputs[0]
    .try_extract_tensor::<f32>()?;
```

**Issue:** Output extraction API changed.

**Solution:** Use ort 2.0 output API:
```rust
let output = outputs["output"].try_extract::<f32>()?;
// or by index:
let output = outputs[0].try_extract::<f32>()?;
```

### 4. Tensor Shape and Data Access

**Current code:**
```rust
let shape = output.shape();
let data: Vec<f32> = output.iter().copied().collect();
```

**Issue:** Tensor API changed for accessing shape and data.

**Solution:** Use ort 2.0 tensor view API:
```rust
let (shape, data) = output.extract_tensor::<f32>()?;
let data_vec: Vec<f32> = data.to_vec();
```

## Files Needing Updates

- `horus_library/nodes/ml_inference/onnx_inference.rs`
- `horus_library/nodes/cv/yolo_detector.rs`
- `horus_library/nodes/cv/segmentation.rs`
- `horus_library/nodes/cv/pose_estimation.rs`

## Testing Strategy

Once API migration is complete:

1. Test with ONNX models:
   ```bash
   cargo build --features ml-inference --example yolo_detection
   ```

2. Test LLM integration:
   ```bash
   export ANTHROPIC_API_KEY=your_key
   cargo run --features ml-inference --example llm_chat
   ```

3. Test Python integration:
   ```bash
   python3 horus_py/examples/pose_estimation.py
   ```

## Reference Documentation

- ORT 2.0 docs: https://docs.rs/ort/2.0.0-rc.10/ort/
- Migration guide: https://ort.pyke.io/migrating/v1-to-v2
