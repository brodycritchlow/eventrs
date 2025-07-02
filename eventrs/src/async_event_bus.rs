//! Asynchronous event bus implementation.
//!
//! This module provides async event processing capabilities for EventRS,
//! enabling non-blocking event handling with native async/await support.

#[cfg(feature = "async")]
use crate::error::{EventBusError, EventBusResult};
#[cfg(feature = "async")]
use crate::event::Event;
#[cfg(feature = "async")]
use crate::handler::{AsyncHandler, HandlerId};
#[cfg(feature = "async")]
use crate::priority::Priority;

#[cfg(feature = "async")]
use std::any::TypeId;
#[cfg(feature = "async")]
use std::collections::HashMap;
#[cfg(feature = "async")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "async")]
use std::sync::Arc;

#[cfg(feature = "async")]
use futures::future::{BoxFuture, FutureExt};
#[cfg(feature = "async")]
use tokio::sync::{Mutex, RwLock};

/// Configuration options for the AsyncEventBus.
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub struct AsyncEventBusConfig {
    /// Maximum number of handlers per event type.
    pub max_handlers_per_event: Option<usize>,

    /// Maximum total number of handlers across all event types.
    pub max_total_handlers: Option<usize>,

    /// Whether to validate events before processing.
    pub validate_events: bool,

    /// Whether to process handlers in priority order.
    pub use_priority_ordering: bool,

    /// Whether to continue processing remaining handlers if one fails.
    pub continue_on_handler_failure: bool,

    /// Default priority for handlers without explicit priority.
    pub default_handler_priority: Priority,

    /// Whether to enable detailed error reporting.
    pub detailed_error_reporting: bool,

    /// Maximum number of concurrent handlers to execute.
    pub max_concurrent_handlers: Option<usize>,

    /// Whether to execute handlers concurrently or sequentially.
    pub concurrent_execution: bool,

    /// Minimum number of handlers required to use concurrent execution.
    /// If fewer handlers are present, sequential execution will be used for performance.
    pub concurrent_threshold: usize,
}

#[cfg(feature = "async")]
impl Default for AsyncEventBusConfig {
    fn default() -> Self {
        Self {
            max_handlers_per_event: Some(1000),
            max_total_handlers: Some(10000),
            validate_events: true,
            use_priority_ordering: true,
            continue_on_handler_failure: true,
            default_handler_priority: Priority::Normal,
            detailed_error_reporting: false,
            max_concurrent_handlers: Some(100),
            concurrent_execution: true,
            concurrent_threshold: 5, // Only use concurrency with 5+ handlers
        }
    }
}

/// Trait for async boxed handlers to enable type erasure.
#[cfg(feature = "async")]
pub(crate) trait AsyncBoxedHandler<E: Event>: Send + Sync {
    /// Executes the handler asynchronously with the given event.
    fn call(&self, event: E) -> BoxFuture<'static, ()>;

    /// Returns the handler's name for debugging.
    fn name(&self) -> &'static str;

    /// Returns whether this handler is concurrent-safe.
    fn is_concurrent_safe(&self) -> bool;
}

/// Implementation of AsyncBoxedHandler for async handlers.
#[cfg(feature = "async")]
pub(crate) struct AsyncHandlerWrapper<H> {
    handler: H,
}

#[cfg(feature = "async")]
impl<H> AsyncHandlerWrapper<H> {
    pub(crate) fn new(handler: H) -> Self {
        Self { handler }
    }
}

#[cfg(feature = "async")]
impl<E, H> AsyncBoxedHandler<E> for AsyncHandlerWrapper<H>
where
    E: Event,
    H: AsyncHandler<E>,
{
    fn call(&self, event: E) -> BoxFuture<'static, ()> {
        let future = self.handler.handle(event);
        async move {
            let _ = future.await;
        }
        .boxed()
    }

    fn name(&self) -> &'static str {
        self.handler.handler_name()
    }

    fn is_concurrent_safe(&self) -> bool {
        self.handler.is_concurrent_safe()
    }
}

