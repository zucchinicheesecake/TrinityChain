#![allow(clippy::multiple_bound_locations)]
//! Caching layer for frequently accessed blockchain data
//!
//! Provides LRU caching for:
//! - Recent blocks (100 block limit)
//! - UTXO set entries (hot triangles)
//! - Address balances
use crate::blockchain::{Block, Sha256Hash};
use crate::geometry::Triangle;
use lru::LruCache;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;

// Re-export for convenience in implementing the trait bounds
pub use std::hash::Hash;

/// A trait for generic cache operations.
/// It uses `std::borrow::Borrow` to allow getting/removing elements without cloning the key.
pub trait CacheInner<K, V> {
    /// Retrieve a value from the cache without mutating the cache's state (e.g., LruCache promotion).
    fn get_non_mut<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq;

    /// Insert a value into the cache.
    fn put(&mut self, key: K, value: V);

    /// Clear all entries from the cache.
    fn clear(&mut self);

    /// Get the number of entries in the cache.
    fn len(&self) -> usize;

    /// Check if the cache is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove a value from the cache.
    fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq;
}

impl<K, V> CacheInner<K, V> for LruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn get_non_mut<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        // Use the non-mutating peek for read operations
        self.peek(key)
    }

    fn put(&mut self, key: K, value: V) {
        self.put(key, value);
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        self.pop(key)
    }
}

// A simple map implementation for the BalanceCache (no eviction)
impl<K, V> CacheInner<K, V> for HashMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn get_non_mut<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        // HashMap get doesn't mutate, so it's the direct implementation
        self.get(key)
    }

    fn put(&mut self, key: K, value: V) {
        self.insert(key, value);
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        self.remove(key)
    }
}

/// Generic, thread-safe cache wrapper using RwLock.
pub struct ThreadSafeCache<K, V, T: CacheInner<K, V>> {
    cache: Arc<RwLock<T>>,
    _phantom_k: PhantomData<K>, // Marker for K
    _phantom_v: PhantomData<V>, // Marker for V
}

impl<K, V, T: CacheInner<K, V>> ThreadSafeCache<K, V, T>
where
    K: Hash + Eq + Clone,
    V: Clone,
    T: Default,
{
    /// Create a new cache with the inner type's default implementation.
    /// This is typically used for HashMap-based caches without a fixed capacity.
    pub fn new_default() -> Self {
        Self {
            cache: Arc::new(RwLock::new(T::default())),
            _phantom_k: PhantomData,
            _phantom_v: PhantomData,
        }
    }
}

impl<K, V> ThreadSafeCache<K, V, LruCache<K, V>>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// Create a new LRU cache with specified capacity.
    pub fn new_lru(capacity: usize) -> Self {
        // Use NonZeroUsize::new_truncate for a safer conversion that handles 0 by mapping it to 1.
        let capacity_nz = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN);

        let cache = LruCache::new(capacity_nz);
        Self {
            cache: Arc::new(RwLock::new(cache)),
            _phantom_k: PhantomData,
            _phantom_v: PhantomData,
        }
    }

    /// Get current capacity of the LRU cache.
    pub async fn capacity(&self) -> usize {
        let cache = self.cache.read().await;
        cache.cap().get()
    }
}

impl<K, V, T: CacheInner<K, V>> ThreadSafeCache<K, V, T>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// Get a value from cache. Uses read lock for non-mutating access.
    pub async fn get<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        let cache = self.cache.read().await;
        cache.get_non_mut(key).cloned()
    }

    /// Put a value in cache. Uses write lock.
    pub async fn put(&self, key: K, value: V) {
        let mut cache = self.cache.write().await;
        cache.put(key, value);
    }

    /// Remove a value from cache. Uses write lock.
    pub async fn remove<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut cache = self.cache.write().await;
        cache.remove(key)
    }

    /// Clear all cached entries. Uses write lock.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache size. Uses read lock.
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if cache is empty. Uses read lock.
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }
}

impl<K, V, T: CacheInner<K, V>> Clone for ThreadSafeCache<K, V, T> {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
            _phantom_k: PhantomData,
            _phantom_v: PhantomData,
        }
    }
}

// Type aliases for specific caches using the generic wrapper

/// Cache for recent blocks
pub type BlockCache = ThreadSafeCache<Sha256Hash, Block, LruCache<Sha256Hash, Block>>;

impl BlockCache {
    pub const DEFAULT_CAPACITY: usize = 100;

    pub fn new(capacity: usize) -> Self {
        Self::new_lru(capacity)
    }

    pub async fn stats(&self) -> (usize, usize) {
        (self.len().await, self.capacity().await)
    }
}

/// Cache for UTXO set entries
pub type UtxoCache = ThreadSafeCache<Sha256Hash, Triangle, LruCache<Sha256Hash, Triangle>>;

