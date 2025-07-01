# Handler API

This document covers the handler API in EventRS, including handler registration, management, and specialized handler types.

## Handler Registration

### Basic Handler Registration

```rust
use eventrs::{EventBus, Event, Handler};

#[derive(Event, Clone, Debug)]
struct UserEvent {
    user_id: u64,
    action: String,
}

let mut bus = EventBus::new();

// Closure handler
bus.on::<UserEvent>(|event| {
    println!("User {} performed: {}", event.user_id, event.action);
});

// Function pointer handler
fn handle_user_event(event: UserEvent) {
    println!("Handling user event: {:?}", event);
}

bus.on::<UserEvent>(handle_user_event);
```

### Handler with Priority

```rust
use eventrs::Priority;

// High priority handler (executes first)
bus.on_with_priority::<UserEvent>(Priority::High, |event| {
    security_check(event.user_id);
});

// Normal priority handler
bus.on::<UserEvent>(|event| {
    process_user_action(event);
});

// Low priority handler (executes last)
bus.on_with_priority::<UserEvent>(Priority::Low, |event| {
    cleanup_after_action(event.user_id);
});
```

### Handler with Custom ID

```rust
// Register handler with custom identifier
let handler_id = bus.on_with_id::<UserEvent>("audit_handler", |event| {
    audit_log.record_user_action(event.user_id, &event.action);
});

// Remove handler by ID
bus.off(handler_id);
```

## Specialized Handler Types

### Fallible Handlers

Handlers that can return errors:

```rust
use eventrs::{FallibleHandler, HandlerResult};

// Fallible handler with custom error type
#[derive(Debug, thiserror::Error)]
enum ProcessingError {
    #[error("Validation failed: {message}")]
    ValidationError { message: String },
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
}

bus.on_fallible::<UserEvent>(|event| -> Result<(), ProcessingError> {
    // Validate event
    if event.user_id == 0 {
        return Err(ProcessingError::ValidationError {
            message: "Invalid user ID".to_string()
        });
    }
    
    // Process event (may fail)
    database::update_user_activity(event.user_id, &event.action)?;
    
    Ok(())
});
```

### Conditional Handlers

Handlers that only execute when conditions are met:

```rust
// Handler with built-in condition
bus.on_conditional::<UserEvent>(
    |event| event.user_id > 1000,  // Condition
    |event| {                       // Handler
        process_premium_user_action(event);
    }
);

// Handler with complex condition
bus.on_conditional::<UserEvent>(
    |event| {
        is_business_hours() && 
        event.action.starts_with("admin_") &&
        user_has_permission(event.user_id, "admin")
    },
    |event| {
        process_admin_action(event);
    }
);
```

### Stateful Handlers

Handlers that maintain state between invocations:

```rust
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Shared state
let user_counts = Arc::new(Mutex::new(HashMap::<u64, u32>::new()));

// Stateful handler
let counts = user_counts.clone();
bus.on::<UserEvent>(move |event| {
    let mut counts = counts.lock().unwrap();
    let counter = counts.entry(event.user_id).or_insert(0);
    *counter += 1;
    println!("User {} action count: {}", event.user_id, counter);
});

// Another handler accessing the same state
let counts = user_counts.clone();
bus.on::<UserStatsRequest>(move |_| {
    let counts = counts.lock().unwrap();
    let total_actions: u32 = counts.values().sum();
    println!("Total user actions: {}", total_actions);
});
```

## Async Handlers

### Basic Async Handlers

```rust
use eventrs::AsyncEventBus;

let mut async_bus = AsyncEventBus::new();

// Simple async handler
async_bus.on::<UserEvent>(|event| async move {
    // Async database operation
    database::log_user_action(event.user_id, &event.action).await;
    
    // Async external API call
    analytics::track_user_event(&event).await;
}).await;
```

### Fallible Async Handlers

```rust
// Async handler that can fail
async_bus.on_fallible::<UserEvent>(|event| async move -> Result<(), ProcessingError> {
    // Async validation
    validate_user_async(event.user_id).await?;
    
    // Async processing
    let result = process_event_async(&event).await?;
    
    // Async storage
    store_result_async(result).await?;
    
    Ok(())
}).await;
```

### Concurrent Async Handlers

```rust
// Handler that processes multiple operations concurrently
async_bus.on::<UserEvent>(|event| async move {
    // Execute multiple async operations concurrently
    let (analytics_result, audit_result, notification_result) = tokio::join!(
        analytics::track_event(&event),
        audit::log_event(&event),
        notifications::send_user_notification(event.user_id)
    );
    
    // Handle results
    if let Err(e) = analytics_result {
        eprintln!("Analytics failed: {}", e);
    }
    
    if let Err(e) = audit_result {
        eprintln!("Audit failed: {}", e);
    }
    
    if let Err(e) = notification_result {
        eprintln!("Notification failed: {}", e);
    }
}).await;
```

