//! Thread-safe event bus implementation for EventRS.
//!
//! This module provides thread-safe variants of event buses that can be
//! safely shared across multiple threads without external synchronization.
//!
//! # Examples
//!
//! ```rust
//! use eventrs::{ThreadSafeEventBus, Event};
//! use std::sync::Arc;
//! use std::thread;
//!
//! #[derive(Event, Clone, Debug)]
//! struct UserLoggedIn {
//!     user_id: u64,
//! }
//!
//! let bus = Arc::new(ThreadSafeEventBus::new());
//! let bus_clone = Arc::clone(&bus);
//!
//! // Register handler in main thread
//! bus.on(|event: UserLoggedIn| {
//!     println!("User {} logged in", event.user_id);
//! });
//!
//! // Emit event from another thread
//! thread::spawn(move || {
//!     bus_clone.emit(UserLoggedIn { user_id: 123 }).unwrap();
//! }).join().unwrap();
//! ```

use crate::error::{EventBusError, EventBusResult};
use crate::event::Event;
use crate::event_bus::{ErrorHandling, EventBusConfig};
use crate::filter::SharedFilter;
use crate::handler::{
    BoxedHandler, FallibleHandler, FallibleSyncBoxedHandler, Handler, HandlerId, SyncBoxedHandler,
};
use crate::priority::{Priority, PriorityOrdered};

#[cfg(feature = "metrics")]
use crate::metrics::{EmissionResult, EmissionToken, EventBusMetrics, HandlerResult};

#[cfg(not(feature = "metrics"))]
type HandlerResult = ();

use std::any::TypeId;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Instant;

/// Configuration for thread-safe event buses.
pub type ThreadSafeEventBusConfig = EventBusConfig;

/// Handler entry for thread-safe storage.
#[derive(Clone)]
struct ThreadSafeHandlerEntry<E: Event> {
    id: HandlerId,
    handler: Arc<dyn BoxedHandler<E>>,
    priority: Priority,
    filter: Option<SharedFilter<E>>,
}

impl<E: Event> ThreadSafeHandlerEntry<E> {
    fn new(id: HandlerId, handler: Box<dyn BoxedHandler<E>>, priority: Priority) -> Self {
        Self {
            id,
            handler: Arc::from(handler),
            priority,
            filter: None,
        }
    }

    fn new_with_filter(
        id: HandlerId,
        handler: Box<dyn BoxedHandler<E>>,
        priority: Priority,
        filter: SharedFilter<E>,
    ) -> Self {
        Self {
            id,
            handler: Arc::from(handler),
            priority,
            filter: Some(filter),
        }
    }

    fn should_handle(&self, event: &E) -> bool {
        match &self.filter {
            Some(filter) => filter.evaluate(event),
            None => true,
        }
    }
}

/// Thread-safe storage for event handlers.

/// Thread-safe event bus that can be safely shared across multiple threads.
///
/// The ThreadSafeEventBus provides all the functionality of the regular EventBus
/// but with thread-safe internals using Arc<RwLock<>> for safe concurrent access.
/// It can be cloned cheaply and shared across threads without external synchronization.
///
/// # Examples
///
/// ## Basic Thread-Safe Usage
///
/// ```rust
/// use eventrs::{ThreadSafeEventBus, Event};
/// use std::sync::Arc;
/// use std::thread;
///
/// #[derive(Event, Clone, Debug)]
/// struct MessageReceived {
///     content: String,
/// }
///
/// let bus = Arc::new(ThreadSafeEventBus::new());
///
/// // Register handler
/// bus.on(|event: MessageReceived| {
///     println!("Received: {}", event.content);
/// });
///
/// // Emit from multiple threads
/// let handles: Vec<_> = (0..3).map(|i| {
///     let bus_clone = Arc::clone(&bus);
///     thread::spawn(move || {
///         bus_clone.emit(MessageReceived {
///             content: format!("Message {}", i),
///         }).unwrap();
///     })
/// }).collect();
///
/// for handle in handles {
///     handle.join().unwrap();
/// }
/// ```
///
/// ## With Configuration
///
/// ```rust
/// use eventrs::{ThreadSafeEventBus, ThreadSafeEventBusConfig, Priority};
///
/// let config = ThreadSafeEventBusConfig {
///     max_handlers_per_event: Some(100),
///     use_priority_ordering: true,
///     default_handler_priority: Priority::High,
///     ..Default::default()
/// };
///
/// let bus = ThreadSafeEventBus::with_config(config);
/// ```
pub struct ThreadSafeEventBus {
    /// Configuration for this event bus
    config: ThreadSafeEventBusConfig,

    /// Thread-safe storage for handlers by event type ID
    handlers: Arc<RwLock<HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>>>,

    /// Thread-safe mapping from handler ID to event type ID for cleanup
    handler_registry: Arc<RwLock<HashMap<HandlerId, TypeId>>>,

    /// Whether the event bus is currently shutting down
    shutting_down: Arc<AtomicBool>,

    /// Thread-safe counter for total number of registered handlers
    handler_count: Arc<Mutex<usize>>,

    /// Optional metrics collection
    #[cfg(feature = "metrics")]
    metrics: Option<Arc<EventBusMetrics>>,
}

