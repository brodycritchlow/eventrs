//! Event filtering system for EventRS.
//!
//! This module provides the filtering infrastructure for events, allowing
//! handlers to be executed conditionally based on event properties.

use crate::event::Event;
use crate::error::FilterError;

/// Trait for event filters.
/// 
/// Filters determine whether an event should be processed by handlers.
/// They can be combined using logical operations (AND, OR, NOT).
pub trait Filter<E: Event>: Send + Sync + 'static {
    /// Evaluates whether the given event should pass through this filter.
    fn evaluate(&self, event: &E) -> bool;
    
    /// Returns the name of this filter for debugging.
    fn filter_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Returns whether this filter is expensive to evaluate.
    fn is_expensive(&self) -> bool {
        false
    }
}

// TODO: Implement complete filtering system
// This is a placeholder implementation that will be expanded later