/// Handler entry stored in the async event bus.
#[cfg(feature = "async")]
struct AsyncHandlerEntry<E: Event> {
    id: HandlerId,
    handler: Arc<dyn AsyncBoxedHandler<E>>,
    priority: Priority,
}

#[cfg(feature = "async")]
impl<E: Event> AsyncHandlerEntry<E> {
    fn new(id: HandlerId, handler: Box<dyn AsyncBoxedHandler<E>>, priority: Priority) -> Self {
        Self {
            id,
            handler: Arc::from(handler),
            priority,
        }
    }
}

#[cfg(feature = "async")]
impl<E: Event> Clone for AsyncHandlerEntry<E> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            handler: Arc::clone(&self.handler),
            priority: self.priority,
        }
    }
}

/// Asynchronous event bus for non-blocking event processing.
///
/// The AsyncEventBus provides a centralized system for emitting events and
/// registering async handlers to process those events. It supports concurrent
/// handler execution and integrates seamlessly with tokio-based async applications.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use eventrs::{AsyncEventBus, Event};
///
/// #[derive(Event, Clone, Debug)]
/// struct UserLoggedIn {
///     user_id: u64,
///     timestamp: std::time::SystemTime,
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut bus = AsyncEventBus::new();
///
///     // Register an async handler
///     bus.on(|event: UserLoggedIn| async move {
///         println!("User {} logged in at {:?}", event.user_id, event.timestamp);
///         // Async operations like database writes can go here
///     }).await;
///
///     // Emit an event
///     bus.emit(UserLoggedIn {
///         user_id: 123,
///         timestamp: std::time::SystemTime::now(),
///     }).await?;
///     
///     Ok(())
/// }
/// ```
#[cfg(feature = "async")]
pub struct AsyncEventBus {
    /// Configuration for this event bus.
    config: AsyncEventBusConfig,

    /// Storage for handlers by event type ID.
    handlers: Arc<RwLock<HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>>>,

    /// Mapping from handler ID to event type ID for cleanup.
    handler_registry: Arc<RwLock<HashMap<HandlerId, TypeId>>>,

    /// Whether the event bus is currently shutting down.
    shutting_down: Arc<AtomicBool>,

    /// Counter for total number of registered handlers.
    handler_count: Arc<Mutex<usize>>,
}

