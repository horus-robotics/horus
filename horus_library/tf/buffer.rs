//! Circular buffer for time-based transform storage
//!
//! Provides efficient storage and retrieval of timestamped transforms
//! with support for interpolation between samples.

use super::transform::Transform;

/// A fixed-capacity circular buffer for storing timestamped data
///
/// Used for buffering transforms over time, allowing interpolation
/// between samples and efficient memory usage.
#[derive(Debug, Clone)]
pub struct CircularBuffer<T> {
    /// Ring buffer storage
    data: Vec<T>,
    /// Maximum capacity
    capacity: usize,
    /// Write position (next insert index)
    head: usize,
    /// Number of valid elements
    len: usize,
}

impl<T: Clone> CircularBuffer<T> {
    /// Create a new circular buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            capacity,
            head: 0,
            len: 0,
        }
    }

    /// Push a new element into the buffer
    ///
    /// If the buffer is full, the oldest element is overwritten.
    pub fn push(&mut self, item: T) {
        if self.data.len() < self.capacity {
            // Buffer not yet full, append
            self.data.push(item);
            self.head = self.data.len();
            self.len = self.data.len();
        } else {
            // Buffer full, overwrite oldest
            let index = self.head % self.capacity;
            self.data[index] = item;
            self.head = (self.head + 1) % self.capacity;
            self.len = self.capacity;
        }
    }

    /// Get the number of elements in the buffer
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Check if the buffer is full
    pub fn is_full(&self) -> bool {
        self.len >= self.capacity
    }

    /// Get the capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all elements from the buffer
    pub fn clear(&mut self) {
        self.data.clear();
        self.head = 0;
        self.len = 0;
    }

    /// Get element at logical index (0 = oldest, len-1 = newest)
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }

        if self.data.len() < self.capacity {
            // Buffer not yet wrapped
            self.data.get(index)
        } else {
            // Buffer has wrapped
            let actual_index = (self.head + index) % self.capacity;
            self.data.get(actual_index)
        }
    }

    /// Get the oldest element
    pub fn get_oldest(&self) -> Option<&T> {
        self.get(0)
    }

    /// Get the newest element
    pub fn get_newest(&self) -> Option<&T> {
        if self.len == 0 {
            None
        } else {
            self.get(self.len - 1)
        }
    }

    /// Iterate over elements from oldest to newest
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        CircularBufferIter {
            buffer: self,
            index: 0,
        }
    }
}

/// Iterator for CircularBuffer
struct CircularBufferIter<'a, T> {
    buffer: &'a CircularBuffer<T>,
    index: usize,
}

impl<'a, T: Clone> Iterator for CircularBufferIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.buffer.len {
            None
        } else {
            let item = self.buffer.get(self.index);
            self.index += 1;
            item
        }
    }
}

// Specialized implementation for timestamped transforms
impl CircularBuffer<(u64, Transform)> {
    /// Get the latest transform and its timestamp
    pub fn get_latest(&self) -> Option<(u64, Transform)> {
        self.get_newest().cloned()
    }

    /// Get the oldest transform and its timestamp
    pub fn get_oldest_transform(&self) -> Option<(u64, Transform)> {
        self.get_oldest().cloned()
    }

    /// Get the time range covered by the buffer
    pub fn time_range(&self) -> Option<(u64, u64)> {
        if self.is_empty() {
            return None;
        }
        let oldest = self.get_oldest()?.0;
        let newest = self.get_newest()?.0;
        Some((oldest, newest))
    }

    /// Check if a timestamp is within the buffer's time range
    pub fn contains_time(&self, timestamp: u64) -> bool {
        match self.time_range() {
            Some((oldest, newest)) => timestamp >= oldest && timestamp <= newest,
            None => false,
        }
    }

    /// Get interpolated transform at the given timestamp
    ///
    /// Returns None if the buffer is empty or the timestamp is out of range.
    /// For timestamps between samples, performs SLERP interpolation.
    pub fn get_interpolated(&self, timestamp: u64) -> Option<Transform> {
        if self.is_empty() {
            return None;
        }

        // Handle single element case
        if self.len == 1 {
            return Some(self.get_oldest()?.1);
        }

        // Check bounds
        let oldest = self.get_oldest()?;
        let newest = self.get_newest()?;

        // If timestamp is before oldest, return oldest (extrapolation not supported)
        if timestamp <= oldest.0 {
            return Some(oldest.1);
        }

        // If timestamp is at or after newest, return newest
        if timestamp >= newest.0 {
            return Some(newest.1);
        }

        // Find the two samples that bracket the timestamp
        let mut before: Option<&(u64, Transform)> = None;
        let mut after: Option<&(u64, Transform)> = None;

        for sample in self.iter() {
            if sample.0 <= timestamp {
                before = Some(sample);
            } else {
                after = Some(sample);
                break;
            }
        }

        match (before, after) {
            (Some(b), Some(a)) => {
                // Interpolate between the two samples
                let t = if a.0 == b.0 {
                    0.0
                } else {
                    (timestamp - b.0) as f64 / (a.0 - b.0) as f64
                };
                Some(b.1.interpolate(&a.1, t))
            }
            (Some(b), None) => Some(b.1),
            (None, Some(a)) => Some(a.1),
            (None, None) => None,
        }
    }

