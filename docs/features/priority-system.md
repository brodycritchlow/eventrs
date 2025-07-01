# Priority System

EventRS includes a sophisticated priority system that controls the order in which event handlers are executed. This document covers priority-based handler ordering, priority inheritance, and advanced priority patterns.

## Basic Priority System

### Priority Levels

EventRS provides several built-in priority levels:

```rust
use eventrs::{Priority, EventBus, Event};

#[derive(Event, Clone, Debug)]
struct SystemEvent {
    message: String,
    severity: String,
}

let mut bus = EventBus::new();

// Register handlers with different priorities
bus.on_with_priority::<SystemEvent>(Priority::Critical, |event| {
    println!("CRITICAL: {}", event.message);
    // Critical handlers execute first
});

bus.on_with_priority::<SystemEvent>(Priority::High, |event| {
    println!("HIGH: {}", event.message);
    // High priority handlers execute second
});

bus.on_with_priority::<SystemEvent>(Priority::Normal, |event| {
    println!("NORMAL: {}", event.message);
    // Normal priority handlers execute third (default)
});

bus.on_with_priority::<SystemEvent>(Priority::Low, |event| {
    println!("LOW: {}", event.message);
    // Low priority handlers execute last
});
```

### Custom Priority Values

Define custom priority values for fine-grained control:

```rust
use eventrs::PriorityValue;

// Custom priority values (higher numbers = higher priority)
const EMERGENCY_PRIORITY: PriorityValue = PriorityValue::new(1000);
const SECURITY_PRIORITY: PriorityValue = PriorityValue::new(800);
const AUDIT_PRIORITY: PriorityValue = PriorityValue::new(600);
const ANALYTICS_PRIORITY: PriorityValue = PriorityValue::new(400);
const LOGGING_PRIORITY: PriorityValue = PriorityValue::new(200);

// Register handlers with custom priorities
bus.on_with_priority::<SystemEvent>(
    Priority::Custom(EMERGENCY_PRIORITY),
    |event| {
        emergency_response(&event);
    }
);

bus.on_with_priority::<SystemEvent>(
    Priority::Custom(SECURITY_PRIORITY),
    |event| {
        security_audit(&event);
    }
);
```

## Priority Inheritance

### Event-Based Priority

Events can specify their own priority, which can influence handler execution:

```rust
use eventrs::{Event, EventMetadata, Priority};

#[derive(Event, Clone, Debug)]
struct PriorityEvent {
    message: String,
    urgency: u8,
}

impl PriorityEvent {
    fn metadata(&self) -> EventMetadata {
        let priority = match self.urgency {
            9..=10 => Priority::Critical,
            7..=8 => Priority::High,
            4..=6 => Priority::Normal,
            1..=3 => Priority::Low,
            _ => Priority::Normal,
        };
        
        EventMetadata::new().with_priority(priority)
    }
}

// Handlers can check event priority
bus.on::<PriorityEvent>(|event| {
    let priority = event.metadata().priority();
    match priority {
        Priority::Critical => handle_critical_event(&event),
        Priority::High => handle_high_priority_event(&event),
        _ => handle_normal_event(&event),
    }
});
```

### Dynamic Priority Assignment

Assign priorities dynamically based on runtime conditions:

```rust
// Dynamic priority based on system load
let system_load = get_system_load();
let dynamic_priority = if system_load > 0.8 {
    Priority::High  // High priority when system is under load
} else {
    Priority::Normal
};

bus.on_with_priority::<SystemEvent>(dynamic_priority, |event| {
    process_system_event(&event);
});
```

## Priority Groups

### Handler Groups with Priorities

Organize related handlers into priority groups:

```rust
use eventrs::{HandlerGroup, Priority};

// Create priority groups
let mut security_group = HandlerGroup::new("security")
    .with_priority(Priority::High);

let mut analytics_group = HandlerGroup::new("analytics")
    .with_priority(Priority::Low);

// Add handlers to groups
security_group.on::<UserAction>(|event| {
    security_audit(event);
});

analytics_group.on::<UserAction>(|event| {
    track_user_action(event);
});

// Register groups (security handlers execute before analytics)
bus.register_group(security_group);
bus.register_group(analytics_group);
```

### Nested Priority Groups

Create hierarchical priority structures:

```rust
// Parent group with high priority
let mut critical_systems = HandlerGroup::new("critical")
    .with_priority(Priority::High);

// Child groups inherit parent priority but can have sub-priorities
let mut auth_handlers = HandlerGroup::new("auth")
    .with_sub_priority(1);  // Within critical systems, auth has priority 1

let mut payment_handlers = HandlerGroup::new("payment")
    .with_sub_priority(2);  // Within critical systems, payment has priority 2

// Add handlers
auth_handlers.on::<UserLoginEvent>(|event| {
    authenticate_user(event);
});

payment_handlers.on::<PaymentEvent>(|event| {
    process_payment(event);
});

// Add child groups to parent
critical_systems.add_child_group(auth_handlers);
critical_systems.add_child_group(payment_handlers);

bus.register_group(critical_systems);
```

