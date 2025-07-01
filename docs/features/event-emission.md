# Event Emission

Event emission is the process of triggering events in EventRS. This document covers the various ways to emit events, from simple synchronous emissions to complex batch operations.

## Basic Event Emission

### Simple Emission

Emit events using the `emit()` method:

```rust
use eventrs::{EventBus, Event};

#[derive(Event, Clone, Debug)]
struct UserLoggedIn {
    user_id: u64,
    timestamp: std::time::SystemTime,
}

let mut bus = EventBus::new();

// Register a handler
bus.on::<UserLoggedIn>(|event| {
    println!("User {} logged in at {:?}", event.user_id, event.timestamp);
});

// Emit the event
bus.emit(UserLoggedIn {
    user_id: 123,
    timestamp: std::time::SystemTime::now(),
});
```

### Immediate vs Deferred Emission

EventRS supports both immediate and deferred event emission:

```rust
// Immediate emission (default)
bus.emit(event); // Handlers execute immediately

// Deferred emission
bus.emit_deferred(event); // Event queued for later processing
bus.process_deferred(); // Process all deferred events
```

## Synchronous Emission

### Direct Emission

Events are processed immediately when emitted:

```rust
println!("Before emission");
bus.emit(UserLoggedIn { user_id: 123, timestamp: std::time::SystemTime::now() });
println!("After emission"); // Handlers have already executed
```

### Emission with Return Values

Get information about the emission process:

```rust
use eventrs::EmissionResult;

let result = bus.emit_with_result(event);
match result {
    EmissionResult::Success { handlers_called } => {
        println!("Event handled by {} handlers", handlers_called);
    }
    EmissionResult::NoHandlers => {
        println!("No handlers registered for this event");
    }
    EmissionResult::HandlerError(e) => {
        eprintln!("Handler error: {}", e);
    }
}
```

## Asynchronous Emission

### Async Event Emission

Use `AsyncEventBus` for asynchronous event handling:

```rust
use eventrs::AsyncEventBus;

let mut async_bus = AsyncEventBus::new();

// Register async handler
async_bus.on::<UserLoggedIn>(|event| async move {
    // Async operations
    update_database(event.user_id).await;
    send_notification(event.user_id).await;
});

// Emit async event
async_bus.emit(UserLoggedIn {
    user_id: 123,
    timestamp: std::time::SystemTime::now(),
}).await;
```

### Fire-and-Forget Emission

Emit events without waiting for completion:

```rust
// Fire-and-forget emission
async_bus.emit_and_forget(event);

// Continue immediately without waiting
println!("Event emitted, continuing...");
```

### Awaiting Completion

Wait for all handlers to complete:

```rust
// Wait for all handlers to complete
let result = async_bus.emit_and_wait(event).await;
println!("All handlers completed: {:?}", result);
```

## Batch Emission

### Batch Processing

Emit multiple events efficiently:

```rust
let events = vec![
    UserLoggedIn { user_id: 1, timestamp: std::time::SystemTime::now() },
    UserLoggedIn { user_id: 2, timestamp: std::time::SystemTime::now() },
    UserLoggedIn { user_id: 3, timestamp: std::time::SystemTime::now() },
];

// Batch emission
bus.emit_batch(events);

// Async batch emission
async_bus.emit_batch(events).await;
```

### Streaming Emission

Process events from a stream:

```rust
use futures::StreamExt;

let event_stream = get_event_stream();

async_bus.emit_stream(event_stream.map(|data| {
    UserLoggedIn {
        user_id: data.user_id,
        timestamp: std::time::SystemTime::now(),
    }
})).await;
```

## Conditional Emission

### Conditional Emission

Emit events only when conditions are met:

```rust
// Emit only if condition is true
if user_is_premium(user_id) {
    bus.emit(PremiumUserLoggedIn { user_id });
}

// Conditional emission with helper
bus.emit_if(
    || is_business_hours(),
    BusinessHoursLogin { user_id }
);
```

### Filtered Emission

Use filters to control emission:

```rust
use eventrs::Filter;

// Emit with filter
bus.emit_with_filter(
    event,
    Filter::custom(|e: &UserLoggedIn| e.user_id > 1000)
);
```

## Error Handling

### Emission Error Handling

Handle errors during emission:

```rust
use eventrs::EmissionError;

match bus.try_emit(event) {
    Ok(result) => println!("Event emitted successfully: {:?}", result),
    Err(EmissionError::NoHandlers) => {
        println!("Warning: No handlers registered");
    }
    Err(EmissionError::HandlerPanic(e)) => {
        eprintln!("Handler panicked: {}", e);
    }
    Err(EmissionError::ValidationFailed(e)) => {
        eprintln!("Event validation failed: {}", e);
    }
}
```

### Async Error Handling

Handle errors in async emission:

```rust
match async_bus.try_emit(event).await {
    Ok(result) => println!("Async event handled: {:?}", result),
    Err(e) => {
        eprintln!("Async emission failed: {}", e);
        // Handle error (retry, log, etc.)
    }
}
```

## Advanced Emission Patterns

### Event Transformation

Transform events during emission:

```rust
// Transform event before emission
let transformed_event = bus.emit_with_transform(
    original_event,
    |event| UserLoggedIn {
        user_id: event.user_id,
        timestamp: std::time::SystemTime::now(), // Update timestamp
    }
);
```

### Event Interception

Intercept events before they reach handlers:

```rust
bus.with_interceptor(|event: &UserLoggedIn| {
    println!("Intercepting login for user {}", event.user_id);
    // Can modify event or prevent emission
    true // Return true to continue emission
});
```

### Event Replay

Replay previously emitted events:

