/// JIT compilation module for ultra-fast node execution
/// Compiles hot paths to native code for 20-50ns latency
mod compiler;
mod dataflow;

pub use compiler::JITCompiler;
pub use dataflow::{BinaryOp, CompiledDataflow, DataflowExpr, DataflowNode, UnaryOp};
