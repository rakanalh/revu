use crate::github::models::DiffContent;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache key for file content
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FileCacheKey {
    pub owner: String,
    pub repo: String,
    pub path: String,
    pub sha: String,
}

/// Cache key for diff content (base_sha -> head_sha for a file)
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct DiffCacheKey {
    pub owner: String,
    pub repo: String,
    pub path: String,
    pub base_sha: String,
    pub head_sha: String,
}

/// Thread-safe LRU cache for file contents
pub struct FileContentCache {
    cache: Arc<RwLock<LruCache<FileCacheKey, String>>>,
}

impl FileContentCache {
    /// Create a new cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(cap))),
        }
    }

    /// Get a file from the cache
    pub async fn get(&self, key: &FileCacheKey) -> Option<String> {
        let mut cache = self.cache.write().await;
        cache.get(key).cloned()
    }

    /// Put a file in the cache
    pub async fn put(&self, key: FileCacheKey, content: String) {
        let mut cache = self.cache.write().await;
        cache.put(key, content);
    }

    /// Clear the cache
    #[allow(dead_code)]
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

impl Clone for FileContentCache {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
        }
    }
}

impl Default for FileContentCache {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Thread-safe LRU cache for diff contents
pub struct DiffCache {
    cache: Arc<RwLock<LruCache<DiffCacheKey, DiffContent>>>,
}

impl DiffCache {
    /// Create a new cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(50).unwrap());
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(cap))),
        }
    }

    /// Get a diff from the cache
    pub async fn get(&self, key: &DiffCacheKey) -> Option<DiffContent> {
        let mut cache = self.cache.write().await;
        cache.get(key).cloned()
    }

    /// Put a diff in the cache
    pub async fn put(&self, key: DiffCacheKey, content: DiffContent) {
        let mut cache = self.cache.write().await;
        cache.put(key, content);
    }

    /// Clear the cache
    #[allow(dead_code)]
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

impl Clone for DiffCache {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
        }
    }
}

impl Default for DiffCache {
    fn default() -> Self {
        Self::new(50)
    }
}
