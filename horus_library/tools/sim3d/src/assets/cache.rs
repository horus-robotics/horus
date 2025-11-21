//! Asset caching system
//!
//! Provides LRU caching for meshes, textures, and other assets to avoid
//! redundant loading from disk.

use bevy::prelude::*;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

/// Asset cache statistics
#[derive(Default, Debug, Clone)]
pub struct CacheStats {
    pub mesh_hits: usize,
    pub mesh_misses: usize,
    pub texture_hits: usize,
    pub texture_misses: usize,
    pub total_meshes_loaded: usize,
    pub total_textures_loaded: usize,
}

impl CacheStats {
    /// Get cache hit rate for meshes
    pub fn mesh_hit_rate(&self) -> f32 {
        let total = self.mesh_hits + self.mesh_misses;
        if total == 0 {
            return 0.0;
        }
        self.mesh_hits as f32 / total as f32
    }

    /// Get cache hit rate for textures
    pub fn texture_hit_rate(&self) -> f32 {
        let total = self.texture_hits + self.texture_misses;
        if total == 0 {
            return 0.0;
        }
        self.texture_hits as f32 / total as f32
    }

    /// Get overall hit rate
    pub fn overall_hit_rate(&self) -> f32 {
        let total_hits = self.mesh_hits + self.texture_hits;
        let total_misses = self.mesh_misses + self.texture_misses;
        let total = total_hits + total_misses;
        if total == 0 {
            return 0.0;
        }
        total_hits as f32 / total as f32
    }
}

/// Asset cache resource for Bevy
#[derive(Resource)]
pub struct AssetCache {
    /// Mesh cache with LRU eviction
    meshes: LruCache<PathBuf, Handle<Mesh>>,
    /// Texture cache with LRU eviction
    textures: LruCache<PathBuf, Handle<Image>>,
    /// Material cache (by name)
    materials: HashMap<String, Handle<StandardMaterial>>,
    /// URDF cache (parsed robot descriptions)
    urdfs: HashMap<PathBuf, urdf_rs::Robot>,
    /// Cache statistics
    stats: CacheStats,
}

impl AssetCache {
    /// Create a new asset cache with specified capacities
    pub fn new(max_meshes: usize, max_textures: usize) -> Self {
        let mesh_capacity = NonZeroUsize::new(max_meshes.max(1)).unwrap();
        let texture_capacity = NonZeroUsize::new(max_textures.max(1)).unwrap();

        Self {
            meshes: LruCache::new(mesh_capacity),
            textures: LruCache::new(texture_capacity),
            materials: HashMap::new(),
            urdfs: HashMap::new(),
            stats: CacheStats::default(),
        }
    }

    /// Create with default capacities (1000 meshes, 500 textures)
    pub fn default_capacity() -> Self {
        Self::new(1000, 500)
    }

    /// Get or load a mesh using a loader function
    pub fn get_or_load_mesh<F>(
        &mut self,
        path: &Path,
        loader: F,
    ) -> Result<Handle<Mesh>, anyhow::Error>
    where
        F: FnOnce() -> Result<Handle<Mesh>, anyhow::Error>,
    {
        let path_buf = path.to_path_buf();

        // Check cache first
        if let Some(handle) = self.meshes.get(&path_buf) {
            self.stats.mesh_hits += 1;
            tracing::info!("Mesh cache HIT: {}", path.display());
            return Ok(handle.clone());
        }

        // Cache miss - load from disk
        self.stats.mesh_misses += 1;
        tracing::info!("Mesh cache MISS, loading: {}", path.display());

        let handle = loader()?;
        self.meshes.put(path_buf, handle.clone());
        self.stats.total_meshes_loaded += 1;

        Ok(handle)
    }

    /// Get a cached mesh if it exists
    pub fn get_mesh(&mut self, path: &Path) -> Option<Handle<Mesh>> {
        self.meshes.get(&path.to_path_buf()).cloned()
    }

    /// Get or load a texture using a loader function
    pub fn get_or_load_texture<F>(
        &mut self,
        path: &Path,
        loader: F,
    ) -> Result<Handle<Image>, anyhow::Error>
    where
        F: FnOnce() -> Result<Handle<Image>, anyhow::Error>,
    {
        let path_buf = path.to_path_buf();

        // Check cache first
        if let Some(handle) = self.textures.get(&path_buf) {
            self.stats.texture_hits += 1;
            tracing::info!("Texture cache HIT: {}", path.display());
            return Ok(handle.clone());
        }

        // Cache miss - load from disk
        self.stats.texture_misses += 1;
        tracing::info!("Texture cache MISS, loading: {}", path.display());

        let handle = loader()?;
        self.textures.put(path_buf, handle.clone());
        self.stats.total_textures_loaded += 1;

        Ok(handle)
    }