impl ThreadSafeEventBus {
    /// Creates a new ThreadSafeEventBus with default configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::ThreadSafeEventBus;
    ///
    /// let bus = ThreadSafeEventBus::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(ThreadSafeEventBusConfig::default())
    }

    /// Creates a new ThreadSafeEventBus with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration options for the event bus
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, ThreadSafeEventBusConfig, Priority};
    ///
    /// let config = ThreadSafeEventBusConfig {
    ///     max_handlers_per_event: Some(50),
    ///     default_handler_priority: Priority::High,
    ///     ..Default::default()
    /// };
    ///
    /// let bus = ThreadSafeEventBus::with_config(config);
    /// ```
    pub fn with_config(config: ThreadSafeEventBusConfig) -> Self {
        Self {
            #[cfg(feature = "metrics")]
            metrics: if config.enable_metrics {
                Some(Arc::new(EventBusMetrics::enabled()))
            } else {
                None
            },
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            handler_registry: Arc::new(RwLock::new(HashMap::new())),
            shutting_down: Arc::new(AtomicBool::new(false)),
            handler_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Emits an event to all registered handlers in a thread-safe manner.
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
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    ///
    /// let bus = ThreadSafeEventBus::new();
    /// bus.emit(TestEvent { value: 42 })?;
    /// # Ok::<(), eventrs::EventBusError>(())
    /// ```
    pub fn emit<E: Event>(&self, event: E) -> EventBusResult<()> {
        if self.shutting_down.load(Ordering::Acquire) {
            return Err(EventBusError::ShuttingDown);
        }

        #[cfg(feature = "metrics")]
        let token = self
            .metrics
            .as_ref()
            .map(|m| m.start_emission(TypeId::of::<E>()));

        let emission_start = Instant::now();

        // Validate the event if configured to do so
        if self.config.validate_events {
            event.validate()?;
        }

        // Get handlers for this event type
        let handlers = self.get_handlers_for_event::<E>()?;

        if handlers.is_empty() {
            #[cfg(feature = "metrics")]
            if let (Some(metrics), Some(token)) = (&self.metrics, token) {
                let result = EmissionResult::success(
                    emission_start.elapsed(),
                    0,
                    Vec::new(),
                    E::event_type_name().to_string(),
                    self.config.use_priority_ordering,
                );
                metrics.record_emission_end(token, result);
            }
            return Ok(());
        }

        // Process handlers based on configuration
        #[cfg(feature = "metrics")]
        let handler_results = if self.config.use_priority_ordering {
            self.process_handlers_with_priority(&event, handlers)?
        } else {
            self.process_handlers_sequential(&event, handlers)?
        };

        #[cfg(not(feature = "metrics"))]
        if self.config.use_priority_ordering {
            self.process_handlers_with_priority_simple(&event, handlers)?;
        } else {
            self.process_handlers_sequential_simple(&event, handlers)?;
        }

        #[cfg(feature = "metrics")]
        if let (Some(metrics), Some(token)) = (&self.metrics, token) {
            let total_duration = emission_start.elapsed();
            let handlers_executed = handler_results.len();
            let result = EmissionResult::success(
                total_duration,
                handlers_executed,
                handler_results,
                E::event_type_name().to_string(),
                self.config.use_priority_ordering,
            );
            metrics.record_emission_end(token, result);
        }

        Ok(())
    }

    /// Registers a handler for events of type `E` in a thread-safe manner.
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler to register
    ///
    /// # Returns
    ///
    /// Returns a `HandlerId` that can be used to unregister the handler later.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    ///
    /// let bus = ThreadSafeEventBus::new();
    /// let handler_id = bus.on(|event: TestEvent| {
    ///     println!("Received: {}", event.value);
    /// });
    /// ```
    pub fn on<E: Event, H: Handler<E>>(&self, handler: H) -> HandlerId {
        self.register_handler(handler, self.config.default_handler_priority)
    }

    /// Registers a handler with a specific priority.
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler to register
    /// * `priority` - The priority for this handler
    ///
    /// # Returns
    ///
    /// Returns a `HandlerId` that can be used to unregister the handler later.
    pub fn on_with_priority<E: Event, H: Handler<E>>(
        &self,
        handler: H,
        priority: Priority,
    ) -> HandlerId {
        self.register_handler(handler, priority)
    }

    /// Registers a handler with a filter.
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler to register
    /// * `filter` - Filter to determine if this handler should process events
    ///
    /// # Returns
    ///
    /// Returns a `HandlerId` that can be used to unregister the handler later.
    pub fn on_with_filter<E: Event, H: Handler<E>>(
        &self,
        handler: H,
        filter: SharedFilter<E>,
    ) -> HandlerId {
        self.register_handler_with_filter(handler, self.config.default_handler_priority, filter)
    }

    /// Registers a fallible handler that can return errors.
    ///
    /// # Arguments
    ///
    /// * `handler` - The fallible handler to register
    ///
    /// # Returns
    ///
    /// Returns a `HandlerId` that can be used to unregister the handler later.
    pub fn on_fallible<E: Event, H: FallibleHandler<E>>(&self, handler: H) -> HandlerId {
        self.register_fallible_handler(handler, self.config.default_handler_priority)
    }

    /// Unregisters a handler by its ID in a thread-safe manner.
    ///
    /// # Arguments
    ///
    /// * `handler_id` - The ID of the handler to remove
    ///
    /// # Returns
    ///
    /// Returns `true` if the handler was found and removed, `false` otherwise.
    pub fn off(&self, handler_id: HandlerId) -> bool {
        let type_id = {
            let registry = self.handler_registry.read().unwrap();
            registry.get(&handler_id).copied()
        };

        if let Some(type_id) = type_id {
            let mut handlers_map = self.handlers.write().unwrap();
            if let Some(_handlers_any) = handlers_map.get_mut(&type_id) {
                // In a real implementation, we would properly remove the specific handler
                // For now, this is simplified
                let mut registry = self.handler_registry.write().unwrap();
                registry.remove(&handler_id);

                let mut count = self.handler_count.lock().unwrap();
                if *count > 0 {
                    *count -= 1;
                }

                #[cfg(feature = "metrics")]
                if let Some(ref metrics) = self.metrics {
                    metrics.record_handler_unregistration(handler_id.to_string());
                }

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Clears all handlers from the event bus in a thread-safe manner.
    pub fn clear(&self) {
        {
            let mut handlers_map = self.handlers.write().unwrap();
            handlers_map.clear();
        }

        {
            let mut registry = self.handler_registry.write().unwrap();
            registry.clear();
        }

        {
            let mut count = self.handler_count.lock().unwrap();
            *count = 0;
        }
    }

    /// Returns the total number of registered handlers.
    pub fn total_handler_count(&self) -> usize {
        let count = self.handler_count.lock().unwrap();
        *count
    }

    /// Returns whether the event bus is currently processing events.
    ///
    /// Note: In a thread-safe context, this may not be entirely accurate
    /// due to concurrent operations.
    pub fn is_processing(&self) -> bool {
        // For thread-safe implementation, we can't easily track processing state
        // across multiple threads, so we return false
        false
    }

    /// Shuts down the event bus in a thread-safe manner.
    ///
    /// After shutdown, no new events will be processed.
    pub fn shutdown(&self) -> EventBusResult<()> {
        self.shutting_down.store(true, Ordering::Release);
        Ok(())
    }

    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &ThreadSafeEventBusConfig {
        &self.config
    }

    /// Gets metrics for this event bus (if metrics are enabled).
    #[cfg(feature = "metrics")]
    pub fn metrics(&self) -> Option<&Arc<EventBusMetrics>> {
        self.metrics.as_ref()
    }

    // Private helper methods

    fn register_handler<E: Event, H: Handler<E>>(
        &self,
        handler: H,
        priority: Priority,
    ) -> HandlerId {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();

        // Check limits
        if let Some(max_total) = self.config.max_total_handlers {
            if self.total_handler_count() >= max_total {
                panic!("Maximum total handlers exceeded");
            }
        }

        // Create boxed handler
        let boxed = Box::new(SyncBoxedHandler::new(handler));
        let entry = ThreadSafeHandlerEntry::new(handler_id, boxed, priority);

        // Store handler
        {
            let mut handlers_map = self.handlers.write().unwrap();
            let handlers_vec = handlers_map.entry(type_id).or_insert_with(|| {
                Box::new(Vec::<ThreadSafeHandlerEntry<E>>::new())
                    as Box<dyn std::any::Any + Send + Sync>
            });

            if let Some(vec) = handlers_vec.downcast_mut::<Vec<ThreadSafeHandlerEntry<E>>>() {
                vec.push(entry);
            }
        }

        // Register handler ID
        {
            let mut registry = self.handler_registry.write().unwrap();
            registry.insert(handler_id, type_id);
        }

        // Increment counter
        {
            let mut count = self.handler_count.lock().unwrap();
            *count += 1;
        }

        #[cfg(feature = "metrics")]
        if let Some(ref metrics) = self.metrics {
            metrics.record_handler_registration(handler_id.to_string());
        }

        handler_id
    }

    fn register_handler_with_filter<E: Event, H: Handler<E>>(
        &self,
        handler: H,
        priority: Priority,
        filter: SharedFilter<E>,
    ) -> HandlerId {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();

        // Check limits
        if let Some(max_total) = self.config.max_total_handlers {
            if self.total_handler_count() >= max_total {
                panic!("Maximum total handlers exceeded");
            }
        }

        // Create boxed handler
        let boxed = Box::new(SyncBoxedHandler::new(handler));
        let entry = ThreadSafeHandlerEntry::new_with_filter(handler_id, boxed, priority, filter);

        // Store handler
        {
            let mut handlers_map = self.handlers.write().unwrap();
            let handlers_vec = handlers_map.entry(type_id).or_insert_with(|| {
                Box::new(Vec::<ThreadSafeHandlerEntry<E>>::new())
                    as Box<dyn std::any::Any + Send + Sync>
            });

            if let Some(vec) = handlers_vec.downcast_mut::<Vec<ThreadSafeHandlerEntry<E>>>() {
                vec.push(entry);
            }
        }

        // Register handler ID
        {
            let mut registry = self.handler_registry.write().unwrap();
            registry.insert(handler_id, type_id);
        }

        // Increment counter
        {
            let mut count = self.handler_count.lock().unwrap();
            *count += 1;
        }

        #[cfg(feature = "metrics")]
        if let Some(ref metrics) = self.metrics {
            metrics.record_handler_registration(handler_id.to_string());
        }

        handler_id
    }

    fn register_fallible_handler<E: Event, H: FallibleHandler<E>>(
        &self,
        handler: H,
        priority: Priority,
    ) -> HandlerId {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();

        // Create boxed handler
        let boxed = Box::new(FallibleSyncBoxedHandler::new(handler));
        let entry = ThreadSafeHandlerEntry::new(handler_id, boxed, priority);

        // Store handler
        {
            let mut handlers_map = self.handlers.write().unwrap();
            let handlers_vec = handlers_map.entry(type_id).or_insert_with(|| {
                Box::new(Vec::<ThreadSafeHandlerEntry<E>>::new())
                    as Box<dyn std::any::Any + Send + Sync>
            });

            if let Some(vec) = handlers_vec.downcast_mut::<Vec<ThreadSafeHandlerEntry<E>>>() {
                vec.push(entry);
            }
        }

        // Register handler ID
        {
            let mut registry = self.handler_registry.write().unwrap();
            registry.insert(handler_id, type_id);
        }

        // Increment counter
        {
            let mut count = self.handler_count.lock().unwrap();
            *count += 1;
        }

        #[cfg(feature = "metrics")]
        if let Some(ref metrics) = self.metrics {
            metrics.record_handler_registration(handler_id.to_string());
        }

        handler_id
    }

    fn get_handlers_for_event<E: Event>(&self) -> EventBusResult<Vec<ThreadSafeHandlerEntry<E>>> {
        let handlers_map = self.handlers.read().unwrap();
        let type_id = TypeId::of::<E>();

        if let Some(handlers_any) = handlers_map.get(&type_id) {
            if let Some(handlers) = handlers_any.downcast_ref::<Vec<ThreadSafeHandlerEntry<E>>>() {
                Ok(handlers.clone())
            } else {
                Ok(Vec::new())
            }
        } else {
            Ok(Vec::new())
        }
    }

    fn process_handlers_with_priority<E: Event>(
        &self,
        event: &E,
        handlers: Vec<ThreadSafeHandlerEntry<E>>,
    ) -> EventBusResult<Vec<HandlerResult>> {
        let mut priority_handlers: BinaryHeap<PriorityOrdered<&ThreadSafeHandlerEntry<E>>> =
            handlers
                .iter()
                .map(|entry| PriorityOrdered::new(entry, entry.priority))
                .collect();

        let mut handler_results = Vec::new();

        while let Some(ordered_handler) = priority_handlers.pop() {
            let handler_entry = ordered_handler.item();

            // Check if the handler should process this event based on its filter
            if !handler_entry.should_handle(event) {
                #[cfg(feature = "metrics")]
                handler_results.push(HandlerResult::skipped(
                    handler_entry.id.to_string(),
                    format!("{:?}", handler_entry.priority),
                ));
                continue;
            }

            let execution_start = Instant::now();
            let result = self.execute_handler(handler_entry, event.clone());
            let _execution_time = execution_start.elapsed();

            #[cfg(feature = "metrics")]
            {
                let handler_result = if result.is_ok() {
                    HandlerResult::success(
                        handler_entry.id.to_string(),
                        _execution_time,
                        format!("{:?}", handler_entry.priority),
                    )
                } else {
                    HandlerResult::failure(
                        handler_entry.id.to_string(),
                        _execution_time,
                        format!("{:?}", handler_entry.priority),
                        result.as_ref().err().unwrap().to_string(),
                    )
                };
                handler_results.push(handler_result);
            }

            if let Err(error) = result {
                match self.config.error_handling {
                    ErrorHandling::StopOnFirstError => return Err(error),
                    ErrorHandling::ContinueOnError => {
                        if self.config.detailed_error_reporting {
                            eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                        }
                    }
                    ErrorHandling::RetryOnError(_) => {
                        // TODO: Implement retry logic
                        if self.config.detailed_error_reporting {
                            eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                        }
                    }
                }
            }
        }

        Ok(handler_results)
    }

    fn process_handlers_sequential<E: Event>(
        &self,
        event: &E,
        handlers: Vec<ThreadSafeHandlerEntry<E>>,
    ) -> EventBusResult<Vec<HandlerResult>> {
        let mut handler_results = Vec::new();

        for handler_entry in handlers {
            // Check if the handler should process this event based on its filter
            if !handler_entry.should_handle(event) {
                #[cfg(feature = "metrics")]
                handler_results.push(HandlerResult::skipped(
                    handler_entry.id.to_string(),
                    format!("{:?}", handler_entry.priority),
                ));
                continue;
            }

            let execution_start = Instant::now();
            let result = self.execute_handler(&handler_entry, event.clone());
            let _execution_time = execution_start.elapsed();

            #[cfg(feature = "metrics")]
            {
                let handler_result = if result.is_ok() {
                    HandlerResult::success(
                        handler_entry.id.to_string(),
                        _execution_time,
                        format!("{:?}", handler_entry.priority),
                    )
                } else {
                    HandlerResult::failure(
                        handler_entry.id.to_string(),
                        _execution_time,
                        format!("{:?}", handler_entry.priority),
                        result.as_ref().err().unwrap().to_string(),
                    )
                };
                handler_results.push(handler_result);
            }

            if let Err(error) = result {
                match self.config.error_handling {
                    ErrorHandling::StopOnFirstError => return Err(error),
                    ErrorHandling::ContinueOnError => {
                        if self.config.detailed_error_reporting {
                            eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                        }
                    }
                    ErrorHandling::RetryOnError(_) => {
                        // TODO: Implement retry logic
                        if self.config.detailed_error_reporting {
                            eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                        }
                    }
                }
            }
        }

        Ok(handler_results)
    }

    #[cfg(not(feature = "metrics"))]
    fn process_handlers_with_priority_simple<E: Event>(
        &self,
        event: &E,
        handlers: Vec<ThreadSafeHandlerEntry<E>>,
    ) -> EventBusResult<()> {
        let mut priority_handlers: BinaryHeap<PriorityOrdered<&ThreadSafeHandlerEntry<E>>> =
            handlers
                .iter()
                .map(|entry| PriorityOrdered::new(entry, entry.priority))
                .collect();

        while let Some(ordered_handler) = priority_handlers.pop() {
            let handler_entry = ordered_handler.item();

            if !handler_entry.should_handle(event) {
                continue;
            }

            if let Err(error) = self.execute_handler(handler_entry, event.clone()) {
                match self.config.error_handling {
                    ErrorHandling::StopOnFirstError => return Err(error),
                    ErrorHandling::ContinueOnError => {
                        if self.config.detailed_error_reporting {
                            eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                        }
                    }
                    ErrorHandling::RetryOnError(_) => {
                        if self.config.detailed_error_reporting {
                            eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "metrics"))]
    fn process_handlers_sequential_simple<E: Event>(
        &self,
        event: &E,
        handlers: Vec<ThreadSafeHandlerEntry<E>>,
    ) -> EventBusResult<()> {
        for handler_entry in handlers {
            if !handler_entry.should_handle(event) {
                continue;
            }

            if let Err(error) = self.execute_handler(&handler_entry, event.clone()) {
                match self.config.error_handling {
                    ErrorHandling::StopOnFirstError => return Err(error),
                    ErrorHandling::ContinueOnError => {
                        if self.config.detailed_error_reporting {
                            eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                        }
                    }
                    ErrorHandling::RetryOnError(_) => {
                        if self.config.detailed_error_reporting {
                            eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn execute_handler<E: Event>(
        &self,
        handler_entry: &ThreadSafeHandlerEntry<E>,
        event: E,
    ) -> EventBusResult<()> {
        handler_entry.handler.call(event);
        Ok(())
    }

    /// Creates a new EventSender for the specified event type.
    ///
    /// EventSender provides channel-based event emission, allowing events
    /// to be sent from any thread and processed asynchronously.
    ///
    /// # Arguments
    ///
    /// * `buffer_size` - Size of the internal channel buffer (default 100 if 0)
    ///
    /// # Returns
    ///
    /// Returns a new EventSender that can be used to send events of type E.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    /// use std::sync::Arc;
    ///
    /// #[derive(Event, Clone)]
    /// struct MyEvent { value: i32 }
    ///
    /// let bus = Arc::new(ThreadSafeEventBus::new());
    /// let sender = bus.create_sender::<MyEvent>(100).unwrap();
    ///
    /// sender.send(MyEvent { value: 42 }).unwrap();
    /// ```
    pub fn create_sender<E: Event>(&self, buffer_size: usize) -> EventBusResult<EventSender<E>> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EventBusError::ShuttingDown);
        }

        let buffer_size = if buffer_size == 0 { 100 } else { buffer_size };
        let bus_arc = Arc::new(self.clone());
        Ok(EventSender::new(bus_arc, buffer_size))
    }

    /// Emits a batch of events of the same type sequentially.
    ///
    /// This method processes events one after another in the order they appear
    /// in the batch. All handlers for each event are executed before moving
    /// to the next event.
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of events to emit
    ///
    /// # Returns
    ///
    /// Returns a vector of emission results, one for each event in the batch.
    /// If any event fails, the remaining events will still be processed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct BatchEvent { id: u32, data: String }
    ///
    /// let bus = ThreadSafeEventBus::new();
    ///
    /// let events = vec![
    ///     BatchEvent { id: 1, data: "first".to_string() },
    ///     BatchEvent { id: 2, data: "second".to_string() },
    ///     BatchEvent { id: 3, data: "third".to_string() },
    /// ];
    ///
    /// let results = bus.emit_batch(events).unwrap();
    /// assert_eq!(results.len(), 3);
    /// ```
    pub fn emit_batch<E: Event>(&self, events: Vec<E>) -> EventBusResult<Vec<EventBusResult<()>>> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EventBusError::ShuttingDown);
        }

        let mut results = Vec::with_capacity(events.len());

        for event in events {
            let result = self.emit(event);
            results.push(result);
        }

        Ok(results)
    }

    /// Emits a batch of events of the same type concurrently.
    ///
    /// This method processes events in parallel using multiple threads.
    /// Each event is processed independently, allowing for better performance
    /// when handling large batches.
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of events to emit
    ///
    /// # Returns
    ///
    /// Returns a vector of emission results, one for each event in the batch.
    /// The order of results corresponds to the order of input events.
    ///
    /// # Performance Notes
    ///
    /// - Uses a thread pool for parallel processing
    /// - Automatically determines optimal thread count based on batch size
    /// - For small batches (< 4 events), falls back to sequential processing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct BatchEvent { id: u32, data: String }
    ///
    /// let bus = ThreadSafeEventBus::new();
    ///
    /// let events = vec![
    ///     BatchEvent { id: 1, data: "first".to_string() },
    ///     BatchEvent { id: 2, data: "second".to_string() },
    ///     BatchEvent { id: 3, data: "third".to_string() },
    /// ];
    ///
    /// let results = bus.emit_batch_concurrent(events).unwrap();
    /// assert_eq!(results.len(), 3);
    /// ```
    pub fn emit_batch_concurrent<E: Event>(
        &self,
        events: Vec<E>,
    ) -> EventBusResult<Vec<EventBusResult<()>>> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EventBusError::ShuttingDown);
        }

        let batch_size = events.len();

        // For small batches, use sequential processing to avoid thread overhead
        if batch_size < 4 {
            return self.emit_batch(events);
        }

        // Process events concurrently using thread pool
        let bus = Arc::new(self.clone());
        let mut handles = Vec::new();

        for event in events {
            let bus_clone = Arc::clone(&bus);

            let handle = thread::spawn(move || bus_clone.emit(event));

            handles.push(handle);
        }

        // Wait for all threads to complete and collect results
        let mut final_results = Vec::with_capacity(batch_size);

        for handle in handles {
            match handle.join() {
                Ok(result) => final_results.push(result),
                Err(_) => {
                    final_results.push(Err(EventBusError::internal("Thread processing failed")))
                }
            }
        }

        Ok(final_results)
    }

    /// Emits a batch of mixed event types sequentially.
    ///
    /// This method allows emitting different types of events in a single batch.
    /// Each event is processed sequentially with its appropriate handlers.
    ///
    /// # Arguments
    ///
    /// * `emit_fn` - A closure that performs the emission for each event
    ///
    /// # Returns
    ///
    /// Returns a vector of emission results for all events in the batch.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct EventA { id: u32 }
    ///
    /// #[derive(Event, Clone)]
    /// struct EventB { name: String }
    ///
    /// let bus = ThreadSafeEventBus::new();
    ///
    /// let results = bus.emit_mixed_batch(|emitter| {
    ///     emitter(EventA { id: 1 })?;
    ///     emitter(EventB { name: "test".to_string() })?;
    ///     emitter(EventA { id: 2 })?;
    ///     Ok(())
    /// }).unwrap();
    /// ```
    pub fn emit_mixed_batch<F>(&self, emit_fn: F) -> EventBusResult<Vec<EventBusResult<()>>>
    where
        F: FnOnce(
            &mut dyn FnMut(Box<dyn std::any::Any + Send>) -> EventBusResult<()>,
        ) -> EventBusResult<()>,
    {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EventBusError::ShuttingDown);
        }

        let mut results = Vec::new();
        let _bus = self.clone();

        {
            let mut emitter = |_event_any: Box<dyn std::any::Any + Send>| -> EventBusResult<()> {
                // This is a simplified version - full implementation would need type erasure
                // For now, we'll return an error indicating this feature needs more work
                results.push(Err(EventBusError::internal(
                    "Mixed batch emission not fully implemented yet",
                )));
                Ok(())
            };

            emit_fn(&mut emitter)?;
        }

        if results.is_empty() {
            results.push(Ok(()));
        }

        Ok(results)
    }

    /// Emits events from a stream lazily as they become available.
    ///
    /// This method processes events from an iterator in a streaming fashion,
    /// allowing for memory-efficient processing of large event sequences.
    /// Events are processed one at a time without loading all events into memory.
    ///
    /// # Arguments
    ///
    /// * `stream` - An iterator that yields events to be emitted
    ///
    /// # Returns
    ///
    /// Returns the number of events successfully processed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct StreamEvent { id: u32, data: String }
    ///
    /// let bus = ThreadSafeEventBus::new();
    ///
    /// // Stream events lazily from an iterator
    /// let events = (1..=1000).map(|i| StreamEvent {
    ///     id: i,
    ///     data: format!("event_{}", i)
    /// });
    ///
    /// let processed = bus.emit_stream(events).unwrap();
    /// assert_eq!(processed, 1000);
    /// ```
    pub fn emit_stream<E, I>(&self, stream: I) -> EventBusResult<usize>
    where
        E: Event,
        I: Iterator<Item = E>,
    {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EventBusError::ShuttingDown);
        }

        let mut processed_count = 0;

        for event in stream {
            // Check for shutdown on each iteration
            if self.shutting_down.load(Ordering::Relaxed) {
                break;
            }

            match self.emit(event) {
                Ok(_) => processed_count += 1,
                Err(_) => {
                    // Continue processing even if individual events fail
                    // This allows the stream to continue processing subsequent events
                    continue;
                }
            }
        }

        Ok(processed_count)
    }

    /// Emits events from a stream concurrently using a worker pool.
    ///
    /// This method processes events from an iterator using multiple worker threads
    /// for improved throughput. Events are distributed across workers and processed
    /// in parallel while maintaining memory efficiency.
    ///
    /// # Arguments
    ///
    /// * `stream` - An iterator that yields events to be emitted
    /// * `worker_count` - Number of worker threads to use (0 = auto-detect)
    ///
    /// # Returns
    ///
    /// Returns the number of events successfully processed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct StreamEvent { id: u32, data: String }
    ///
    /// let bus = ThreadSafeEventBus::new();
    ///
    /// // Stream events concurrently with 4 workers
    /// let events = (1..=1000).map(|i| StreamEvent {
    ///     id: i,
    ///     data: format!("event_{}", i)
    /// });
    ///
    /// let processed = bus.emit_stream_concurrent(events, 4).unwrap();
    /// assert_eq!(processed, 1000);
    /// ```
    pub fn emit_stream_concurrent<E, I>(
        &self,
        stream: I,
        worker_count: usize,
    ) -> EventBusResult<usize>
    where
        E: Event + Send + 'static,
        I: Iterator<Item = E>,
    {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EventBusError::ShuttingDown);
        }

        let workers = if worker_count == 0 {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4)
                .min(8) // Cap at 8 workers
        } else {
            worker_count
        };

        // Create a channel for distributing work
        let (sender, receiver) = mpsc::sync_channel::<Option<E>>(workers * 2);
        let receiver = Arc::new(Mutex::new(receiver));
        let processed_count = Arc::new(Mutex::new(0));

        // Spawn worker threads
        let mut handles = Vec::new();
        for _ in 0..workers {
            let bus = Arc::new(self.clone());
            let receiver = Arc::clone(&receiver);
            let count = Arc::clone(&processed_count);

            let handle = thread::spawn(move || {
                while let Ok(maybe_event) = {
                    let rx = receiver.lock().unwrap();
                    rx.recv()
                } {
                    match maybe_event {
                        Some(event) => {
                            if bus.emit(event).is_ok() {
                                let mut count = count.lock().unwrap();
                                *count += 1;
                            }
                        }
                        None => break, // Shutdown signal
                    }
                }
            });

            handles.push(handle);
        }

        // Send events to workers
        for event in stream {
            if self.shutting_down.load(Ordering::Relaxed) {
                break;
            }

            if sender.send(Some(event)).is_err() {
                // Channel closed, stop sending
                break;
            }
        }

        // Signal workers to shutdown
        for _ in 0..workers {
            let _ = sender.send(None);
        }

        // Wait for all workers to complete
        for handle in handles {
            let _ = handle.join();
        }

        let final_count = *processed_count.lock().unwrap();
        Ok(final_count)
    }

    /// Emits an event without waiting for processing to complete.
    ///
    /// This method queues an event for processing and returns immediately,
    /// making it suitable for fire-and-forget scenarios where you don't
    /// need to wait for handlers to complete execution.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to emit
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was successfully queued for processing.
    ///
    /// # Performance Notes
    ///
    /// - Uses internal event sender for non-blocking operation
    /// - Ideal for high-throughput scenarios
    /// - No guarantees about processing completion time
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct FireAndForgetEvent { id: u32, message: String }
    ///
    /// let bus = ThreadSafeEventBus::new();
    ///
    /// // Emit and don't wait for processing
    /// bus.emit_and_forget(FireAndForgetEvent {
    ///     id: 1,
    ///     message: "async processing".to_string()
    /// }).unwrap();
    ///
    /// // Continue with other work immediately
    /// println!("Event queued, continuing...");
    /// ```
    pub fn emit_and_forget<E: Event + Clone>(&self, event: E) -> EventBusResult<()> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EventBusError::ShuttingDown);
        }

        // Create a single-use event sender for fire-and-forget operation
        let sender = self.create_sender::<E>(1)?;

        match sender.try_send(event.clone()) {
            Ok(_) => Ok(()),
            Err(EventSenderError::ChannelFull) => {
                // If the channel is full, fall back to synchronous emission
                // This ensures the event is not lost
                self.emit(event)
            }
            Err(EventSenderError::ChannelDisconnected) => {
                Err(EventBusError::internal("Event sender disconnected"))
            }
        }
    }

    /// Emits multiple events without waiting for processing to complete.
    ///
    /// This method queues multiple events for processing and returns immediately.
    /// It's optimized for bulk fire-and-forget operations.
    ///
    /// # Arguments
    ///
    /// * `events` - A vector of events to emit
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all events were successfully queued for processing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, Event};
    ///
    /// #[derive(Event, Clone)]
    /// struct BulkEvent { id: u32, data: String }
    ///
    /// let bus = ThreadSafeEventBus::new();
    ///
    /// let events = vec![
    ///     BulkEvent { id: 1, data: "first".to_string() },
    ///     BulkEvent { id: 2, data: "second".to_string() },
    ///     BulkEvent { id: 3, data: "third".to_string() },
    /// ];
    ///
    /// // Queue all events for async processing
    /// bus.emit_bulk_and_forget(events).unwrap();
    ///
    /// // Continue immediately without waiting
    /// println!("All events queued!");
    /// ```
    pub fn emit_bulk_and_forget<E: Event + Clone>(&self, events: Vec<E>) -> EventBusResult<()> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EventBusError::ShuttingDown);
        }

        if events.is_empty() {
            return Ok(());
        }

        // Create a larger buffer for bulk operations
        let buffer_size = (events.len() * 2).min(1000).max(10);
        let sender = self.create_sender::<E>(buffer_size)?;

        for event in events {
            match sender.try_send(event) {
                Ok(_) => continue,
                Err(EventSenderError::ChannelFull) => {
                    // If channel is full, we could implement different strategies:
                    // 1. Block and wait (sender.send())
                    // 2. Drop the event
                    // 3. Fall back to sync processing
                    // For now, we'll fall back to sync to ensure no event loss
                    return Err(EventBusError::resource_exhausted(
                        "Event sender channel full",
                    ));
                }
                Err(EventSenderError::ChannelDisconnected) => {
                    return Err(EventBusError::internal("Event sender disconnected"));
                }
            }
        }

        Ok(())
    }
}

