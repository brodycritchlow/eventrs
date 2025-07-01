# Core Traits

This document covers the fundamental traits that form the foundation of EventRS. Understanding these traits is essential for working with the library effectively.

## Event Trait

The `Event` trait is the core abstraction for all events in EventRS.

```rust
/// Core trait that all events must implement.
/// 
/// This trait can be derived for most use cases using `#[derive(Event)]`.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::Event;
/// 
/// #[derive(Event, Clone, Debug)]
/// struct UserLoggedIn {
///     user_id: u64,
///     timestamp: std::time::SystemTime,
/// }
/// ```
pub trait Event: Clone + Send + Sync + 'static {
    /// Returns the name of the event type.
    /// 
    /// This is used for debugging, logging, and type identification.
    /// The default implementation uses the type name.
    fn event_type_name() -> &'static str
    where
        Self: Sized,
    {
        std::any::type_name::<Self>()
    }
    
    /// Returns metadata associated with this event.
    /// 
    /// Override this method to provide custom metadata such as
    /// timestamps, priorities, or source information.
    fn metadata(&self) -> EventMetadata {
        EventMetadata::default()
    }
    
    /// Returns the size of this event in bytes.
    /// 
    /// Used for memory management and performance optimization.
    /// The default implementation uses `std::mem::size_of`.
    fn size_hint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
    
    /// Validates the event data.
    /// 
    /// Override this method to implement custom validation logic.
    /// Events that fail validation may be rejected by the event bus.
    fn validate(&self) -> Result<(), EventValidationError> {
        Ok(())
    }
}
```

### Event Metadata

Events can include metadata for additional context:

```rust
/// Metadata associated with an event.
#[derive(Debug, Clone)]
pub struct EventMetadata {
    timestamp: Option<std::time::SystemTime>,
    priority: Priority,
    source: Option<String>,
    category: Option<String>,
    tags: Vec<String>,
    correlation_id: Option<String>,
    trace_id: Option<String>,
    custom_fields: std::collections::HashMap<String, String>,
}

impl EventMetadata {
    /// Creates new empty metadata.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Sets the timestamp for this event.
    pub fn with_timestamp(mut self, timestamp: std::time::SystemTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
    
    /// Sets the priority for this event.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Sets the source for this event.
    pub fn with_source<S: Into<String>>(mut self, source: S) -> Self {
        self.source = Some(source.into());
        self
    }
    
    /// Sets the category for this event.
    pub fn with_category<S: Into<String>>(mut self, category: S) -> Self {
        self.category = Some(category.into());
        self
    }
    
    /// Adds tags to this event.
    pub fn with_tags<I, S>(mut self, tags: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(|s| s.into()));
        self
    }
    
    /// Sets a correlation ID for this event.
    pub fn with_correlation_id<S: Into<String>>(mut self, id: S) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
    
    /// Sets a trace ID for this event.
    pub fn with_trace_id<S: Into<String>>(mut self, id: S) -> Self {
        self.trace_id = Some(id.into());
        self
    }
    
    /// Adds a custom field to this event.
    pub fn with_custom_field<K, V>(mut self, key: K, value: V) -> Self 
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.custom_fields.insert(key.into(), value.into());
        self
    }
    
    // Getters
    pub fn timestamp(&self) -> Option<std::time::SystemTime> { self.timestamp }
    pub fn priority(&self) -> Priority { self.priority }
    pub fn source(&self) -> Option<&str> { self.source.as_deref() }
    pub fn category(&self) -> Option<&str> { self.category.as_deref() }
    pub fn tags(&self) -> &[String] { &self.tags }
    pub fn correlation_id(&self) -> Option<&str> { self.correlation_id.as_deref() }
    pub fn trace_id(&self) -> Option<&str> { self.trace_id.as_deref() }
    pub fn custom_field(&self, key: &str) -> Option<&str> { 
        self.custom_fields.get(key).map(|s| s.as_str()) 
    }
}
```

## Handler Trait

The `Handler` trait defines how event handlers are implemented.

```rust
/// Trait for event handlers.
/// 
/// Handlers process events when they are emitted to the event bus.
/// This trait is implemented automatically for closures and function pointers.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{Handler, Event};
/// 
/// #[derive(Event, Clone)]
/// struct TestEvent { value: i32 }
/// 
/// // Closure handler
/// let handler = |event: TestEvent| {
///     println!("Received: {}", event.value);
/// };
/// 
/// // Function pointer handler
/// fn handle_test_event(event: TestEvent) {
///     println!("Handled: {}", event.value);
/// }
/// ```
pub trait Handler<E: Event>: Send + Sync + 'static {
    /// The result type returned by this handler.
    type Output;
    
    /// Handles the given event.
    /// 
    /// This method is called when an event of type `E` is emitted
    /// and this handler is registered for that event type.
    fn handle(&self, event: E) -> Self::Output;
    
    /// Returns the name of this handler for debugging.
    fn handler_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Returns whether this handler can run concurrently with other handlers.
    fn is_concurrent_safe(&self) -> bool {
        true
    }
    
    /// Returns the estimated execution time for this handler.
    /// 
    /// Used for performance optimization and scheduling.
    fn estimated_execution_time(&self) -> Option<std::time::Duration> {
        None
    }
}
```

### Sync Handler Implementation

```rust
/// Synchronous handler that returns nothing.
impl<F, E> Handler<E> for F
where
    F: Fn(E) + Send + Sync + 'static,
    E: Event,
{
    type Output = ();
    
    fn handle(&self, event: E) -> Self::Output {
        self(event)
    }
}