    /// Get a cached texture if it exists
    pub fn get_texture(&mut self, path: &Path) -> Option<Handle<Image>> {
        self.textures.get(&path.to_path_buf()).cloned()
    }

    /// Cache a material by name
    pub fn cache_material(&mut self, name: String, handle: Handle<StandardMaterial>) {
        self.materials.insert(name, handle);
    }

    /// Get a cached material by name
    pub fn get_material(&self, name: &str) -> Option<Handle<StandardMaterial>> {
        self.materials.get(name).cloned()
    }

    /// Cache a parsed URDF
    pub fn cache_urdf(&mut self, path: PathBuf, urdf: urdf_rs::Robot) {
        self.urdfs.insert(path, urdf);
    }

    /// Get a cached URDF
    pub fn get_urdf(&self, path: &Path) -> Option<&urdf_rs::Robot> {
        self.urdfs.get(path)
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Get mesh cache hit rate
    pub fn mesh_hit_rate(&self) -> f32 {
        self.stats.mesh_hit_rate()
    }

    /// Get texture cache hit rate
    pub fn texture_hit_rate(&self) -> f32 {
        self.stats.texture_hit_rate()
    }

    /// Get overall cache hit rate
    pub fn overall_hit_rate(&self) -> f32 {
        self.stats.overall_hit_rate()
    }

    /// Clear all caches
    pub fn clear(&mut self) {
        self.meshes.clear();
        self.textures.clear();
        self.materials.clear();
        self.urdfs.clear();
        tracing::info!("Asset cache cleared");
    }

    /// Clear only mesh cache
    pub fn clear_meshes(&mut self) {
        self.meshes.clear();
        tracing::info!("Mesh cache cleared");
    }

    /// Clear only texture cache
    pub fn clear_textures(&mut self) {
        self.textures.clear();
        tracing::info!("Texture cache cleared");
    }

    /// Get number of cached meshes
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    /// Get number of cached textures
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Get number of cached materials
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }

    /// Get number of cached URDFs
    pub fn urdf_count(&self) -> usize {
        self.urdfs.len()
    }

    /// Print cache statistics
    pub fn print_stats(&self) {
        let stats = &self.stats;
        tracing::info!("=== Asset Cache Statistics ===");
        tracing::info!("Meshes:");
        tracing::info!("  Cached: {}", self.mesh_count());
        tracing::info!("  Hits: {}", stats.mesh_hits);
        tracing::info!("  Misses: {}", stats.mesh_misses);
        tracing::info!("  Hit rate: {:.1}%", stats.mesh_hit_rate() * 100.0);
        tracing::info!("  Total loaded: {}", stats.total_meshes_loaded);
        tracing::info!("Textures:");
        tracing::info!("  Cached: {}", self.texture_count());
        tracing::info!("  Hits: {}", stats.texture_hits);
        tracing::info!("  Misses: {}", stats.texture_misses);
        tracing::info!("  Hit rate: {:.1}%", stats.texture_hit_rate() * 100.0);
        tracing::info!("  Total loaded: {}", stats.total_textures_loaded);
        tracing::info!("Overall hit rate: {:.1}%", stats.overall_hit_rate() * 100.0);
        tracing::info!("Materials cached: {}", self.material_count());
        tracing::info!("URDFs cached: {}", self.urdf_count());
    }
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::default_capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::default();
        assert_eq!(stats.mesh_hit_rate(), 0.0);

        stats.mesh_hits = 8;
        stats.mesh_misses = 2;
        assert_eq!(stats.mesh_hit_rate(), 0.8);

        stats.texture_hits = 9;
        stats.texture_misses = 1;
        assert_eq!(stats.texture_hit_rate(), 0.9);

        assert_eq!(stats.overall_hit_rate(), 0.85);
    }

    #[test]
    fn test_cache_creation() {
        let cache = AssetCache::new(100, 50);
        assert_eq!(cache.mesh_count(), 0);
        assert_eq!(cache.texture_count(), 0);
        assert_eq!(cache.material_count(), 0);
    }
}
