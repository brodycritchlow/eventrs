# Event Bus API

This document covers the API for EventRS event buses, including synchronous, asynchronous, and specialized bus implementations.

## EventBus

The main synchronous event bus implementation.

```rust
/// Synchronous event bus for handling events in a single thread.
/// 
/// `EventBus` provides high-performance event processing with zero-cost
/// abstractions for simple use cases. All handlers execute synchronously
/// in the order they were registered (unless priorities are specified).
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{EventBus, Event};
/// 
/// #[derive(Event, Clone)]
/// struct UserLoggedIn { user_id: u64 }
/// 
/// let mut bus = EventBus::new();
/// bus.on::<UserLoggedIn>(|event| {
///     println!("User {} logged in", event.user_id);
/// });
/// 
/// bus.emit(UserLoggedIn { user_id: 123 });
/// ```
pub struct EventBus {
    // Internal implementation details
}

impl EventBus {
    /// Creates a new event bus with default configuration.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let mut bus = EventBus::new();
    /// ```
    pub fn new() -> Self {
        Self::builder().build()
    }
    
    /// Creates a new event bus builder for custom configuration.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let bus = EventBus::builder()
    ///     .with_capacity(1000)
    ///     .with_middleware(LoggingMiddleware)
    ///     .build();
    /// ```
    pub fn builder() -> EventBusBuilder {
        EventBusBuilder::new()
    }
    
    /// Registers a handler for events of type `E`.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The handler function or closure
    /// 
    /// # Returns
    /// 
    /// Returns a `HandlerId` that can be used to unregister the handler.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let handler_id = bus.on::<UserLoggedIn>(|event| {
    ///     println!("Handler called for user {}", event.user_id);
    /// });
    /// ```
    pub fn on<E, H>(&mut self, handler: H) -> HandlerId
    where
        E: Event,
        H: Handler<E>,
    {
        self.on_with_priority(Priority::Normal, handler)
    }
    
