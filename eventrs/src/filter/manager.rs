//! The filter manager for the EventRS system.
//!
//! This module provides a centralized system for managing, organizing,
//! and evaluating event filters. It supports hierarchical filters,
//! dynamic filter management, and performance optimizations like caching.

use crate::filter::any::{AnyEvent, AnyEventFilter, BoxedAnyFilter};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::any::TypeId;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Manages a collection of filters for an event bus.
///
/// The `FilterManager` provides a hierarchical filtering system,
/// allowing for global, bus-level, and handler-level filters.
/// It also supports dynamic modification of filters at runtime.
pub struct FilterManager {
    parent: Option<Arc<FilterManager>>,
    filters: RwLock<Vec<FilterEntry>>,
    cache: RwLock<HashMap<String, CacheEntry>>,
    cache_ttl: Duration,
    enabled: RwLock<bool>,
}

struct FilterEntry {
    id: String,
    filter: BoxedAnyFilter,
    enabled: bool,
    priority: i32,
}

impl std::fmt::Debug for FilterEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterEntry")
            .field("id", &self.id)
            .field("filter", &"<AnyEventFilter>")
            .field("enabled", &self.enabled)
            .field("priority", &self.priority)
            .finish()
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    result: bool,
    timestamp: Instant,
    event_hash: u64,
}

impl FilterManager {
    /// Creates a new, empty `FilterManager`.
    pub fn new() -> Self {
        Self {
            parent: None,
            filters: RwLock::new(Vec::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(60), // Default 1 minute cache
            enabled: RwLock::new(true),
        }
    }

    /// Creates a new `FilterManager` with a parent.
    ///
    /// Filters from the parent manager will be evaluated before
    /// filters from this manager.
    pub fn with_parent(parent: Arc<FilterManager>) -> Self {
        Self {
            parent: Some(parent),
            filters: RwLock::new(Vec::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(60),
            enabled: RwLock::new(true),
        }
    }

    /// Creates a new `FilterManager` with custom cache TTL.
    pub fn with_cache_ttl(cache_ttl: Duration) -> Self {
        Self {
            parent: None,
            filters: RwLock::new(Vec::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl,
            enabled: RwLock::new(true),
        }
    }

    /// Adds a filter to the manager with the given ID.
    pub fn add_filter<S: Into<String>>(&self, id: S, filter: BoxedAnyFilter) {
        self.add_filter_with_priority(id, filter, 0);
    }

    /// Adds a filter to the manager with the given ID and priority.
    pub fn add_filter_with_priority<S: Into<String>>(&self, id: S, filter: BoxedAnyFilter, priority: i32) {
        let mut filters = self.filters.write().unwrap();
        let entry = FilterEntry {
            id: id.into(),
            filter,
            enabled: true,
            priority,
        };
        filters.push(entry);
        // Sort by priority (higher priority first)
        filters.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Removes a filter by its ID.
    pub fn remove_filter(&self, id: &str) -> bool {
        let mut filters = self.filters.write().unwrap();
        if let Some(pos) = filters.iter().position(|entry| entry.id == id) {
            filters.remove(pos);
            // Clear cache since filter configuration changed
            self.clear_cache();
            true
        } else {
            false
        }
    }

    /// Enables or disables a filter by its ID.
    pub fn set_filter_enabled(&self, id: &str, enabled: bool) -> bool {
        let mut filters = self.filters.write().unwrap();
        if let Some(entry) = filters.iter_mut().find(|entry| entry.id == id) {
            entry.enabled = enabled;
            // Clear cache since filter configuration changed
            drop(filters);
            self.clear_cache();
            true
        } else {
            false
        }
    }

    /// Returns whether a filter with the given ID exists and is enabled.
    pub fn is_filter_enabled(&self, id: &str) -> bool {
        let filters = self.filters.read().unwrap();
        filters.iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.enabled)
            .unwrap_or(false)
    }

    /// Lists all filter IDs in this manager.
    pub fn list_filters(&self) -> Vec<String> {
        let filters = self.filters.read().unwrap();
        filters.iter().map(|entry| entry.id.clone()).collect()
    }

    /// Enables or disables the entire filter manager.
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().unwrap() = enabled;
        if !enabled {
            self.clear_cache();
        }
    }

    /// Returns whether the filter manager is enabled.
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read().unwrap()
    }

    /// Clears the filter cache.
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }

    /// Sets the cache TTL for expensive filters.
    pub fn set_cache_ttl(&mut self, ttl: Duration) {
        self.cache_ttl = ttl;
        self.clear_cache();
    }

    /// Returns the number of filters in this manager.
    pub fn filter_count(&self) -> usize {
        self.filters.read().unwrap().len()
    }

    /// Returns the total number of filters including parent hierarchy.
    pub fn total_filter_count(&self) -> usize {
        let local_count = self.filter_count();
        let parent_count = self.parent.as_ref()
            .map(|p| p.total_filter_count())
            .unwrap_or(0);
        local_count + parent_count
    }

    /// Evaluates all filters in the manager against the given event.
    ///
    /// This method evaluates filters in a hierarchical manner, starting
    /// with the parent manager (if one exists) and then evaluating
    /// the filters in this manager.
    pub fn evaluate(&self, event: &dyn AnyEvent) -> bool {
        // If manager is disabled, allow all events
        if !self.is_enabled() {
            return true;
        }

        // Evaluate parent filters first
        if let Some(parent) = &self.parent {
            if !parent.evaluate(event) {
                return false;
            }
        }

        // Evaluate local filters with caching
        self.evaluate_local_filters(event)
    }

    /// Evaluates only the local filters (no parent evaluation).
    pub fn evaluate_local(&self, event: &dyn AnyEvent) -> bool {
        if !self.is_enabled() {
            return true;
        }
        self.evaluate_local_filters(event)
    }

    fn evaluate_local_filters(&self, event: &dyn AnyEvent) -> bool {
        let filters = self.filters.read().unwrap();
        
        for entry in filters.iter() {
            if !entry.enabled {
                continue;
            }

            // Evaluate filter (caching disabled for content-based filters)
            // TODO: Implement smart caching that considers event content,
            // or add a marker trait for cacheable filters
            let result = entry.filter.evaluate(event);
            
            if !result {
                return false;
            }
        }
        
        true
    }

    fn check_cache(&self, key: &str) -> Option<bool> {
        let cache = self.cache.read().unwrap();
        if let Some(entry) = cache.get(key) {
            if entry.timestamp.elapsed() < self.cache_ttl {
                Some(entry.result)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn update_cache(&self, key: String, result: bool, event_type_id: TypeId) {
        let mut cache = self.cache.write().unwrap();
        let mut hasher = DefaultHasher::new();
        event_type_id.hash(&mut hasher);
        let event_hash = hasher.finish();
        
        cache.insert(key, CacheEntry {
            result,
            timestamp: Instant::now(),
            event_hash,
        });
        
        // Clean up old entries if cache gets too large
        if cache.len() > 1000 {
            let now = Instant::now();
            cache.retain(|_, entry| now.duration_since(entry.timestamp) < self.cache_ttl);
        }
    }
}

impl Default for FilterManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating FilterManager instances with custom configuration.
pub struct FilterManagerBuilder {
    parent: Option<Arc<FilterManager>>,
    cache_ttl: Duration,
    enabled: bool,
}

impl FilterManagerBuilder {
    /// Creates a new FilterManagerBuilder.
    pub fn new() -> Self {
        Self {
            parent: None,
            cache_ttl: Duration::from_secs(60),
            enabled: true,
        }
    }

    /// Sets the parent FilterManager.
    pub fn with_parent(mut self, parent: Arc<FilterManager>) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Sets the cache TTL for expensive filters.
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Sets whether the FilterManager starts enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Builds the FilterManager.
    pub fn build(self) -> FilterManager {
        let manager = if let Some(parent) = self.parent {
            FilterManager::with_parent(parent)
        } else {
            FilterManager::with_cache_ttl(self.cache_ttl)
        };
        
        manager.set_enabled(self.enabled);
        manager
    }
}

impl Default for FilterManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}