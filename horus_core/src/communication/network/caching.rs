//! Topic caching with TTL for last-value semantics
//!
//! Provides in-memory caching for topics where you only need the latest value,
//! such as configuration, status, or slowly-changing data. Cache hits are 0ns.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Default cache TTL (time-to-live)
const DEFAULT_TTL: Duration = Duration::from_secs(60);
/// Default maximum cache entries
const DEFAULT_MAX_ENTRIES: usize = 1000;
/// Default maximum entry size (1MB)
const DEFAULT_MAX_ENTRY_SIZE: usize = 1024 * 1024;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Default TTL for entries
    pub default_ttl: Duration,
    /// Maximum number of entries
    pub max_entries: usize,
    /// Maximum size per entry (bytes)
    pub max_entry_size: usize,
    /// Whether to track hit/miss statistics
    pub track_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: DEFAULT_TTL,
            max_entries: DEFAULT_MAX_ENTRIES,
            max_entry_size: DEFAULT_MAX_ENTRY_SIZE,
            track_stats: true,
        }
    }
}

impl CacheConfig {
    /// Short-lived cache (for frequently updating data)
    pub fn short_lived() -> Self {
        Self {
            default_ttl: Duration::from_secs(1),
            ..Default::default()
        }
    }

    /// Long-lived cache (for configuration data)
    pub fn long_lived() -> Self {
        Self {
            default_ttl: Duration::from_secs(3600), // 1 hour
            ..Default::default()
        }
    }

    /// Unlimited entries (be careful with memory)
    pub fn unlimited() -> Self {
        Self {
            max_entries: usize::MAX,
            ..Default::default()
        }
    }
}

/// A cached entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Cached data
    data: Vec<u8>,
    /// When the entry was created
    created_at: Instant,
    /// TTL for this entry
    ttl: Duration,
    /// Number of times this entry was accessed
    access_count: u64,
    /// Last access time
    last_accessed: Instant,
}

impl CacheEntry {
    fn new(data: Vec<u8>, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            data,
            created_at: now,
            ttl,
            access_count: 0,
            last_accessed: now,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
    }

    fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    fn time_until_expiry(&self) -> Duration {
        self.ttl.saturating_sub(self.created_at.elapsed())
    }
}

/// Cache statistics
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of entries currently in cache
    pub entries: usize,
    /// Total bytes stored
    pub bytes_stored: usize,
    /// Number of evictions
    pub evictions: u64,
    /// Number of expirations
    pub expirations: u64,
}

