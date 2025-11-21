//! Determinism and reproducibility tests

use bevy::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Determinism test result
#[derive(Debug, Clone)]
pub struct DeterminismTest {
    pub name: String,
    pub seed: u64,
    pub step_count: usize,
    pub state_hashes: Vec<u64>,
}

impl DeterminismTest {
    pub fn new(name: impl Into<String>, seed: u64, step_count: usize) -> Self {
        Self {
            name: name.into(),
            seed,
            step_count,
            state_hashes: Vec::with_capacity(step_count),
        }
    }

    /// Record state hash at current step
    pub fn record_state<T: Hash>(&mut self, state: &T) {
        let mut hasher = DefaultHasher::new();
        state.hash(&mut hasher);
        self.state_hashes.push(hasher.finish());
    }

    /// Compare with another run for determinism
    pub fn is_deterministic(&self, other: &DeterminismTest) -> bool {
        if self.seed != other.seed {
            return false;
        }

        if self.state_hashes.len() != other.state_hashes.len() {
            return false;
        }

        self.state_hashes == other.state_hashes
    }

    /// Find first divergence point
    pub fn find_divergence(&self, other: &DeterminismTest) -> Option<usize> {
        for (i, (hash1, hash2)) in self
            .state_hashes
            .iter()
            .zip(&other.state_hashes)
            .enumerate()
        {
            if hash1 != hash2 {
                return Some(i);
            }
        }
        None
    }
}

/// Hash physics state for determinism checking
pub fn hash_transform(transform: &Transform) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Hash position (quantized to avoid floating point errors)
    let pos_x = (transform.translation.x * 1000.0) as i64;
    let pos_y = (transform.translation.y * 1000.0) as i64;
    let pos_z = (transform.translation.z * 1000.0) as i64;
    pos_x.hash(&mut hasher);
    pos_y.hash(&mut hasher);
    pos_z.hash(&mut hasher);

    // Hash rotation (quantized)
    let rot_x = (transform.rotation.x * 10000.0) as i64;
    let rot_y = (transform.rotation.y * 10000.0) as i64;
    let rot_z = (transform.rotation.z * 10000.0) as i64;
    let rot_w = (transform.rotation.w * 10000.0) as i64;
    rot_x.hash(&mut hasher);
    rot_y.hash(&mut hasher);
    rot_z.hash(&mut hasher);
    rot_w.hash(&mut hasher);

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determinism_test_creation() {
        let test = DeterminismTest::new("physics_sim", 42, 100);
        assert_eq!(test.seed, 42);
        assert_eq!(test.step_count, 100);
        assert_eq!(test.state_hashes.len(), 0);
    }

    #[test]
    fn test_record_state() {
        let mut test = DeterminismTest::new("test", 0, 10);
        test.record_state(&42u64);
        test.record_state(&43u64);
        assert_eq!(test.state_hashes.len(), 2);
    }

    #[test]
    fn test_is_deterministic_same() {
        let mut test1 = DeterminismTest::new("test", 42, 10);
        let mut test2 = DeterminismTest::new("test", 42, 10);

        for i in 0..10 {
            test1.record_state(&i);
            test2.record_state(&i);
        }

        assert!(test1.is_deterministic(&test2));
    }

    #[test]
    fn test_is_deterministic_different() {
        let mut test1 = DeterminismTest::new("test", 42, 10);
        let mut test2 = DeterminismTest::new("test", 42, 10);

        for i in 0..10 {
            test1.record_state(&i);
            test2.record_state(&(i + 1)); // Different values
        }

        assert!(!test1.is_deterministic(&test2));
    }

    #[test]
    fn test_find_divergence() {
        let mut test1 = DeterminismTest::new("test", 42, 10);
        let mut test2 = DeterminismTest::new("test", 42, 10);

        for i in 0..5 {
            test1.record_state(&i);
            test2.record_state(&i);
        }

        // Diverge at step 5
        for i in 5..10 {
            test1.record_state(&i);
            test2.record_state(&(i + 100));
        }

        assert_eq!(test1.find_divergence(&test2), Some(5));
    }

    #[test]
    fn test_hash_transform() {
        let transform = Transform::from_xyz(1.0, 2.0, 3.0);
        let hash1 = hash_transform(&transform);
        let hash2 = hash_transform(&transform);
        assert_eq!(hash1, hash2); // Same transform = same hash
    }

    #[test]
    fn test_hash_transform_different() {
        let transform1 = Transform::from_xyz(1.0, 2.0, 3.0);
        let transform2 = Transform::from_xyz(1.0, 2.0, 3.001); // Slightly different
        let hash1 = hash_transform(&transform1);
        let hash2 = hash_transform(&transform2);
        // Should be same due to quantization (1000x precision)
        assert_eq!(hash1, hash2);
    }
}