    /// Registers a handler with a specific priority.
    /// 
    /// Higher priority handlers execute before lower priority handlers.
    /// 
    /// # Arguments
    /// 
    /// * `priority` - The priority for this handler
    /// * `handler` - The handler function or closure
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.on_with_priority::<UserLoggedIn>(Priority::High, |event| {
    ///     // This handler runs before normal priority handlers
    ///     security_check(event.user_id);
    /// });
    /// ```
    pub fn on_with_priority<E, H>(&mut self, priority: Priority, handler: H) -> HandlerId
    where
        E: Event,
        H: Handler<E>,
    {
        // Implementation details
    }
    
    /// Registers a handler with a custom identifier.
    /// 
    /// # Arguments
    /// 
    /// * `id` - Custom identifier for the handler
    /// * `handler` - The handler function or closure
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.on_with_id::<UserLoggedIn>("security_handler", |event| {
    ///     audit_login(event.user_id);
    /// });
    /// ```
    pub fn on_with_id<E, H>(&mut self, id: &str, handler: H) -> HandlerId
    where
        E: Event,
        H: Handler<E>,
    {
        // Implementation details
    }
    
    /// Registers a filtered handler that only processes events matching the filter.
    /// 
    /// # Arguments
    /// 
    /// * `filter` - The filter to apply
    /// * `handler` - The handler function or closure
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.on_filtered::<UserLoggedIn>(
    ///     Filter::field(|event| event.user_id > 1000),
    ///     |event| println!("Premium user logged in: {}", event.user_id)
    /// );
    /// ```
    pub fn on_filtered<E, F, H>(&mut self, filter: F, handler: H) -> HandlerId
    where
        E: Event,
        F: Filter<E>,
        H: Handler<E>,
    {
        // Implementation details
    }
    
    /// Registers a fallible handler that can return errors.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The fallible handler function or closure
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.on_fallible::<UserLoggedIn>(|event| -> Result<(), ProcessingError> {
    ///     validate_user(event.user_id)?;
    ///     update_login_count(event.user_id)?;
    ///     Ok(())
    /// });
    /// ```
    pub fn on_fallible<E, H>(&mut self, handler: H) -> HandlerId
    where
        E: Event,
        H: FallibleHandler<E>,
    {
        // Implementation details
    }
    
    /// Unregisters a handler by its ID.
    /// 
    /// # Arguments
    /// 
    /// * `handler_id` - The ID of the handler to remove
    /// 
    /// # Returns
    /// 
    /// Returns `true` if the handler was found and removed, `false` otherwise.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let handler_id = bus.on::<UserLoggedIn>(|_| {});
    /// let removed = bus.off(handler_id);
    /// assert!(removed);
    /// ```
    pub fn off(&mut self, handler_id: HandlerId) -> bool {
        // Implementation details
    }
    
    /// Emits an event to all registered handlers.
    /// 
    /// Handlers execute synchronously in priority order.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to emit
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.emit(UserLoggedIn { user_id: 123 });
    /// ```
    pub fn emit<E: Event>(&mut self, event: E) {
        self.try_emit(event).unwrap()
    }
    
    /// Emits an event and returns the result.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to emit
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if all handlers executed successfully,
    /// or an error if any handler failed.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// match bus.try_emit(UserLoggedIn { user_id: 123 }) {
    ///     Ok(()) => println!("Event processed successfully"),
    ///     Err(e) => eprintln!("Event processing failed: {}", e),
    /// }
    /// ```
    pub fn try_emit<E: Event>(&mut self, event: E) -> Result<(), EventBusError> {
        // Implementation details
    }
    
    /// Emits an event with detailed result information.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to emit
    /// 
    /// # Returns
    /// 
    /// Returns detailed information about the emission process.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let result = bus.emit_with_result(UserLoggedIn { user_id: 123 });
    /// println!("Handlers called: {}", result.handlers_called);
    /// println!("Execution time: {:?}", result.execution_time);
    /// ```
    pub fn emit_with_result<E: Event>(&mut self, event: E) -> EmissionResult {
        // Implementation details
    }
    
    /// Emits multiple events in a batch.
    /// 
    /// This is more efficient than calling `emit()` multiple times
    /// as it can optimize handler execution and memory allocation.
    /// 
    /// # Arguments
    /// 
    /// * `events` - Iterator of events to emit
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let events = vec![
    ///     UserLoggedIn { user_id: 1 },
    ///     UserLoggedIn { user_id: 2 },
    ///     UserLoggedIn { user_id: 3 },
    /// ];
    /// bus.emit_batch(events);
    /// ```
    pub fn emit_batch<E, I>(&mut self, events: I)
    where
        E: Event,
        I: IntoIterator<Item = E>,
    {
        // Implementation details
    }
    
    /// Emits an event conditionally based on a predicate.
    /// 
    /// # Arguments
    /// 
    /// * `condition` - Function that determines whether to emit
    /// * `event` - The event to emit if condition is true
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.emit_if(
    ///     || is_business_hours(),
    ///     BusinessHoursEvent { timestamp: SystemTime::now() }
    /// );
    /// ```
    pub fn emit_if<E, F>(&mut self, condition: F, event: E)
    where
        E: Event,
        F: FnOnce() -> bool,
    {
        if condition() {
            self.emit(event);
        }
    }
    
    /// Returns the number of handlers registered for a specific event type.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let count = bus.handler_count::<UserLoggedIn>();
    /// println!("UserLoggedIn has {} handlers", count);
    /// ```
    pub fn handler_count<E: Event>(&self) -> usize {
        // Implementation details
    }
    
    /// Returns the total number of registered handlers across all event types.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let total = bus.total_handler_count();
    /// println!("Total handlers: {}", total);
    /// ```
    pub fn total_handler_count(&self) -> usize {
        // Implementation details
    }
    
    /// Clears all registered handlers.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.clear();
    /// assert_eq!(bus.total_handler_count(), 0);
    /// ```
    pub fn clear(&mut self) {
        // Implementation details
    }
    
    /// Returns metrics about the event bus performance.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let metrics = bus.metrics();
    /// println!("Events processed: {}", metrics.events_processed);
    /// println!("Average latency: {:?}", metrics.average_latency);
    /// ```
    pub fn metrics(&self) -> EventBusMetrics {
        // Implementation details
    }
}
```

## AsyncEventBus

Asynchronous event bus for async/await support.

```rust
/// Asynchronous event bus for handling events with async handlers.
/// 
/// `AsyncEventBus` provides first-class support for async/await patterns,
/// allowing handlers to perform I/O operations without blocking.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{AsyncEventBus, Event};
/// 
/// #[derive(Event, Clone)]
/// struct DataReceived { data: Vec<u8> }
/// 
/// #[tokio::main]
/// async fn main() {
///     let mut bus = AsyncEventBus::new();
///     
///     bus.on::<DataReceived>(|event| async move {
///         save_to_database(event.data).await;
///     }).await;
///     
///     bus.emit(DataReceived { data: vec![1, 2, 3] }).await;
/// }
/// ```
pub struct AsyncEventBus {
    // Internal implementation details
}

impl AsyncEventBus {
    /// Creates a new async event bus with default configuration.
    pub fn new() -> Self {
        Self::builder().build()
    }
    
    /// Creates a new async event bus builder for custom configuration.
    pub fn builder() -> AsyncEventBusBuilder {
        AsyncEventBusBuilder::new()
    }
    
    /// Registers an async handler for events of type `E`.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The async handler function or closure
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.on::<DataReceived>(|event| async move {
    ///     process_data(event.data).await;
    /// }).await;
    /// ```
    pub async fn on<E, H, Fut>(&mut self, handler: H) -> HandlerId
    where
        E: Event,
        H: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.on_with_priority(Priority::Normal, handler).await
    }
    
    /// Registers an async handler with a specific priority.
    pub async fn on_with_priority<E, H, Fut>(&mut self, priority: Priority, handler: H) -> HandlerId
    where
        E: Event,
        H: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        // Implementation details
    }
    
    /// Registers a fallible async handler.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.on_fallible::<DataReceived>(|event| async move {
    ///     validate_data(&event.data).await?;
    ///     save_data(event.data).await?;
    ///     Ok(())
    /// }).await;
    /// ```
    pub async fn on_fallible<E, H, Fut, Err>(&mut self, handler: H) -> HandlerId
    where
        E: Event,
        H: Fn(E) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), Err>> + Send + 'static,
        Err: std::error::Error + Send + Sync + 'static,
    {
        // Implementation details
    }
    
    /// Emits an event to all registered async handlers.
    /// 
    /// All handlers execute concurrently and this method waits
    /// for all of them to complete.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to emit
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.emit(DataReceived { data: vec![1, 2, 3] }).await;
    /// ```
    pub async fn emit<E: Event>(&mut self, event: E) {
        self.try_emit(event).await.unwrap()
    }
    
    /// Emits an event and returns the result.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to emit
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if all handlers executed successfully.
    pub async fn try_emit<E: Event>(&mut self, event: E) -> Result<(), EventBusError> {
        // Implementation details
    }
    
    /// Emits an event without waiting for handlers to complete.
    /// 
    /// This is a "fire and forget" operation that starts handler
    /// execution but doesn't wait for completion.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to emit
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// bus.emit_and_forget(LogEvent { message: "Info".to_string() });
    /// // Continues immediately without waiting
    /// ```
    pub fn emit_and_forget<E: Event>(&mut self, event: E) {
        // Implementation details
    }
    
    /// Emits multiple events concurrently.
    /// 
    /// All events are processed concurrently, which can provide
    /// significant performance benefits for I/O heavy handlers.
    /// 
    /// # Arguments
    /// 
    /// * `events` - Iterator of events to emit
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let events = vec![
    ///     DataReceived { data: vec![1] },
    ///     DataReceived { data: vec![2] },
    ///     DataReceived { data: vec![3] },
    /// ];
    /// bus.emit_batch_concurrent(events).await;
    /// ```
    pub async fn emit_batch_concurrent<E, I>(&mut self, events: I)
    where
        E: Event,
        I: IntoIterator<Item = E>,
    {
        // Implementation details
    }
    
    /// Emits multiple events sequentially.
    /// 
    /// Events are processed one after another, which ensures
    /// ordering but may be slower than concurrent processing.
    /// 
    /// # Arguments
    /// 
    /// * `events` - Iterator of events to emit
    pub async fn emit_batch_sequential<E, I>(&mut self, events: I)
    where
        E: Event,
        I: IntoIterator<Item = E>,
    {
        for event in events {
            self.emit(event).await;
        }
    }
    
    /// Processes events from a stream.
    /// 
    /// # Arguments
    /// 
    /// * `stream` - Stream of events to process
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use futures::StreamExt;
    /// 
    /// let stream = get_event_stream();
    /// bus.emit_stream(stream).await;
    /// ```
    pub async fn emit_stream<E, S>(&mut self, mut stream: S)
    where
        E: Event,
        S: futures::Stream<Item = E> + Unpin,
    {
        while let Some(event) = stream.next().await {
            self.emit(event).await;
        }
    }
    
    /// Shuts down the async event bus gracefully.
    /// 
    /// Waits for all pending async operations to complete
    /// before shutting down.
    pub async fn shutdown(self) -> Result<(), EventBusError> {
        // Implementation details
    }
}
```

## ThreadSafeEventBus

Thread-safe event bus for multi-threaded scenarios.

```rust
/// Thread-safe event bus that can be shared across multiple threads.
/// 
/// Uses internal synchronization to ensure safe concurrent access
/// while maintaining good performance characteristics.
/// 
/// # Examples
/// 
/// ```rust
/// use std::sync::Arc;
/// use std::thread;
/// 
/// let bus = Arc::new(ThreadSafeEventBus::new());
/// 
/// // Share across threads
/// let bus_clone = bus.clone();
/// thread::spawn(move || {
///     bus_clone.emit(UserLoggedIn { user_id: 123 });
/// });
/// ```
pub struct ThreadSafeEventBus {
    // Internal implementation details
}

impl ThreadSafeEventBus {
    /// Creates a new thread-safe event bus.
    pub fn new() -> Self {
        // Implementation details
    }
    
    /// Registers a thread-safe handler.
    /// 
    /// The handler must be `Send + Sync` to be used across threads.
    pub fn on<E, H>(&self, handler: H) -> HandlerId
    where
        E: Event,
        H: Handler<E> + Send + Sync + 'static,
    {
        // Implementation details
    }
    
    /// Emits an event in a thread-safe manner.
    pub fn emit<E: Event>(&self, event: E) {
        // Implementation details
    }
    
    /// Creates a channel-based sender for this bus.
    /// 
    /// Returns a sender that can be used to send events
    /// to this bus from other threads.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let sender = bus.sender();
    /// thread::spawn(move || {
    ///     sender.send(UserLoggedIn { user_id: 123 });
    /// });
    /// ```
    pub fn sender<E: Event>(&self) -> EventSender<E> {
        // Implementation details
    }
}
```

## Event Bus Builders

Builders for configuring event buses.

```rust
/// Builder for configuring EventBus instances.
pub struct EventBusBuilder {
    // Internal configuration
}

impl EventBusBuilder {
    /// Creates a new event bus builder.
    pub fn new() -> Self {
        // Implementation details
    }
    
    /// Sets the initial capacity for handlers.
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        // Implementation details
        self
    }
    
    /// Adds middleware to the event bus.
    pub fn with_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<dyn Event> + 'static,
    {
        // Implementation details
        self
    }
    
    /// Sets the error handling strategy.
    pub fn with_error_handling(mut self, strategy: ErrorHandling) -> Self {
        // Implementation details
        self
    }
    
    /// Enables or disables metrics collection.
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        // Implementation details
        self
    }
    
    /// Sets a custom executor for handler execution.
    pub fn with_executor<E>(mut self, executor: E) -> Self
    where
        E: Executor + 'static,
    {
        // Implementation details
        self
    }
    
    /// Builds the configured event bus.
    pub fn build(self) -> EventBus {
        // Implementation details
    }
}

/// Builder for configuring AsyncEventBus instances.
pub struct AsyncEventBusBuilder {
    // Internal configuration
}

impl AsyncEventBusBuilder {
    /// Creates a new async event bus builder.
    pub fn new() -> Self {
        // Implementation details
    }
    
    /// Sets the maximum number of concurrent handlers.
    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        // Implementation details
        self
    }
    
    /// Sets the async runtime to use.
    pub fn with_runtime(mut self, runtime: tokio::runtime::Handle) -> Self {
        // Implementation details
        self
    }
    
    /// Enables backpressure handling.
    pub fn with_backpressure(mut self, enabled: bool) -> Self {
        // Implementation details
        self
    }
    
    /// Builds the configured async event bus.
    pub fn build(self) -> AsyncEventBus {
        // Implementation details
    }
}
```

## Supporting Types

```rust
/// Unique identifier for registered handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerId(u64);

/// Result of emitting an event.
#[derive(Debug)]
pub struct EmissionResult {
    /// Number of handlers that processed the event
    pub handlers_called: usize,
    /// Total execution time
    pub execution_time: std::time::Duration,
    /// Whether all handlers succeeded
    pub success: bool,
    /// Errors from failed handlers
    pub errors: Vec<HandlerError>,
}

/// Metrics collected by the event bus.
#[derive(Debug)]
pub struct EventBusMetrics {
    /// Total number of events processed
    pub events_processed: u64,
    /// Total number of handlers executed
    pub handlers_executed: u64,
    /// Average event processing latency
    pub average_latency: std::time::Duration,
    /// Peak memory usage
    pub peak_memory_usage: usize,
    /// Number of failed handler executions
    pub failed_handlers: u64,
}

/// Error handling strategies.
#[derive(Debug, Clone, Copy)]
pub enum ErrorHandling {
    /// Stop processing on first error
    StopOnFirstError,
    /// Continue processing despite errors
    ContinueOnError,
    /// Retry failed handlers up to N times
    RetryOnError(u32),
}

/// Channel sender for thread-safe event emission.
pub struct EventSender<E: Event> {
    // Internal implementation
}

impl<E: Event> EventSender<E> {
    /// Sends an event to the associated event bus.
    pub fn send(&self, event: E) -> Result<(), SendError<E>> {
        // Implementation details
    }
    
    /// Tries to send an event without blocking.
    pub fn try_send(&self, event: E) -> Result<(), TrySendError<E>> {
        // Implementation details
    }
}

/// Error when sending events through a channel.
#[derive(Debug, thiserror::Error)]
pub enum SendError<E> {
    #[error("Channel is disconnected")]
    Disconnected(E),
    #[error("Channel is full")]
    Full(E),
}

/// Error when trying to send events through a channel.
#[derive(Debug, thiserror::Error)]
pub enum TrySendError<E> {
    #[error("Channel is disconnected")]
    Disconnected(E),
    #[error("Channel is full")]
    Full(E),
}
```

This API provides comprehensive event bus functionality with support for synchronous, asynchronous, and thread-safe event processing patterns. The builder pattern allows for flexible configuration while maintaining performance and type safety.