impl CacheStats {
    /// Get hit ratio (0.0 to 1.0)
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Topic cache with TTL support
pub struct TopicCache {
    /// Configuration
    config: CacheConfig,
    /// Cached entries by topic
    entries: RwLock<HashMap<String, CacheEntry>>,
    /// Statistics
    stats: RwLock<CacheStats>,
}

impl TopicCache {
    /// Create a new cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: RwLock::new(HashMap::new()),
            stats: RwLock::new(CacheStats::default()),
        }
    }

    /// Get a cached value by topic
    pub fn get(&self, topic: &str) -> Option<Vec<u8>> {
        // First, try a read lock
        {
            let entries = self.entries.read().unwrap();
            if let Some(entry) = entries.get(topic) {
                if !entry.is_expired() {
                    // Clone the data (we can't hold the lock while returning)
                    let data = entry.data.clone();

                    // Update stats
                    if self.config.track_stats {
                        let mut stats = self.stats.write().unwrap();
                        stats.hits += 1;
                    }

                    // We need a write lock to update access count
                    drop(entries);
                    if let Ok(mut entries) = self.entries.write() {
                        if let Some(entry) = entries.get_mut(topic) {
                            entry.touch();
                        }
                    }

                    return Some(data);
                }
            }
        }

        // Cache miss
        if self.config.track_stats {
            let mut stats = self.stats.write().unwrap();
            stats.misses += 1;
        }

        None
    }

    /// Put a value into the cache with default TTL
    pub fn put(&self, topic: &str, data: Vec<u8>) -> bool {
        self.put_with_ttl(topic, data, self.config.default_ttl)
    }

    /// Put a value with custom TTL
    pub fn put_with_ttl(&self, topic: &str, data: Vec<u8>, ttl: Duration) -> bool {
        // Check entry size
        if data.len() > self.config.max_entry_size {
            return false;
        }

        let mut entries = self.entries.write().unwrap();

        // Evict if necessary
        if entries.len() >= self.config.max_entries && !entries.contains_key(topic) {
            self.evict_one(&mut entries);
        }

        let data_len = data.len();
        let entry = CacheEntry::new(data, ttl);
        let is_new = entries.insert(topic.to_string(), entry).is_none();

        // Update stats
        if self.config.track_stats {
            let mut stats = self.stats.write().unwrap();
            stats.entries = entries.len();
            if is_new {
                stats.bytes_stored += data_len;
            }
        }

        true
    }

    /// Remove a cached value
    pub fn remove(&self, topic: &str) -> bool {
        let mut entries = self.entries.write().unwrap();

        if let Some(entry) = entries.remove(topic) {
            if self.config.track_stats {
                let mut stats = self.stats.write().unwrap();
                stats.entries = entries.len();
                stats.bytes_stored = stats.bytes_stored.saturating_sub(entry.data.len());
            }
            true
        } else {
            false
        }
    }

    /// Check if a topic is cached (and not expired)
    pub fn contains(&self, topic: &str) -> bool {
        let entries = self.entries.read().unwrap();
        entries
            .get(topic)
            .map(|e| !e.is_expired())
            .unwrap_or(false)
    }

    /// Get time until a cached entry expires
    pub fn time_until_expiry(&self, topic: &str) -> Option<Duration> {
        let entries = self.entries.read().unwrap();
        entries.get(topic).map(|e| e.time_until_expiry())
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();

        if self.config.track_stats {
            let mut stats = self.stats.write().unwrap();
            stats.entries = 0;
            stats.bytes_stored = 0;
        }
    }

    /// Remove all expired entries
    pub fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.write().unwrap();
        let initial_count = entries.len();

        let expired: Vec<String> = entries
            .iter()
            .filter(|(_, e)| e.is_expired())
            .map(|(k, _)| k.clone())
            .collect();

        let mut bytes_freed = 0;
        for topic in &expired {
            if let Some(entry) = entries.remove(topic) {
                bytes_freed += entry.data.len();
            }
        }

        let removed = initial_count - entries.len();

        if self.config.track_stats && removed > 0 {
            let mut stats = self.stats.write().unwrap();
            stats.entries = entries.len();
            stats.bytes_stored = stats.bytes_stored.saturating_sub(bytes_freed);
            stats.expirations += removed as u64;
        }

        removed
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }

    /// Get all cached topic names
    pub fn topics(&self) -> Vec<String> {
        let entries = self.entries.read().unwrap();
        entries.keys().cloned().collect()
    }

    /// Get number of cached entries
    pub fn len(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().unwrap().is_empty()
    }

    // Evict one entry (LRU - least recently used)
    fn evict_one(&self, entries: &mut HashMap<String, CacheEntry>) {
        // Find the least recently accessed entry
        let oldest = entries
            .iter()
            .min_by_key(|(_, e)| e.last_accessed)
            .map(|(k, _)| k.clone());

        if let Some(topic) = oldest {
            if let Some(entry) = entries.remove(&topic) {
                if self.config.track_stats {
                    let mut stats = self.stats.write().unwrap();
                    stats.evictions += 1;
                    stats.bytes_stored = stats.bytes_stored.saturating_sub(entry.data.len());
                }
            }
        }
    }
}

impl Default for TopicCache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

/// Thread-safe cache wrapper
pub type SharedCache = Arc<TopicCache>;

/// Create a new shared cache
pub fn create_shared_cache(config: CacheConfig) -> SharedCache {
    Arc::new(TopicCache::new(config))
}

/// Cache entry info (for inspection)
#[derive(Debug, Clone)]
pub struct CacheEntryInfo {
    pub topic: String,
    pub size: usize,
    pub age: Duration,
    pub time_until_expiry: Duration,
    pub access_count: u64,
}

