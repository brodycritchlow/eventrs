//! Event handler system for EventRS.
//!
//! This module provides the core handler traits and types for processing events.
//! Handlers define how events are processed when they are emitted to the event bus.

use crate::event::Event;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Unique identifier for event handlers.
///
/// Handler IDs are used to register and unregister handlers from the event bus.
/// Each registered handler receives a unique ID that can be used to remove it later.
///
/// # Examples
///
/// ```rust
/// use eventrs::{EventBus, HandlerId};
///
/// let mut bus = EventBus::new();
/// #[derive(Clone, Debug)]
/// struct TestEvent { value: i32 }
/// impl eventrs::Event for TestEvent {}
///
/// let handler_id: HandlerId = bus.on(|event: TestEvent| {
///     println!("Handling event: {:?}", event);
/// });
///
/// // Later, remove the handler
/// bus.off(handler_id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerId(u64);

impl HandlerId {
    /// Creates a new unique handler ID.
    pub(crate) fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns the numeric value of this handler ID.
    pub fn value(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for HandlerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HandlerId({})", self.0)
    }
}

/// Trait for synchronous event handlers.
///
/// Handlers process events when they are emitted to the event bus.
/// This trait is automatically implemented for closures and function pointers
/// that accept an event and return nothing or a Result.
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
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle
    ///
    /// # Returns
    ///
    /// Returns the handler's output, which can be any type.
    fn handle(&self, event: E) -> Self::Output;

    /// Returns the name of this handler for debugging and logging.
    ///
    /// The default implementation uses the type name.
    fn handler_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Returns whether this handler can run concurrently with other handlers.
    ///
    /// Handlers that return `true` (the default) can be executed in parallel
    /// with other concurrent-safe handlers. Handlers that return `false` will
    /// be executed sequentially.
    fn is_concurrent_safe(&self) -> bool {
        true
    }

    /// Returns the estimated execution time for this handler.
    ///
    /// This information is used for performance optimization and scheduling.
    /// Handlers with longer execution times may be scheduled differently
    /// to minimize impact on overall system performance.
    fn estimated_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns the maximum allowed execution time for this handler.
    ///
    /// If a handler takes longer than this time to execute, it may be
    /// cancelled or flagged as slow. Returning `None` means no timeout.
    fn max_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns whether this handler should be retried on failure.
    ///
    /// Only applicable to fallible handlers that return `Result` types.
    fn should_retry_on_failure(&self) -> bool {
        false
    }

    /// Returns the maximum number of retry attempts for this handler.
    ///
    /// Only used if `should_retry_on_failure()` returns `true`.
    fn max_retry_attempts(&self) -> usize {
        3
    }
}

/// Synchronous handler implementation for closures that return nothing.
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

/// Trait for fallible synchronous handlers that can return errors.
///
/// This trait allows handlers to signal failure conditions that can be
/// handled by the event bus (e.g., retries, error logging, etc.).
///
/// # Examples
///
/// ```rust
/// use eventrs::{FallibleHandler, Event};
/// use std::io;
///
/// #[derive(Clone)]
/// struct FileEvent { path: String }
/// impl Event for FileEvent {}
///
/// let handler = |event: FileEvent| -> Result<(), io::Error> {
///     std::fs::write(&event.path, b"content")?;
///     Ok(())
/// };
/// ```
pub trait FallibleHandler<E: Event>: Send + Sync + 'static {
    /// The error type returned by this handler.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Handles the event and returns a result.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was handled successfully,
    /// or an error if handling failed.
    fn handle(&self, event: E) -> Result<(), Self::Error>;

    /// Returns the name of this handler for debugging and logging.
    fn handler_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Returns whether this handler can run concurrently with other handlers.
    fn is_concurrent_safe(&self) -> bool {
        true
    }

    /// Returns the estimated execution time for this handler.
    fn estimated_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns the maximum allowed execution time for this handler.
    fn max_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns whether this handler should be retried on failure.
    fn should_retry_on_failure(&self) -> bool {
        true
    }

    /// Returns the maximum number of retry attempts for this handler.
    fn max_retry_attempts(&self) -> usize {
        3
    }
}

/// Fallible handler implementation for closures that return Results.
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

