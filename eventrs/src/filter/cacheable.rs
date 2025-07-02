//! Caching support for filters.

use crate::filter::any::AnyEvent;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Trait for events that can provide a content hash for caching.
///
/// Events implementing this trait can participate in content-aware filter caching.
pub trait CacheableEvent: AnyEvent {
    /// Returns a hash of the event's content.
    ///
    /// This hash should change whenever the event's content changes in a way
    /// that might affect filter results.
    fn content_hash(&self) -> u64;
}

/// Helper function to compute a hash for any hashable value.
pub fn compute_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// A dynamic filter that can be updated at runtime.
///
/// This filter wraps a closure that can be replaced dynamically,
/// allowing filter behavior to be modified without recreating the filter.
pub struct DynamicFilter {
    name: String,
    predicate: std::sync::RwLock<Box<dyn Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static>>,
    cacheable: bool,
}

impl DynamicFilter {
    /// Creates a new dynamic filter.
    pub fn new<S: Into<String>, F>(name: S, predicate: F) -> Self
    where
        F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            predicate: std::sync::RwLock::new(Box::new(predicate)),
            cacheable: false,
        }
    }

    /// Creates a new dynamic filter wrapped in Arc for sharing.
    pub fn new_shared<S: Into<String>, F>(name: S, predicate: F) -> Arc<Self>
    where
        F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
    {
        Arc::new(Self::new(name, predicate))
    }

    /// Creates a new cacheable dynamic filter.
    ///
    /// Only use this for filters that depend solely on event type,
    /// not event content.
    pub fn new_cacheable<S: Into<String>, F>(name: S, predicate: F) -> Self
    where
        F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            predicate: std::sync::RwLock::new(Box::new(predicate)),
            cacheable: true,
        }
    }

    /// Updates the filter predicate.
    pub fn update_predicate<F>(&self, new_predicate: F)
    where
        F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
    {
        *self.predicate.write().unwrap() = Box::new(new_predicate);
    }

    /// Returns the name of this filter.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl crate::filter::any::AnyEventFilter for DynamicFilter {
    fn evaluate(&self, event: &dyn AnyEvent) -> bool {
        let predicate = self.predicate.read().unwrap();
        predicate(event)
    }

    fn name(&self) -> &'static str {
        // We leak the string to get a 'static lifetime
        // This is acceptable for filter names which are typically long-lived
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn is_cacheable(&self) -> bool {
        self.cacheable
    }
}

impl crate::filter::any::AnyEventFilter for Arc<DynamicFilter> {
    fn evaluate(&self, event: &dyn AnyEvent) -> bool {
        (**self).evaluate(event)
    }

    fn name(&self) -> &'static str {
        // We leak the string to get a 'static lifetime
        // This is acceptable for filter names which are typically long-lived
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn is_cacheable(&self) -> bool {
        (**self).is_cacheable()
    }
}

/// A filter that caches results for events based on their content hash.
pub struct ContentAwareCacheFilter<F>
where
    F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
{
    name: String,
    predicate: F,
    cache: std::sync::RwLock<std::collections::HashMap<u64, bool>>,
    max_cache_size: usize,
}

impl<F> ContentAwareCacheFilter<F>
where
    F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
{
    /// Creates a new content-aware caching filter.
    pub fn new<S: Into<String>>(name: S, predicate: F) -> Self {
        Self {
            name: name.into(),
            predicate,
            cache: std::sync::RwLock::new(std::collections::HashMap::new()),
            max_cache_size: 1000,
        }
    }

    /// Sets the maximum cache size.
    pub fn with_max_cache_size(mut self, size: usize) -> Self {
        self.max_cache_size = size;
        self
    }

    fn get_content_hash(&self, event: &dyn AnyEvent) -> u64 {
        // For now, just use type-based hash
        // In a real implementation, events could implement CacheableEvent directly
        compute_hash(&event.event_type_id())
    }
}

impl<F> crate::filter::any::AnyEventFilter for ContentAwareCacheFilter<F>
where
    F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
{
    fn evaluate(&self, event: &dyn AnyEvent) -> bool {
        let hash = self.get_content_hash(event);

        // Check cache
        {
            let cache = self.cache.read().unwrap();
            if let Some(&result) = cache.get(&hash) {
                return result;
            }
        }

        // Evaluate predicate
        let result = (self.predicate)(event);

        // Update cache
        {
            let mut cache = self.cache.write().unwrap();

            // Evict old entries if cache is too large
            if cache.len() >= self.max_cache_size {
                // Simple eviction: remove half the entries
                let to_remove = cache.len() / 2;
                let keys: Vec<_> = cache.keys().take(to_remove).cloned().collect();
                for key in keys {
                    cache.remove(&key);
                }
            }

            cache.insert(hash, result);
        }

        result
    }

    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn is_cacheable(&self) -> bool {
        false // This filter has its own internal cache
    }

    fn is_expensive(&self) -> bool {
        true // Content-aware filters are typically expensive
    }
}