## Filtered Handlers

### Field-Based Filters

```rust
use eventrs::Filter;

// Handler for premium users only
bus.on_filtered::<UserEvent>(
    Filter::field(|event| event.user_id > 1000),
    |event| {
        process_premium_user_event(event);
    }
);

// Handler for specific actions
bus.on_filtered::<UserEvent>(
    Filter::field(|event| event.action == "purchase"),
    |event| {
        process_purchase_event(event);
    }
);
```

### Complex Filters

```rust
// Multiple filter conditions
let complex_filter = Filter::all_of([
    Filter::field(|event: &UserEvent| event.user_id > 100),
    Filter::field(|event: &UserEvent| event.action.starts_with("admin_")),
    Filter::custom(|event: &UserEvent| is_business_hours()),
]);

bus.on_filtered::<UserEvent>(complex_filter, |event| {
    process_admin_event_during_business_hours(event);
});
```

### Dynamic Filters

```rust
use std::sync::{Arc, RwLock};

// Configurable filter
let filter_config = Arc::new(RwLock::new(FilterConfig {
    min_user_id: 1000,
    allowed_actions: vec!["login".to_string(), "purchase".to_string()],
}));

let config = filter_config.clone();
bus.on_filtered::<UserEvent>(
    Filter::custom(move |event| {
        let config = config.read().unwrap();
        event.user_id >= config.min_user_id && 
        config.allowed_actions.contains(&event.action)
    }),
    |event| {
        process_filtered_event(event);
    }
);

// Update filter configuration at runtime
{
    let mut config = filter_config.write().unwrap();
    config.min_user_id = 500;
    config.allowed_actions.push("admin_action".to_string());
}
```

## Handler Groups

### Basic Handler Groups

```rust
use eventrs::HandlerGroup;

// Create a handler group for user management
let mut user_group = HandlerGroup::new("user_management");

user_group.on::<UserEvent>(|event| {
    update_user_activity(event.user_id);
});

user_group.on::<UserLoginEvent>(|event| {
    update_login_status(event.user_id);
});

user_group.on::<UserLogoutEvent>(|event| {
    update_logout_status(event.user_id);
});

// Register the entire group
bus.register_group(user_group);
```

### Handler Groups with Priority

```rust
// High priority security group
let mut security_group = HandlerGroup::new("security")
    .with_priority(Priority::High);

security_group.on::<UserEvent>(|event| {
    security_audit(event.user_id, &event.action);
});

// Normal priority processing group
let mut processing_group = HandlerGroup::new("processing")
    .with_priority(Priority::Normal);

processing_group.on::<UserEvent>(|event| {
    process_user_event(event);
});

// Register groups (security handlers run first)
bus.register_group(security_group);
bus.register_group(processing_group);
```

### Conditional Handler Groups

```rust
// Group that only activates during business hours
let mut business_hours_group = HandlerGroup::new("business_hours")
    .with_condition(|| is_business_hours());

business_hours_group.on::<UserEvent>(|event| {
    process_business_event(event);
});

business_hours_group.on::<OrderEvent>(|event| {
    process_business_order(event);
});

bus.register_group(business_hours_group);
```

## Handler Management

### Handler Lifecycle

```rust
// Register handler and get ID
let handler_id = bus.on::<UserEvent>(|event| {
    println!("Processing: {:?}", event);
});

// Check if handler exists
if bus.has_handler(handler_id) {
    println!("Handler is registered");
}

// Get handler count for event type
let count = bus.handler_count::<UserEvent>();
println!("UserEvent has {} handlers", count);

// Remove specific handler
if bus.off(handler_id) {
    println!("Handler removed successfully");
}

// Remove all handlers for event type
bus.clear_handlers::<UserEvent>();

// Remove all handlers
bus.clear();
```

### Handler Metadata

```rust
// Register handler with metadata
let handler_id = bus.on_with_metadata::<UserEvent>(
    HandlerMetadata::new()
        .with_name("user_processor")
        .with_description("Processes user events")
        .with_category("business_logic")
        .with_estimated_execution_time(Duration::from_millis(10)),
    |event| {
        process_user_event(event);
    }
);

// Get handler metadata
if let Some(metadata) = bus.handler_metadata(handler_id) {
    println!("Handler: {}", metadata.name());
    println!("Description: {}", metadata.description());
    println!("Estimated time: {:?}", metadata.estimated_execution_time());
}
```

