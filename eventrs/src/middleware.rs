//! Middleware system for EventRS.
//!
//! This module provides middleware functionality for intercepting and
//! processing events before they reach handlers. Middleware components can
//! transform events, perform logging, validation, metrics collection, and more.
//!
//! # Examples
//!
//! ```rust
//! use eventrs::{Event, Middleware, MiddlewareContext, MiddlewareResult};
//!
//! struct LoggingMiddleware;
//!
//! impl<E: Event> Middleware<E> for LoggingMiddleware {
//!     fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
//!         println!("Processing event: {:?}", event);
//!         context.next(event) // Continue to next middleware or handlers
//!     }
//! }
//! ```

use crate::event::Event;
use crate::error::MiddlewareResult;
use std::collections::HashMap;
use std::any::Any;
use std::time::{Duration, Instant};

/// Trait for middleware components that can intercept and process events.
/// 
/// Middleware components are executed in the order they were registered,
/// forming a chain of responsibility pattern. Each middleware can:
/// - Inspect and modify the event
/// - Add metadata to the context
/// - Short-circuit the chain by not calling `context.next()`
/// - Perform cross-cutting concerns like logging, metrics, validation
pub trait Middleware<E: Event>: Send + Sync + 'static {
    /// Handles the event and middleware chain.
    /// 
    /// # Arguments
    /// * `event` - The event being processed
    /// * `context` - Middleware context for continuing the chain
    /// 
    /// # Returns
    /// Returns `Ok(())` if processing should continue, or an error to halt processing.
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult;
    
    /// Returns the name of this middleware for debugging and logging.
    fn middleware_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Called when the middleware is first registered.
    /// Override this to perform initialization.
    fn on_register(&self) -> MiddlewareResult {
        Ok(())
    }
    
    /// Called when the middleware is unregistered or the bus is shut down.
    /// Override this to perform cleanup.
    fn on_unregister(&self) -> MiddlewareResult {
        Ok(())
    }
}

/// Context passed to middleware for chain continuation and metadata sharing.
/// 
/// The MiddlewareContext provides:
/// - Chain continuation via `next()`
/// - Metadata storage for cross-middleware communication
/// - Execution timing and performance metrics
/// - Short-circuiting capabilities
pub struct MiddlewareContext<E: Event> {
    /// Current position in the middleware chain
    current_index: usize,
    /// Total number of middleware in the chain
    chain_length: usize,
    /// Metadata storage for cross-middleware communication
    metadata: HashMap<String, Box<dyn Any + Send + Sync>>,
    /// Execution start time for performance tracking
    start_time: Instant,
    /// Whether the chain has been short-circuited
    short_circuited: bool,
    /// Execution metrics for each middleware
    execution_metrics: Vec<MiddlewareMetrics>,
    /// Function to call the next middleware in the chain
    next_fn: Option<Box<dyn FnOnce(&E) -> MiddlewareResult + Send>>,
}

/// Metrics collected for middleware execution
#[derive(Debug, Clone)]
pub struct MiddlewareMetrics {
    /// Name of the middleware
    pub middleware_name: String,
    /// Time taken to execute this middleware
    pub execution_time: Duration,
    /// Whether this middleware called next()
    pub called_next: bool,
    /// Any error that occurred during execution
    pub error: Option<String>,
}

impl<E: Event> MiddlewareContext<E> {
    /// Creates a new middleware context.
    pub fn new(chain_length: usize) -> Self {
        Self {
            current_index: 0,
            chain_length,
            metadata: HashMap::new(),
            start_time: Instant::now(),
            short_circuited: false,
            execution_metrics: Vec::with_capacity(chain_length),
            next_fn: None,
        }
    }
    
    /// Sets the next function to be called when `next()` is invoked.
    pub fn set_next_fn<F>(&mut self, next_fn: F) 
    where 
        F: FnOnce(&E) -> MiddlewareResult + Send + 'static
    {
        self.next_fn = Some(Box::new(next_fn));
    }
    
    /// Continues processing the event through the next middleware or handlers.
    /// 
    /// This method should be called by middleware that want to continue the chain.
    /// If not called, the chain will be short-circuited.
    /// 
    /// # Arguments
    /// * `event` - The event to continue processing
    /// 
    /// # Returns
    /// Returns the result of the remaining middleware chain.
    pub fn next(&mut self, event: &E) -> MiddlewareResult {
        if self.short_circuited {
            return Ok(()); // Already short-circuited
        }
        
        if let Some(next_fn) = self.next_fn.take() {
            next_fn(event)
        } else {
            // End of chain - processing complete
            Ok(())
        }
    }
    
    /// Stores metadata that can be shared between middleware components.
    /// 
    /// # Arguments
    /// * `key` - The metadata key
    /// * `value` - The metadata value
    pub fn set_metadata<T: Any + Send + Sync>(&mut self, key: String, value: T) {
        self.metadata.insert(key, Box::new(value));
    }
    