/// Fallible synchronous handler that can return errors.
pub trait FallibleHandler<E: Event>: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;
    
    fn handle(&self, event: E) -> Result<(), Self::Error>;
}

impl<F, E, Err> FallibleHandler<E> for F
where
    F: Fn(E) -> Result<(), Err> + Send + Sync + 'static,
    E: Event,
    Err: std::error::Error + Send + Sync + 'static,
{
    type Error = Err;
    
    fn handle(&self, event: E) -> Result<(), Self::Error> {
        self(event)
    }
}
```

### Async Handler Implementation

```rust
/// Asynchronous handler trait.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{AsyncHandler, Event};
/// 
/// #[derive(Event, Clone)]
/// struct AsyncEvent { data: Vec<u8> }
/// 
/// let handler = |event: AsyncEvent| async move {
///     // Async processing
///     process_data(event.data).await;
/// };
/// ```
pub trait AsyncHandler<E: Event>: Send + Sync + 'static {
    type Output: Send + 'static;
    type Future: std::future::Future<Output = Self::Output> + Send + 'static;
    
    /// Handles the event asynchronously.
    fn handle(&self, event: E) -> Self::Future;
    
    /// Returns the name of this handler for debugging.
    fn handler_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Returns whether this handler can run concurrently with other handlers.
    fn is_concurrent_safe(&self) -> bool {
        true
    }
}

/// Implementation for async closures.
impl<F, E, Fut> AsyncHandler<E> for F
where
    F: Fn(E) -> Fut + Send + Sync + 'static,
    E: Event,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    type Output = ();
    type Future = Fut;
    
    fn handle(&self, event: E) -> Self::Future {
        self(event)
    }
}
```

## EventBusImpl Trait

The `EventBusImpl` trait defines the interface for event bus implementations.

```rust
/// Core trait for event bus implementations.
/// 
/// This trait defines the basic operations that all event buses must support.
/// Different implementations can provide different performance characteristics,
/// threading models, or special features.
pub trait EventBusImpl: Send + Sync {
    /// Emits an event to all registered handlers.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to emit
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if the event was successfully processed,
    /// or an error if processing failed.
    fn emit<E: Event>(&self, event: E) -> Result<(), EventBusError>;
    
    /// Registers a handler for events of type `E`.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The handler to register
    /// 
    /// # Returns
    /// 
    /// Returns a `HandlerId` that can be used to unregister the handler.
    fn on<E: Event, H: Handler<E>>(&self, handler: H) -> HandlerId;
    
    /// Unregisters a previously registered handler.
    /// 
    /// # Arguments
    /// 
    /// * `handler_id` - The ID of the handler to unregister
    /// 
    /// # Returns
    /// 
    /// Returns `true` if the handler was found and removed, `false` otherwise.
    fn off(&self, handler_id: HandlerId) -> bool;
    
    /// Returns the number of registered handlers for a specific event type.
    fn handler_count<E: Event>(&self) -> usize;
    
    /// Returns the total number of registered handlers.
    fn total_handler_count(&self) -> usize;
    
    /// Clears all registered handlers.
    fn clear(&self);
    
    /// Returns whether the event bus is currently processing events.
    fn is_processing(&self) -> bool;
    
