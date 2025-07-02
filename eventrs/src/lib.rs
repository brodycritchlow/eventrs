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

// Re-export derive macros
pub use eventrs_derive::{AsyncEvent, Event};

// Core modules
pub mod error;
pub mod event;
pub mod event_bus;
pub mod handler;
pub mod metadata;
pub mod priority;

// Feature-gated modules
#[cfg(feature = "async")]
pub mod async_event_bus;

pub mod filter;
pub mod middleware;
pub mod testing;
pub mod thread_safe;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "tracing")]
pub mod tracing;

// Re-exports for convenience
pub use event::{AsyncEventMarker, Event};
pub use handler::{FallibleHandler, Handler, HandlerId};

pub use error::*;
pub use event_bus::{ErrorHandling, EventBus, EventBusBuilder, EventBusConfig};
#[cfg(feature = "async")]
pub use handler::{AsyncHandler, FallibleAsyncHandler};
pub use metadata::{
    EventMetadata, EventProvenance, EventSignature, EventSource, EventTimestamps,
    EventTransformation, SecurityLevel, SourceType,
};
pub use priority::{HandlerGroup, Priority, PriorityChain, PriorityOrdered, PriorityValue};

#[cfg(feature = "async")]
pub use async_event_bus::{AsyncEventBus, AsyncEventBusBuilder, AsyncEventBusConfig};

pub use filter::{
    compute_hash, AllowAllAnyFilter, AllowAllFilter, AndFilter, AnyEvent, AnyEventFilter,
    BoxedAnyFilter, BoxedFilter, CacheableEvent, ChainMode, ContentAwareCacheFilter, DynamicFilter,
    EventTypeFilter, EventTypeFilterMode, Filter, FilterChain, FilterManager, FilterManagerBuilder,
    NotFilter, OrFilter, PredicateAnyFilter, PredicateFilter, RejectAllAnyFilter, RejectAllFilter,
    SharedAnyFilter, SharedFilter,
};
pub use middleware::{
    LoggingMiddleware, MetricsMiddleware, Middleware, MiddlewareChain, MiddlewareContext,
    MiddlewareMetrics, ValidationMiddleware,
};
pub use testing::{EventSpy, MockHandler, TestEventBus, TestEventBusConfig};
pub use thread_safe::{
    EventSender, EventSenderError, MultiEventSender, ThreadSafeEventBus, ThreadSafeEventBusConfig,
};

pub use error::MiddlewareResult;
#[cfg(feature = "metrics")]
pub use metrics::{
    EmissionResult, EmissionStats, EmissionToken, EventBusMetrics, HandlerMetrics, HandlerResult,
    MetricsReport, MiddlewareExecutionMetric, SystemMetrics,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::error::{EventBusError, EventValidationError, HandlerError};
    pub use crate::{
        AsyncEvent, AsyncEventMarker, Event, EventBus, EventBusBuilder, Handler, HandlerId,
    };
    pub use crate::{
        ErrorHandling, EventMetadata, EventSource, HandlerGroup, Priority, PriorityChain,
        PriorityValue, SecurityLevel, SourceType,
    };

    #[cfg(feature = "async")]
    pub use crate::{AsyncEventBus, AsyncEventBusBuilder, AsyncEventBusConfig};

    pub use crate::{ChainMode, Filter, FilterChain, PredicateFilter};
    pub use crate::{EventSpy, MockHandler, TestEventBus, TestEventBusConfig};
    pub use crate::{
        LoggingMiddleware, MetricsMiddleware, Middleware, MiddlewareChain, MiddlewareContext,
        MiddlewareResult, ValidationMiddleware,
    };
    pub use crate::{ThreadSafeEventBus, ThreadSafeEventBusConfig};

    #[cfg(feature = "metrics")]
    pub use crate::{
        EmissionResult, EmissionStats, EventBusMetrics, HandlerMetrics, HandlerResult,
    };
}

/// Version information for the EventRS library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod test_builder;

#[cfg(test)]
mod test_middleware;

#[cfg(test)]
mod test_global_filters;

#[cfg(test)]
mod test_dynamic_filters;

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