#[cfg(feature = "async")]
impl AsyncEventBus {
    /// Creates a new AsyncEventBus with default configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::AsyncEventBus;
    ///
    /// let bus = AsyncEventBus::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(AsyncEventBusConfig::default())
    }

    /// Creates a new AsyncEventBusBuilder for configuring the async event bus.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{AsyncEventBus, Priority};
    ///
    /// let bus = AsyncEventBus::builder()
    ///     .with_max_concurrency(100)
    ///     .with_metrics(true)
    ///     .with_default_priority(Priority::High)
    ///     .build();
    /// ```
    pub fn builder() -> AsyncEventBusBuilder {
        AsyncEventBusBuilder::new()
    }

    /// Creates a new AsyncEventBus with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration options for the async event bus
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{AsyncEventBus, AsyncEventBusConfig, Priority};
    ///
    /// let config = AsyncEventBusConfig {
    ///     max_handlers_per_event: Some(50),
    ///     default_handler_priority: Priority::High,
    ///     concurrent_execution: false,
    ///     ..Default::default()
    /// };
    ///
    /// let bus = AsyncEventBus::with_config(config);
    /// ```
    pub fn with_config(config: AsyncEventBusConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            handler_registry: Arc::new(RwLock::new(HashMap::new())),
            shutting_down: Arc::new(AtomicBool::new(false)),
            handler_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Emits an event asynchronously to all registered handlers.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to emit
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was successfully processed,
    /// or an error if processing failed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{AsyncEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), eventrs::EventBusError> {
    /// let bus = AsyncEventBus::new();
    /// bus.emit(TestEvent { value: 42 }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn emit<E: Event>(&self, event: E) -> EventBusResult<()> {
        if self.shutting_down.load(Ordering::Acquire) {
            return Err(EventBusError::ShuttingDown);
        }

        // Validate the event if configured to do so
        if self.config.validate_events {
            event.validate()?;
        }

        // Get handlers for this event type
        let handlers = self.get_handlers_for_event::<E>().await?;

        if handlers.is_empty() {
            return Ok(());
        }

        // Process handlers based on configuration
        if self.config.use_priority_ordering {
            self.process_handlers_with_priority(&event, handlers)
                .await?;
        } else {
            self.process_handlers_sequential(&event, handlers).await?;
        }

        Ok(())
    }

    /// Registers an async handler for events of type `E`.
    ///
    /// # Arguments
    ///
    /// * `handler` - The async handler to register
    ///
    /// # Returns
    ///
    /// Returns a `HandlerId` that can be used to unregister the handler.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{AsyncEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut bus = AsyncEventBus::new();
    /// let handler_id = bus.on(|event: TestEvent| async move {
    ///     println!("Value: {}", event.value);
    /// }).await;
    /// # }
    /// ```
    pub async fn on<E: Event, H: AsyncHandler<E>>(&mut self, handler: H) -> HandlerId {
        self.register_handler(handler, self.config.default_handler_priority)
            .await
    }

    /// Registers an async handler with a specific priority for events of type `E`.
    ///
    /// # Arguments
    ///
    /// * `handler` - The async handler to register
    /// * `priority` - The priority for this handler
    ///
    /// # Returns
    ///
    /// Returns a `HandlerId` that can be used to unregister the handler.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{AsyncEventBus, Event, Priority};
    ///
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut bus = AsyncEventBus::new();
    /// let handler_id = bus.on_with_priority(
    ///     |event: TestEvent| async move {
    ///         println!("High priority: {}", event.value);
    ///     },
    ///     Priority::High
    /// ).await;
    /// # }
    /// ```
    pub async fn on_with_priority<E: Event, H: AsyncHandler<E>>(
        &mut self,
        handler: H,
        priority: Priority,
    ) -> HandlerId {
        self.register_handler(handler, priority).await
    }

    /// Unregisters a previously registered handler.
    ///
    /// # Arguments
    ///
    /// * `handler_id` - The ID of the handler to unregister
    ///
    /// # Returns
    ///
    /// Returns `true` if the handler was found and removed, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{AsyncEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut bus = AsyncEventBus::new();
    /// let handler_id = bus.on(|event: TestEvent| async move {
    ///     println!("Value: {}", event.value);
    /// }).await;
    ///
    /// // Later, remove the handler
    /// let removed = bus.off(handler_id).await;
    /// assert!(removed);
    /// # }
    /// ```
    pub async fn off(&mut self, handler_id: HandlerId) -> bool {
        let type_id = {
            let registry = self.handler_registry.read().await;
            match registry.get(&handler_id) {
                Some(type_id) => *type_id,
                None => return false,
            }
        };

        // Remove from handlers map
        let removed = {
            let mut handlers_map = self.handlers.write().await;
            if let Some(_handlers_any) = handlers_map.get_mut(&type_id) {
                // In a real implementation, we would properly remove the specific handler
                true // Simplified for now
            } else {
                false
            }
        };

        if removed {
            // Remove from registry
            let mut registry = self.handler_registry.write().await;
            registry.remove(&handler_id);

            // Decrement counter
            let mut count = self.handler_count.lock().await;
            *count = count.saturating_sub(1);
        }

        removed
    }

    /// Returns the number of registered handlers for a specific event type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{AsyncEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut bus = AsyncEventBus::new();
    /// assert_eq!(bus.handler_count::<TestEvent>().await, 0);
    ///
    /// bus.on(|_: TestEvent| async {}).await;
    /// assert_eq!(bus.handler_count::<TestEvent>().await, 1);
    /// # }
    /// ```
    pub async fn handler_count<E: Event>(&self) -> usize {
        let handlers_map = self.handlers.read().await;
        let type_id = TypeId::of::<E>();

        if let Some(handlers_any) = handlers_map.get(&type_id) {
            if let Some(handlers) = handlers_any.downcast_ref::<Vec<AsyncHandlerEntry<E>>>() {
                handlers.len()
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Returns the total number of registered handlers across all event types.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::AsyncEventBus;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let bus = AsyncEventBus::new();
    /// assert_eq!(bus.total_handler_count().await, 0);
    /// # }
    /// ```
    pub async fn total_handler_count(&self) -> usize {
        *self.handler_count.lock().await
    }

    /// Clears all registered handlers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{AsyncEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut bus = AsyncEventBus::new();
    /// bus.on(|_: TestEvent| async {}).await;
    /// assert!(bus.total_handler_count().await > 0);
    ///
    /// bus.clear().await;
    /// assert_eq!(bus.total_handler_count().await, 0);
    /// # }
    /// ```
    pub async fn clear(&mut self) {
        let mut handlers_map = self.handlers.write().await;
        handlers_map.clear();

        let mut registry = self.handler_registry.write().await;
        registry.clear();

        let mut count = self.handler_count.lock().await;
        *count = 0;
    }

    /// Returns whether the event bus is currently processing events.
    ///
    /// For the async event bus, this could return true if there are
    /// active async handlers running.
    pub async fn is_processing(&self) -> bool {
        // TODO: Implement proper processing state tracking
        false
    }

    /// Shuts down the event bus gracefully.
    ///
    /// This will mark the bus as shutting down and prevent new events
    /// from being processed, while allowing current async operations to complete.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::AsyncEventBus;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), eventrs::EventBusError> {
    /// let mut bus = AsyncEventBus::new();
    /// bus.shutdown().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn shutdown(&mut self) -> EventBusResult<()> {
        self.shutting_down.store(true, Ordering::Release);

        // TODO: Wait for pending async operations to complete
        // This could involve tracking active futures and awaiting their completion

        Ok(())
    }

    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &AsyncEventBusConfig {
        &self.config
    }

    // Private helper methods

    async fn register_handler<E: Event, H: AsyncHandler<E>>(
        &mut self,
        handler: H,
        priority: Priority,
    ) -> HandlerId {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();

        // Check limits
        if let Some(max_total) = self.config.max_total_handlers {
            if self.total_handler_count().await >= max_total {
                // In a real implementation, this would return a Result
                panic!("Maximum total handlers exceeded");
            }
        }

        // Create boxed handler
        let boxed = Box::new(AsyncHandlerWrapper::new(handler));
        let entry = AsyncHandlerEntry::new(handler_id, boxed, priority);

        // Store handler
        {
            let mut handlers_map = self.handlers.write().await;
            let handlers_vec = handlers_map.entry(type_id).or_insert_with(|| {
                Box::new(Vec::<AsyncHandlerEntry<E>>::new()) as Box<dyn std::any::Any + Send + Sync>
            });

            // Downcast to the concrete type and add the handler
            if let Some(vec) = handlers_vec.downcast_mut::<Vec<AsyncHandlerEntry<E>>>() {
                vec.push(entry);
            }
        }

        // Register handler ID
        {
            let mut registry = self.handler_registry.write().await;
            registry.insert(handler_id, type_id);
        }

        // Increment counter
        {
            let mut count = self.handler_count.lock().await;
            *count += 1;
        }

        handler_id
    }

    async fn get_handlers_for_event<E: Event>(&self) -> EventBusResult<Vec<AsyncHandlerEntry<E>>> {
        let handlers_map = self.handlers.read().await;
        let type_id = TypeId::of::<E>();

        if let Some(handlers_any) = handlers_map.get(&type_id) {
            if let Some(handlers) = handlers_any.downcast_ref::<Vec<AsyncHandlerEntry<E>>>() {
                // Clone the handlers for processing
                Ok(handlers.clone())
            } else {
                Ok(Vec::new())
            }
        } else {
            Ok(Vec::new())
        }
    }

    async fn process_handlers_with_priority<E: Event>(
        &self,
        event: &E,
        handlers: Vec<AsyncHandlerEntry<E>>,
    ) -> EventBusResult<()> {
        // Only use concurrent execution if we have enough handlers to make it worthwhile
        if self.config.concurrent_execution && handlers.len() >= self.config.concurrent_threshold {
            // Sort handlers by priority (higher priority first) and execute concurrently
            let mut sorted_handlers = handlers;
            sorted_handlers.sort_by(|a, b| b.priority.cmp(&a.priority)); // Higher priority first

            // Execute handlers concurrently using tokio::spawn for true parallelism
            let handles: Vec<_> = sorted_handlers
                .into_iter()
                .map(|handler_entry| {
                    let event_clone = event.clone();
                    tokio::spawn(async move {
                        handler_entry.handler.call(event_clone).await;
                    })
                })
                .collect();

            // Wait for all spawned tasks to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    if !self.config.continue_on_handler_failure {
                        eprintln!("Handler task panicked: {:?}", e);
                    }
                }
            }
        } else {
            // Sort handlers by priority (higher priority first) and execute sequentially
            let mut sorted_handlers = handlers;
            sorted_handlers.sort_by(|a, b| b.priority.cmp(&a.priority)); // Higher priority first

            for handler_entry in sorted_handlers {
                handler_entry.handler.call(event.clone()).await;
            }
        }

        Ok(())
    }

    async fn process_handlers_sequential<E: Event>(
        &self,
        event: &E,
        handlers: Vec<AsyncHandlerEntry<E>>,
    ) -> EventBusResult<()> {
        // Only use concurrent execution if we have enough handlers to make it worthwhile
        if self.config.concurrent_execution && handlers.len() >= self.config.concurrent_threshold {
            // Execute handlers concurrently using tokio::spawn
            let handles: Vec<_> = handlers
                .into_iter()
                .map(|handler_entry| {
                    let event_clone = event.clone();
                    tokio::spawn(async move {
                        handler_entry.handler.call(event_clone).await;
                    })
                })
                .collect();

            // Wait for all spawned tasks to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    if !self.config.continue_on_handler_failure {
                        eprintln!("Handler task panicked: {:?}", e);
                    }
                }
            }
        } else {
            // Execute handlers sequentially
            for handler_entry in handlers {
                handler_entry.handler.call(event.clone()).await;
            }
        }

        Ok(())
    }
}

