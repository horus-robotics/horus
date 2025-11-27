//! Redundancy and voting system for fault tolerance
//!
//! Implements Triple Modular Redundancy (TMR) and other voting schemes
//! for safety-critical applications.

use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

/// Voting strategy for redundant execution
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VotingStrategy {
    /// Majority voting (2 of 3 must agree)
    Majority,
    /// All must agree (unanimous)
    Unanimous,
    /// Any single result (no voting, just redundancy for availability)
    Any,
    /// Use first successful result
    FirstSuccess,
    /// Median value (for numeric outputs)
    Median,
}

/// Result of a voting operation
#[derive(Debug, Clone)]
pub enum VoteResult<T> {
    /// Consensus reached with this value
    Consensus(T),
    /// No consensus - disagreement detected
    Disagreement { values: Vec<T>, reason: String },
    /// Partial failure - some replicas failed
    PartialFailure {
        successful: Vec<T>,
        failed_count: usize,
    },
    /// Total failure - all replicas failed
    TotalFailure,
}

/// A redundant value with multiple copies
#[derive(Debug, Clone)]
pub struct RedundantValue<T> {
    /// Values from each replica
    pub values: Vec<Option<T>>,
    /// Execution times for each replica
    pub exec_times: Vec<Duration>,
    /// Which replicas succeeded
    pub succeeded: Vec<bool>,
}

impl<T: Clone + PartialEq> RedundantValue<T> {
    /// Create a new redundant value with capacity for n replicas
    pub fn new(replica_count: usize) -> Self {
        Self {
            values: vec![None; replica_count],
            exec_times: vec![Duration::ZERO; replica_count],
            succeeded: vec![false; replica_count],
        }
    }

    /// Record a result from a replica
    pub fn record(&mut self, replica_id: usize, value: T, exec_time: Duration) {
        if replica_id < self.values.len() {
            self.values[replica_id] = Some(value);
            self.exec_times[replica_id] = exec_time;
            self.succeeded[replica_id] = true;
        }
    }

    /// Record a failure from a replica
    pub fn record_failure(&mut self, replica_id: usize) {
        if replica_id < self.succeeded.len() {
            self.succeeded[replica_id] = false;
        }
    }

    /// Get successful values
    pub fn successful_values(&self) -> Vec<&T> {
        self.values
            .iter()
            .zip(self.succeeded.iter())
            .filter_map(|(v, s)| if *s { v.as_ref() } else { None })
            .collect()
    }

    /// Count successful replicas
    pub fn success_count(&self) -> usize {
        self.succeeded.iter().filter(|&&s| s).count()
    }
}