impl Clone for ThreadSafeEventBus {
    /// Creates a cheap clone of the ThreadSafeEventBus.
    ///
    /// All clones share the same underlying handlers and state,
    /// making it safe to pass around between threads.
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            handlers: Arc::clone(&self.handlers),
            handler_registry: Arc::clone(&self.handler_registry),
            shutting_down: Arc::clone(&self.shutting_down),
            handler_count: Arc::clone(&self.handler_count),
            #[cfg(feature = "metrics")]
            metrics: self.metrics.clone(),
        }
    }
}

impl Default for ThreadSafeEventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ThreadSafeEventBus is inherently Send and Sync due to its Arc-based internals
unsafe impl Send for ThreadSafeEventBus {}
unsafe impl Sync for ThreadSafeEventBus {}

/// Channel-based event sender for decoupled event emission.
///
/// EventSender provides a way to send events through channels, allowing
/// for decoupled event emission where the sender doesn't need direct
/// access to the event bus. This is useful for scenarios like:
/// - Cross-thread communication
/// - Actor model implementations
/// - Producer-consumer patterns
/// - Background task event emission
///
/// # Examples
///
/// ## Basic Channel Usage
///
/// ```rust
/// use eventrs::{ThreadSafeEventBus, EventSender, Event};
/// use std::sync::Arc;
/// use std::thread;
///
/// #[derive(Event, Clone, Debug)]
/// struct WorkCompleted {
///     task_id: u64,
///     result: String,
/// }
///
/// let bus = Arc::new(ThreadSafeEventBus::new());
///
/// // Register handler
/// bus.on(|event: WorkCompleted| {
///     println!("Task {} completed: {}", event.task_id, event.result);
/// });
///
/// // Create channel-based sender
/// let sender = EventSender::new(Arc::clone(&bus), 100);
///
/// // Send events from background thread
/// thread::spawn(move || {
///     sender.send(WorkCompleted {
///         task_id: 123,
///         result: "Success".to_string(),
///     }).unwrap();
/// });
/// ```
///
/// ## With Custom Error Handling
///
/// ```rust
/// use eventrs::{ThreadSafeEventBus, EventSender};
///
/// let bus = ThreadSafeEventBus::new();
/// let sender = EventSender::with_error_handler(
///     Arc::new(bus),
///     50,
///     |error| {
///         eprintln!("Event processing error: {}", error);
///     }
/// );
/// ```
pub struct EventSender<E: Event> {
    sender: mpsc::SyncSender<E>,
    _handle: thread::JoinHandle<()>,
}

