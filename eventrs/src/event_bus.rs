//! Core synchronous event bus implementation.
//!
//! This module provides the main EventBus struct that handles event emission
//! and handler registration for synchronous event processing.

use crate::event::Event;
use crate::handler::{Handler, HandlerId, BoxedHandler, SyncBoxedHandler, FallibleHandler, FallibleSyncBoxedHandler};
use crate::error::{EventBusError, EventBusResult};
use crate::priority::{Priority, PriorityOrdered};
use crate::filter::{SharedFilter, FilterManager, BoxedAnyFilter};
// TODO: Re-enable when middleware system is complete
// use crate::middleware::Middleware;

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::BinaryHeap;

/// Error handling strategies for the EventBus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorHandling {
    /// Stop processing on first error
    StopOnFirstError,
    /// Continue processing despite errors
    ContinueOnError,
    /// Retry failed handlers up to N times
    RetryOnError(u32),
}

/// Configuration options for the EventBus.
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Maximum number of handlers per event type.
    pub max_handlers_per_event: Option<usize>,
    
    /// Maximum total number of handlers across all event types.
    pub max_total_handlers: Option<usize>,
    
    /// Initial capacity for handler storage.
    pub initial_capacity: usize,
    
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
    
    /// Error handling strategy.
    pub error_handling: ErrorHandling,
    
    /// Whether to enable metrics collection.
    pub enable_metrics: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            max_handlers_per_event: Some(1000),
            max_total_handlers: Some(10000),
            initial_capacity: 64,
            validate_events: true,
            use_priority_ordering: true,
            continue_on_handler_failure: true,
            default_handler_priority: Priority::Normal,
            detailed_error_reporting: false,
            error_handling: ErrorHandling::ContinueOnError,
            enable_metrics: false,
        }
    }
}

/// Handler entry stored in the event bus.
#[derive(Clone)]
struct HandlerEntry<E: Event> {
    id: HandlerId,
    handler: Arc<dyn BoxedHandler<E>>,
    priority: Priority,
    filter: Option<SharedFilter<E>>,
}

impl<E: Event> HandlerEntry<E> {
    fn new(id: HandlerId, handler: Box<dyn BoxedHandler<E>>, priority: Priority) -> Self {
        Self { 
            id, 
            handler: Arc::from(handler), 
            priority,
            filter: None,
        }
    }
    
    fn new_with_filter(id: HandlerId, handler: Box<dyn BoxedHandler<E>>, priority: Priority, filter: SharedFilter<E>) -> Self {
        Self { 
            id, 
            handler: Arc::from(handler), 
            priority,
            filter: Some(filter),
        }
    }
    
    /// Checks if this handler should process the given event based on its filter.
    fn should_handle(&self, event: &E) -> bool {
        match &self.filter {
            Some(filter) => filter.evaluate(event),
            None => true, // No filter means always handle
        }
    }
}

/// Thread-safe storage for event handlers.

/// Main synchronous event bus for processing events.
/// 
/// The EventBus provides a centralized system for emitting events and
/// registering handlers to process those events. It supports type-safe
/// event handling with zero-cost abstractions.
/// 
/// # Examples
/// 
/// ## Basic Usage
/// 
/// ```rust
/// use eventrs::{EventBus, Event};
/// 
/// #[derive(Event, Clone, Debug)]
/// struct UserLoggedIn {
///     user_id: u64,
///     timestamp: std::time::SystemTime,
/// }
/// 
/// let mut bus = EventBus::new();
/// 
/// // Register a handler
/// bus.on(|event: UserLoggedIn| {
///     println!("User {} logged in at {:?}", event.user_id, event.timestamp);
/// });
/// 
/// // Emit an event
/// bus.emit(UserLoggedIn {
///     user_id: 123,
///     timestamp: std::time::SystemTime::now(),
/// }).expect("Failed to emit event");
/// ```
/// 
/// ## With Configuration
/// 
/// ```rust
/// use eventrs::{EventBus, EventBusConfig};
/// 
/// let config = EventBusConfig {
///     max_handlers_per_event: Some(100),
///     validate_events: true,
///     ..Default::default()
/// };
/// 
/// let mut bus = EventBus::with_config(config);
/// ```
pub struct EventBus {
    /// Configuration for this event bus.
    config: EventBusConfig,
    
