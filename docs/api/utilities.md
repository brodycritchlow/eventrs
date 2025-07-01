# Utilities API

This document covers utility functions, helper types, and convenience APIs provided by EventRS.

## Filter Utilities

### Built-in Filters

```rust
use eventrs::Filter;

/// Creates a filter that always passes.
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::accept_all();
/// assert!(filter.evaluate(&any_event));
/// ```
pub fn accept_all<E: Event>() -> impl Filter<E> {
    Filter::custom(|_| true)
}

/// Creates a filter that never passes.
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::reject_all();
/// assert!(!filter.evaluate(&any_event));
/// ```
pub fn reject_all<E: Event>() -> impl Filter<E> {
    Filter::custom(|_| false)
}

/// Creates a filter based on a field value.
/// 
/// # Arguments
/// 
/// * `field_accessor` - Function to extract the field value
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::field(|event: &UserEvent| event.user_id > 1000);
/// ```
pub fn field<E, F, T>(field_accessor: F) -> impl Filter<E>
where
    E: Event,
    F: Fn(&E) -> T + Send + Sync + 'static,
    T: PartialOrd + 'static,
{
    // Implementation details
}

/// Creates a filter that checks if a field is within a range.
/// 
/// # Arguments
/// 
/// * `field_accessor` - Function to extract the field value
/// * `range` - The range to check against
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::range(
///     |event: &MetricEvent| event.value,
///     10.0..100.0
/// );
/// ```
pub fn range<E, F, T>(field_accessor: F, range: std::ops::Range<T>) -> impl Filter<E>
where
    E: Event,
    F: Fn(&E) -> T + Send + Sync + 'static,
    T: PartialOrd + Copy + 'static,
{
    Filter::custom(move |event| {
        let value = field_accessor(event);
        range.contains(&value)
    })
}

/// Creates a filter that matches string patterns.
/// 
/// # Arguments
/// 
/// * `field_accessor` - Function to extract the string field
/// * `pattern` - Glob pattern to match
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::pattern(
///     |event: &LogEvent| &event.message,
///     "ERROR*"
/// );
/// ```
pub fn pattern<E, F>(field_accessor: F, pattern: &str) -> impl Filter<E>
where
    E: Event,
    F: Fn(&E) -> &str + Send + Sync + 'static,
{
    let pattern = glob::Pattern::new(pattern).unwrap();
    Filter::custom(move |event| {
        pattern.matches(field_accessor(event))
    })
}

/// Creates a filter that matches regular expressions.
/// 
/// # Arguments
/// 
/// * `field_accessor` - Function to extract the string field
/// * `regex` - Regular expression pattern
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::regex(
///     |event: &UserEvent| &event.action,
///     r"^admin_\w+$"
/// );
/// ```
pub fn regex<E, F>(field_accessor: F, regex: &str) -> impl Filter<E>
where
    E: Event,
    F: Fn(&E) -> &str + Send + Sync + 'static,
{
    let regex = regex::Regex::new(regex).unwrap();
    Filter::custom(move |event| {
        regex.is_match(field_accessor(event))
    })
}
```

### Filter Combinators

```rust
/// Combines multiple filters with logical AND.
/// 
/// # Arguments
/// 
/// * `filters` - Iterator of filters to combine
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::all_of([
///     Filter::field(|e: &UserEvent| e.user_id > 1000),
///     Filter::field(|e: &UserEvent| e.action == "purchase"),
/// ]);
/// ```
pub fn all_of<E, I, F>(filters: I) -> impl Filter<E>
where
    E: Event,
    I: IntoIterator<Item = F>,
    F: Filter<E>,
{
    // Implementation details
}

/// Combines multiple filters with logical OR.
/// 
/// # Arguments
/// 
/// * `filters` - Iterator of filters to combine
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::any_of([
///     Filter::field(|e: &UserEvent| e.user_id == 1),
///     Filter::field(|e: &UserEvent| e.action == "emergency"),
/// ]);
/// ```
pub fn any_of<E, I, F>(filters: I) -> impl Filter<E>
where
    E: Event,
    I: IntoIterator<Item = F>,
    F: Filter<E>,
{
    // Implementation details
}

/// Creates a filter that applies different filters based on a condition.
/// 
/// # Arguments
/// 
/// * `condition` - Function that determines which filter to use
/// * `true_filter` - Filter to use when condition is true
/// * `false_filter` - Filter to use when condition is false
/// 
/// # Examples
/// 
/// ```rust
/// let filter = Filter::conditional(
///     || is_weekend(),
///     Filter::field(|e: &UserEvent| e.action != "work"),
///     Filter::accept_all()
/// );
/// ```
pub fn conditional<E, C, TF, FF>(
    condition: C,
    true_filter: TF,
    false_filter: FF,
) -> impl Filter<E>
where
    E: Event,
    C: Fn() -> bool + Send + Sync + 'static,
    TF: Filter<E>,
    FF: Filter<E>,
{
    // Implementation details
}
```

## Event Utilities

### Event Builders

```rust
/// Utility for building events with metadata.
/// 
/// # Examples
/// 
/// ```rust
/// let event = EventBuilder::new(UserLoggedIn { user_id: 123 })
///     .with_timestamp(SystemTime::now())
///     .with_priority(Priority::High)
///     .with_source("authentication_service")
///     .build();
/// ```
pub struct EventBuilder<E: Event> {
    event: E,
    metadata: EventMetadata,
}

impl<E: Event> EventBuilder<E> {
    /// Creates a new event builder.
    pub fn new(event: E) -> Self {
        Self {
            event,
            metadata: EventMetadata::new(),
        }
    }
    
    /// Sets the timestamp for the event.
    pub fn with_timestamp(mut self, timestamp: std::time::SystemTime) -> Self {
        self.metadata = self.metadata.with_timestamp(timestamp);
        self
    }
    
    /// Sets the priority for the event.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.metadata = self.metadata.with_priority(priority);
        self
    }
    
    /// Sets the source for the event.
    pub fn with_source<S: Into<String>>(mut self, source: S) -> Self {
        self.metadata = self.metadata.with_source(source);
        self
    }
    
    /// Sets the category for the event.
    pub fn with_category<S: Into<String>>(mut self, category: S) -> Self {
        self.metadata = self.metadata.with_category(category);
        self
    }
    
    /// Adds tags to the event.
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.metadata = self.metadata.with_tags(tags);
        self
    }
    
    /// Sets a correlation ID for the event.
    pub fn with_correlation_id<S: Into<String>>(mut self, id: S) -> Self {
        self.metadata = self.metadata.with_correlation_id(id);
        self
    }
    
    /// Builds the event with the configured metadata.
    pub fn build(mut self) -> E {
        // Attach metadata to event
        self.event.set_metadata(self.metadata);
        self.event
    }
}
```

### Event Validation

```rust
/// Validates an event using a set of validation rules.
/// 
/// # Arguments
/// 
/// * `event` - The event to validate
/// * `rules` - Validation rules to apply
/// 
/// # Returns
/// 
/// Returns `Ok(())` if validation passes, or an error describing the failure.
/// 
/// # Examples
/// 
/// ```rust
/// let rules = ValidationRules::new()
///     .add_rule(|e: &UserEvent| e.user_id > 0, "User ID must be positive")
///     .add_rule(|e: &UserEvent| !e.action.is_empty(), "Action cannot be empty");
/// 
/// validate_event(&event, &rules)?;
/// ```
pub fn validate_event<E: Event>(
    event: &E,
    rules: &ValidationRules<E>,
) -> Result<(), EventValidationError> {
    rules.validate(event)
}

/// Collection of validation rules for events.
pub struct ValidationRules<E: Event> {
    rules: Vec<ValidationRule<E>>,
}

impl<E: Event> ValidationRules<E> {
    /// Creates a new empty set of validation rules.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }
    
    /// Adds a validation rule.
    /// 
    /// # Arguments
    /// 
    /// * `predicate` - Function that returns true if the event is valid
    /// * `error_message` - Error message if validation fails
    pub fn add_rule<F>(mut self, predicate: F, error_message: &'static str) -> Self
    where
        F: Fn(&E) -> bool + Send + Sync + 'static,
    {
        self.rules.push(ValidationRule {
            predicate: Box::new(predicate),
            error_message,
        });
        self
    }
    
    /// Validates an event against all rules.
    pub fn validate(&self, event: &E) -> Result<(), EventValidationError> {
        for rule in &self.rules {
            if !(rule.predicate)(event) {
                return Err(EventValidationError::Custom {
                    message: rule.error_message.to_string(),
                });
            }
        }
        Ok(())
    }
}

struct ValidationRule<E: Event> {
    predicate: Box<dyn Fn(&E) -> bool + Send + Sync>,
    error_message: &'static str,
}
```

## Handler Utilities

### Handler Decorators

```rust
/// Decorates a handler with retry logic.
/// 
/// # Arguments
/// 
/// * `handler` - The handler to decorate
/// * `max_retries` - Maximum number of retry attempts
/// * `delay` - Delay between retries
/// 
/// # Examples
/// 
/// ```rust
/// let retrying_handler = with_retry(
///     |event: UserEvent| -> Result<(), ProcessingError> {
///         risky_operation(event)?;
///         Ok(())
///     },
///     3,
///     Duration::from_millis(100)
/// );
/// 
/// bus.on_fallible::<UserEvent>(retrying_handler);
/// ```
pub fn with_retry<E, H, Err>(
    handler: H,
    max_retries: u32,
    delay: std::time::Duration,
) -> impl FallibleHandler<E, Error = Err>
where
    E: Event + Clone,
    H: FallibleHandler<E, Error = Err>,
    Err: std::error::Error + Send + Sync + 'static,
{
    move |event: E| -> Result<(), Err> {
        let mut attempts = 0;
        
        loop {
            match handler.handle(event.clone()) {
                Ok(()) => return Ok(()),
                Err(e) if attempts < max_retries => {
                    attempts += 1;
                    std::thread::sleep(delay);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// Decorates a handler with timeout protection.
/// 
/// # Arguments
/// 
/// * `handler` - The handler to decorate
/// * `timeout` - Maximum execution time
/// 
/// # Examples
/// 
/// ```rust
/// let timeout_handler = with_timeout(
///     |event: UserEvent| {
///         slow_operation(event);
///     },
///     Duration::from_secs(5)
/// );
/// 
/// bus.on::<UserEvent>(timeout_handler);
/// ```
pub fn with_timeout<E, H>(
    handler: H,
    timeout: std::time::Duration,
) -> impl Handler<E>
where
    E: Event,
    H: Handler<E> + Send + 'static,
{
    move |event: E| {
        let (sender, receiver) = std::sync::mpsc::channel();
        
        std::thread::spawn(move || {
            handler.handle(event);
            let _ = sender.send(());
        });
        
        match receiver.recv_timeout(timeout) {
            Ok(()) => {},
            Err(_) => {
                eprintln!("Handler timed out after {:?}", timeout);
            }
        }
    }
}

/// Decorates a handler with rate limiting.
/// 
/// # Arguments
/// 
/// * `handler` - The handler to decorate
/// * `max_calls` - Maximum calls per time window
/// * `window` - Time window duration
/// 
/// # Examples
/// 
/// ```rust
/// let rate_limited_handler = with_rate_limit(
///     |event: UserEvent| {
///         expensive_operation(event);
///     },
///     10,
///     Duration::from_secs(60)
/// );
/// 
/// bus.on::<UserEvent>(rate_limited_handler);
/// ```
pub fn with_rate_limit<E, H>(
    handler: H,
    max_calls: u32,
    window: std::time::Duration,
) -> impl Handler<E>
where
    E: Event,
    H: Handler<E>,
{
    use std::sync::{Arc, Mutex};
    use std::collections::VecDeque;
    use std::time::Instant;
    
    let call_times = Arc::new(Mutex::new(VecDeque::new()));
    
    move |event: E| {
        let now = Instant::now();
        let mut times = call_times.lock().unwrap();
        
        // Remove old calls outside the window
        while let Some(&front) = times.front() {
            if now.duration_since(front) > window {
                times.pop_front();
            } else {
                break;
            }
        }
        
        // Check rate limit
        if times.len() < max_calls as usize {
            times.push_back(now);
            drop(times);
            handler.handle(event);
        } else {
            eprintln!("Rate limit exceeded, dropping event");
        }
    }
}
```

### Handler Combinators

```rust
/// Chains multiple handlers to execute in sequence.
/// 
/// # Arguments
/// 
/// * `handlers` - Iterator of handlers to chain
/// 
/// # Examples
/// 
/// ```rust
/// let chained_handler = chain_handlers([
///     |event: UserEvent| validate_event(&event),
///     |event: UserEvent| process_event(&event),
///     |event: UserEvent| log_event(&event),
/// ]);
/// 
/// bus.on::<UserEvent>(chained_handler);
/// ```
pub fn chain_handlers<E, I, H>(handlers: I) -> impl Handler<E>
where
    E: Event + Clone,
    I: IntoIterator<Item = H>,
    H: Handler<E>,
{
    let handlers: Vec<H> = handlers.into_iter().collect();
    
    move |event: E| {
        for handler in &handlers {
            handler.handle(event.clone());
        }
    }
}

/// Creates a handler that only executes if a condition is met.
/// 
/// # Arguments
/// 
/// * `condition` - Function that determines whether to execute
/// * `handler` - Handler to execute conditionally
/// 
/// # Examples
/// 
/// ```rust
/// let conditional_handler = conditional_handler(
///     |event: &UserEvent| event.user_id > 1000,
///     |event: UserEvent| process_premium_user(event)
/// );
/// 
/// bus.on::<UserEvent>(conditional_handler);
/// ```
pub fn conditional_handler<E, C, H>(condition: C, handler: H) -> impl Handler<E>
where
    E: Event,
    C: Fn(&E) -> bool + Send + Sync + 'static,
    H: Handler<E>,
{
    move |event: E| {
        if condition(&event) {
            handler.handle(event);
        }
    }
}
```

## Event Bus Utilities

### Bus Factory Functions

```rust
/// Creates a pre-configured event bus for common use cases.
/// 
/// # Examples
/// 
/// ```rust
/// // High-performance bus for simple events
/// let fast_bus = create_fast_bus();
/// 
/// // Bus optimized for complex events with middleware
/// let enterprise_bus = create_enterprise_bus();
/// 
/// // Bus for development with debugging features
/// let debug_bus = create_debug_bus();
/// ```

/// Creates a high-performance event bus optimized for simple events.
pub fn create_fast_bus() -> EventBus {
    EventBus::builder()
        .with_capacity(10000)
        .with_error_handling(ErrorHandling::ContinueOnError)
        .with_metrics(false)
        .build()
}

/// Creates an enterprise-grade event bus with full features.
pub fn create_enterprise_bus() -> EventBus {
    EventBus::builder()
        .with_capacity(50000)
        .with_middleware(LoggingMiddleware::new())
        .with_middleware(MetricsMiddleware::new())
        .with_middleware(ValidationMiddleware::new())
        .with_error_handling(ErrorHandling::RetryOnError(3))
        .with_metrics(true)
        .build()
}

/// Creates a debug-enabled event bus for development.
pub fn create_debug_bus() -> EventBus {
    EventBus::builder()
        .with_capacity(1000)
        .with_middleware(DebugMiddleware::new())
        .with_middleware(LoggingMiddleware::new())
        .with_error_handling(ErrorHandling::StopOnFirstError)
        .with_metrics(true)
        .build()
}

/// Creates an async event bus optimized for I/O heavy workloads.
pub fn create_async_io_bus() -> AsyncEventBus {
    AsyncEventBus::builder()
        .with_max_concurrency(100)
        .with_backpressure(true)
        .build()
}
```

### Testing Utilities

```rust
/// Testing utilities for EventRS.
pub mod testing {
    use super::*;
    
    /// Test event bus that captures events for verification.
    pub struct TestEventBus {
        // Internal implementation
    }
    
    impl TestEventBus {
        /// Creates a new test event bus.
        pub fn new() -> Self {
            // Implementation
        }
        
        /// Captures all events of a specific type.
        /// 
        /// # Examples
        /// 
        /// ```rust
        /// let mut bus = TestEventBus::new();
        /// let captured = bus.capture_events::<UserEvent>();
        /// 
        /// bus.emit(UserEvent { user_id: 123, action: "test".to_string() });
        /// 
        /// let events = captured.get_events();
        /// assert_eq!(events.len(), 1);
        /// ```
        pub fn capture_events<E: Event>(&self) -> EventCapture<E> {
            // Implementation
        }
        
        /// Counts events of a specific type.
        pub fn count_events<E: Event>(&self) -> EventCounter<E> {
            // Implementation
        }
        
        /// Verifies that an event was emitted.
        pub fn verify_event_emitted<E: Event + PartialEq>(&self, expected: &E) -> bool {
            // Implementation
        }
        
        /// Waits for a specific number of events to be emitted.
        pub fn wait_for_events<E: Event>(&self, count: usize, timeout: Duration) -> bool {
            // Implementation
        }
    }
    
    /// Captures events for testing.
    pub struct EventCapture<E: Event> {
        events: Arc<Mutex<Vec<E>>>,
    }
    
    impl<E: Event> EventCapture<E> {
        /// Gets all captured events.
        pub fn get_events(&self) -> Vec<E> {
            self.events.lock().unwrap().clone()
        }
        
        /// Gets the last captured event.
        pub fn last_event(&self) -> Option<E> {
            self.events.lock().unwrap().last().cloned()
        }
        
        /// Clears all captured events.
        pub fn clear(&self) {
            self.events.lock().unwrap().clear();
        }
        
        /// Returns the number of captured events.
        pub fn len(&self) -> usize {
            self.events.lock().unwrap().len()
        }
    }
    
    /// Counts events for testing.
    pub struct EventCounter<E: Event> {
        count: Arc<AtomicUsize>,
        _phantom: PhantomData<E>,
    }
    
    impl<E: Event> EventCounter<E> {
        /// Gets the current count.
        pub fn get_count(&self) -> usize {
            self.count.load(Ordering::SeqCst)
        }
        
        /// Resets the counter to zero.
        pub fn reset(&self) {
            self.count.store(0, Ordering::SeqCst);
        }
        
        /// Waits for the count to reach a specific value.
        pub fn wait_for_count(&self, expected: usize, timeout: Duration) -> bool {
            let start = Instant::now();
            while start.elapsed() < timeout {
                if self.get_count() >= expected {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            false
        }
    }
    
    /// Mock handler for testing.
    pub struct MockHandler<E: Event> {
        call_count: Arc<AtomicUsize>,
        events: Arc<Mutex<Vec<E>>>,
    }
    
    impl<E: Event> MockHandler<E> {
        pub fn new() -> Self {
            Self {
                call_count: Arc::new(AtomicUsize::new(0)),
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }
        
        pub fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
        
        pub fn last_event(&self) -> Option<E> {
            self.events.lock().unwrap().last().cloned()
        }
        
        pub fn all_events(&self) -> Vec<E> {
            self.events.lock().unwrap().clone()
        }
    }
    
    impl<E: Event> Handler<E> for MockHandler<E> {
        type Output = ();
        
        fn handle(&self, event: E) -> Self::Output {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            self.events.lock().unwrap().push(event);
        }
    }
}
```

## Performance Utilities

### Benchmarking

```rust
/// Benchmarking utilities for EventRS performance testing.
pub mod bench {
    use super::*;
    use std::time::{Duration, Instant};
    
    /// Benchmarks event emission performance.
    /// 
    /// # Arguments
    /// 
    /// * `bus` - The event bus to benchmark
    /// * `event` - The event to emit repeatedly
    /// * `iterations` - Number of iterations to run
    /// 
    /// # Returns
    /// 
    /// Returns performance metrics.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let mut bus = EventBus::new();
    /// bus.on::<UserEvent>(|_| {});
    /// 
    /// let metrics = bench::emission_benchmark(
    ///     &mut bus,
    ///     UserEvent { user_id: 123, action: "test".to_string() },
    ///     10000
    /// );
    /// 
    /// println!("Average latency: {:?}", metrics.average_latency);
    /// println!("Throughput: {} events/sec", metrics.throughput);
    /// ```
    pub fn emission_benchmark<E: Event + Clone>(
        bus: &mut EventBus,
        event: E,
        iterations: usize,
    ) -> BenchmarkMetrics {
        let start = Instant::now();
        
        for _ in 0..iterations {
            bus.emit(event.clone());
        }
        
        let total_duration = start.elapsed();
        
        BenchmarkMetrics {
            total_duration,
            iterations,
            average_latency: total_duration / iterations as u32,
            throughput: (iterations as f64 / total_duration.as_secs_f64()) as u64,
        }
    }
    
    /// Results of a benchmark run.
    #[derive(Debug)]
    pub struct BenchmarkMetrics {
        pub total_duration: Duration,
        pub iterations: usize,
        pub average_latency: Duration,
        pub throughput: u64,
    }
}
```

These utility functions and types provide convenient ways to work with EventRS, from building and validating events to testing and benchmarking event bus performance. They encapsulate common patterns and provide reusable components for building robust event-driven systems.