```rust
// Record events for replay
let recorder = EventRecorder::new();
bus.with_recorder(recorder.clone());

// Emit events (they get recorded)
bus.emit(event1);
bus.emit(event2);

// Replay recorded events
let recorded_events = recorder.get_events();
for event in recorded_events {
    bus.emit(event);
}
```

## Performance Optimization

### Zero-Copy Emission

Optimize emission for performance:

```rust
// Zero-copy emission for Copy types
#[derive(Event, Copy, Clone, Debug)]
struct FastEvent {
    id: u32,
    value: f64,
}

bus.emit(FastEvent { id: 1, value: 3.14 }); // No allocations
```

### Pre-allocated Emission

Use pre-allocated buffers for high-frequency events:

```rust
// Pre-allocate event buffer
let mut event_buffer = EventBuffer::with_capacity(1000);

loop {
    // Reuse buffer to avoid allocations
    let event = event_buffer.create_event::<HighFrequencyEvent>();
    event.data = get_sensor_data();
    
    bus.emit_from_buffer(&event_buffer);
    event_buffer.clear();
}
```

### Batch Optimization

Optimize batch operations:

```rust
// Efficient batch emission
let events: Vec<_> = (0..1000)
    .map(|i| SimpleEvent { id: i })
    .collect();

// Single allocation for entire batch
bus.emit_batch_optimized(events);
```

## Event Scheduling

### Delayed Emission

Schedule events for future emission:

```rust
use std::time::Duration;

// Emit after delay
bus.emit_after(
    Duration::from_secs(30),
    UserSessionTimeout { user_id: 123 }
);

// Emit at specific time
bus.emit_at(
    std::time::SystemTime::now() + Duration::from_hours(1),
    ReminderEvent { message: "Meeting in 1 hour".to_string() }
);
```

### Recurring Emission

Set up recurring event emission:

```rust
// Emit every 5 minutes
bus.emit_every(
    Duration::from_secs(300),
    HealthCheckEvent { timestamp: std::time::SystemTime::now() }
);

// Emit with custom schedule
bus.emit_with_schedule(
    Schedule::cron("0 */5 * * * *"), // Every 5 minutes
    || StatusUpdateEvent { status: get_system_status() }
);
```

## Emission Monitoring

### Emission Metrics

Monitor emission performance:

```rust
// Enable emission metrics
let bus = EventBus::builder()
    .with_metrics(true)
    .build();

// Access metrics
let metrics = bus.emission_metrics();
println!("Total events emitted: {}", metrics.total_events);
println!("Average emission time: {:?}", metrics.average_emission_time);
println!("Failed emissions: {}", metrics.failed_emissions);
```

### Emission Tracing

Trace event emission for debugging:

```rust
use tracing::{info, instrument};

#[instrument]
fn emit_user_event(bus: &mut EventBus, user_id: u64) {
    info!("Emitting user event for user {}", user_id);
    
    bus.emit(UserLoggedIn {
        user_id,
        timestamp: std::time::SystemTime::now(),
    });
    
    info!("User event emitted successfully");
}
```

## Testing Emission

### Testing Event Emission

Test emission behavior:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use eventrs::testing::*;
    
    #[test]
    fn test_event_emission() {
        let mut bus = TestEventBus::new();
        let captured_events = bus.capture_events::<UserLoggedIn>();
        
        // Emit event
        bus.emit(UserLoggedIn {
            user_id: 123,
            timestamp: std::time::SystemTime::now(),
        });
        
        // Verify emission
        let events = captured_events.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, 123);
    }
    
    #[test]
    fn test_batch_emission() {
        let mut bus = TestEventBus::new();
        let counter = bus.count_events::<UserLoggedIn>();
        
        // Emit batch
        let events = vec![
            UserLoggedIn { user_id: 1, timestamp: std::time::SystemTime::now() },
            UserLoggedIn { user_id: 2, timestamp: std::time::SystemTime::now() },
        ];
        
        bus.emit_batch(events);
        
        // Verify batch emission
        assert_eq!(counter.get_count(), 2);
    }
}
```

### Async Emission Testing

Test async emission:

```rust
#[tokio::test]
async fn test_async_emission() {
    let mut bus = AsyncTestEventBus::new();
    let captured_events = bus.capture_events::<UserLoggedIn>();
    
    // Emit async event
    bus.emit(UserLoggedIn {
        user_id: 123,
        timestamp: std::time::SystemTime::now(),
    }).await;
    
    // Verify async emission
    let events = captured_events.get_events().await;
    assert_eq!(events.len(), 1);
}
```

## Best Practices

### Emission Strategy
1. **Choose appropriate emission type**: Sync vs async based on requirements
2. **Use batch emission**: For multiple related events
3. **Handle errors gracefully**: Always check emission results
4. **Monitor emission performance**: Track metrics and timing

### Performance
1. **Prefer Copy types**: For high-frequency events
2. **Use batch operations**: For better throughput
3. **Avoid unnecessary cloning**: Design events to minimize allocations
4. **Consider deferred emission**: For decoupling emission from processing

### Error Handling
1. **Always handle emission errors**: Use `try_emit()` when appropriate
2. **Implement retry logic**: For transient failures
3. **Log emission failures**: For debugging and monitoring
4. **Validate events**: Before emission when possible

### Testing
1. **Test emission paths**: Verify events are emitted correctly
2. **Test error conditions**: Ensure proper error handling
3. **Test performance**: Verify emission meets performance requirements
4. **Mock external dependencies**: For reliable testing

Event emission is the trigger that drives your event-driven architecture. Proper emission strategies ensure your system is responsive, reliable, and performant.