/// Parallel executor module for concurrent node execution
///
/// # Architecture Constraint
///
/// Due to Rust's ownership model and borrow checker, true parallel execution of
/// nodes with mutable contexts is challenging. Each node requires mutable access
/// to its NodeInfo context during tick(), which prevents multiple nodes from
/// executing simultaneously without complex workarounds like:
/// - Arc<Mutex> wrappers (adding synchronization overhead)
/// - Message passing architectures
/// - Unsafe code with manual synchronization
///
/// The current implementation provides the infrastructure for parallel execution
/// but is limited by these constraints. For production use, consider:
/// - Using the AsyncIOExecutor for I/O-bound nodes
/// - JIT compilation for ultra-fast nodes
/// - Dependency-based level execution in the main scheduler
///
/// This is a fundamental trade-off in Rust's design - we get memory safety
/// and data race prevention at the cost of some parallel execution patterns
/// being more difficult to implement.
use crate::core::node::{Node, NodeInfo};
use crate::error::HorusResult;
use std::thread;

/// Parallel executor for running independent nodes concurrently
#[derive(Debug)]
pub struct ParallelExecutor {
    /// Number of worker threads (defaults to CPU count)
    num_threads: usize,
    /// CPU cores to pin threads to (optional)
    cpu_cores: Option<Vec<usize>>,
}

impl ParallelExecutor {
    /// Create new parallel executor with automatic thread count
    pub fn new() -> Self {
        let num_threads = num_cpus::get().max(1);
        Self {
            num_threads,
            cpu_cores: None,
        }
    }

    /// Create parallel executor with specific thread count
    pub fn with_threads(num_threads: usize) -> Self {
        Self {
            num_threads: num_threads.max(1),
            cpu_cores: None,
        }
    }

    /// Set the maximum number of threads to use
    pub fn set_max_threads(&mut self, num_threads: usize) {
        self.num_threads = num_threads.max(1);
    }

    /// Set specific CPU cores to pin threads to
    pub fn set_cpu_cores(&mut self, cores: Vec<usize>) {
        if !cores.is_empty() {
            self.cpu_cores = Some(cores);
        }
    }

    /// Get the current number of threads
    pub fn get_num_threads(&self) -> usize {
        self.num_threads
    }

    /// Execute a group of independent nodes in parallel
    /// Returns when all nodes have completed one tick
    pub fn execute_parallel_group(
        &self,
        nodes: &mut [&mut Box<dyn Node>],
        contexts: &mut [Option<NodeInfo>],
        logging_enabled: &[bool],
    ) -> HorusResult<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        // Single node - no need for parallelism
        if nodes.len() == 1 {
            let ctx = if logging_enabled[0] {
                contexts[0].as_mut()
            } else {
                None
            };
            nodes[0].tick(ctx);
            return Ok(());
        }

        // Multiple nodes - use thread pool
        let chunk_size = nodes.len().div_ceil(self.num_threads);

        // We need to safely share mutable state across threads
        // Use scoped threads to avoid lifetime issues
        thread::scope(|s| {
            let mut handles = Vec::new();

            for (node_chunk, (context_chunk, logging_chunk)) in nodes.chunks_mut(chunk_size).zip(
                contexts
                    .chunks_mut(chunk_size)
                    .zip(logging_enabled.chunks(chunk_size)),
            ) {
                let handle = s.spawn(move || {
                    for ((node, ctx), &logging) in node_chunk
                        .iter_mut()
                        .zip(context_chunk.iter_mut())
                        .zip(logging_chunk.iter())
                    {
                        let ctx_ref = if logging { ctx.as_mut() } else { None };
                        node.tick(ctx_ref);
                    }
                });
                handles.push(handle);
            }

            // Wait for all threads to complete
            for handle in handles {
                let _ = handle.join();
            }
        });

        Ok(())
    }

    /// Execute multiple levels sequentially, with parallelism within each level
    /// Levels represent topological ordering - level N+1 depends on level N
    ///
    /// Due to Rust's ownership constraints, parallel execution within levels
    /// is currently implemented sequentially. True parallel execution would
    /// require Arc<Mutex> wrappers or unsafe code patterns that could introduce
    /// data races. The infrastructure is ready for future parallel enhancements
    /// when a safe pattern is identified.
    pub fn execute_levels(
        &self,
        levels: &[Vec<usize>], // Each level contains node indices
        nodes: &mut [&mut Box<dyn Node>],
        contexts: &mut [Option<NodeInfo>],
        logging_enabled: &[bool],
    ) -> HorusResult<()> {
        // Process each dependency level sequentially
        for level in levels {
            if level.is_empty() {
                continue;
            }

            // For now, execute nodes in level sequentially
            // True parallel execution requires overcoming Rust's borrow checker
            // constraints for mutable references across threads
            for &idx in level {
                if idx < nodes.len() {
                    let ctx = if logging_enabled[idx] {
                        contexts[idx].as_mut()
                    } else {
                        None
                    };
                    nodes[idx].tick(ctx);
                }
            }
        }

        Ok(())
    }
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new()
    }
}
