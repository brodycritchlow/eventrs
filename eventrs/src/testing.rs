//! Testing utilities for EventRS applications.
//!
//! This module provides mock and stub implementations of event buses and related
//! components to facilitate unit testing of event-driven applications.

use crate::error::{EventBusError, EventBusResult};
use crate::event::Event;
use crate::handler::HandlerId;
use crate::metadata::EventMetadata;
use crate::priority::Priority;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

/// A mock event bus for testing purposes.
///
/// `TestEventBus` provides a controllable environment for testing event-driven code
/// without the complexity of a full event bus implementation. It records all
/// interactions and allows assertions about event emissions and handler registrations.
///
/// # Examples
///
/// ```rust
/// use eventrs::testing::TestEventBus;
/// use eventrs::Event;
///
/// #[derive(Clone, Debug, PartialEq)]
/// struct UserLoggedIn { user_id: u64 }
/// impl Event for UserLoggedIn {}
///
/// let mut test_bus = TestEventBus::new();
///
/// // Register handler
/// test_bus.on(|event: UserLoggedIn| {
///     println!("User {} logged in", event.user_id);
/// });
///
/// // Emit event
/// test_bus.emit(UserLoggedIn { user_id: 123 }).unwrap();
///
/// // Assert event was emitted
/// assert_eq!(test_bus.emitted_events::<UserLoggedIn>().len(), 1);
/// assert_eq!(test_bus.emitted_events::<UserLoggedIn>()[0].user_id, 123);
/// ```
#[derive(Debug)]
pub struct TestEventBus {
    /// Recorded events, keyed by type ID
    emitted_events: Arc<Mutex<HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>>>,

    /// Registered handlers for testing
    handlers: RwLock<HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>>,

    /// Recorded handler registrations
    registrations: Arc<Mutex<Vec<HandlerRegistration>>>,

    /// Configuration for test behavior
    config: TestEventBusConfig,

    /// Whether the bus should simulate failures
    should_fail: Arc<Mutex<bool>>,

    /// Simulated processing delays
    processing_delay: Arc<Mutex<Option<Duration>>>,

    /// Event emission history with metadata
    emission_history: Arc<Mutex<Vec<EmissionRecord>>>,
}

/// Configuration for test event bus behavior.
#[derive(Debug, Clone)]
pub struct TestEventBusConfig {
    /// Whether to actually execute handlers
    execute_handlers: bool,

    /// Whether to record detailed emission history
    record_history: bool,

    /// Maximum number of events to keep in history
    max_history_size: usize,

    /// Whether to validate events before emission
    validate_events: bool,
}

/// Record of a handler registration for testing.
#[derive(Debug, Clone)]
pub struct HandlerRegistration {
    /// Type of event the handler was registered for
    pub event_type: TypeId,

    /// Event type name for debugging
    pub event_type_name: String,

    /// When the handler was registered
    pub registered_at: SystemTime,

    /// Handler ID
    pub handler_id: HandlerId,

    /// Priority of the handler
    pub priority: Priority,
}

/// Record of an event emission for testing.
#[derive(Debug, Clone)]
pub struct EmissionRecord {
    /// Type of event emitted
    pub event_type: TypeId,

    /// Event type name for debugging
    pub event_type_name: String,

    /// When the event was emitted
    pub emitted_at: SystemTime,

    /// Event metadata
    pub metadata: EventMetadata,

    /// Whether the emission was successful
    pub success: bool,

    /// Number of handlers that processed the event
    pub handlers_executed: usize,

    /// Processing duration
    pub processing_duration: Option<Duration>,
}

impl Default for TestEventBusConfig {
    fn default() -> Self {
        Self {
            execute_handlers: true,
            record_history: true,
            max_history_size: 1000,
            validate_events: true,
        }
    }
}

impl TestEventBus {
    /// Creates a new test event bus with default configuration.
    pub fn new() -> Self {
        Self::with_config(TestEventBusConfig::default())
    }