    /// Get exact transform at timestamp (no interpolation)
    pub fn get_exact(&self, timestamp: u64) -> Option<Transform> {
        for sample in self.iter() {
            if sample.0 == timestamp {
                return Some(sample.1);
            }
        }
        None
    }

    /// Find the closest transform to a timestamp
    pub fn get_closest(&self, timestamp: u64) -> Option<(u64, Transform)> {
        if self.is_empty() {
            return None;
        }

        let mut closest: Option<&(u64, Transform)> = None;
        let mut min_diff = u64::MAX;

        for sample in self.iter() {
            let diff = if sample.0 > timestamp {
                sample.0 - timestamp
            } else {
                timestamp - sample.0
            };

            if diff < min_diff {
                min_diff = diff;
                closest = Some(sample);
            }
        }

        closest.cloned()
    }

    /// Remove transforms older than the given timestamp
    pub fn prune_before(&mut self, timestamp: u64) {
        // We can't efficiently remove from the middle of a ring buffer
        // So we rebuild the buffer with only the newer elements
        let items: Vec<_> = self
            .iter()
            .filter(|(ts, _)| *ts >= timestamp)
            .cloned()
            .collect();

        self.clear();
        for item in items {
            self.push(item);
        }
    }
}

/// Higher-level TF buffer that manages transforms for a single frame pair
///
/// Provides caching, interpolation, and time-based queries.
#[derive(Debug, Clone)]
pub struct TFBuffer {
    /// Parent frame ID
    parent_frame: String,
    /// Child frame ID
    child_frame: String,
    /// Transform storage
    buffer: CircularBuffer<(u64, Transform)>,
    /// Maximum age of transforms in nanoseconds
    max_age_ns: u64,
}

impl TFBuffer {
    /// Create a new TF buffer
    pub fn new(parent: impl Into<String>, child: impl Into<String>, capacity: usize) -> Self {
        Self {
            parent_frame: parent.into(),
            child_frame: child.into(),
            buffer: CircularBuffer::new(capacity),
            max_age_ns: 10_000_000_000, // 10 seconds default
        }
    }

    /// Set the maximum age of transforms
    pub fn set_max_age(&mut self, seconds: f64) {
        self.max_age_ns = (seconds * 1e9) as u64;
    }

    /// Get the parent frame ID
    pub fn parent_frame(&self) -> &str {
        &self.parent_frame
    }

    /// Get the child frame ID
    pub fn child_frame(&self) -> &str {
        &self.child_frame
    }

    /// Add a new transform to the buffer
    pub fn add(&mut self, transform: Transform, timestamp: u64) {
        self.buffer.push((timestamp, transform));
    }

    /// Get transform at specific timestamp (with interpolation)
    pub fn get(&self, timestamp: u64) -> Option<Transform> {
        self.buffer.get_interpolated(timestamp)
    }

    /// Get the latest transform
    pub fn get_latest(&self) -> Option<(u64, Transform)> {
        self.buffer.get_latest()
    }

    /// Check if the buffer has data for the given timestamp
    pub fn has_data(&self, timestamp: u64) -> bool {
        self.buffer.contains_time(timestamp)
    }

    /// Get the time range of buffered transforms
    pub fn time_range(&self) -> Option<(u64, u64)> {
        self.buffer.time_range()
    }

    /// Get the number of buffered transforms
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Prune old transforms based on max age
    pub fn prune(&mut self, current_time: u64) {
        if current_time > self.max_age_ns {
            self.buffer.prune_before(current_time - self.max_age_ns);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_buffer_basic() {
        let mut buf: CircularBuffer<i32> = CircularBuffer::new(3);
        assert!(buf.is_empty());
        assert_eq!(buf.capacity(), 3);

        buf.push(1);
        buf.push(2);
        buf.push(3);
        assert_eq!(buf.len(), 3);
        assert!(buf.is_full());
    }

    #[test]
    fn test_circular_buffer_wrap() {
        let mut buf: CircularBuffer<i32> = CircularBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4); // Overwrites 1

        assert_eq!(buf.len(), 3);
        assert_eq!(*buf.get_oldest().unwrap(), 2);
        assert_eq!(*buf.get_newest().unwrap(), 4);
    }

    #[test]
    fn test_circular_buffer_iter() {
        let mut buf: CircularBuffer<i32> = CircularBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);

        let items: Vec<_> = buf.iter().cloned().collect();
        assert_eq!(items, vec![2, 3, 4]);
    }

