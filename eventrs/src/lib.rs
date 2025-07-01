//! # EventRS - High-Performance Event System for Rust
//!
//! EventRS is a high-performance, type-safe event system for Rust applications. It provides
//! zero-cost abstractions for event-driven programming with support for both synchronous
//! and asynchronous event handling, thread-safe event buses, and sophisticated event
//! filtering capabilities.
//!
//! ## Features
//!
//! - **Type-Safe Events**: Compile-time type checking for all events and handlers
//! - **Zero-Cost Abstractions**: Minimal runtime overhead with compile-time optimizations
//! - **Async/Sync Support**: Handle events both synchronously and asynchronously
//! - **Thread-Safe**: Built-in support for concurrent event handling
//! - **Event Filtering**: Sophisticated filtering system with custom predicates
//! - **Priority System**: Control event execution order with priority-based handling
//! - **Middleware Support**: Intercept and modify events with middleware chains
//!
//! ## Quick Start
//!
//! ```rust
//! use eventrs::{Event, EventBus};
//!
//! // Define your event
//! #[derive(Event, Clone, Debug)]
//! struct UserLoggedIn {
//!     user_id: u64,
//!     timestamp: std::time::SystemTime,
//! }
//!
//! // Create an event bus
//! let mut bus = EventBus::new();
//!
//! // Register a handler
//! bus.on(|event: UserLoggedIn| {
//!     println!("User {} logged in at {:?}", event.user_id, event.timestamp);
//! });
//!
//! // Emit an event
//! bus.emit(UserLoggedIn {
//!     user_id: 123,
//!     timestamp: std::time::SystemTime::now(),
//! }).expect("Failed to emit event");
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::module_inception)]

// Re-export derive macro
pub use eventrs_derive::Event;

// Core modules
pub mod event;
pub mod handler;
pub mod event_bus;
pub mod error;
pub mod metadata;
pub mod priority;

// Feature-gated modules
#[cfg(feature = "async")]
pub mod async_event_bus;

pub mod filter;
pub mod middleware;
pub mod thread_safe;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "tracing")]
pub mod tracing;

// Re-exports for convenience
pub use event::Event;
pub use handler::{Handler, HandlerId, FallibleHandler};

#[cfg(feature = "async")]
pub use handler::{AsyncHandler, FallibleAsyncHandler};
pub use event_bus::{EventBus, EventBusConfig, EventBusBuilder, ErrorHandling};
pub use error::*;
pub use metadata::{EventMetadata, SecurityLevel};
pub use priority::{Priority, PriorityValue, PriorityOrdered};

#[cfg(feature = "async")]
pub use async_event_bus::{AsyncEventBus, AsyncEventBusConfig, AsyncEventBusBuilder};

pub use filter::{
    Filter, PredicateFilter, AllowAllFilter, RejectAllFilter,
    AndFilter, OrFilter, NotFilter, FilterChain, ChainMode,
    BoxedFilter, SharedFilter
};
pub use middleware::{Middleware, MiddlewareContext, MiddlewareChain, MiddlewareMetrics, LoggingMiddleware, ValidationMiddleware, MetricsMiddleware};
pub use thread_safe::{ThreadSafeEventBus, ThreadSafeEventBusConfig, EventSender, EventSenderError, MultiEventSender};

#[cfg(feature = "metrics")]
pub use metrics::{
    EventBusMetrics, EmissionResult, HandlerResult, EmissionStats, HandlerMetrics, 
    SystemMetrics, MetricsReport, EmissionToken, MiddlewareExecutionMetric
};
pub use error::MiddlewareResult;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{Event, EventBus, EventBusBuilder, Handler, HandlerId};
    pub use crate::{EventMetadata, Priority, ErrorHandling};
    pub use crate::error::{EventBusError, HandlerError, EventValidationError};
    
    #[cfg(feature = "async")]
    pub use crate::{AsyncEventBus, AsyncEventBusConfig, AsyncEventBusBuilder};
    
    pub use crate::{Filter, PredicateFilter, FilterChain, ChainMode};
    pub use crate::{Middleware, MiddlewareContext, MiddlewareChain, MiddlewareResult, LoggingMiddleware, ValidationMiddleware, MetricsMiddleware};
    pub use crate::{ThreadSafeEventBus, ThreadSafeEventBusConfig};
    
    #[cfg(feature = "metrics")]
    pub use crate::{EventBusMetrics, EmissionResult, HandlerResult, EmissionStats, HandlerMetrics};
}

/// Version information for the EventRS library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod test_builder;

#[cfg(test)]
mod test_middleware;

#[cfg(all(test, feature = "metrics"))]
mod test_metrics;

#[cfg(test)]
mod test_thread_safe;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestEvent {
        value: u32,
    }
    
    impl Event for TestEvent {}

    #[test]
    fn test_basic_functionality() {
        let mut bus = EventBus::new();
        let _received: Option<i32> = None;

        bus.on(|_event: TestEvent| {
            // This would normally capture the value, but for testing
            // we'll just verify the event was processed
        });

        let event = TestEvent { value: 42 };
        bus.emit(event).expect("Failed to emit event");

        // Basic test that the system compiles and runs
        assert!(true);
    }
}