## Handler Error Handling

### Error Handling Strategies

```rust
use eventrs::ErrorHandling;

// Configure error handling for the bus
let bus = EventBus::builder()
    .with_error_handling(ErrorHandling::ContinueOnError)
    .build();

// Handler that might fail
bus.on_fallible::<UserEvent>(|event| -> Result<(), ProcessingError> {
    if event.user_id == 0 {
        return Err(ProcessingError::InvalidUserId);
    }
    
    // Process event
    process_user_event(event)?;
    Ok(())
});

// Another handler that continues even if the first one fails
bus.on::<UserEvent>(|event| {
    log_user_event(event);
});
```

### Custom Error Handlers

```rust
// Global error handler
bus.on_error(|error: &HandlerError, event: &dyn Event| {
    eprintln!("Handler error for {}: {}", event.event_type_name(), error);
    
    // Send error to monitoring system
    monitoring::report_handler_error(error, event);
});

// Error handler for specific event type
bus.on_error_for::<UserEvent>(|error: &HandlerError, event: &UserEvent| {
    eprintln!("User event handler error for user {}: {}", event.user_id, error);
    
    // Specific error handling for user events
    if let HandlerError::Timeout { .. } = error {
        // Handle timeout specifically
        handle_user_event_timeout(event);
    }
});
```

## Handler Testing

### Mock Handlers

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use eventrs::testing::*;
    
    #[test]
    fn test_user_event_handler() {
        let mut bus = TestEventBus::new();
        let captured_events = Arc::new(Mutex::new(Vec::new()));
        
        // Mock handler that captures events
        let events = captured_events.clone();
        bus.on::<UserEvent>(move |event| {
            events.lock().unwrap().push(event);
        });
        
        // Emit test event
        bus.emit(UserEvent {
            user_id: 123,
            action: "test_action".to_string(),
        });
        
        // Verify handler was called
        let events = captured_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, 123);
        assert_eq!(events[0].action, "test_action");
    }
    
    #[test]
    fn test_fallible_handler() {
        let mut bus = TestEventBus::new();
        let error_count = Arc::new(AtomicU32::new(0));
        
        let counter = error_count.clone();
        bus.on_fallible::<UserEvent>(move |event| -> Result<(), ProcessingError> {
            if event.user_id == 0 {
                counter.fetch_add(1, Ordering::SeqCst);
                return Err(ProcessingError::InvalidUserId);
            }
            Ok(())
        });
        
        // Test successful case
        bus.emit(UserEvent { user_id: 123, action: "valid".to_string() });
        assert_eq!(error_count.load(Ordering::SeqCst), 0);
        
        // Test error case
        bus.emit(UserEvent { user_id: 0, action: "invalid".to_string() });
        assert_eq!(error_count.load(Ordering::SeqCst), 1);
    }
}
```

### Handler Performance Testing

```rust
#[cfg(test)]
mod tests {
    use std::time::Instant;
    
    #[test]
    fn test_handler_performance() {
        let mut bus = EventBus::new();
        
        // Register many handlers
        for i in 0..1000 {
            bus.on::<UserEvent>(move |_| {
                // Simulate work
                std::hint::black_box(i * 2);
            });
        }
        
        // Measure execution time
        let start = Instant::now();
        bus.emit(UserEvent {
            user_id: 123,
            action: "performance_test".to_string(),
        });
        let duration = start.elapsed();
        
        println!("1000 handlers executed in: {:?}", duration);
        assert!(duration.as_millis() < 100);
    }
}
```

## Best Practices

### Handler Design
1. **Keep handlers focused**: One responsibility per handler
2. **Make handlers pure when possible**: Avoid side effects
3. **Handle errors gracefully**: Use fallible handlers for operations that can fail
4. **Use appropriate handler types**: Sync vs async based on the work being done

### Performance
1. **Minimize captures**: Reduce data captured by closures
2. **Use move semantics**: When transferring ownership to handlers
3. **Avoid heavy computations**: In synchronous handlers on the main thread
4. **Consider handler priority**: For ordering guarantees

### Error Handling
1. **Use fallible handlers**: For operations that can fail
2. **Implement error recovery**: Where appropriate
3. **Log handler errors**: For debugging and monitoring
4. **Choose appropriate error handling strategy**: Based on application requirements

### Testing
1. **Test handlers in isolation**: Unit test handler logic
2. **Use mock handlers**: For testing event emission
3. **Test error conditions**: Verify error handling works
4. **Performance test**: Ensure handlers meet performance requirements

The handler API in EventRS provides flexible and powerful ways to process events while maintaining type safety and performance. Proper use of the different handler types and patterns enables building robust event-driven systems.