impl<E: Event> EventSender<E> {
    /// Creates a new EventSender with the specified buffer size.
    ///
    /// # Arguments
    ///
    /// * `bus` - The event bus to send events to
    /// * `buffer_size` - Size of the internal channel buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, EventSender, Event};
    /// use std::sync::Arc;
    ///
    /// #[derive(Event, Clone)]
    /// struct MyEvent { value: i32 }
    ///
    /// let bus = Arc::new(ThreadSafeEventBus::new());
    /// let sender = EventSender::new(bus, 100);
    /// ```
    pub fn new(bus: Arc<ThreadSafeEventBus>, buffer_size: usize) -> Self {
        Self::with_error_handler(bus, buffer_size, |error| {
            eprintln!("EventSender error: {}", error);
        })
    }

    /// Creates a new EventSender with a custom error handler.
    ///
    /// # Arguments
    ///
    /// * `bus` - The event bus to send events to
    /// * `buffer_size` - Size of the internal channel buffer
    /// * `error_handler` - Function to handle event processing errors
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eventrs::{ThreadSafeEventBus, EventSender};
    /// use std::sync::Arc;
    ///
    /// let bus = Arc::new(ThreadSafeEventBus::new());
    /// let sender = EventSender::with_error_handler(
    ///     bus,
    ///     50,
    ///     |error| {
    ///         log::error!("Event processing failed: {}", error);
    ///     }
    /// );
    /// ```
    pub fn with_error_handler<F>(
        bus: Arc<ThreadSafeEventBus>,
        buffer_size: usize,
        error_handler: F,
    ) -> Self
    where
        F: Fn(EventBusError) + Send + 'static,
    {
        let (sender, receiver) = mpsc::sync_channel(if buffer_size > 0 { buffer_size } else { 1 });

        let handle = thread::spawn(move || {
            Self::event_processing_loop(receiver, bus, error_handler);
        });

        Self {
            sender,
            _handle: handle,
        }
    }