## Advanced Priority Patterns

### Conditional Priority

Adjust handler priority based on conditions:

```rust
struct ConditionalPriorityHandler {
    base_priority: Priority,
    condition: Box<dyn Fn(&SystemEvent) -> bool + Send + Sync>,
    boosted_priority: Priority,
}

impl ConditionalPriorityHandler {
    fn get_priority(&self, event: &SystemEvent) -> Priority {
        if (self.condition)(event) {
            self.boosted_priority
        } else {
            self.base_priority
        }
    }
}

// Create conditional priority handler
let conditional_handler = ConditionalPriorityHandler {
    base_priority: Priority::Normal,
    condition: Box::new(|event| event.severity == "ERROR"),
    boosted_priority: Priority::High,
};

// Register with dynamic priority
bus.on_with_dynamic_priority::<SystemEvent>(
    move |event| conditional_handler.get_priority(event),
    |event| {
        handle_system_event(event);
    }
);
```

### Priority Chains

Create chains of handlers that execute in priority order:

```rust
use eventrs::PriorityChain;

let mut chain = PriorityChain::new();

// Add handlers to chain in priority order
chain.add(Priority::Critical, |event: &SystemEvent| {
    if validate_event(event) {
        ChainResult::Continue
    } else {
        ChainResult::Stop  // Stop chain execution
    }
});

chain.add(Priority::High, |event: &SystemEvent| {
    process_event(event);
    ChainResult::Continue
});

chain.add(Priority::Normal, |event: &SystemEvent| {
    log_event(event);
    ChainResult::Continue
});

chain.add(Priority::Low, |event: &SystemEvent| {
    cleanup_after_event(event);
    ChainResult::Continue
});

// Register chain as a single handler
bus.on::<SystemEvent>(move |event| {
    chain.execute(&event);
});
```

## Priority-Based Scheduling

### Time-Sliced Execution

Execute handlers in time slices based on priority:

```rust
use eventrs::{PriorityScheduler, TimeSlice};

let mut scheduler = PriorityScheduler::new()
    .with_time_slice(Priority::Critical, TimeSlice::from_millis(100))
    .with_time_slice(Priority::High, TimeSlice::from_millis(50))
    .with_time_slice(Priority::Normal, TimeSlice::from_millis(20))
    .with_time_slice(Priority::Low, TimeSlice::from_millis(10));

let bus = EventBus::builder()
    .with_scheduler(scheduler)
    .build();

// Critical handlers get more execution time
bus.on_with_priority::<SystemEvent>(Priority::Critical, |event| {
    // Can run for up to 100ms
    expensive_critical_operation(event);
});
```

### Priority Queues

Use priority queues for event processing:

```rust
use eventrs::PriorityQueue;

// Create priority queue for events
let mut event_queue = PriorityQueue::new();

// Add events with priorities
event_queue.push(event1, Priority::High);
event_queue.push(event2, Priority::Critical);
event_queue.push(event3, Priority::Normal);

// Process events in priority order
while let Some((event, priority)) = event_queue.pop() {
    println!("Processing event with priority: {:?}", priority);
    bus.emit(event);
}
```

## Priority Monitoring

### Priority Metrics

Monitor priority-based execution:

```rust
use eventrs::PriorityMetrics;

let bus = EventBus::builder()
    .with_priority_metrics(true)
    .build();

// After processing events
let metrics = bus.priority_metrics();
println!("Critical handlers executed: {}", metrics.critical_executions);
println!("High priority handlers executed: {}", metrics.high_executions);
println!("Average execution time by priority: {:?}", metrics.avg_execution_time);
```

### Priority Debugging

Debug priority-based execution:

```rust
// Enable priority debugging
let bus = EventBus::builder()
    .with_priority_debugging(true)
    .build();

// Debug output will show handler execution order
bus.emit(SystemEvent {
    message: "Test event".to_string(),
    severity: "INFO".to_string(),
});

// Output:
// [DEBUG] Executing handler 'critical_handler' (Priority: Critical)
// [DEBUG] Executing handler 'high_handler' (Priority: High)  
// [DEBUG] Executing handler 'normal_handler' (Priority: Normal)
```

## Async Priority Handling

### Async Priority Execution

Handle priorities in async contexts:

```rust
use eventrs::AsyncEventBus;

let mut async_bus = AsyncEventBus::new();

// Async handlers with priorities
async_bus.on_with_priority::<SystemEvent>(Priority::High, |event| async move {
    // High priority async operation
    critical_async_operation(event).await;
}).await;

async_bus.on_with_priority::<SystemEvent>(Priority::Normal, |event| async move {
    // Normal priority async operation
    normal_async_operation(event).await;
}).await;

// Emit event - high priority handlers execute first
async_bus.emit(SystemEvent {
    message: "Async event".to_string(),
    severity: "INFO".to_string(),
}).await;
```

### Priority-Based Concurrency Control

Control concurrency based on priority:

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

// Different semaphores for different priorities
let critical_semaphore = Arc::new(Semaphore::new(10));  // More permits for critical
let normal_semaphore = Arc::new(Semaphore::new(5));     // Fewer permits for normal

async_bus.on_with_priority::<SystemEvent>(Priority::Critical, {
    let semaphore = critical_semaphore.clone();
    move |event| {
        let semaphore = semaphore.clone();
        async move {
            let _permit = semaphore.acquire().await.unwrap();
            critical_async_operation(event).await;
        }
    }
}).await;

async_bus.on_with_priority::<SystemEvent>(Priority::Normal, {
    let semaphore = normal_semaphore.clone();
    move |event| {
        let semaphore = semaphore.clone();
        async move {
            let _permit = semaphore.acquire().await.unwrap();
            normal_async_operation(event).await;
        }
    }
}).await;
```

## Testing Priority Systems

### Priority Testing

Test priority-based execution order:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    
    #[test]
    fn test_priority_execution_order() {
        let mut bus = EventBus::new();
        let execution_order = Arc::new(Mutex::new(Vec::new()));
        
        // Register handlers in reverse priority order
        let order = execution_order.clone();
        bus.on_with_priority::<SystemEvent>(Priority::Low, move |_| {
            order.lock().unwrap().push("low");
        });
        
        let order = execution_order.clone();
        bus.on_with_priority::<SystemEvent>(Priority::High, move |_| {
            order.lock().unwrap().push("high");
        });
        
        let order = execution_order.clone();
        bus.on_with_priority::<SystemEvent>(Priority::Critical, move |_| {
            order.lock().unwrap().push("critical");
        });
        
        // Emit event
        bus.emit(SystemEvent {
            message: "test".to_string(),
            severity: "INFO".to_string(),
        });
        
        // Verify execution order
        let order = execution_order.lock().unwrap();
        assert_eq!(*order, vec!["critical", "high", "low"]);
    }
}
```

### Performance Testing

Test priority system performance:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_priority_performance() {
        let mut bus = EventBus::new();
        
        // Register many handlers with different priorities
        for i in 0..1000 {
            let priority = match i % 4 {
                0 => Priority::Critical,
                1 => Priority::High,
                2 => Priority::Normal,
                3 => Priority::Low,
                _ => unreachable!(),
            };
            
            bus.on_with_priority::<SystemEvent>(priority, move |_| {
                // Simulate work
                std::hint::black_box(i * 2);
            });
        }
        
        // Measure execution time
        let start = Instant::now();
        bus.emit(SystemEvent {
            message: "performance test".to_string(),
            severity: "INFO".to_string(),
        });
        let duration = start.elapsed();
        
        println!("Priority execution took: {:?}", duration);
        assert!(duration.as_millis() < 100);  // Should be fast
    }
}
```

## Best Practices

### Priority Design
1. **Use meaningful priorities**: Choose priorities that reflect business importance
2. **Avoid too many priority levels**: Keep priority hierarchy simple
3. **Document priority decisions**: Explain why certain handlers have specific priorities
4. **Consider system resources**: High priority handlers consume more resources

### Performance
1. **Balance priority granularity**: Too many levels can impact performance
2. **Monitor priority distribution**: Ensure balanced priority usage
3. **Optimize high-priority handlers**: They execute first and most frequently
4. **Use priority groups**: For related handlers that should execute together

### Maintainability
1. **Use constants for custom priorities**: Avoid magic numbers
2. **Group related handlers**: Use handler groups for organization
3. **Test priority behavior**: Verify execution order in tests
4. **Profile priority performance**: Measure impact of priority system

### Error Handling
1. **Handle priority conflicts**: When handlers have the same priority
2. **Graceful degradation**: Lower priority handlers can be skipped under load
3. **Priority-based error handling**: Critical handlers may need different error handling
4. **Monitor priority violations**: Detect when priorities are not respected

The priority system in EventRS enables sophisticated control over handler execution order, ensuring that critical operations are processed first while maintaining system performance and reliability.