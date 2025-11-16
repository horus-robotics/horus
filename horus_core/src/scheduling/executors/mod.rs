/// Parallel executor for running independent nodes concurrently
pub mod parallel;

/// Async I/O executor for non-blocking operations
pub mod async_io;

pub use async_io::{AsyncIOExecutor, AsyncResult};
pub use parallel::ParallelExecutor;