/// Voter for redundant outputs
pub struct Voter<T> {
    /// Voting strategy
    strategy: VotingStrategy,
    /// Minimum replicas required for consensus
    min_replicas: usize,
    /// Phantom data for type T
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone + PartialEq + Eq + Hash> Voter<T> {
    /// Create a new voter with majority strategy
    pub fn majority(min_replicas: usize) -> Self {
        Self {
            strategy: VotingStrategy::Majority,
            min_replicas,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a voter with unanimous strategy
    pub fn unanimous(min_replicas: usize) -> Self {
        Self {
            strategy: VotingStrategy::Unanimous,
            min_replicas,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Vote on redundant values
    pub fn vote(&self, redundant: &RedundantValue<T>) -> VoteResult<T> {
        let successful = redundant.successful_values();
        let success_count = successful.len();

        // Check minimum replicas
        if success_count == 0 {
            return VoteResult::TotalFailure;
        }

        if success_count < self.min_replicas {
            return VoteResult::PartialFailure {
                successful: successful.into_iter().cloned().collect(),
                failed_count: redundant.values.len() - success_count,
            };
        }

        match self.strategy {
            VotingStrategy::Majority => self.majority_vote(&successful),
            VotingStrategy::Unanimous => self.unanimous_vote(&successful),
            VotingStrategy::Any => VoteResult::Consensus(successful[0].clone()),
            VotingStrategy::FirstSuccess => VoteResult::Consensus(successful[0].clone()),
            VotingStrategy::Median => {
                // For median, we need numeric type - fall back to majority
                self.majority_vote(&successful)
            }
        }
    }

    /// Majority voting
    fn majority_vote(&self, values: &[&T]) -> VoteResult<T> {
        let mut counts: HashMap<&T, usize> = HashMap::new();

        for value in values {
            *counts.entry(*value).or_insert(0) += 1;
        }

        let total = values.len();
        let threshold = (total / 2) + 1;

        // Find majority
        for (value, count) in counts {
            if count >= threshold {
                return VoteResult::Consensus(value.clone());
            }
        }

        VoteResult::Disagreement {
            values: values.iter().map(|&v| v.clone()).collect(),
            reason: "No majority consensus".to_string(),
        }
    }

    /// Unanimous voting
    fn unanimous_vote(&self, values: &[&T]) -> VoteResult<T> {
        if values.is_empty() {
            return VoteResult::TotalFailure;
        }

        let first = values[0];
        if values.iter().all(|&v| v == first) {
            VoteResult::Consensus(first.clone())
        } else {
            VoteResult::Disagreement {
                values: values.iter().map(|&v| v.clone()).collect(),
                reason: "Not unanimous".to_string(),
            }
        }
    }
}

/// Numeric voter with median support
impl Voter<i64> {
    /// Vote with median for numeric values
    pub fn median_vote(&self, redundant: &RedundantValue<i64>) -> VoteResult<i64> {
        let mut values: Vec<i64> = redundant.successful_values().into_iter().cloned().collect();

        if values.is_empty() {
            return VoteResult::TotalFailure;
        }

        values.sort();
        let median = values[values.len() / 2];

        VoteResult::Consensus(median)
    }
}

impl Voter<f64> {
    /// Vote with median for floating point values
    pub fn median_vote(&self, redundant: &RedundantValue<f64>) -> VoteResult<f64> {
        let mut values: Vec<f64> = redundant.successful_values().into_iter().cloned().collect();

        if values.is_empty() {
            return VoteResult::TotalFailure;
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = values[values.len() / 2];

        VoteResult::Consensus(median)
    }
}

/// Redundancy manager for a scheduler
pub struct RedundancyManager {
    /// Number of replicas
    replica_count: usize,
    /// Voting strategy
    strategy: VotingStrategy,
    /// Fault statistics
    fault_stats: FaultStats,
    /// Whether redundancy is enabled
    enabled: bool,
}

/// Fault statistics
#[derive(Debug, Default)]
pub struct FaultStats {
    /// Total votes performed
    pub total_votes: u64,
    /// Consensus reached
    pub consensus_count: u64,
    /// Disagreements
    pub disagreement_count: u64,
    /// Partial failures
    pub partial_failure_count: u64,
    /// Total failures
    pub total_failure_count: u64,
}

impl RedundancyManager {
    /// Create a new redundancy manager
    pub fn new(replica_count: usize, strategy: VotingStrategy) -> Self {
        Self {
            replica_count: replica_count.max(1),
            strategy,
            fault_stats: FaultStats::default(),
            enabled: replica_count > 1,
        }
    }

    /// Create with TMR (Triple Modular Redundancy)
    pub fn tmr() -> Self {
        Self::new(3, VotingStrategy::Majority)
    }

    /// Create with dual redundancy
    pub fn dual() -> Self {
        Self::new(2, VotingStrategy::Unanimous)
    }

    /// Get replica count
    pub fn replica_count(&self) -> usize {
        self.replica_count
    }

    /// Execute a function with redundancy
    pub fn execute_redundant<T, F>(&mut self, func: F) -> VoteResult<T>
    where
        T: Clone + PartialEq + Eq + Hash,
        F: Fn(usize) -> Option<T>,
    {
        if !self.enabled {
            // No redundancy - just execute once
            return match func(0) {
                Some(v) => VoteResult::Consensus(v),
                None => VoteResult::TotalFailure,
            };
        }

        let mut redundant = RedundantValue::new(self.replica_count);

        for replica_id in 0..self.replica_count {
            let start = Instant::now();
            match func(replica_id) {
                Some(value) => {
                    redundant.record(replica_id, value, start.elapsed());
                }
                None => {
                    redundant.record_failure(replica_id);
                }
            }
        }

        let voter = Voter {
            strategy: self.strategy,
            min_replicas: (self.replica_count / 2) + 1,
            _phantom: std::marker::PhantomData,
        };

        let result = voter.vote(&redundant);

        // Update stats
        self.fault_stats.total_votes += 1;
        match &result {
            VoteResult::Consensus(_) => self.fault_stats.consensus_count += 1,
            VoteResult::Disagreement { .. } => self.fault_stats.disagreement_count += 1,
            VoteResult::PartialFailure { .. } => self.fault_stats.partial_failure_count += 1,
            VoteResult::TotalFailure => self.fault_stats.total_failure_count += 1,
        }

        result
    }

    /// Get fault statistics
    pub fn stats(&self) -> &FaultStats {
        &self.fault_stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.fault_stats = FaultStats::default();
    }

    /// Enable or disable redundancy
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled && self.replica_count > 1;
    }
}

impl Default for RedundancyManager {
    fn default() -> Self {
        Self::new(1, VotingStrategy::Any) // Disabled by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_majority_vote() {
        let voter: Voter<i32> = Voter::majority(2);
        let mut redundant = RedundantValue::new(3);

        // Two agree, one disagrees
        redundant.record(0, 42, Duration::from_micros(100));
        redundant.record(1, 42, Duration::from_micros(110));
        redundant.record(2, 99, Duration::from_micros(105));

        match voter.vote(&redundant) {
            VoteResult::Consensus(v) => assert_eq!(v, 42),
            _ => panic!("Expected consensus"),
        }
    }

    #[test]
    fn test_unanimous_vote() {
        let voter: Voter<i32> = Voter::unanimous(3);
        let mut redundant = RedundantValue::new(3);

        // All agree
        redundant.record(0, 42, Duration::from_micros(100));
        redundant.record(1, 42, Duration::from_micros(110));
        redundant.record(2, 42, Duration::from_micros(105));

        match voter.vote(&redundant) {
            VoteResult::Consensus(v) => assert_eq!(v, 42),
            _ => panic!("Expected consensus"),
        }
    }

    #[test]
    fn test_disagreement() {
        let voter: Voter<i32> = Voter::majority(2);
        let mut redundant = RedundantValue::new(3);

        // All different
        redundant.record(0, 1, Duration::from_micros(100));
        redundant.record(1, 2, Duration::from_micros(110));
        redundant.record(2, 3, Duration::from_micros(105));

        match voter.vote(&redundant) {
            VoteResult::Disagreement { .. } => {}
            _ => panic!("Expected disagreement"),
        }
    }

    #[test]
    fn test_redundancy_manager() {
        let mut rm = RedundancyManager::tmr();

        let result = rm.execute_redundant(|replica_id| {
            // All replicas return same value
            Some(42 + replica_id as i32 * 0) // Same value
        });

        match result {
            VoteResult::Consensus(v) => assert_eq!(v, 42),
            _ => panic!("Expected consensus"),
        }

        assert_eq!(rm.stats().consensus_count, 1);
    }
}