    /// Shuts down the event bus gracefully.
    /// 
    /// This method should wait for all pending events to be processed
    /// before returning.
    fn shutdown(&self) -> Result<(), EventBusError>;
}
```

## Filter Trait

The `Filter` trait defines how event filtering works.

```rust
/// Trait for event filters.
/// 
/// Filters determine whether an event should be processed by handlers.
/// They can be combined using logical operations (AND, OR, NOT).
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{Filter, Event};
/// 
/// #[derive(Event, Clone)]
/// struct UserEvent { user_id: u64 }
/// 
/// let filter = Filter::field(|event: &UserEvent| event.user_id > 1000);
/// ```
pub trait Filter<E: Event>: Send + Sync + 'static {
    /// Evaluates whether the given event should pass through this filter.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to evaluate
    /// 
    /// # Returns
    /// 
    /// Returns `true` if the event should be processed, `false` otherwise.
    fn evaluate(&self, event: &E) -> bool;
    
    /// Returns the name of this filter for debugging.
    fn filter_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Returns whether this filter is expensive to evaluate.
    /// 
    /// Used for optimization - expensive filters may be cached
    /// or evaluated last in a filter chain.
    fn is_expensive(&self) -> bool {
        false
    }
    
    /// Combines this filter with another using logical AND.
    fn and<F: Filter<E>>(self, other: F) -> AndFilter<Self, F>
    where
        Self: Sized,
    {
        AndFilter::new(self, other)
    }
    
    /// Combines this filter with another using logical OR.
    fn or<F: Filter<E>>(self, other: F) -> OrFilter<Self, F>
    where
        Self: Sized,
    {
        OrFilter::new(self, other)
    }
    
    /// Negates this filter.
    fn not(self) -> NotFilter<Self>
    where
        Self: Sized,
    {
        NotFilter::new(self)
    }
}
```

## Middleware Trait

The `Middleware` trait defines how middleware components work.

```rust
/// Trait for middleware components.
/// 
/// Middleware can intercept events before and after they are processed
/// by handlers, allowing for cross-cutting concerns like logging,
/// authentication, and metrics collection.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{Middleware, MiddlewareContext, Event};
/// 
/// struct LoggingMiddleware;
/// 
/// impl<E: Event> Middleware<E> for LoggingMiddleware {
///     fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
///         println!("Processing event: {:?}", event);
///         context.next(event)
///     }
/// }
/// ```
pub trait Middleware<E: Event>: Send + Sync + 'static {
    /// Handles the event and middleware chain.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event being processed
    /// * `context` - The middleware context for continuing the chain
    /// 
    /// # Returns
    /// 
    /// Returns the result of processing the event through the middleware chain.
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult;
    
    /// Returns the name of this middleware for debugging.
    fn middleware_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Returns the priority of this middleware.
    /// 
    /// Higher priority middleware runs earlier in the chain.
    fn priority(&self) -> MiddlewarePriority {
        MiddlewarePriority::Normal
    }
    
    /// Returns whether this middleware should be skipped for certain events.
    fn should_skip(&self, event: &E) -> bool {
        false
    }
}

/// Context passed to middleware for continuing the chain.
pub struct MiddlewareContext<E: Event> {
    // Internal implementation details
}

impl<E: Event> MiddlewareContext<E> {
    /// Continues processing the event through the next middleware or handlers.
    pub fn next(&mut self, event: &E) -> MiddlewareResult {
        // Implementation details
    }
    
    /// Skips the remaining middleware and goes directly to handlers.
    pub fn skip_to_handlers(&mut self, event: &E) -> MiddlewareResult {
        // Implementation details
    }
    
    /// Stops processing the event entirely.
    pub fn stop(&mut self) -> MiddlewareResult {
        // Implementation details
    }
    
    /// Returns the number of handlers that will process this event.
    pub fn handler_count(&self) -> usize {
        // Implementation details
    }
    
    /// Returns metadata about the current processing context.
    pub fn context_metadata(&self) -> &ContextMetadata {
        // Implementation details
    }
}
```

## Priority Types

```rust
/// Priority levels for events and handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Critical priority - highest precedence
    Critical,
    /// High priority
    High,
    /// Normal priority (default)
    Normal,
    /// Low priority
    Low,
    /// Custom priority with explicit value
    Custom(PriorityValue),
}

/// Custom priority value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PriorityValue(u16);

impl PriorityValue {
    /// Creates a new priority value.
    /// 
    /// Higher values indicate higher priority.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    
    /// Returns the numeric value of this priority.
    pub fn value(self) -> u16 {
        self.0
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}
```

## Error Types

```rust
/// Errors that can occur during event processing.
#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("Handler execution failed: {0}")]
    HandlerError(#[from] HandlerError),
    
    #[error("Event validation failed: {0}")]
    ValidationError(#[from] EventValidationError),
    
    #[error("Middleware error: {0}")]
    MiddlewareError(#[from] MiddlewareError),
    
    #[error("Bus is shutting down")]
    ShuttingDown,
    
    #[error("Bus is not initialized")]
    NotInitialized,
    
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },
    
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
}

/// Errors that can occur during handler execution.
#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("Handler panicked: {message}")]
    Panic { message: String },
    
    #[error("Handler timeout after {duration:?}")]
    Timeout { duration: std::time::Duration },
    
    #[error("Handler failed with custom error: {0}")]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

/// Errors that can occur during event validation.
#[derive(Debug, thiserror::Error)]
pub enum EventValidationError {
    #[error("Required field missing: {field}")]
    MissingField { field: String },
    
    #[error("Invalid field value: {field} = {value}")]
    InvalidValue { field: String, value: String },
    
    #[error("Custom validation error: {message}")]
    Custom { message: String },
}

/// Result type for middleware operations.
pub type MiddlewareResult = Result<(), MiddlewareError>;

/// Errors that can occur in middleware.
#[derive(Debug, thiserror::Error)]
pub enum MiddlewareError {
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Authorization failed: {reason}")]
    AuthorizationFailed { reason: String },
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
    
    #[error("Middleware chain interrupted")]
    ChainInterrupted,
    
    #[error("Custom middleware error: {message}")]
    Custom { message: String },
}
```

These core traits form the foundation of EventRS and provide the building blocks for all event processing functionality. Understanding these traits is essential for both using EventRS effectively and extending it with custom implementations.