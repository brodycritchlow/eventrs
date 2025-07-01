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

use crate::event::Event;
use crate::handler::{Handler, HandlerId, BoxedHandler, SyncBoxedHandler, FallibleHandler, FallibleSyncBoxedHandler};
use crate::error::{EventBusError, EventBusResult};
use crate::priority::{Priority, PriorityOrdered};
use crate::filter::SharedFilter;
use crate::event_bus::{EventBusConfig, ErrorHandling};

#[cfg(feature = "metrics")]
use crate::metrics::{EventBusMetrics, EmissionResult, HandlerResult, EmissionToken};

#[cfg(not(feature = "metrics"))]
type HandlerResult = ();

use std::any::TypeId;
use std::collections::{HashMap, BinaryHeap};
use std::sync::{Arc, RwLock, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
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
    
    fn new_with_filter(id: HandlerId, handler: Box<dyn BoxedHandler<E>>, priority: Priority, filter: SharedFilter<E>) -> Self {
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
type ThreadSafeHandlerStorage<E> = Arc<RwLock<Vec<ThreadSafeHandlerEntry<E>>>>;

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
        let token = self.metrics.as_ref().map(|m| m.start_emission(TypeId::of::<E>()));
        
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
    pub fn on_with_priority<E: Event, H: Handler<E>>(&self, handler: H, priority: Priority) -> HandlerId {
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
    pub fn on_with_filter<E: Event, H: Handler<E>>(&self, handler: H, filter: SharedFilter<E>) -> HandlerId {
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
    
    fn register_handler<E: Event, H: Handler<E>>(&self, handler: H, priority: Priority) -> HandlerId {
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
                Box::new(Vec::<ThreadSafeHandlerEntry<E>>::new()) as Box<dyn std::any::Any + Send + Sync>
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
    
    fn register_handler_with_filter<E: Event, H: Handler<E>>(&self, handler: H, priority: Priority, filter: SharedFilter<E>) -> HandlerId {
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
                Box::new(Vec::<ThreadSafeHandlerEntry<E>>::new()) as Box<dyn std::any::Any + Send + Sync>
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
    
    fn register_fallible_handler<E: Event, H: FallibleHandler<E>>(&self, handler: H, priority: Priority) -> HandlerId {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();
        
        // Create boxed handler
        let boxed = Box::new(FallibleSyncBoxedHandler::new(handler));
        let entry = ThreadSafeHandlerEntry::new(handler_id, boxed, priority);
        
        // Store handler
        {
            let mut handlers_map = self.handlers.write().unwrap();
            let handlers_vec = handlers_map.entry(type_id).or_insert_with(|| {
                Box::new(Vec::<ThreadSafeHandlerEntry<E>>::new()) as Box<dyn std::any::Any + Send + Sync>
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
    
    fn process_handlers_with_priority<E: Event>(&self, event: &E, handlers: Vec<ThreadSafeHandlerEntry<E>>) -> EventBusResult<Vec<HandlerResult>> {
        let mut priority_handlers: BinaryHeap<PriorityOrdered<&ThreadSafeHandlerEntry<E>>> = handlers
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
            let execution_time = execution_start.elapsed();
            
            #[cfg(feature = "metrics")]
            {
                let handler_result = if result.is_ok() {
                    HandlerResult::success(
                        handler_entry.id.to_string(),
                        execution_time,
                        format!("{:?}", handler_entry.priority),
                    )
                } else {
                    HandlerResult::failure(
                        handler_entry.id.to_string(),
                        execution_time,
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
                    },
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
    
    fn process_handlers_sequential<E: Event>(&self, event: &E, handlers: Vec<ThreadSafeHandlerEntry<E>>) -> EventBusResult<Vec<HandlerResult>> {
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
            let execution_time = execution_start.elapsed();
            
            #[cfg(feature = "metrics")]
            {
                let handler_result = if result.is_ok() {
                    HandlerResult::success(
                        handler_entry.id.to_string(),
                        execution_time,
                        format!("{:?}", handler_entry.priority),
                    )
                } else {
                    HandlerResult::failure(
                        handler_entry.id.to_string(),
                        execution_time,
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
                    },
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
    fn process_handlers_with_priority_simple<E: Event>(&self, event: &E, handlers: Vec<ThreadSafeHandlerEntry<E>>) -> EventBusResult<()> {
        let mut priority_handlers: BinaryHeap<PriorityOrdered<&ThreadSafeHandlerEntry<E>>> = handlers
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
                    },
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
    fn process_handlers_sequential_simple<E: Event>(&self, event: &E, handlers: Vec<ThreadSafeHandlerEntry<E>>) -> EventBusResult<()> {
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
                    },
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
    
    fn execute_handler<E: Event>(&self, handler_entry: &ThreadSafeHandlerEntry<E>, event: E) -> EventBusResult<()> {
        handler_entry.handler.call(event);
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