    /// Creates a new test event bus with custom configuration.
    pub fn with_config(config: TestEventBusConfig) -> Self {
        Self {
            emitted_events: Arc::new(Mutex::new(HashMap::new())),
            handlers: RwLock::new(HashMap::new()),
            registrations: Arc::new(Mutex::new(Vec::new())),
            config,
            should_fail: Arc::new(Mutex::new(false)),
            processing_delay: Arc::new(Mutex::new(None)),
            emission_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Creates a test event bus that doesn't execute handlers (stub mode).
    pub fn stub() -> Self {
        Self::with_config(TestEventBusConfig {
            execute_handlers: false,
            record_history: true,
            max_history_size: 1000,
            validate_events: false,
        })
    }

    /// Registers a handler for the specified event type.
    pub fn on<E, F>(&mut self, handler: F) -> HandlerId
    where
        E: Event + 'static,
        F: Fn(E) + Send + Sync + 'static,
    {
        self.on_with_priority(handler, Priority::Normal)
    }

    /// Registers a handler with a specific priority.
    pub fn on_with_priority<E, F>(&mut self, handler: F, priority: Priority) -> HandlerId
    where
        E: Event + 'static,
        F: Fn(E) + Send + Sync + 'static,
    {
        let handler_id = HandlerId::new();
        let type_id = TypeId::of::<E>();

        // Store the handler
        let mut handlers = self.handlers.write().unwrap();
        let handlers_for_type = handlers.entry(type_id).or_default();
        handlers_for_type.push(Box::new(Box::new(handler) as Box<dyn Fn(E) + Send + Sync>));
        drop(handlers);

        // Record the registration
        let registration = HandlerRegistration {
            event_type: type_id,
            event_type_name: E::event_type_name().to_string(),
            registered_at: SystemTime::now(),
            handler_id,
            priority,
        };

        self.registrations.lock().unwrap().push(registration);

        handler_id
    }

    /// Emits an event to all registered handlers.
    pub fn emit<E>(&self, event: E) -> EventBusResult<()>
    where
        E: Event + Clone + 'static,
    {
        self.emit_with_metadata(event, EventMetadata::new())
    }

    /// Emits an event with custom metadata.
    pub fn emit_with_metadata<E>(&self, event: E, mut metadata: EventMetadata) -> EventBusResult<()>
    where
        E: Event + Clone + 'static,
    {
        let start_time = SystemTime::now();
        metadata.mark_received();
        metadata.mark_processing_started();

        // Check if we should simulate failure
        if *self.should_fail.lock().unwrap() {
            return Err(EventBusError::ConfigurationError {
                message: "Simulated failure for testing".to_string(),
            });
        }

        // Simulate processing delay if configured
        if let Some(delay) = *self.processing_delay.lock().unwrap() {
            std::thread::sleep(delay);
        }

        let type_id = TypeId::of::<E>();
        let mut handlers_executed = 0;

        // Record the event
        {
            let mut events = self.emitted_events.lock().unwrap();
            let events_for_type = events.entry(type_id).or_default();
            events_for_type.push(Box::new(event.clone()));
        }

        // Execute handlers if configured
        if self.config.execute_handlers {
            let handlers = self.handlers.read().unwrap();
            if let Some(handlers_for_type) = handlers.get(&type_id) {
                for handler in handlers_for_type {
                    if let Some(handler_fn) = handler.downcast_ref::<Box<dyn Fn(E) + Send + Sync>>()
                    {
                        handler_fn(event.clone());
                        handlers_executed += 1;
                    }
                }
            }
        }

        metadata.mark_processing_completed();

        // Record emission history if configured
        if self.config.record_history {
            let processing_duration = start_time.elapsed().ok();
            let record = EmissionRecord {
                event_type: type_id,
                event_type_name: E::event_type_name().to_string(),
                emitted_at: start_time,
                metadata,
                success: true,
                handlers_executed,
                processing_duration,
            };

            let mut history = self.emission_history.lock().unwrap();
            history.push(record);

            // Limit history size
            if history.len() > self.config.max_history_size {
                history.remove(0);
            }
        }

        Ok(())
    }

    /// Returns all emitted events of the specified type.
    pub fn emitted_events<E>(&self) -> Vec<E>
    where
        E: Event + Clone + 'static,
    {
        let events = self.emitted_events.lock().unwrap();
        let type_id = TypeId::of::<E>();

        events
            .get(&type_id)
            .map(|events| {
                events
                    .iter()
                    .filter_map(|e| e.downcast_ref::<E>().cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns the number of emitted events of the specified type.
    pub fn emitted_count<E>(&self) -> usize
    where
        E: Event + 'static,
    {
        let events = self.emitted_events.lock().unwrap();
        let type_id = TypeId::of::<E>();
        events.get(&type_id).map(|v| v.len()).unwrap_or(0)
    }

    /// Returns all handler registrations.
    pub fn registrations(&self) -> Vec<HandlerRegistration> {
        self.registrations.lock().unwrap().clone()
    }

    /// Returns the number of registered handlers for the specified event type.
    pub fn handler_count<E>(&self) -> usize
    where
        E: Event + 'static,
    {
        let handlers = self.handlers.read().unwrap();
        let type_id = TypeId::of::<E>();
        handlers.get(&type_id).map(|v| v.len()).unwrap_or(0)
    }

    /// Returns the emission history.
    pub fn emission_history(&self) -> Vec<EmissionRecord> {
        self.emission_history.lock().unwrap().clone()
    }

    /// Clears all emitted events from the test bus.
    pub fn clear_events(&mut self) {
        self.emitted_events.lock().unwrap().clear();
        self.emission_history.lock().unwrap().clear();
    }

    /// Clears all registered handlers from the test bus.
    pub fn clear_handlers(&mut self) {
        self.handlers.write().unwrap().clear();
        self.registrations.lock().unwrap().clear();
    }

    /// Clears all recorded data from the test bus.
    pub fn clear_all(&mut self) {
        self.clear_events();
        self.clear_handlers();
    }

    /// Configures the test bus to simulate failures.
    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }

    /// Sets a processing delay to simulate slow operations.
    pub fn set_processing_delay(&self, delay: Option<Duration>) {
        *self.processing_delay.lock().unwrap() = delay;
    }

    /// Returns whether any events of the specified type were emitted.
    pub fn was_emitted<E>(&self) -> bool
    where
        E: Event + 'static,
    {
        self.emitted_count::<E>() > 0
    }

    /// Returns the last emitted event of the specified type.
    pub fn last_emitted<E>(&self) -> Option<E>
    where
        E: Event + Clone + 'static,
    {
        self.emitted_events::<E>().last().cloned()
    }

    /// Returns the first emitted event of the specified type.
    pub fn first_emitted<E>(&self) -> Option<E>
    where
        E: Event + Clone + 'static,
    {
        self.emitted_events::<E>().first().cloned()
    }

    /// Asserts that exactly one event of the specified type was emitted.
    pub fn assert_emitted_once<E>(&self)
    where
        E: Event + 'static,
    {
        let count = self.emitted_count::<E>();
        assert_eq!(
            count,
            1,
            "Expected exactly 1 {} event, but found {}",
            E::event_type_name(),
            count
        );
    }

    /// Asserts that the specified number of events were emitted.
    pub fn assert_emitted_count<E>(&self, expected: usize)
    where
        E: Event + 'static,
    {
        let count = self.emitted_count::<E>();
        assert_eq!(
            count,
            expected,
            "Expected {} {} events, but found {}",
            expected,
            E::event_type_name(),
            count
        );
    }

    /// Asserts that no events of the specified type were emitted.
    pub fn assert_not_emitted<E>(&self)
    where
        E: Event + 'static,
    {
        let count = self.emitted_count::<E>();
        assert_eq!(
            count,
            0,
            "Expected no {} events, but found {}",
            E::event_type_name(),
            count
        );
    }

    /// Asserts that at least one event of the specified type was emitted.
    pub fn assert_emitted<E>(&self)
    where
        E: Event + 'static,
    {
        assert!(
            self.was_emitted::<E>(),
            "Expected at least one {} event to be emitted",
            E::event_type_name()
        );
    }
}

impl Default for TestEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// A spy that records method calls for testing.
///
/// This is useful for verifying that specific methods were called with
/// expected parameters during testing.
#[derive(Debug, Clone)]
pub struct EventSpy {
    /// Recorded method calls
    calls: Arc<Mutex<Vec<SpyCall>>>,
}

/// Record of a method call captured by a spy.
#[derive(Debug, Clone)]
pub struct SpyCall {
    /// Name of the method called
    pub method_name: String,

    /// Arguments passed to the method (as strings for simplicity)
    pub arguments: Vec<String>,

    /// When the call was made
    pub called_at: SystemTime,
}

impl EventSpy {
    /// Creates a new event spy.
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Records a method call.
    pub fn record_call<S: Into<String>>(&self, method_name: S, arguments: Vec<String>) {
        let call = SpyCall {
            method_name: method_name.into(),
            arguments,
            called_at: SystemTime::now(),
        };

        self.calls.lock().unwrap().push(call);
    }

    /// Returns all recorded calls.
    pub fn calls(&self) -> Vec<SpyCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Returns the number of times a method was called.
    pub fn call_count(&self, method_name: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| call.method_name == method_name)
            .count()
    }

    /// Returns whether a method was called.
    pub fn was_called(&self, method_name: &str) -> bool {
        self.call_count(method_name) > 0
    }

    /// Clears all recorded calls.
    pub fn clear(&self) {
        self.calls.lock().unwrap().clear();
    }

    /// Asserts that a method was called exactly once.
    pub fn assert_called_once(&self, method_name: &str) {
        let count = self.call_count(method_name);
        assert_eq!(
            count, 1,
            "Expected {} to be called exactly once, but it was called {} times",
            method_name, count
        );
    }

    /// Asserts that a method was called a specific number of times.
    pub fn assert_called_times(&self, method_name: &str, expected: usize) {
        let count = self.call_count(method_name);
        assert_eq!(
            count, expected,
            "Expected {} to be called {} times, but it was called {} times",
            method_name, expected, count
        );
    }

    /// Asserts that a method was never called.
    pub fn assert_not_called(&self, method_name: &str) {
        let count = self.call_count(method_name);
        assert_eq!(
            count, 0,
            "Expected {} to never be called, but it was called {} times",
            method_name, count
        );
    }
}

impl Default for EventSpy {
    fn default() -> Self {
        Self::new()
    }
}

/// A mock handler that records when it's called.
///
/// Useful for testing handler execution without side effects.
#[derive(Debug)]
pub struct MockHandler<E: Event> {
    /// Spy for recording calls
    spy: EventSpy,

    /// Events received by this handler
    received_events: Arc<Mutex<Vec<E>>>,

    /// Whether this handler should simulate an error
    should_error: Arc<Mutex<bool>>,
}

impl<E: Event + Clone + std::fmt::Debug> MockHandler<E> {
    /// Creates a new mock handler.
    pub fn new() -> Self {
        Self {
            spy: EventSpy::new(),
            received_events: Arc::new(Mutex::new(Vec::new())),
            should_error: Arc::new(Mutex::new(false)),
        }
    }

    /// Creates a handler function for use with event buses.
    pub fn handler(&self) -> impl Fn(E) + Clone {
        let spy = self.spy.clone();
        let received = self.received_events.clone();
        let should_error = self.should_error.clone();

        move |event: E| {
            spy.record_call("handle", vec![format!("{:?}", event)]);
            received.lock().unwrap().push(event.clone());

            if *should_error.lock().unwrap() {
                panic!("Simulated handler error");
            }
        }
    }

    /// Returns all events received by this handler.
    pub fn received_events(&self) -> Vec<E> {
        self.received_events.lock().unwrap().clone()
    }

    /// Returns the number of events received.
    pub fn received_count(&self) -> usize {
        self.received_events.lock().unwrap().len()
    }

    /// Returns the spy for this handler.
    pub fn spy(&self) -> &EventSpy {
        &self.spy
    }

    /// Configures this handler to simulate errors.
    pub fn set_should_error(&self, should_error: bool) {
        *self.should_error.lock().unwrap() = should_error;
    }

    /// Clears all received events.
    pub fn clear(&self) {
        self.received_events.lock().unwrap().clear();
        self.spy.clear();
    }
}

impl<E: Event + Clone + std::fmt::Debug> Default for MockHandler<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Event;

    #[derive(Clone, Debug, PartialEq)]
    struct TestEvent {
        pub value: i32,
        pub message: String,
    }

    impl Event for TestEvent {
        fn event_type_name() -> &'static str {
            "TestEvent"
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct OtherEvent {
        pub data: String,
    }

    impl Event for OtherEvent {
        fn event_type_name() -> &'static str {
            "OtherEvent"
        }
    }

    #[test]
    fn test_test_event_bus_creation() {
        let test_bus = TestEventBus::new();
        assert_eq!(test_bus.emitted_count::<TestEvent>(), 0);
        assert_eq!(test_bus.handler_count::<TestEvent>(), 0);
    }

    #[test]
    fn test_event_emission_and_recording() {
        let test_bus = TestEventBus::new();

        let event1 = TestEvent {
            value: 42,
            message: "Hello".to_string(),
        };
        let event2 = TestEvent {
            value: 100,
            message: "World".to_string(),
        };

        test_bus.emit(event1.clone()).unwrap();
        test_bus.emit(event2.clone()).unwrap();

        assert_eq!(test_bus.emitted_count::<TestEvent>(), 2);
        assert_eq!(test_bus.emitted_events::<TestEvent>(), vec![event1, event2]);

        assert_eq!(test_bus.emitted_count::<OtherEvent>(), 0);
    }

    #[test]
    fn test_handler_registration_and_execution() {
        let mut test_bus = TestEventBus::new();
        let received_events = Arc::new(Mutex::new(Vec::new()));

        let received_clone = Arc::clone(&received_events);
        test_bus.on(move |event: TestEvent| {
            received_clone.lock().unwrap().push(event);
        });

        assert_eq!(test_bus.handler_count::<TestEvent>(), 1);
        assert_eq!(test_bus.registrations().len(), 1);

        let event = TestEvent {
            value: 123,
            message: "Test".to_string(),
        };
        test_bus.emit(event.clone()).unwrap();

        // Check that handler was executed
        assert_eq!(received_events.lock().unwrap().len(), 1);
        assert_eq!(received_events.lock().unwrap()[0], event);
    }

    #[test]
    fn test_stub_mode() {
        let mut test_bus = TestEventBus::stub();
        let received_events = Arc::new(Mutex::new(Vec::new()));

        let received_clone = Arc::clone(&received_events);
        test_bus.on(move |event: TestEvent| {
            received_clone.lock().unwrap().push(event);
        });

        let event = TestEvent {
            value: 456,
            message: "Stub".to_string(),
        };
        test_bus.emit(event.clone()).unwrap();

        // Event should be recorded but handler should not execute
        assert_eq!(test_bus.emitted_count::<TestEvent>(), 1);
        assert_eq!(received_events.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_assertion_methods() {
        let test_bus = TestEventBus::new();

        // Test not emitted assertion
        test_bus.assert_not_emitted::<TestEvent>();

        // Emit some events
        test_bus
            .emit(TestEvent {
                value: 1,
                message: "One".to_string(),
            })
            .unwrap();
        test_bus
            .emit(TestEvent {
                value: 2,
                message: "Two".to_string(),
            })
            .unwrap();

        // Test various assertions
        test_bus.assert_emitted::<TestEvent>();
        test_bus.assert_emitted_count::<TestEvent>(2);
        test_bus.assert_not_emitted::<OtherEvent>();

        // Test first/last event access
        assert_eq!(test_bus.first_emitted::<TestEvent>().unwrap().value, 1);
        assert_eq!(test_bus.last_emitted::<TestEvent>().unwrap().value, 2);
    }

    #[test]
    fn test_failure_simulation() {
        let test_bus = TestEventBus::new();

        // Normal emission should work
        assert!(test_bus
            .emit(TestEvent {
                value: 1,
                message: "Normal".to_string()
            })
            .is_ok());

        // Enable failure simulation
        test_bus.set_should_fail(true);
        assert!(test_bus
            .emit(TestEvent {
                value: 2,
                message: "Fail".to_string()
            })
            .is_err());

        // Disable failure simulation
        test_bus.set_should_fail(false);
        assert!(test_bus
            .emit(TestEvent {
                value: 3,
                message: "Normal".to_string()
            })
            .is_ok());
    }

    #[test]
    fn test_event_spy() {
        let spy = EventSpy::new();

        spy.record_call("method1", vec!["arg1".to_string(), "arg2".to_string()]);
        spy.record_call("method2", vec![]);
        spy.record_call("method1", vec!["arg3".to_string()]);

        assert_eq!(spy.call_count("method1"), 2);
        assert_eq!(spy.call_count("method2"), 1);
        assert_eq!(spy.call_count("method3"), 0);

        assert!(spy.was_called("method1"));
        assert!(spy.was_called("method2"));
        assert!(!spy.was_called("method3"));

        spy.assert_called_times("method1", 2);
        spy.assert_called_once("method2");
        spy.assert_not_called("method3");

        assert_eq!(spy.calls().len(), 3);
    }

    #[test]
    fn test_mock_handler() {
        let mock = MockHandler::<TestEvent>::new();
        let handler = mock.handler();

        let event1 = TestEvent {
            value: 10,
            message: "First".to_string(),
        };
        let event2 = TestEvent {
            value: 20,
            message: "Second".to_string(),
        };

        handler(event1.clone());
        handler(event2.clone());

        assert_eq!(mock.received_count(), 2);
        assert_eq!(mock.received_events(), vec![event1, event2]);
        assert_eq!(mock.spy().call_count("handle"), 2);
    }

    #[test]
    fn test_processing_delay() {
        let test_bus = TestEventBus::new();

        // Set a small delay
        test_bus.set_processing_delay(Some(Duration::from_millis(10)));

        let start = SystemTime::now();
        test_bus
            .emit(TestEvent {
                value: 1,
                message: "Delayed".to_string(),
            })
            .unwrap();
        let elapsed = start.elapsed().unwrap();

        // Should have taken at least the delay time
        assert!(elapsed >= Duration::from_millis(10));
    }

    #[test]
    fn test_emission_history() {
        let test_bus = TestEventBus::new();

        test_bus
            .emit(TestEvent {
                value: 1,
                message: "First".to_string(),
            })
            .unwrap();
        test_bus
            .emit(OtherEvent {
                data: "Second".to_string(),
            })
            .unwrap();

        let history = test_bus.emission_history();
        assert_eq!(history.len(), 2);

        assert_eq!(history[0].event_type_name, "TestEvent");
        assert_eq!(history[1].event_type_name, "OtherEvent");
        assert!(history[0].success);
        assert!(history[1].success);
    }

    #[test]
    fn test_clear_operations() {
        let mut test_bus = TestEventBus::new();

        // Add some data
        test_bus.on(|_: TestEvent| {});
        test_bus
            .emit(TestEvent {
                value: 1,
                message: "Test".to_string(),
            })
            .unwrap();

        assert_eq!(test_bus.emitted_count::<TestEvent>(), 1);
        assert_eq!(test_bus.handler_count::<TestEvent>(), 1);

        // Clear events only
        test_bus.clear_events();
        assert_eq!(test_bus.emitted_count::<TestEvent>(), 0);
        assert_eq!(test_bus.handler_count::<TestEvent>(), 1);

        // Add event back and clear handlers
        test_bus
            .emit(TestEvent {
                value: 2,
                message: "Test2".to_string(),
            })
            .unwrap();
        test_bus.clear_handlers();
        assert_eq!(test_bus.emitted_count::<TestEvent>(), 1);
        assert_eq!(test_bus.handler_count::<TestEvent>(), 0);

        // Clear all
        test_bus.clear_all();
        assert_eq!(test_bus.emitted_count::<TestEvent>(), 0);
        assert_eq!(test_bus.handler_count::<TestEvent>(), 0);
    }
}