    #[test]
    fn test_transform_buffer_interpolation() {
        let mut buf: CircularBuffer<(u64, Transform)> = CircularBuffer::new(10);

        // Add transforms at t=0 and t=100
        buf.push((0, Transform::from_translation([0.0, 0.0, 0.0])));
        buf.push((100, Transform::from_translation([10.0, 0.0, 0.0])));

        // Get interpolated at t=50 (should be [5.0, 0.0, 0.0])
        let tf = buf.get_interpolated(50).unwrap();
        assert!((tf.translation[0] - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_transform_buffer_bounds() {
        let mut buf: CircularBuffer<(u64, Transform)> = CircularBuffer::new(10);

        buf.push((100, Transform::from_translation([1.0, 0.0, 0.0])));
        buf.push((200, Transform::from_translation([2.0, 0.0, 0.0])));

        // Before range - should return oldest
        let tf = buf.get_interpolated(50).unwrap();
        assert!((tf.translation[0] - 1.0).abs() < 1e-6);

        // After range - should return newest
        let tf = buf.get_interpolated(300).unwrap();
        assert!((tf.translation[0] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_transform_buffer_time_range() {
        let mut buf: CircularBuffer<(u64, Transform)> = CircularBuffer::new(10);

        assert!(buf.time_range().is_none());

        buf.push((100, Transform::identity()));
        buf.push((200, Transform::identity()));
        buf.push((300, Transform::identity()));

        let (oldest, newest) = buf.time_range().unwrap();
        assert_eq!(oldest, 100);
        assert_eq!(newest, 300);
    }

    #[test]
    fn test_transform_buffer_get_closest() {
        let mut buf: CircularBuffer<(u64, Transform)> = CircularBuffer::new(10);

        buf.push((100, Transform::from_translation([1.0, 0.0, 0.0])));
        buf.push((200, Transform::from_translation([2.0, 0.0, 0.0])));
        buf.push((300, Transform::from_translation([3.0, 0.0, 0.0])));

        let (ts, tf) = buf.get_closest(190).unwrap();
        assert_eq!(ts, 200);
        assert!((tf.translation[0] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_transform_buffer_prune() {
        let mut buf: CircularBuffer<(u64, Transform)> = CircularBuffer::new(10);

        buf.push((100, Transform::identity()));
        buf.push((200, Transform::identity()));
        buf.push((300, Transform::identity()));
        buf.push((400, Transform::identity()));

        buf.prune_before(250);

        assert_eq!(buf.len(), 2);
        assert_eq!(buf.get_oldest().unwrap().0, 300);
    }

    #[test]
    fn test_tf_buffer() {
        let mut tf_buf = TFBuffer::new("world", "robot", 100);

        assert_eq!(tf_buf.parent_frame(), "world");
        assert_eq!(tf_buf.child_frame(), "robot");
        assert!(tf_buf.is_empty());

        tf_buf.add(Transform::from_translation([1.0, 0.0, 0.0]), 100);
        tf_buf.add(Transform::from_translation([2.0, 0.0, 0.0]), 200);

        assert_eq!(tf_buf.len(), 2);

        let tf = tf_buf.get(150).unwrap();
        assert!((tf.translation[0] - 1.5).abs() < 1e-6);
    }

    #[test]
    fn test_tf_buffer_prune() {
        let mut tf_buf = TFBuffer::new("a", "b", 100);
        tf_buf.set_max_age(2.0); // 2 seconds = 2_000_000_000 ns

        tf_buf.add(Transform::identity(), 1_000_000_000);
        tf_buf.add(Transform::identity(), 2_000_000_000);
        tf_buf.add(Transform::identity(), 3_000_000_000);

        // Prune with current time at 3.5 seconds
        // Cutoff = 3.5s - 2.0s = 1.5s = 1_500_000_000 ns
        // Items >= 1.5s are kept: 2.0s and 3.0s
        tf_buf.prune(3_500_000_000);

        // Should have removed the 1.0s entry, keeping 2.0s and 3.0s
        assert_eq!(tf_buf.len(), 2);
    }
}