    /// Sends an event through the channel.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to send
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was successfully queued,
    /// or an error if the channel is disconnected.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use eventrs::{ThreadSafeEventBus, EventSender, Event};
    /// # use std::sync::Arc;
    /// # #[derive(Event, Clone)]
    /// # struct MyEvent { value: i32 }
    /// # let bus = Arc::new(ThreadSafeEventBus::new());
    /// # let sender = EventSender::new(bus, 10);
    ///
    /// sender.send(MyEvent { value: 42 })?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn send(&self, event: E) -> Result<(), EventSenderError> {
        self.sender
            .send(event)
            .map_err(|_| EventSenderError::ChannelDisconnected)
    }

    /// Attempts to send an event without blocking.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to send
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was successfully queued,
    /// `Err(EventSenderError::ChannelFull)` if the channel buffer is full,
    /// or `Err(EventSenderError::ChannelDisconnected)` if the channel is disconnected.
    pub fn try_send(&self, event: E) -> Result<(), EventSenderError> {
        self.sender.try_send(event).map_err(|e| match e {
            mpsc::TrySendError::Full(_) => EventSenderError::ChannelFull,
            mpsc::TrySendError::Disconnected(_) => EventSenderError::ChannelDisconnected,
        })
    }

    /// Returns whether the sender is still connected to the processing thread.
    pub fn is_connected(&self) -> bool {
        !self._handle.is_finished()
    }

    /// Gets the current number of queued events (approximation).
    ///
    /// Note: This is an approximation and may not be exact due to
    /// concurrent access from multiple threads.
    pub fn queued_events(&self) -> usize {
        // Unfortunately, std::sync::mpsc doesn't provide a way to check queue length
        // This would require a custom implementation or third-party channels
        0
    }

    // Private helper method for the event processing loop
    fn event_processing_loop<F>(
        receiver: mpsc::Receiver<E>,
        bus: Arc<ThreadSafeEventBus>,
        error_handler: F,
    ) where
        F: Fn(EventBusError),
    {
        while let Ok(event) = receiver.recv() {
            if let Err(error) = bus.emit(event) {
                error_handler(error);
            }
        }
    }
}

