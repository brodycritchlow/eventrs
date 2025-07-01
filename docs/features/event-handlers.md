# Event Handlers

Event handlers are functions or closures that process events in EventRS. This document covers how to create, register, and manage event handlers effectively.

## Basic Handler Registration

### Simple Function Handlers

Register handlers using closures or function pointers:

```rust
use eventrs::{EventBus, Event};

#[derive(Event, Clone, Debug)]
struct UserLoggedIn {
    user_id: u64,
    username: String,
}

let mut bus = EventBus::new();

// Closure handler
bus.on::<UserLoggedIn>(|event| {
    println!("User {} (ID: {}) logged in", event.username, event.user_id);
});

// Function pointer handler
fn handle_user_login(event: UserLoggedIn) {
    println!("Processing login for user {}", event.user_id);
}

bus.on::<UserLoggedIn>(handle_user_login);
```

### Method Handlers

Register methods as handlers using closures:

```rust
struct UserService;

impl UserService {
    fn handle_login(&self, event: UserLoggedIn) {
        println!("UserService processing login for {}", event.user_id);
    }
}

let service = UserService;
bus.on::<UserLoggedIn>(move |event| service.handle_login(event));
```

## Handler Types

### Synchronous Handlers

Standard handlers that execute synchronously:

```rust
// Simple sync handler
bus.on::<UserLoggedIn>(|event| {
    update_last_login(event.user_id);
    log_user_activity(event.user_id, "login");
});

// Handler with error handling
bus.on::<UserLoggedIn>(|event| {
    if let Err(e) = process_login(&event) {
        eprintln!("Login processing failed: {}", e);
    }
});
```

### Async Handlers

Handlers that perform asynchronous operations:

```rust
use eventrs::AsyncEventBus;

let mut async_bus = AsyncEventBus::new();

// Async handler
async_bus.on::<UserLoggedIn>(|event| async move {
    // Async database update
    database::update_last_login(event.user_id).await;
    
    // Async external API call
    analytics::track_login(event.user_id).await;
    
    // Async notification
    notification::send_welcome_email(&event.username).await;
});
```

### Fallible Handlers

Handlers that can return errors:

```rust
use eventrs::{EventBus, HandlerResult};

bus.on_fallible::<UserLoggedIn>(|event| -> HandlerResult<()> {
    validate_user(event.user_id)?;
    update_session(event.user_id)?;
    Ok(())
});

// Error handling is built-in
bus.on_fallible::<UserLoggedIn>(|event| {
    database::update_user_status(event.user_id, "online")
        .map_err(|e| HandlerError::Database(e))
});
```

## Handler Registration Patterns

### Multiple Handlers for Same Event

Register multiple handlers for the same event type:

```rust
// Analytics handler
bus.on::<UserLoggedIn>(|event| {
    analytics::track_event("user_login", event.user_id);
});

// Audit handler
bus.on::<UserLoggedIn>(|event| {
    audit::log_security_event("LOGIN", event.user_id);
});

// Notification handler
bus.on::<UserLoggedIn>(|event| {
    notifications::send_login_alert(event.user_id);
});
```

### Generic Handlers

Create handlers that work with multiple event types:

```rust
use eventrs::Event;

// Generic logging handler
fn log_event<E: Event + std::fmt::Debug>(event: E) {
    println!("[{}] Event: {:?}", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        event
    );
}

bus.on::<UserLoggedIn>(log_event);
bus.on::<UserLoggedOut>(log_event);
```

### Conditional Handlers

Handlers that only execute under certain conditions:

```rust
// Handler with condition
bus.on::<UserLoggedIn>(|event| {
    if is_business_hours() {
        process_business_login(event);
    } else {
        process_after_hours_login(event);
    }
});

// Handler for specific users
bus.on::<UserLoggedIn>(|event| {
    if event.user_id > 1000 {
        process_premium_user_login(event);
    }
});
```

## Handler Management

### Handler Registration with IDs

Register handlers with identifiers for later management:

```rust
use eventrs::HandlerId;

let handler_id = bus.on_with_id::<UserLoggedIn>("audit_handler", |event| {
    audit::log_user_login(event.user_id);
});

// Remove handler later
bus.remove_handler(handler_id);
```

### Handler Groups

Group related handlers together:

```rust
use eventrs::HandlerGroup;

let mut user_handlers = HandlerGroup::new("user_management");

user_handlers.on::<UserLoggedIn>(|event| {
    update_user_status(event.user_id, "online");
});

user_handlers.on::<UserLoggedOut>(|event| {
    update_user_status(event.user_id, "offline");
});

// Register entire group
bus.register_group(user_handlers);
```

### Handler Priorities

Control handler execution order with priorities:

```rust
use eventrs::Priority;

// High priority - executes first
bus.on_with_priority::<UserLoggedIn>(Priority::High, |event| {
    security::validate_session(event.user_id);
});

// Normal priority
bus.on::<UserLoggedIn>(|event| {
    update_user_activity(event.user_id);
});

// Low priority - executes last
bus.on_with_priority::<UserLoggedIn>(Priority::Low, |event| {
    cleanup_old_sessions(event.user_id);
});
```

## Advanced Handler Patterns

### Stateful Handlers

Handlers that maintain state between invocations:

```rust
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Shared state
let user_counts = Arc::new(Mutex::new(HashMap::<u64, u32>::new()));

// Handler with state
let counts = user_counts.clone();
bus.on::<UserLoggedIn>(move |event| {
    let mut counts = counts.lock().unwrap();
    let count = counts.entry(event.user_id).or_insert(0);
    *count += 1;
    println!("User {} has logged in {} times", event.user_id, count);
});
```

### Handler Factories

Create handlers dynamically:

```rust
fn create_user_handler(service_name: String) -> impl Fn(UserLoggedIn) {
    move |event| {
        println!("[{}] Processing login for user {}", service_name, event.user_id);
    }
}

// Create and register handlers
bus.on::<UserLoggedIn>(create_user_handler("analytics".to_string()));
bus.on::<UserLoggedIn>(create_user_handler("audit".to_string()));
```

### Middleware-Style Handlers

Create handlers that can modify events or control flow:

```rust
use eventrs::{Event, HandlerChain};

// Middleware handler
fn logging_middleware<E: Event + std::fmt::Debug>(
    event: E,
    next: HandlerChain<E>
) -> HandlerResult<()> {
    println!("Before handling: {:?}", event);
    let result = next.call(event);
    println!("After handling: {:?}", result);
    result
}

bus.use_middleware(logging_middleware);
```

## Handler Error Handling

### Error Recovery

Implement error recovery in handlers:

```rust
bus.on::<UserLoggedIn>(|event| {
    match process_user_login(event.user_id) {
        Ok(()) => {
            println!("Login processed successfully");
        }
        Err(ProcessingError::TemporaryFailure(e)) => {
            println!("Temporary failure, will retry: {}", e);
            schedule_retry(event);
        }
        Err(ProcessingError::PermanentFailure(e)) => {
            println!("Permanent failure: {}", e);
            log_error("login_processing", &e);
        }
    }
});
```

### Error Propagation

Configure how handler errors are handled:

```rust
use eventrs::ErrorHandling;

let bus = EventBus::builder()
    .with_error_handling(ErrorHandling::StopOnFirstError)
    .build();

// Or continue on errors
let bus = EventBus::builder()
    .with_error_handling(ErrorHandling::ContinueOnError)
    .build();
```

## Testing Handlers

### Unit Testing Handlers

Test handlers in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_login_handler() {
        let event = UserLoggedIn {
            user_id: 123,
            username: "test_user".to_string(),
        };
        
        // Test handler directly
        handle_user_login(event.clone());
        
        // Verify side effects
        assert!(user_is_marked_online(123));
    }
    
    #[test]
    fn test_handler_with_mock() {
        let mut mock_service = MockUserService::new();
        mock_service
            .expect_update_status()
            .with(eq(123))
            .times(1)
            .returning(|_| Ok(()));
        
        let handler = |event: UserLoggedIn| {
            mock_service.update_status(event.user_id).unwrap();
        };
        
        handler(UserLoggedIn {
            user_id: 123,
            username: "test".to_string(),
        });
    }
}
```

### Integration Testing

Test handlers within the event bus:

```rust
#[cfg(test)]
mod tests {
    use eventrs::testing::*;
    
    #[test]
    fn test_handler_integration() {
        let mut bus = TestEventBus::new();
        let captured_events = Arc::new(Mutex::new(Vec::new()));
        
        let events = captured_events.clone();
        bus.on::<UserLoggedIn>(move |event| {
            events.lock().unwrap().push(event);
        });
        
        // Emit test event
        bus.emit(UserLoggedIn {
            user_id: 123,
            username: "test".to_string(),
        });
        
        // Verify handler was called
        let events = captured_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, 123);
    }
}
```

## Performance Considerations

### Handler Optimization

Optimize handlers for performance:

```rust
// Fast path for simple handlers
bus.on::<SimpleEvent>(|event| {
    // Inline operations
    COUNTER.fetch_add(1, Ordering::Relaxed);
});

// Avoid allocations in hot handlers
bus.on::<HighFrequencyEvent>(|event| {
    // Use stack allocation
    let mut buffer = [0u8; 1024];
    process_data(&event.data, &mut buffer);
});
```

### Async Handler Optimization

Optimize async handlers:

```rust
// Use spawn_local for CPU-bound work
async_bus.on::<DataEvent>(|event| async move {
    tokio::task::spawn_local(async move {
        cpu_intensive_processing(event.data);
    }).await.unwrap();
});

// Batch async operations
async_bus.on::<BatchEvent>(|event| async move {
    let futures: Vec<_> = event.items
        .into_iter()
        .map(|item| process_item_async(item))
        .collect();
    
    futures::future::join_all(futures).await;
});
```

## Best Practices

### Handler Design
1. **Keep handlers focused**: One responsibility per handler
2. **Make handlers idempotent**: Safe to call multiple times
3. **Handle errors gracefully**: Don't let exceptions escape
4. **Log important operations**: For debugging and monitoring

### Performance
1. **Minimize allocations**: Especially in hot paths
2. **Use appropriate handler types**: Sync vs async vs fallible
3. **Consider handler priorities**: For ordering guarantees
4. **Profile handler performance**: Identify bottlenecks

### Testing
1. **Test handlers in isolation**: Unit test handler logic
2. **Test error conditions**: Verify error handling
3. **Integration test**: Test handlers within the bus
4. **Mock external dependencies**: For reliable tests

Event handlers are the workhorses of EventRS applications. Well-designed handlers make your system robust, maintainable, and performant.