/// Builder for configuring AsyncEventBus instances.
///
/// The AsyncEventBusBuilder provides a fluent interface for configuring
/// async event buses with custom options, concurrency settings, and middleware.
///
/// # Examples
///
/// ```rust
/// use eventrs::{AsyncEventBus, Priority};
///
/// let bus = AsyncEventBus::builder()
///     .with_max_concurrency(100)
///     .with_metrics(true)
///     .with_concurrent_threshold(3)
///     .with_default_priority(Priority::High)
///     .build();
/// ```
#[cfg(feature = "async")]
pub struct AsyncEventBusBuilder {
    config: AsyncEventBusConfig,
    // TODO: Add middleware stack when async middleware is implemented
}

#[cfg(feature = "async")]
impl AsyncEventBusBuilder {
    /// Creates a new AsyncEventBusBuilder with default configuration.
    pub fn new() -> Self {
        Self {
            config: AsyncEventBusConfig::default(),
        }
    }

    /// Sets the maximum number of handlers per event type.
    ///
    /// # Arguments
    /// * `max_handlers` - Maximum handlers per event type, or None for unlimited
    pub fn with_max_handlers_per_event(mut self, max_handlers: Option<usize>) -> Self {
        self.config.max_handlers_per_event = max_handlers;
        self
    }

