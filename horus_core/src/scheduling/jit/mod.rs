/// JIT compilation module for ultra-fast node execution
/// Compiles hot paths to native code for 20-50ns latency
mod compiler;
mod dataflow;
mod example_nodes;

pub use compiler::JITCompiler;
pub use dataflow::CompiledDataflow;
pub use example_nodes::ScalingNode;