/// Trait for asynchronous event handlers.
///
/// Async handlers allow for non-blocking event processing, which is essential
/// for I/O-bound operations or when integrating with async frameworks.
///
/// # Examples
///
/// ```rust
/// use eventrs::{AsyncHandler, Event};
///
/// #[derive(Clone)]
/// struct AsyncEvent { data: Vec<u8> }
/// impl Event for AsyncEvent {}
///
/// let handler = |event: AsyncEvent| async move {
///     // Async processing
///     tokio::time::sleep(std::time::Duration::from_millis(100)).await;
///     println!("Processed {} bytes", event.data.len());
/// };
/// ```
#[cfg(feature = "async")]
pub trait AsyncHandler<E: Event>: Send + Sync + 'static {
    /// The output type of the future returned by this handler.
    type Output: Send + 'static;

    /// The future type returned by this handler.
    type Future: Future<Output = Self::Output> + Send + 'static;

    /// Handles the event asynchronously.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle
    ///
    /// # Returns
    ///
    /// Returns a future that will resolve to the handler's output.
    fn handle(&self, event: E) -> Self::Future;

    /// Returns the name of this handler for debugging and logging.
    fn handler_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Returns whether this handler can run concurrently with other handlers.
    fn is_concurrent_safe(&self) -> bool {
        true
    }

    /// Returns the estimated execution time for this handler.
    fn estimated_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns the maximum allowed execution time for this handler.
    fn max_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns whether this handler should be retried on failure.
    fn should_retry_on_failure(&self) -> bool {
        false
    }

    /// Returns the maximum number of retry attempts for this handler.
    fn max_retry_attempts(&self) -> usize {
        3
    }
}

/// Async handler implementation for closures that return futures.
#[cfg(feature = "async")]
impl<F, E, Fut> AsyncHandler<E> for F
where
    F: Fn(E) -> Fut + Send + Sync + 'static,
    E: Event,
    Fut: Future<Output = ()> + Send + 'static,
{
    type Output = ();
    type Future = Fut;

    fn handle(&self, event: E) -> Self::Future {
        self(event)
    }
}

/// Trait for fallible asynchronous handlers that can return errors.
///
/// This combines async handling with error handling capabilities.
///
/// # Examples
///
/// ```rust
/// use eventrs::{FallibleAsyncHandler, Event};
/// use std::io;
///
/// #[derive(Clone)]
/// struct AsyncFileEvent { path: String, content: Vec<u8> }
/// impl Event for AsyncFileEvent {}
///
/// async fn example() -> Result<(), io::Error> {
///     let handler = |event: AsyncFileEvent| async move {
///         // tokio::fs::write(&event.path, &event.content).await?;
///         Ok::<(), io::Error>(())
///     };
///     Ok(())
/// }
/// ```
#[cfg(feature = "async")]
pub trait FallibleAsyncHandler<E: Event>: Send + Sync + 'static {
    /// The error type returned by this handler.
    type Error: std::error::Error + Send + Sync + 'static;

    /// The future type returned by this handler.
    type Future: Future<Output = Result<(), Self::Error>> + Send + 'static;

    /// Handles the event asynchronously and returns a result.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to handle
    ///
    /// # Returns
    ///
    /// Returns a future that will resolve to either success or an error.
    fn handle(&self, event: E) -> Self::Future;

    /// Returns the name of this handler for debugging and logging.
    fn handler_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Returns whether this handler can run concurrently with other handlers.
    fn is_concurrent_safe(&self) -> bool {
        true
    }

    /// Returns the estimated execution time for this handler.
    fn estimated_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns the maximum allowed execution time for this handler.
    fn max_execution_time(&self) -> Option<Duration> {
        None
    }

    /// Returns whether this handler should be retried on failure.
    fn should_retry_on_failure(&self) -> bool {
        true
    }

    /// Returns the maximum number of retry attempts for this handler.
    fn max_retry_attempts(&self) -> usize {
        3
    }
}

/// Fallible async handler implementation for closures that return Result futures.
#[cfg(feature = "async")]
impl<F, E, Fut, Err> FallibleAsyncHandler<E> for F
where
    F: Fn(E) -> Fut + Send + Sync + 'static,
    E: Event,
    Fut: Future<Output = Result<(), Err>> + Send + 'static,
    Err: std::error::Error + Send + Sync + 'static,
{
    type Error = Err;
    type Future = Fut;

    fn handle(&self, event: E) -> Self::Future {
        self(event)
    }
}

/// Internal handler wrapper that erases the handler type for storage.
///
/// This allows the event bus to store handlers of different types and
/// signatures in the same collection while maintaining type safety.
pub(crate) trait BoxedHandler<E: Event>: Send + Sync {
    /// Executes the handler with the given event.
    fn call(&self, event: E);

    /// Returns the handler's name for debugging.
    fn name(&self) -> &'static str;

    /// Returns whether this handler is concurrent-safe.
    fn is_concurrent_safe(&self) -> bool;

    /// Returns the estimated execution time.
    fn estimated_execution_time(&self) -> Option<Duration>;

    /// Returns the maximum execution time.
    fn max_execution_time(&self) -> Option<Duration>;
}