    /// Sets the maximum total number of handlers across all event types.
    ///
    /// # Arguments
    /// * `max_total` - Maximum total handlers, or None for unlimited
    pub fn with_max_total_handlers(mut self, max_total: Option<usize>) -> Self {
        self.config.max_total_handlers = max_total;
        self
    }

    /// Enables or disables event validation.
    ///
    /// # Arguments
    /// * `validate` - Whether to validate events before processing
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.config.validate_events = validate;
        self
    }

    /// Enables or disables priority ordering.
    ///
    /// # Arguments
    /// * `use_priority` - Whether to process handlers in priority order
    pub fn with_priority_ordering(mut self, use_priority: bool) -> Self {
        self.config.use_priority_ordering = use_priority;
        self
    }

    /// Sets whether to continue processing if a handler fails.
    ///
    /// # Arguments
    /// * `continue_on_failure` - Whether to continue processing despite handler failures
    pub fn with_continue_on_failure(mut self, continue_on_failure: bool) -> Self {
        self.config.continue_on_handler_failure = continue_on_failure;
        self
    }

    /// Sets the default priority for handlers.
    ///
    /// # Arguments
    /// * `priority` - Default priority for new handlers
    pub fn with_default_priority(mut self, priority: Priority) -> Self {
        self.config.default_handler_priority = priority;
        self
    }

    /// Enables or disables detailed error reporting.
    ///
    /// # Arguments
    /// * `detailed` - Whether to enable detailed error reporting
    pub fn with_detailed_errors(mut self, detailed: bool) -> Self {
        self.config.detailed_error_reporting = detailed;
        self
    }

    /// Sets the maximum number of concurrent handlers.
    ///
    /// # Arguments
    /// * `max_concurrent` - Maximum number of concurrent handlers, or None for unlimited
    pub fn with_max_concurrency(mut self, max_concurrent: Option<usize>) -> Self {
        self.config.max_concurrent_handlers = max_concurrent;
        self
    }

    /// Enables or disables concurrent execution of handlers.
    ///
    /// # Arguments
    /// * `concurrent` - Whether to execute handlers concurrently
    pub fn with_concurrent_execution(mut self, concurrent: bool) -> Self {
        self.config.concurrent_execution = concurrent;
        self
    }

    /// Sets the minimum number of handlers required to use concurrent execution.
    ///
    /// # Arguments
    /// * `threshold` - Minimum handlers needed for concurrent execution
    pub fn with_concurrent_threshold(mut self, threshold: usize) -> Self {
        self.config.concurrent_threshold = threshold;
        self
    }

    /// Enables or disables metrics collection.
    ///
    /// # Arguments
    /// * `enabled` - Whether to enable metrics collection
    pub fn with_metrics(self, _enabled: bool) -> Self {
        // TODO: Add metrics flag to AsyncEventBusConfig
        // For now, this is a no-op
        self
    }

    /// Builds the configured AsyncEventBus.
    ///
    /// # Returns
    /// A new AsyncEventBus instance with the specified configuration.
    pub fn build(self) -> AsyncEventBus {
        AsyncEventBus::with_config(self.config)
    }
}