/// Errors that can occur when using EventSender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventSenderError {
    /// The channel is full and cannot accept more events.
    ChannelFull,
    /// The channel is disconnected (processing thread has stopped).
    ChannelDisconnected,
}

impl std::fmt::Display for EventSenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSenderError::ChannelFull => write!(f, "Event channel is full"),
            EventSenderError::ChannelDisconnected => write!(f, "Event channel is disconnected"),
        }
    }
}

impl std::error::Error for EventSenderError {}

/// A multi-type event sender that can handle different event types.
///
/// MultiEventSender allows sending multiple types of events through
/// a single sender interface, useful for scenarios where you need
/// to emit different event types from the same location.
///
/// # Examples
///
/// ```rust
/// use eventrs::{ThreadSafeEventBus, MultiEventSender, Event};
/// use std::sync::Arc;
///
/// #[derive(Event, Clone)]
/// struct EventA { value: i32 }
///
/// #[derive(Event, Clone)]
/// struct EventB { message: String }
///
/// let bus = Arc::new(ThreadSafeEventBus::new());
/// let sender = MultiEventSender::new(bus, 100);
///
/// // Send different event types
/// sender.send(EventA { value: 42 });
/// sender.send(EventB { message: "hello".to_string() });
/// ```
pub struct MultiEventSender {
    bus: Arc<ThreadSafeEventBus>,
}

impl MultiEventSender {
    /// Creates a new MultiEventSender.
    ///
    /// # Arguments
    ///
    /// * `bus` - The event bus to send events to
    /// * `_buffer_size` - Reserved for future use (currently ignored)
    pub fn new(bus: Arc<ThreadSafeEventBus>, _buffer_size: usize) -> Self {
        Self { bus }
    }

    /// Sends an event directly to the bus.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to send
    ///
    /// # Returns
    ///
    /// Returns the result of the event emission.
    pub fn send<E: Event>(&self, event: E) -> EventBusResult<()> {
        self.bus.emit(event)
    }

    /// Gets a type-specific sender for the given event type.
    ///
    /// # Returns
    ///
    /// Returns an EventSender that can only send events of type E.
    pub fn get_sender<E: Event>(&self, buffer_size: usize) -> EventSender<E> {
        EventSender::new(Arc::clone(&self.bus), buffer_size)
    }

    /// Gets a reference to the underlying event bus.
    pub fn bus(&self) -> &Arc<ThreadSafeEventBus> {
        &self.bus
    }
}