/// Implementation of BoxedHandler for sync handlers.
pub(crate) struct SyncBoxedHandler<H> {
    handler: H,
}

impl<H> SyncBoxedHandler<H> {
    pub(crate) fn new(handler: H) -> Self {
        Self { handler }
    }
}

impl<E, H> BoxedHandler<E> for SyncBoxedHandler<H>
where
    E: Event,
    H: Handler<E>,
{
    fn call(&self, event: E) {
        let _ = self.handler.handle(event);
    }

    fn name(&self) -> &'static str {
        self.handler.handler_name()
    }

    fn is_concurrent_safe(&self) -> bool {
        self.handler.is_concurrent_safe()
    }

    fn estimated_execution_time(&self) -> Option<Duration> {
        self.handler.estimated_execution_time()
    }

    fn max_execution_time(&self) -> Option<Duration> {
        self.handler.max_execution_time()
    }
}

/// Implementation of BoxedHandler for fallible sync handlers.
pub(crate) struct FallibleSyncBoxedHandler<H> {
    handler: H,
}

impl<H> FallibleSyncBoxedHandler<H> {
    pub(crate) fn new(handler: H) -> Self {
        Self { handler }
    }
}

impl<E, H> BoxedHandler<E> for FallibleSyncBoxedHandler<H>
where
    E: Event,
    H: FallibleHandler<E>,
{
    fn call(&self, event: E) {
        if let Err(error) = self.handler.handle(event) {
            // In a real implementation, this would be logged or handled
            // according to the event bus configuration
            eprintln!("Handler {} failed: {}", self.name(), error);
        }
    }

    fn name(&self) -> &'static str {
        self.handler.handler_name()
    }

    fn is_concurrent_safe(&self) -> bool {
        self.handler.is_concurrent_safe()
    }

    fn estimated_execution_time(&self) -> Option<Duration> {
        self.handler.estimated_execution_time()
    }

    fn max_execution_time(&self) -> Option<Duration> {
        self.handler.max_execution_time()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;
    use std::sync::{Arc, Mutex};

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
    fn test_handler_id_generation() {
        let id1 = HandlerId::new();
        let id2 = HandlerId::new();

        assert_ne!(id1, id2);
        assert!(id2.value() > id1.value());
    }

    #[test]
    fn test_handler_id_display() {
        let id = HandlerId::new();
        let display = format!("{}", id);
        assert!(display.starts_with("HandlerId("));
        assert!(display.ends_with(")"));
    }

    #[test]
    fn test_sync_handler() {
        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);

        let handler = move |event: TestEvent| {
            *result_clone.lock().unwrap() = Some(event.value);
        };

        let event = TestEvent { value: 42 };
        handler.handle(event);

        assert_eq!(*result.lock().unwrap(), Some(42));
    }

    #[derive(Debug)]
    struct TestError(&'static str);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn test_fallible_handler() {
        let handler = |event: TestEvent| -> Result<(), TestError> {
            if event.value > 0 {
                Ok(())
            } else {
                Err(TestError("Negative value"))
            }
        };

        assert!(handler.handle(TestEvent { value: 42 }).is_ok());
        assert!(handler.handle(TestEvent { value: -1 }).is_err());
    }

    #[test]
    fn test_handler_metadata() {
        let handler = |_event: TestEvent| {};

        // Test default implementations
        assert!(handler.is_concurrent_safe());
        assert!(handler.estimated_execution_time().is_none());
        assert!(handler.max_execution_time().is_none());
        assert!(!handler.should_retry_on_failure());
        assert_eq!(handler.max_retry_attempts(), 3);
    }

    #[test]
    fn test_boxed_handler() {
        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);

        let handler = move |event: TestEvent| {
            *result_clone.lock().unwrap() = Some(event.value);
        };

        let boxed = SyncBoxedHandler::new(handler);
        let event = TestEvent { value: 123 };

        boxed.call(event);
        assert_eq!(*result.lock().unwrap(), Some(123));

        assert!(boxed.is_concurrent_safe());
        assert!(boxed.estimated_execution_time().is_none());
    }

    #[test]
    fn test_fallible_boxed_handler() {
        let handler = |event: TestEvent| -> Result<(), TestError> {
            if event.value > 0 {
                Ok(())
            } else {
                Err(TestError("Negative value"))
            }
        };

        let boxed = FallibleSyncBoxedHandler::new(handler);

        // This should not panic even though the handler returns an error
        boxed.call(TestEvent { value: -1 });
        boxed.call(TestEvent { value: 42 });
    }

    // TODO: Implement async handler tests once AsyncEventBus is complete
    // #[cfg(feature = "async")]
    // #[tokio::test]
    // async fn test_async_handler() {
    //     // Async handler tests will be implemented when AsyncEventBus is complete
    // }
}