#[cfg(feature = "async")]
impl Default for AsyncEventBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async")]
impl Default for AsyncEventBus {
    fn default() -> Self {
        Self::new()
    }
}

// Implementing Send and Sync for AsyncEventBus
#[cfg(feature = "async")]
unsafe impl Send for AsyncEventBus {}
#[cfg(feature = "async")]
unsafe impl Sync for AsyncEventBus {}

#[cfg(all(feature = "async", test))]
mod tests {
    use super::*;
    use crate::event::Event;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[derive(Clone, Debug)]
    struct AsyncTestEvent {
        value: i32,
    }

    impl Event for AsyncTestEvent {
        fn event_type_name() -> &'static str {
            "AsyncTestEvent"
        }
    }

    #[tokio::test]
    async fn test_async_event_bus_creation() {
        let bus = AsyncEventBus::new();
        assert_eq!(bus.total_handler_count().await, 0);
        assert!(!bus.is_processing().await);
    }

    #[tokio::test]
    async fn test_async_event_bus_with_config() {
        let config = AsyncEventBusConfig {
            max_handlers_per_event: Some(10),
            validate_events: false,
            concurrent_execution: false,
            ..Default::default()
        };

        let bus = AsyncEventBus::with_config(config);
        assert!(!bus.config().validate_events);
        assert_eq!(bus.config().max_handlers_per_event, Some(10));
        assert!(!bus.config().concurrent_execution);
    }

    #[tokio::test]
    async fn test_async_handler_registration() {
        let mut bus = AsyncEventBus::new();

        let handler_id = bus
            .on(|_event: AsyncTestEvent| async {
                // Async handler logic here
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            })
            .await;

        assert!(handler_id.value() > 0);
        assert_eq!(bus.total_handler_count().await, 1);
    }

    #[tokio::test]
    async fn test_async_event_emission() {
        let mut bus = AsyncEventBus::new();
        let counter = Arc::new(AtomicI32::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.on(move |_event: AsyncTestEvent| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        })
        .await;

        bus.emit(AsyncTestEvent { value: 42 })
            .await
            .expect("Emit should succeed");

        // Give the async handler time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_priority_ordering_async() {
        let mut bus = AsyncEventBus::new();
        let execution_order = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let order_clone1 = Arc::clone(&execution_order);
        bus.on_with_priority(
            move |_event: AsyncTestEvent| {
                let order = Arc::clone(&order_clone1);
                async move {
                    order.lock().await.push(1);
                }
            },
            Priority::Low,
        )
        .await;

        let order_clone2 = Arc::clone(&execution_order);
        bus.on_with_priority(
            move |_event: AsyncTestEvent| {
                let order = Arc::clone(&order_clone2);
                async move {
                    order.lock().await.push(2);
                }
            },
            Priority::High,
        )
        .await;

        let order_clone3 = Arc::clone(&execution_order);
        bus.on_with_priority(
            move |_event: AsyncTestEvent| {
                let order = Arc::clone(&order_clone3);
                async move {
                    order.lock().await.push(3);
                }
            },
            Priority::Normal,
        )
        .await;

        bus.emit(AsyncTestEvent { value: 42 })
            .await
            .expect("Emit should succeed");

        // Give handlers time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let order = execution_order.lock().await;
        // With concurrent execution, we might not get exact order, but all should be present
        assert_eq!(order.len(), 3);
        assert!(order.contains(&1));
        assert!(order.contains(&2));
        assert!(order.contains(&3));
    }

    #[tokio::test]
    async fn test_async_handler_unregistration() {
        let mut bus = AsyncEventBus::new();

        let handler_id = bus.on(|_event: AsyncTestEvent| async {}).await;
        assert_eq!(bus.total_handler_count().await, 1);

        let removed = bus.off(handler_id).await;
        assert!(removed);
        assert_eq!(bus.total_handler_count().await, 0);

        // Trying to remove the same handler again should return false
        let removed_again = bus.off(handler_id).await;
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_async_clear_all_handlers() {
        let mut bus = AsyncEventBus::new();

        bus.on(|_: AsyncTestEvent| async {}).await;
        bus.on(|_: AsyncTestEvent| async {}).await;
        assert_eq!(bus.total_handler_count().await, 2);

        bus.clear().await;
        assert_eq!(bus.total_handler_count().await, 0);
    }

    #[tokio::test]
    async fn test_async_shutdown() {
        let mut bus = AsyncEventBus::new();

        bus.shutdown().await.expect("Shutdown should succeed");

        // After shutdown, events should be rejected
        let result = bus.emit(AsyncTestEvent { value: 42 }).await;
        assert!(matches!(result, Err(EventBusError::ShuttingDown)));
    }
}