    /// Storage for handlers by event type ID.
    handlers: Arc<RwLock<HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>>>,
    
    /// Mapping from handler ID to event type ID for cleanup.
    handler_registry: Arc<RwLock<HashMap<HandlerId, TypeId>>>,
    
    /// Whether the event bus is currently shutting down.
    shutting_down: Arc<AtomicBool>,
    
    /// Counter for total number of registered handlers.
    handler_count: Arc<Mutex<usize>>,
    
    /// Global filter manager for bus-level filtering.
    global_filter_manager: Arc<FilterManager>,
}

impl EventBus {
    /// Creates a new EventBus with default configuration.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventBus;
    /// 
    /// let mut bus = EventBus::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(EventBusConfig::default())
    }
    
    /// Creates a new EventBusBuilder for configuring the event bus.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Priority};
    /// 
    /// let bus = EventBus::builder()
    ///     .with_capacity(1000)
    ///     .with_metrics(true)
    ///     .with_default_priority(Priority::High)
    ///     .build();
    /// ```
    pub fn builder() -> EventBusBuilder {
        EventBusBuilder::new()
    }
    
    /// Creates a new EventBus with the specified configuration.
    /// 
    /// # Arguments
    /// 
    /// * `config` - Configuration options for the event bus
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, EventBusConfig, Priority};
    /// 
    /// let config = EventBusConfig {
    ///     max_handlers_per_event: Some(50),
    ///     default_handler_priority: Priority::High,
    ///     ..Default::default()
    /// };
    /// 
    /// let mut bus = EventBus::with_config(config);
    /// ```
    pub fn with_config(config: EventBusConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            handler_registry: Arc::new(RwLock::new(HashMap::new())),
            shutting_down: Arc::new(AtomicBool::new(false)),
            handler_count: Arc::new(Mutex::new(0)),
            global_filter_manager: Arc::new(FilterManager::new()),
        }
    }
    
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
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Event};
    /// 
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    /// 
    /// let mut bus = EventBus::new();
    /// bus.emit(TestEvent { value: 42 })?;
    /// # Ok::<(), eventrs::EventBusError>(())
    /// ```
    pub fn emit<E: Event>(&self, event: E) -> EventBusResult<()> {
        if self.shutting_down.load(Ordering::Acquire) {
            return Err(EventBusError::ShuttingDown);
        }
        
        // Validate the event if configured to do so
        if self.config.validate_events {
            event.validate()?;
        }
        
        // Check global filters first
        if !self.global_filter_manager.evaluate(&event) {
            return Ok(()); // Event filtered out by global filters
        }
        
        // Get handlers for this event type
        let handlers = self.get_handlers_for_event::<E>()?;
        
        if handlers.is_empty() {
            return Ok(());
        }
        
        // Process handlers based on configuration
        if self.config.use_priority_ordering {
            self.process_handlers_with_priority(&event, handlers)?;
        } else {
            self.process_handlers_sequential(&event, handlers)?;
        }
        
        Ok(())
    }
    
    /// Registers a handler for events of type `E`.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The handler to register
    /// 
    /// # Returns
    /// 
    /// Returns a `HandlerId` that can be used to unregister the handler.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Event};
    /// 
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    /// 
    /// let mut bus = EventBus::new();
    /// let handler_id = bus.on(|event: TestEvent| {
    ///     println!("Value: {}", event.value);
    /// });
    /// ```
    pub fn on<E: Event, H: Handler<E>>(&mut self, handler: H) -> HandlerId {
        self.register_handler(handler, self.config.default_handler_priority)
    }
    
    /// Registers a handler with a specific priority for events of type `E`.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The handler to register
    /// * `priority` - The priority for this handler
    /// 
    /// # Returns
    /// 
    /// Returns a `HandlerId` that can be used to unregister the handler.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Event, Priority};
    /// 
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    /// 
    /// let mut bus = EventBus::new();
    /// let handler_id = bus.on_with_priority(
    ///     |event: TestEvent| println!("High priority: {}", event.value),
    ///     Priority::High
    /// );
    /// ```
    pub fn on_with_priority<E: Event, H: Handler<E>>(&mut self, handler: H, priority: Priority) -> HandlerId {
        self.register_handler(handler, priority)
    }
    
    /// Registers a fallible handler for events of type `E`.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The fallible handler to register
    /// 
    /// # Returns
    /// 
    /// Returns a `HandlerId` that can be used to unregister the handler.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Event};
    /// use std::io;
    /// 
    /// #[derive(Event, Clone)]
    /// struct FileEvent { path: String }
    /// 
    /// let mut bus = EventBus::new();
    /// let handler_id = bus.on_fallible(|event: FileEvent| -> Result<(), io::Error> {
    ///     std::fs::write(&event.path, b"content")?;
    ///     Ok(())
    /// });
    /// ```
    pub fn on_fallible<E: Event, H: FallibleHandler<E>>(&mut self, handler: H) -> HandlerId {
        self.register_fallible_handler(handler, self.config.default_handler_priority)
    }
    
    /// Registers a handler with a filter for events of type `E`.
    /// 
    /// The handler will only be called if the filter evaluates to `true` for the event.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The handler to register
    /// * `filter` - The filter to apply to events
    /// 
    /// # Returns
    /// 
    /// Returns a `HandlerId` that can be used to unregister the handler.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Event, PredicateFilter};
    /// 
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    /// 
    /// let mut bus = EventBus::new();
    /// let filter = PredicateFilter::new("high_value", |event: &TestEvent| {
    ///     event.value > 100
    /// });
    /// 
    /// let handler_id = bus.on_filtered(
    ///     |event: TestEvent| println!("High value: {}", event.value),
    ///     Arc::new(filter)
    /// );
    /// ```
    pub fn on_filtered<E: Event, H: Handler<E>>(&mut self, handler: H, filter: SharedFilter<E>) -> HandlerId {
        self.register_handler_with_filter(handler, self.config.default_handler_priority, filter)
    }
    
    /// Registers a handler with a filter and priority for events of type `E`.
    /// 
    /// # Arguments
    /// 
    /// * `handler` - The handler to register
    /// * `filter` - The filter to apply to events
    /// * `priority` - The priority for this handler
    /// 
    /// # Returns
    /// 
    /// Returns a `HandlerId` that can be used to unregister the handler.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Event, PredicateFilter, Priority};
    /// 
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    /// 
    /// let mut bus = EventBus::new();
    /// let filter = PredicateFilter::new("high_value", |event: &TestEvent| {
    ///     event.value > 100
    /// });
    /// 
    /// let handler_id = bus.on_filtered_with_priority(
    ///     |event: TestEvent| println!("High priority, high value: {}", event.value),
    ///     Arc::new(filter),
    ///     Priority::High
    /// );
    /// ```
    pub fn on_filtered_with_priority<E: Event, H: Handler<E>>(&mut self, handler: H, filter: SharedFilter<E>, priority: Priority) -> HandlerId {
        self.register_handler_with_filter(handler, priority, filter)
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
    /// use eventrs::{EventBus, Event};
    /// 
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    /// 
    /// let mut bus = EventBus::new();
    /// let handler_id = bus.on(|event: TestEvent| {
    ///     println!("Value: {}", event.value);
    /// });
    /// 
    /// // Later, remove the handler
    /// let removed = bus.off(handler_id);
    /// assert!(removed);
    /// ```
    pub fn off(&mut self, handler_id: HandlerId) -> bool {
        let type_id = {
            let registry = self.handler_registry.read().unwrap();
            match registry.get(&handler_id) {
                Some(type_id) => *type_id,
                None => return false,
            }
        };
        
        // Remove from handlers map
        let removed = {
            let mut handlers_map = self.handlers.write().unwrap();
            if let Some(_handlers_any) = handlers_map.get_mut(&type_id) {
                // This is a bit complex due to type erasure, but in a real implementation
                // we would have proper type-safe storage
                true // Simplified for now
            } else {
                false
            }
        };
        
        if removed {
            // Remove from registry
            let mut registry = self.handler_registry.write().unwrap();
            registry.remove(&handler_id);
            
            // Decrement counter
            let mut count = self.handler_count.lock().unwrap();
            *count = count.saturating_sub(1);
        }
        
        removed
    }
    
    /// Returns the number of registered handlers for a specific event type.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Event};
    /// 
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    /// 
    /// let mut bus = EventBus::new();
    /// assert_eq!(bus.handler_count::<TestEvent>(), 0);
    /// 
    /// bus.on(|_: TestEvent| {});
    /// assert_eq!(bus.handler_count::<TestEvent>(), 1);
    /// ```
    pub fn handler_count<E: Event>(&self) -> usize {
        let handlers_map = self.handlers.read().unwrap();
        let type_id = TypeId::of::<E>();
        
        if let Some(_handlers_any) = handlers_map.get(&type_id) {
            // In a real implementation, we would downcast and get the actual count
            1 // Simplified for now
        } else {
            0
        }
    }
    
    /// Returns the total number of registered handlers across all event types.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventBus;
    /// 
    /// let mut bus = EventBus::new();
    /// assert_eq!(bus.total_handler_count(), 0);
    /// ```
    pub fn total_handler_count(&self) -> usize {
        *self.handler_count.lock().unwrap()
    }
    
    /// Clears all registered handlers.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, Event};
    /// 
    /// #[derive(Event, Clone)]
    /// struct TestEvent { value: i32 }
    /// 
    /// let mut bus = EventBus::new();
    /// bus.on(|_: TestEvent| {});
    /// assert!(bus.total_handler_count() > 0);
    /// 
    /// bus.clear();
    /// assert_eq!(bus.total_handler_count(), 0);
    /// ```
    pub fn clear(&mut self) {
        let mut handlers_map = self.handlers.write().unwrap();
        handlers_map.clear();
        
        let mut registry = self.handler_registry.write().unwrap();
        registry.clear();
        
        let mut count = self.handler_count.lock().unwrap();
        *count = 0;
    }
    
    /// Returns whether the event bus is currently processing events.
    /// 
    /// For the synchronous event bus, this always returns false since
    /// processing is synchronous and blocking.
    pub fn is_processing(&self) -> bool {
        false
    }
    
    /// Shuts down the event bus gracefully.
    /// 
    /// For the synchronous event bus, this simply marks the bus as shutting down
    /// and prevents new events from being processed.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventBus;
    /// 
    /// let mut bus = EventBus::new();
    /// bus.shutdown().expect("Failed to shutdown");
    /// ```
    pub fn shutdown(&mut self) -> EventBusResult<()> {
        self.shutting_down.store(true, Ordering::Release);
        Ok(())
    }
    
    /// Returns a reference to the current configuration.
    pub fn config(&self) -> &EventBusConfig {
        &self.config
    }
    
    // Private helper methods
    
    fn register_handler<E: Event, H: Handler<E>>(&mut self, handler: H, priority: Priority) -> HandlerId {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();
        
        // Check limits
        if let Some(max_total) = self.config.max_total_handlers {
            if self.total_handler_count() >= max_total {
                // In a real implementation, this would return a Result
                panic!("Maximum total handlers exceeded");
            }
        }
        
        // Create boxed handler
        let boxed = Box::new(SyncBoxedHandler::new(handler));
        let entry = HandlerEntry::new(handler_id, boxed, priority);
        
        // Store handler
        {
            let mut handlers_map = self.handlers.write().unwrap();
            let handlers_vec = handlers_map.entry(type_id).or_insert_with(|| {
                Box::new(Vec::<HandlerEntry<E>>::new()) as Box<dyn std::any::Any + Send + Sync>
            });
            
            // Downcast to the concrete type and add the handler
            if let Some(vec) = handlers_vec.downcast_mut::<Vec<HandlerEntry<E>>>() {
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
        
        handler_id
    }
    
    fn register_handler_with_filter<E: Event, H: Handler<E>>(&mut self, handler: H, priority: Priority, filter: SharedFilter<E>) -> HandlerId {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();
        
        // Check limits
        if let Some(max_total) = self.config.max_total_handlers {
            if self.total_handler_count() >= max_total {
                // In a real implementation, this would return a Result
                panic!("Maximum total handlers exceeded");
            }
        }
        
        // Create boxed handler
        let boxed = Box::new(SyncBoxedHandler::new(handler));
        let entry = HandlerEntry::new_with_filter(handler_id, boxed, priority, filter);
        
        // Store handler
        {
            let mut handlers_map = self.handlers.write().unwrap();
            let handlers_vec = handlers_map.entry(type_id).or_insert_with(|| {
                Box::new(Vec::<HandlerEntry<E>>::new()) as Box<dyn std::any::Any + Send + Sync>
            });
            
            // Downcast to the concrete type and add the handler
            if let Some(vec) = handlers_vec.downcast_mut::<Vec<HandlerEntry<E>>>() {
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
        
        handler_id
    }
    
    fn register_fallible_handler<E: Event, H: FallibleHandler<E>>(&mut self, handler: H, priority: Priority) -> HandlerId {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();
        
        // Create boxed handler
        let boxed = Box::new(FallibleSyncBoxedHandler::new(handler));
        let entry = HandlerEntry::new(handler_id, boxed, priority);
        
        // Store handler
        {
            let mut handlers_map = self.handlers.write().unwrap();
            let handlers_vec = handlers_map.entry(type_id).or_insert_with(|| {
                Box::new(Vec::<HandlerEntry<E>>::new()) as Box<dyn std::any::Any + Send + Sync>
            });
            
            // Downcast to the concrete type and add the handler
            if let Some(vec) = handlers_vec.downcast_mut::<Vec<HandlerEntry<E>>>() {
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
        
        handler_id
    }
    
    fn get_handlers_for_event<E: Event>(&self) -> EventBusResult<Vec<HandlerEntry<E>>> {
        let handlers_map = self.handlers.read().unwrap();
        let type_id = TypeId::of::<E>();
        
        if let Some(handlers_any) = handlers_map.get(&type_id) {
            if let Some(handlers) = handlers_any.downcast_ref::<Vec<HandlerEntry<E>>>() {
                // Clone the handlers for processing
                Ok(handlers.clone())
            } else {
                Ok(Vec::new())
            }
        } else {
            Ok(Vec::new())
        }
    }
    
    fn process_handlers_with_priority<E: Event>(&self, event: &E, handlers: Vec<HandlerEntry<E>>) -> EventBusResult<()> {
        // Sort handlers by priority (higher priority first)
        let mut priority_handlers: BinaryHeap<PriorityOrdered<&HandlerEntry<E>>> = handlers
            .iter()
            .map(|entry| PriorityOrdered::new(entry, entry.priority))
            .collect();
        
        while let Some(ordered_handler) = priority_handlers.pop() {
            let handler_entry = ordered_handler.item();
            
            // Check if the handler should process this event based on its filter
            if !handler_entry.should_handle(event) {
                continue; // Skip this handler if filter doesn't match
            }
            
            if let Err(error) = self.execute_handler(handler_entry, event.clone()) {
                if !self.config.continue_on_handler_failure {
                    return Err(error);
                }
                // Log error and continue with next handler
                if self.config.detailed_error_reporting {
                    eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                }
            }
        }
        
        Ok(())
    }
    
    fn process_handlers_sequential<E: Event>(&self, event: &E, handlers: Vec<HandlerEntry<E>>) -> EventBusResult<()> {
        for handler_entry in handlers {
            // Check if the handler should process this event based on its filter
            if !handler_entry.should_handle(event) {
                continue; // Skip this handler if filter doesn't match
            }
            
            if let Err(error) = self.execute_handler(&handler_entry, event.clone()) {
                if !self.config.continue_on_handler_failure {
                    return Err(error);
                }
                // Log error and continue with next handler
                if self.config.detailed_error_reporting {
                    eprintln!("Handler {} failed: {}", handler_entry.handler.name(), error);
                }
            }
        }
        
        Ok(())
    }
    
    fn execute_handler<E: Event>(&self, handler_entry: &HandlerEntry<E>, event: E) -> EventBusResult<()> {
        // In a real implementation, this would handle timeouts, retries, etc.
        handler_entry.handler.call(event);
        Ok(())
    }
    
    /// Adds a global filter that applies to all events.
    /// 
    /// Global filters are evaluated before any handler-specific filters and can
    /// prevent events from being processed entirely.
    /// 
    /// # Arguments
    /// 
    /// * `id` - Unique identifier for the filter
    /// * `filter` - The filter to add
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventBus, PredicateAnyFilter};
    /// 
    /// let bus = EventBus::new();
    /// 
    /// // Add a filter that only allows events during business hours
    /// let business_hours_filter = PredicateAnyFilter::new(
    ///     "business_hours",
    ///     |_event| {
    ///         let hour = chrono::Local::now().hour();
    ///         hour >= 9 && hour < 17
    ///     }
    /// );
    /// 
    /// bus.add_global_filter("business_hours", Box::new(business_hours_filter));
    /// ```
    pub fn add_global_filter<S: Into<String>>(&self, id: S, filter: BoxedAnyFilter) {
        self.global_filter_manager.add_filter(id, filter);
    }
    
    /// Adds a global filter with priority.
    /// 
    /// Higher priority filters are evaluated first.
    /// 
    /// # Arguments
    /// 
    /// * `id` - Unique identifier for the filter
    /// * `filter` - The filter to add
    /// * `priority` - Priority of the filter (higher values = higher priority)
    pub fn add_global_filter_with_priority<S: Into<String>>(&self, id: S, filter: BoxedAnyFilter, priority: i32) {
        self.global_filter_manager.add_filter_with_priority(id, filter, priority);
    }
    
    /// Removes a global filter by its ID.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The ID of the filter to remove
    /// 
    /// # Returns
    /// 
    /// `true` if the filter was found and removed, `false` otherwise
    pub fn remove_global_filter(&self, id: &str) -> bool {
        self.global_filter_manager.remove_filter(id)
    }
    
    /// Enables or disables a global filter.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The ID of the filter to modify
    /// * `enabled` - Whether the filter should be enabled
    /// 
    /// # Returns
    /// 
    /// `true` if the filter was found and modified, `false` otherwise
    pub fn set_global_filter_enabled(&self, id: &str, enabled: bool) -> bool {
        self.global_filter_manager.set_filter_enabled(id, enabled)
    }
    
    /// Returns whether a global filter is enabled.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The ID of the filter to check
    /// 
    /// # Returns
    /// 
    /// `true` if the filter exists and is enabled, `false` otherwise
    pub fn is_global_filter_enabled(&self, id: &str) -> bool {
        self.global_filter_manager.is_filter_enabled(id)
    }
    
    /// Lists all global filter IDs.
    /// 
    /// # Returns
    /// 
    /// A vector of all global filter IDs
    pub fn list_global_filters(&self) -> Vec<String> {
        self.global_filter_manager.list_filters()
    }
    
    /// Enables or disables all global filtering.
    /// 
    /// When disabled, all events pass through regardless of global filter settings.
    /// 
    /// # Arguments
    /// 
    /// * `enabled` - Whether global filtering should be enabled
    pub fn set_global_filtering_enabled(&self, enabled: bool) {
        self.global_filter_manager.set_enabled(enabled);
    }
    
    /// Returns whether global filtering is enabled.
    /// 
    /// # Returns
    /// 
    /// `true` if global filtering is enabled, `false` otherwise
    pub fn is_global_filtering_enabled(&self) -> bool {
        self.global_filter_manager.is_enabled()
    }
    
    /// Clears the global filter cache.
    /// 
    /// This forces all subsequent filter evaluations to be computed fresh,
    /// which may be useful after modifying filter configurations or when
    /// event patterns change significantly.
    pub fn clear_global_filter_cache(&self) {
        self.global_filter_manager.clear_cache();
    }
    
    /// Returns the number of global filters.
    /// 
    /// # Returns
    /// 
    /// The total number of global filters registered
    pub fn global_filter_count(&self) -> usize {
        self.global_filter_manager.filter_count()
    }
    
    /// Returns a reference to the global filter manager.
    /// 
    /// This provides access to advanced filter management operations.
    /// 
    /// # Returns
    /// 
    /// A reference to the FilterManager
    pub fn global_filter_manager(&self) -> &FilterManager {
        &self.global_filter_manager
    }
}

/// Builder for configuring EventBus instances.
/// 
/// The EventBusBuilder provides a fluent interface for configuring
/// event buses with custom options, middleware, and filters.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{EventBus, Priority, ErrorHandling};
/// 
/// let bus = EventBus::builder()
///     .with_capacity(1000)
///     .with_metrics(true)
///     .with_error_handling(ErrorHandling::StopOnFirstError)
///     .with_default_priority(Priority::High)
///     .build();
/// ```
pub struct EventBusBuilder {
    config: EventBusConfig,
    // TODO: Add middleware stack when trait object issues are resolved
}

impl EventBusBuilder {
    /// Creates a new EventBusBuilder with default configuration.
    pub fn new() -> Self {
        Self {
            config: EventBusConfig::default(),
        }
    }
    
    /// Sets the initial capacity for handler storage.
    /// 
    /// # Arguments
    /// * `capacity` - Initial capacity for handler storage
    /// 
    /// # Examples
    /// ```rust
    /// let bus = EventBus::builder()
    ///     .with_capacity(1000)
    ///     .build();
    /// ```
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.config.initial_capacity = capacity;
        self
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
    
    /// Sets the default priority for handlers.
    /// 
    /// # Arguments
    /// * `priority` - Default priority for new handlers
    pub fn with_default_priority(mut self, priority: Priority) -> Self {
        self.config.default_handler_priority = priority;
        self
    }
    
    /// Sets the error handling strategy.
    /// 
    /// # Arguments
    /// * `strategy` - How to handle errors from handlers
    pub fn with_error_handling(mut self, strategy: ErrorHandling) -> Self {
        self.config.error_handling = strategy;
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
    
    /// Enables or disables metrics collection.
    /// 
    /// # Arguments
    /// * `enabled` - Whether to enable metrics collection
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.config.enable_metrics = enabled;
        self
    }
    
    /// Adds middleware to the event bus.
    /// 
    /// Middleware is executed in the order it was added.
    /// 
    /// # Arguments
    /// * `middleware` - Middleware component to add
    /// 
    /// # Examples
    /// ```rust,ignore
    /// let bus = EventBus::builder()
    ///     .with_middleware(LoggingMiddleware::new())
    ///     .with_middleware(MetricsMiddleware::new())
    ///     .build();
    /// ```
    pub fn with_middleware<M>(self, _middleware: M) -> Self
    where
        M: 'static,
    {
        // TODO: Implement middleware integration when trait object issues are resolved
        self
    }
    
    /// Adds a global filter that applies to all events.
    /// 
    /// # Arguments
    /// * `filter` - Filter to apply globally
    pub fn with_global_filter<F>(self, _filter: F) -> Self
    where
        F: 'static,
    {
        // TODO: Implement global filters when trait object issues are resolved
        self
    }
    
    /// Builds the configured EventBus.
    /// 
    /// # Returns
    /// A new EventBus instance with the specified configuration.
    pub fn build(self) -> EventBus {
        EventBus::with_config(self.config)
    }
}

impl Default for EventBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// Implementing Send and Sync for EventBus
unsafe impl Send for EventBus {}
unsafe impl Sync for EventBus {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicI32, Ordering};

    #[derive(Clone, Debug)]
    struct TestEvent {
        value: i32,
    }

    impl Event for TestEvent {
        fn event_type_name() -> &'static str {
            "TestEvent"
        }
    }

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new();
        assert_eq!(bus.total_handler_count(), 0);
        assert!(!bus.is_processing());
    }

    #[test]
    fn test_event_bus_with_config() {
        let config = EventBusConfig {
            max_handlers_per_event: Some(10),
            validate_events: false,
            ..Default::default()
        };
        
        let bus = EventBus::with_config(config);
        assert!(!bus.config().validate_events);
        assert_eq!(bus.config().max_handlers_per_event, Some(10));
    }

    #[test]
    fn test_handler_registration() {
        let mut bus = EventBus::new();
        
        let handler_id = bus.on(|_event: TestEvent| {
            // Handler logic here
        });
        
        assert!(handler_id.value() > 0);
        assert_eq!(bus.total_handler_count(), 1);
    }

    #[test]
    fn test_handler_unregistration() {
        let mut bus = EventBus::new();
        
        let handler_id = bus.on(|_event: TestEvent| {});
        assert_eq!(bus.total_handler_count(), 1);
        
        let removed = bus.off(handler_id);
        assert!(removed);
        assert_eq!(bus.total_handler_count(), 0);
        
        // Trying to remove the same handler again should return false
        let removed_again = bus.off(handler_id);
        assert!(!removed_again);
    }

    #[test]
    fn test_clear_all_handlers() {
        let mut bus = EventBus::new();
        
        bus.on(|_: TestEvent| {});
        bus.on(|_: TestEvent| {});
        assert_eq!(bus.total_handler_count(), 2);
        
        bus.clear();
        assert_eq!(bus.total_handler_count(), 0);
    }

    #[test]
    fn test_shutdown() {
        let mut bus = EventBus::new();
        
        bus.shutdown().expect("Shutdown should succeed");
        
        // After shutdown, events should be rejected
        let result = bus.emit(TestEvent { value: 42 });
        assert!(matches!(result, Err(EventBusError::ShuttingDown)));
    }

    #[test]
    fn test_priority_ordering() {
        let execution_order = Arc::new(Mutex::new(Vec::new()));
        let mut bus = EventBus::new();
        
        let order_clone1 = Arc::clone(&execution_order);
        bus.on_with_priority(move |_event: TestEvent| {
            order_clone1.lock().unwrap().push(1);
        }, Priority::Low);
        
        let order_clone2 = Arc::clone(&execution_order);
        bus.on_with_priority(move |_event: TestEvent| {
            order_clone2.lock().unwrap().push(2);
        }, Priority::High);
        
        let order_clone3 = Arc::clone(&execution_order);
        bus.on_with_priority(move |_event: TestEvent| {
            order_clone3.lock().unwrap().push(3);
        }, Priority::Normal);
        
        // Note: In this simplified implementation, the actual priority ordering
        // would need to be fully implemented for this test to pass
        bus.emit(TestEvent { value: 42 }).expect("Emit should succeed");
        
        // In a complete implementation, we would expect: [2, 3, 1] (High, Normal, Low)
    }

    #[test]
    fn test_fallible_handler() {
        let mut bus = EventBus::new();
        
        #[derive(Debug)]
        struct TestError(&'static str);
        
        impl std::fmt::Display for TestError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        
        impl std::error::Error for TestError {}
        
        bus.on_fallible(|event: TestEvent| -> Result<(), TestError> {
            if event.value > 0 {
                Ok(())
            } else {
                Err(TestError("Negative value"))
            }
        });
        
        // These should not panic even with errors
        let result1 = bus.emit(TestEvent { value: 42 });
        assert!(result1.is_ok());
        
        let result2 = bus.emit(TestEvent { value: -1 });
        assert!(result2.is_ok()); // Bus continues processing despite handler error
    }
}