impl UtxoCache {
    pub const DEFAULT_CAPACITY: usize = 10000;

    pub fn new(capacity: usize) -> Self {
        Self::new_lru(capacity)
    }

    pub async fn stats(&self) -> (usize, usize) {
        (self.len().await, self.capacity().await)
    }
}

/// Cache for address balances. Uses a u64 for safe currency representation (satoshis).
pub type BalanceCache = ThreadSafeCache<String, u64, HashMap<String, u64>>;

impl BalanceCache {
    /// Create a new balance cache
    pub fn new() -> Self {
        Self::new_default()
    }

    /// Set balance for address
    pub async fn set(&self, address: String, balance: u64) {
        self.put(address, balance).await;
    }

    /// Get cached balance for address
    pub async fn get_balance(&self, address: &str) -> Option<u64> {
        // Uses the generic `get<Q>` with Q=&str, which avoids cloning String key.
        self.get(address).await
    }

    /// Invalidate all cached balances
    pub async fn invalidate_all(&self) {
        self.clear().await;
    }

    /// Invalidate specific address balance
    pub async fn invalidate(&self, address: &str) -> Option<u64> {
        self.remove(address).await
    }

    pub async fn size(&self) -> usize {
        self.len().await
    }
}

impl Default for BalanceCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined cache for all blockchain data
pub struct BlockchainCache {
    pub blocks: BlockCache,
    pub utxo: UtxoCache,
    pub balances: BalanceCache,
}

impl BlockchainCache {
    /// Create a new blockchain cache
    pub fn new(block_capacity: usize, utxo_capacity: usize) -> Self {
        Self {
            blocks: BlockCache::new(block_capacity),
            utxo: UtxoCache::new(utxo_capacity),
            balances: BalanceCache::new(),
        }
    }

    /// Create a new blockchain cache with default capacities
    pub fn new_default() -> Self {
        Self {
            blocks: BlockCache::new(BlockCache::DEFAULT_CAPACITY),
            utxo: UtxoCache::new(UtxoCache::DEFAULT_CAPACITY),
            balances: BalanceCache::new_default(),
        }
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        self.blocks.clear().await;
        self.utxo.clear().await;
        self.balances.invalidate_all().await;
    }
}

impl Clone for BlockchainCache {
    fn clone(&self) -> Self {
        Self {
            blocks: self.blocks.clone(),
            utxo: self.utxo.clone(),
            balances: self.balances.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::BlockHeader;

    #[tokio::test]
    async fn test_block_cache() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let cache = BlockCache::new(10);
            let hash = [0u8; 32];
            let block = Block {
                header: BlockHeader {
                    height: 1,
                    previous_hash: [0; 32],
                    timestamp: 0,
                    difficulty: 1,
                    nonce: 0,
                    merkle_root: [0; 32],
                },
                transactions: vec![],
            };

            cache.put(hash, block.clone()).await;
            let retrieved = cache.get(&hash).await;
            assert!(retrieved.is_some());
            assert_eq!(cache.len().await, 1);

            let removed = cache.remove(&hash).await;
            assert!(removed.is_some());
            assert_eq!(cache.len().await, 0);
        }).await.expect("test_block_cache timed out");
    }

    #[tokio::test]
    async fn test_balance_cache() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let cache = BalanceCache::new();
            let addr = "test_address".to_string();

            cache.set(addr.clone(), 100500).await;
            let balance = cache.get_balance(&addr).await;
            assert_eq!(balance, Some(100500));

            let removed = cache.invalidate(&addr).await;
            assert_eq!(removed, Some(100500));
            let balance = cache.get_balance(&addr).await;
            assert!(balance.is_none());
        }).await.expect("test_balance_cache timed out");
    }

    #[tokio::test]
    async fn test_utxo_cache_lru_eviction() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let cache = UtxoCache::new(5);
            let triangle = Triangle::genesis();

            // Fill cache to capacity
            for i in 0..5 {
                let mut hash = [0u8; 32];
                hash[0] = i as u8;
                cache.put(hash, triangle.clone()).await;
            }

            let (size, cap) = cache.stats().await;
            assert_eq!(size, 5);
            assert_eq!(cap, 5);

            // Add one more, should evict oldest (0)
            let hash_to_evict = [0u8; 32];
            let mut hash_new = [0u8; 32];
            hash_new[0] = 6;
            cache.put(hash_new, triangle.clone()).await;

            let (size, _) = cache.stats().await;
            assert_eq!(size, 5);

            let evicted_check = cache.get(&hash_to_evict).await;
            assert!(evicted_check.is_none());
        
            let new_check = cache.get(&hash_new).await;
            assert!(new_check.is_some());
        }).await.expect("test_utxo_cache_lru_eviction timed out");
    }
}