    /// Retrieves metadata stored by previous middleware.
    /// 
    /// # Arguments
    /// * `key` - The metadata key
    /// 
    /// # Returns
    /// Returns a reference to the metadata value if found and of the correct type.
    pub fn get_metadata<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.metadata.get(key)?
            .downcast_ref::<T>()
    }
    
    /// Short-circuits the middleware chain, preventing further processing.
    /// 
    /// This can be used by middleware that want to halt processing,
    /// such as validation middleware that detects invalid events.
    pub fn short_circuit(&mut self) {
        self.short_circuited = true;
    }
    
    /// Returns whether the chain has been short-circuited.
    pub fn is_short_circuited(&self) -> bool {
        self.short_circuited
    }
    
    /// Returns the total time elapsed since context creation.
    pub fn total_elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Returns execution metrics for all middleware that have been executed.
    pub fn execution_metrics(&self) -> &[MiddlewareMetrics] {
        &self.execution_metrics
    }
    
    /// Returns the current position in the middleware chain.
    pub fn current_position(&self) -> usize {
        self.current_index
    }
    
    /// Returns the total number of middleware in the chain.
    pub fn chain_length(&self) -> usize {
        self.chain_length
    }
    
    /// Records metrics for middleware execution.
    pub fn record_metrics(&mut self, metrics: MiddlewareMetrics) {
        self.execution_metrics.push(metrics);
    }
}

/// A middleware chain executor that manages the execution of multiple middleware components.
/// 
/// The MiddlewareChain provides a way to compose multiple middleware components
/// and execute them in sequence for each event.
pub struct MiddlewareChain<E: Event> {
    middleware: Vec<Box<dyn Middleware<E>>>,
}

impl<E: Event> MiddlewareChain<E> {
    /// Creates a new empty middleware chain.
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }
    
    /// Adds middleware to the end of the chain.
    /// 
    /// # Arguments
    /// * `middleware` - The middleware component to add
    pub fn add<M: Middleware<E>>(&mut self, middleware: M) -> MiddlewareResult {
        middleware.on_register()?;
        self.middleware.push(Box::new(middleware));
        Ok(())
    }
    
    /// Executes the middleware chain for the given event.
    /// 
    /// # Arguments
    /// * `event` - The event to process through the chain
    /// 
    /// # Returns
    /// Returns the result of the middleware chain execution.
    pub fn execute(&self, event: &E) -> MiddlewareResult {
        if self.middleware.is_empty() {
            return Ok(());
        }
        
        self.execute_recursive(event, 0)
    }
    
    /// Recursively executes middleware in the chain.
    fn execute_recursive(&self, event: &E, index: usize) -> MiddlewareResult {
        if index >= self.middleware.len() {
            return Ok(());
        }
        
        let middleware = &self.middleware[index];
        let middleware_start = Instant::now();
        let middleware_name = middleware.middleware_name().to_string();
        
        let mut context = MiddlewareContext::new(self.middleware.len());
        context.current_index = index;
        
        // Set up the next function for recursive call
        let next_index = index + 1;
        if next_index < self.middleware.len() {
            context.set_next_fn(move |_event: &E| {
                // This would need to be handled differently in a real implementation
                // due to borrowing issues. For now, this is a conceptual structure.
                Ok(())
            });
        }
        
        let result = middleware.handle(event, &mut context);
        
        // Record metrics (not used yet, but structure for future implementation)
        let execution_time = middleware_start.elapsed();
        let _metrics = MiddlewareMetrics {
            middleware_name,
            execution_time,
            called_next: !context.is_short_circuited(),
            error: result.as_ref().err().map(|e| e.to_string()),
        };
        
        result?;
        
        if context.is_short_circuited() {
            return Ok(());
        }
        
        // Continue to next middleware
        self.execute_recursive(event, index + 1)
    }
    
    /// Returns the number of middleware components in the chain.
    pub fn len(&self) -> usize {
        self.middleware.len()
    }
    
    /// Returns whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.middleware.is_empty()
    }
    
    /// Clears all middleware from the chain.
    pub fn clear(&mut self) {
        for middleware in &self.middleware {
            let _ = middleware.on_unregister();
        }
        self.middleware.clear();
    }
}

impl<E: Event> Default for MiddlewareChain<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Event> Drop for MiddlewareChain<E> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Built-in logging middleware that logs event processing.
pub struct LoggingMiddleware {
    prefix: String,
}

impl LoggingMiddleware {
    /// Creates a new logging middleware with the given prefix.
    pub fn new(prefix: String) -> Self {
        Self { prefix }
    }
    
    /// Creates a new logging middleware with a default prefix.
    pub fn default() -> Self {
        Self::new("EventRS".to_string())
    }
}

impl<E: Event> Middleware<E> for LoggingMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        println!("{}: Processing event {}", self.prefix, E::event_type_name());
        context.next(event)
    }
    
    fn middleware_name(&self) -> &'static str {
        "LoggingMiddleware"
    }
}

/// Built-in validation middleware that validates events before processing.
pub struct ValidationMiddleware;

impl ValidationMiddleware {
    /// Creates a new validation middleware.
    pub fn new() -> Self {
        Self
    }
}

impl<E: Event> Middleware<E> for ValidationMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        // Validate the event
        event.validate()?;
        context.next(event)
    }
    
    fn middleware_name(&self) -> &'static str {
        "ValidationMiddleware"
    }
}

impl Default for ValidationMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in metrics middleware that collects performance metrics.
pub struct MetricsMiddleware {
    name: String,
}

impl MetricsMiddleware {
    /// Creates a new metrics middleware with the given name.
    pub fn new(name: String) -> Self {
        Self { name }
    }
    
    /// Creates a new metrics middleware with a default name.
    pub fn default() -> Self {
        Self::new("default".to_string())
    }
}

impl<E: Event> Middleware<E> for MetricsMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        let start_time = Instant::now();
        context.set_metadata("metrics_start".to_string(), start_time);
        
        let result = context.next(event);
        
        let end_time = Instant::now();
        let duration = end_time.duration_since(start_time);
        
        println!("{}: Event {} processed in {:?}", 
                 self.name, E::event_type_name(), duration);
        
        result
    }
    
    fn middleware_name(&self) -> &'static str {
        "MetricsMiddleware"
    }
}