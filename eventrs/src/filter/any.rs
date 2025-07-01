//! Filters that can operate on any event type using std::any::Any.

use crate::event::Event;
use std::any::{Any, TypeId};

/// A trait for events that can be filtered at the global level.
/// 
/// This trait provides the minimal interface needed for global filtering
/// without requiring Clone, making it dyn-compatible.
pub trait AnyEvent: Send + Sync + 'static {
    /// Returns the type name of the event.
    fn event_type_name(&self) -> &'static str;
    
    /// Returns the TypeId of the event.
    fn event_type_id(&self) -> TypeId;
    
    /// Returns a reference to self as Any for downcasting.
    fn as_any(&self) -> &dyn Any;
}

// Blanket implementation for all Event types
impl<T: Event> AnyEvent for T {
    fn event_type_name(&self) -> &'static str {
        T::event_type_name()
    }
    
    fn event_type_id(&self) -> TypeId {
        T::event_type_id()
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A filter that can be applied to any event type.
///
/// This is used for global, bus-level filters.
pub trait AnyEventFilter: Send + Sync + 'static {
    /// Evaluates the filter against a type-erased event.
    fn evaluate(&self, event: &dyn AnyEvent) -> bool;

    /// Returns the name of the filter.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Returns whether this filter's results can be cached.
    /// 
    /// Filters that depend only on event type (not content) should return true.
    /// Filters that examine event content should typically return false.
    fn is_cacheable(&self) -> bool {
        false
    }
    
    /// Generates a cache key for the given event.
    /// 
    /// The cache key should uniquely identify the combination of this filter's
    /// state and the relevant aspects of the event.
    fn cache_key(&self, event: &dyn AnyEvent) -> String {
        format!("{:?}", event.event_type_id())
    }
    
    /// Returns a hint about whether this filter is expensive to evaluate.
    fn is_expensive(&self) -> bool {
        false
    }
}

/// A type-erased, heap-allocated `AnyEventFilter`.
pub type BoxedAnyFilter = Box<dyn AnyEventFilter>;

/// A thread-safe, reference-counted `AnyEventFilter`.
pub type SharedAnyFilter = std::sync::Arc<dyn AnyEventFilter>;

/// A predicate-based `AnyEventFilter`.
pub struct PredicateAnyFilter<F>
where
    F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
{
    predicate: F,
    name: &'static str,
}

impl<F> PredicateAnyFilter<F>
where
    F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
{
    /// Creates a new predicate-based global filter.
    pub fn new(name: &'static str, predicate: F) -> Self {
        Self { predicate, name }
    }
}

impl<F> AnyEventFilter for PredicateAnyFilter<F>
where
    F: Fn(&dyn AnyEvent) -> bool + Send + Sync + 'static,
{
    fn evaluate(&self, event: &dyn AnyEvent) -> bool {
        (self.predicate)(event)
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

/// An always-allow filter for global use.
pub struct AllowAllAnyFilter;

impl AnyEventFilter for AllowAllAnyFilter {
    fn evaluate(&self, _event: &dyn AnyEvent) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "AllowAllAnyFilter"
    }
    
    fn is_cacheable(&self) -> bool {
        true // Result is always the same
    }
}

/// An always-reject filter for global use.
pub struct RejectAllAnyFilter;

impl AnyEventFilter for RejectAllAnyFilter {
    fn evaluate(&self, _event: &dyn AnyEvent) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "RejectAllAnyFilter"
    }
    
    fn is_cacheable(&self) -> bool {
        true // Result is always the same
    }
}

/// A filter that checks event type names.
pub struct EventTypeFilter {
    allowed_types: Vec<String>,
    mode: EventTypeFilterMode,
}

/// Mode for EventTypeFilter operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTypeFilterMode {
    /// Allow only the specified event types
    Allow,
    /// Block the specified event types
    Block,
}

impl EventTypeFilter {
    /// Creates a new event type filter that allows only the specified types.
    pub fn allow_types<I, S>(types: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            allowed_types: types.into_iter().map(|s| s.into()).collect(),
            mode: EventTypeFilterMode::Allow,
        }
    }

    /// Creates a new event type filter that blocks the specified types.
    pub fn block_types<I, S>(types: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            allowed_types: types.into_iter().map(|s| s.into()).collect(),
            mode: EventTypeFilterMode::Block,
        }
    }

    /// Adds an event type to the filter.
    pub fn add_type<S: Into<String>>(&mut self, event_type: S) {
        self.allowed_types.push(event_type.into());
    }

    /// Removes an event type from the filter.
    pub fn remove_type(&mut self, event_type: &str) -> bool {
        if let Some(pos) = self.allowed_types.iter().position(|t| t == event_type) {
            self.allowed_types.remove(pos);
            true
        } else {
            false
        }
    }

    /// Returns the list of event types in this filter.
    pub fn event_types(&self) -> &[String] {
        &self.allowed_types
    }

    /// Returns the filter mode.
    pub fn mode(&self) -> EventTypeFilterMode {
        self.mode
    }
}

impl AnyEventFilter for EventTypeFilter {
    fn evaluate(&self, event: &dyn AnyEvent) -> bool {
        let event_type_name = event.event_type_name();
        let contains = self.allowed_types.iter().any(|t| t == event_type_name);
        
        match self.mode {
            EventTypeFilterMode::Allow => contains,
            EventTypeFilterMode::Block => !contains,
        }
    }

    fn name(&self) -> &'static str {
        match self.mode {
            EventTypeFilterMode::Allow => "AllowEventTypeFilter",
            EventTypeFilterMode::Block => "BlockEventTypeFilter",
        }
    }
    
    fn is_cacheable(&self) -> bool {
        true // Only depends on event type
    }
}