impl TopicCache {
    /// Get info about all cached entries
    pub fn entries_info(&self) -> Vec<CacheEntryInfo> {
        let entries = self.entries.read().unwrap();
        entries
            .iter()
            .map(|(topic, entry)| CacheEntryInfo {
                topic: topic.clone(),
                size: entry.data.len(),
                age: entry.age(),
                time_until_expiry: entry.time_until_expiry(),
                access_count: entry.access_count,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_get() {
        let cache = TopicCache::new(CacheConfig::default());

        cache.put("test", vec![1, 2, 3]);
        let result = cache.get("test").unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_expiry() {
        let config = CacheConfig {
            default_ttl: Duration::from_millis(50),
            ..Default::default()
        };
        let cache = TopicCache::new(config);

        cache.put("test", vec![1, 2, 3]);
        assert!(cache.get("test").is_some());

        std::thread::sleep(Duration::from_millis(100));

        assert!(cache.get("test").is_none());
    }

    #[test]
    fn test_custom_ttl() {
        let cache = TopicCache::new(CacheConfig::default());

        cache.put_with_ttl("short", vec![1], Duration::from_millis(50));
        cache.put_with_ttl("long", vec![2], Duration::from_secs(60));

        std::thread::sleep(Duration::from_millis(100));

        assert!(cache.get("short").is_none());
        assert!(cache.get("long").is_some());
    }

    #[test]
    fn test_remove() {
        let cache = TopicCache::new(CacheConfig::default());

        cache.put("test", vec![1, 2, 3]);
        assert!(cache.contains("test"));

        cache.remove("test");
        assert!(!cache.contains("test"));
    }

    #[test]
    fn test_cleanup_expired() {
        let config = CacheConfig {
            default_ttl: Duration::from_millis(50),
            ..Default::default()
        };
        let cache = TopicCache::new(config);

        cache.put("test1", vec![1]);
        cache.put("test2", vec![2]);
        cache.put("test3", vec![3]);

        assert_eq!(cache.len(), 3);

        std::thread::sleep(Duration::from_millis(100));

        let removed = cache.cleanup_expired();
        assert_eq!(removed, 3);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_eviction() {
        let config = CacheConfig {
            max_entries: 2,
            ..Default::default()
        };
        let cache = TopicCache::new(config);

        cache.put("a", vec![1]);
        std::thread::sleep(Duration::from_millis(10));
        cache.put("b", vec![2]);
        std::thread::sleep(Duration::from_millis(10));

        // Access "a" to make it more recent
        cache.get("a");

        // Adding "c" should evict "b" (least recently used)
        cache.put("c", vec![3]);

        assert!(cache.contains("a"));
        assert!(!cache.contains("b")); // Evicted
        assert!(cache.contains("c"));
    }

    #[test]
    fn test_stats() {
        let cache = TopicCache::new(CacheConfig::default());

        cache.put("test", vec![1, 2, 3]);

        // Hit
        cache.get("test");
        cache.get("test");

        // Miss
        cache.get("nonexistent");

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_ratio() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_max_entry_size() {
        let config = CacheConfig {
            max_entry_size: 10,
            ..Default::default()
        };
        let cache = TopicCache::new(config);

        // Should succeed (10 bytes)
        assert!(cache.put("small", vec![0; 10]));

        // Should fail (11 bytes)
        assert!(!cache.put("large", vec![0; 11]));
    }

    #[test]
    fn test_clear() {
        let cache = TopicCache::new(CacheConfig::default());

        cache.put("a", vec![1]);
        cache.put("b", vec![2]);
        cache.put("c", vec![3]);

        assert_eq!(cache.len(), 3);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_topics() {
        let cache = TopicCache::new(CacheConfig::default());

        cache.put("topic_a", vec![1]);
        cache.put("topic_b", vec![2]);

        let topics = cache.topics();
        assert!(topics.contains(&"topic_a".to_string()));
        assert!(topics.contains(&"topic_b".to_string()));
    }

    #[test]
    fn test_entries_info() {
        let cache = TopicCache::new(CacheConfig::default());

        cache.put("test", vec![1, 2, 3, 4, 5]);
        cache.get("test");
        cache.get("test");

        let infos = cache.entries_info();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].topic, "test");
        assert_eq!(infos[0].size, 5);
        assert_eq!(infos[0].access_count, 2);
    }

    #[test]
    fn test_shared_cache() {
        let cache = create_shared_cache(CacheConfig::default());

        let cache1 = cache.clone();
        let cache2 = cache.clone();

        // Write from one clone
        cache1.put("shared", vec![42]);

        // Read from another
        assert_eq!(cache2.get("shared").unwrap(), vec![42]);
    }
}
