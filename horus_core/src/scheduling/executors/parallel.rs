use crate::core::node::{Node, NodeInfo};
use crate::error::HorusResult;
use std::sync::{Arc, Mutex};
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
        let chunk_size = (nodes.len() + self.num_threads - 1) / self.num_threads;

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
    /// Note: Currently executes nodes within each level sequentially.
    /// Parallel execution within levels is reserved for a future enhancement
    /// to ensure thread safety with mutable node references.
    pub fn execute_levels(
        &self,
        levels: &[Vec<usize>], // Each level contains node indices
        nodes: &mut [&mut Box<dyn Node>],
        contexts: &mut [Option<NodeInfo>],
        logging_enabled: &[bool],
    ) -> HorusResult<()> {
        for level in levels {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::node::{Node, NodeInfo, TopicMetadata};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // Test node that tracks execution count
    struct CounterNode {
        name: &'static str,
        counter: Arc<AtomicUsize>,
    }

    impl Node for CounterNode {
        fn name(&self) -> &'static str {
            self.name
        }

        fn init(&mut self, _ctx: &mut NodeInfo) -> HorusResult<()> {
            Ok(())
        }

        fn tick(&mut self, _ctx: Option<&mut NodeInfo>) {
            self.counter.fetch_add(1, Ordering::SeqCst);
            // Simulate some work
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        fn shutdown(&mut self, _ctx: &mut NodeInfo) -> HorusResult<()> {
            Ok(())
        }

        fn get_publishers(&self) -> Vec<TopicMetadata> {
            Vec::new()
        }

        fn get_subscribers(&self) -> Vec<TopicMetadata> {
            Vec::new()
        }
    }

    #[test]
    fn test_parallel_executor_creation() {
        let executor = ParallelExecutor::new();
        assert!(executor.num_threads > 0);

        let executor = ParallelExecutor::with_threads(4);
        assert_eq!(executor.num_threads, 4);
    }

    #[test]
    fn test_single_node_execution() {
        let executor = ParallelExecutor::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let mut node: Box<dyn Node> = Box::new(CounterNode {
            name: "TestNode",
            counter: Arc::clone(&counter),
        });

        let mut nodes = vec![&mut node];
        let mut contexts = vec![None];
        let logging = vec![false];

        executor
            .execute_parallel_group(&mut nodes, &mut contexts, &logging)
            .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_multiple_nodes_parallel() {
        let executor = ParallelExecutor::new();

        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));
        let counter3 = Arc::new(AtomicUsize::new(0));

        let mut node1: Box<dyn Node> = Box::new(CounterNode {
            name: "Node1",
            counter: Arc::clone(&counter1),
        });

        let mut node2: Box<dyn Node> = Box::new(CounterNode {
            name: "Node2",
            counter: Arc::clone(&counter2),
        });

        let mut node3: Box<dyn Node> = Box::new(CounterNode {
            name: "Node3",
            counter: Arc::clone(&counter3),
        });

        let mut nodes = vec![&mut node1, &mut node2, &mut node3];
        let mut contexts = vec![None, None, None];
        let logging = vec![false, false, false];

        let start = std::time::Instant::now();
        executor
            .execute_parallel_group(&mut nodes, &mut contexts, &logging)
            .unwrap();
        let elapsed = start.elapsed();

        // All nodes should have executed
        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
        assert_eq!(counter3.load(Ordering::SeqCst), 1);

        // Should be faster than sequential (3 * 100μs = 300μs)
        // Parallel should be close to 100μs (with some overhead)
        assert!(elapsed.as_micros() < 250);
    }

    #[test]
    fn test_empty_group() {
        let executor = ParallelExecutor::new();
        let mut nodes: Vec<&mut Box<dyn Node>> = Vec::new();
        let mut contexts: Vec<Option<NodeInfo>> = Vec::new();
        let logging: Vec<bool> = Vec::new();

        // Should not panic
        executor
            .execute_parallel_group(&mut nodes, &mut contexts, &logging)
            .unwrap();
    }
}
