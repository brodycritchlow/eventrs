# Event Filtering

EventRS provides powerful event filtering capabilities that allow you to control which events reach which handlers. This document covers the filtering system, from basic predicates to complex filtering chains.

## Basic Filtering

### Simple Filters

Apply basic filters to event handlers:

```rust
use eventrs::{EventBus, Filter, Event};

#[derive(Event, Clone, Debug)]
struct UserAction {
    user_id: u64,
    action: String,
    timestamp: std::time::SystemTime,
}

let mut bus = EventBus::new();

// Handler with simple filter
bus.on_filtered::<UserAction>(
    Filter::field(|event| event.user_id > 1000),
    |event| {
        println!("Premium user {} performed: {}", event.user_id, event.action);
    }
);
```

### Predicate Filters

Use custom predicates for filtering:

```rust
// Custom predicate function
fn is_business_hours(event: &UserAction) -> bool {
    let now = chrono::Local::now();
    let hour = now.hour();
    hour >= 9 && hour < 17
}

// Apply predicate filter
bus.on_filtered::<UserAction>(
    Filter::predicate(is_business_hours),
    |event| {
        println!("Business hours action: {}", event.action);
    }
);
```

## Filter Types

### Field Filters

Filter based on event field values:

```rust
// Filter by single field
bus.on_filtered::<UserAction>(
    Filter::field(|event| event.user_id == 123),
    |event| println!("User 123 performed action")
);

// Filter by multiple fields
bus.on_filtered::<UserAction>(
    Filter::field(|event| {
        event.user_id > 1000 && event.action.starts_with("admin_")
    }),
    |event| println!("Admin action by premium user")
);
```

### Type Filters

Filter events by type (useful for generic handlers):

```rust
// Filter specific event types
bus.on_filtered_any(
    Filter::event_type::<UserAction>(),
    |event: Box<dyn Event>| {
        if let Some(user_action) = event.downcast_ref::<UserAction>() {
            println!("User action: {}", user_action.action);
        }
    }
);
```

### Range Filters

Filter based on numeric ranges:

```rust
// Filter by value range
bus.on_filtered::<MetricEvent>(
    Filter::range(|event| event.value, 10.0..100.0),
    |event| {
        println!("Metric in normal range: {}", event.value);
    }
);

// Filter by time range
bus.on_filtered::<UserAction>(
    Filter::time_range(
        |event| event.timestamp,
        start_time..end_time
    ),
    |event| {
        println!("Action within time window");
    }
);
```

### Pattern Filters

Filter using pattern matching:

```rust
// String pattern filter
bus.on_filtered::<LogEvent>(
    Filter::pattern(|event| &event.message, "ERROR*"),
    |event| {
        eprintln!("Error log: {}", event.message);
    }
);

// Regex filter
bus.on_filtered::<UserAction>(
    Filter::regex(|event| &event.action, r"^admin_\w+$"),
    |event| {
        println!("Admin action: {}", event.action);
    }
);
```

## Complex Filters

### Combinatorial Filters

Combine multiple filters using logical operations:

```rust
use eventrs::Filter;

// AND filter
let and_filter = Filter::all_of([
    Filter::field(|event: &UserAction| event.user_id > 1000),
    Filter::field(|event: &UserAction| event.action.contains("admin")),
    Filter::predicate(is_business_hours),
]);

bus.on_filtered::<UserAction>(and_filter, |event| {
    println!("Premium admin action during business hours");
});

// OR filter
let or_filter = Filter::any_of([
    Filter::field(|event: &UserAction| event.user_id == 1),  // Admin user
    Filter::field(|event: &UserAction| event.action == "emergency"),
]);

bus.on_filtered::<UserAction>(or_filter, |event| {
    println!("High priority action");
});

// NOT filter
let not_filter = Filter::not(
    Filter::field(|event: &UserAction| event.action.starts_with("test_"))
);

bus.on_filtered::<UserAction>(not_filter, |event| {
    println!("Production action: {}", event.action);
});
```

### Nested Filters

Create nested filter combinations:

```rust
// Complex nested filter
let complex_filter = Filter::all_of([
    Filter::field(|event: &UserAction| event.user_id > 100),
    Filter::any_of([
        Filter::field(|event: &UserAction| event.action == "login"),
        Filter::all_of([
            Filter::field(|event: &UserAction| event.action == "purchase"),
            Filter::field(|event: &UserAction| extract_amount(event) > 1000.0),
        ]),
    ]),
]);

bus.on_filtered::<UserAction>(complex_filter, |event| {
    println!("Significant user action: {}", event.action);
});
```

## Dynamic Filters

### Runtime Filter Configuration

Configure filters at runtime:

```rust
use std::sync::{Arc, RwLock};

// Shared filter configuration
let filter_config = Arc::new(RwLock::new(FilterConfig {
    min_user_id: 1000,
    allowed_actions: vec!["login".to_string(), "purchase".to_string()],
}));

// Dynamic filter
let config = filter_config.clone();
let dynamic_filter = Filter::custom(move |event: &UserAction| {
    let config = config.read().unwrap();
    event.user_id >= config.min_user_id && 
    config.allowed_actions.contains(&event.action)
});

bus.on_filtered::<UserAction>(dynamic_filter, |event| {
    println!("Dynamically filtered action: {}", event.action);
});

// Update filter configuration at runtime
{
    let mut config = filter_config.write().unwrap();
    config.min_user_id = 500;
    config.allowed_actions.push("admin_action".to_string());
}
```

### Conditional Filters

Apply filters conditionally:

```rust
// Filter based on system state
let conditional_filter = Filter::conditional(
    || system_is_under_load(),  // Condition check
    Filter::field(|event: &UserAction| event.action != "background_task"),  // Filter when true
    Filter::accept_all()  // Filter when false (accept everything)
);

bus.on_filtered::<UserAction>(conditional_filter, |event| {
    println!("Conditionally filtered action: {}", event.action);
});
```

## Filter Performance

### Efficient Filters

Design filters for optimal performance:

```rust
// Fast field access (prefer direct field access)
let fast_filter = Filter::field(|event: &UserAction| event.user_id > 1000);

// Avoid expensive operations in filters
let slow_filter = Filter::custom(|event: &UserAction| {
    // Avoid: expensive database lookup
    // database::is_premium_user(event.user_id)
    
    // Prefer: cached or computed values
    event.user_id > 1000  // Assuming premium users have high IDs
});
```

### Filter Caching

Cache filter results for repeated evaluations:

```rust
use std::collections::HashMap;
use std::sync::Mutex;

struct CachedFilter<T> {
    cache: Arc<Mutex<HashMap<u64, bool>>>,
    predicate: Box<dyn Fn(&T) -> bool + Send + Sync>,
}

impl<T> CachedFilter<T> {
    fn new<F>(predicate: F) -> Self 
    where 
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            predicate: Box::new(predicate),
        }
    }
    
    fn evaluate(&self, key: u64, event: &T) -> bool {
        let mut cache = self.cache.lock().unwrap();
        
        if let Some(&result) = cache.get(&key) {
            result
        } else {
            let result = (self.predicate)(event);
            cache.insert(key, result);
            result
        }
    }
}

// Usage
let cached_filter = CachedFilter::new(|event: &UserAction| {
    expensive_computation(event.user_id)
});

bus.on_filtered::<UserAction>(
    Filter::custom(move |event| cached_filter.evaluate(event.user_id, event)),
    |event| println!("Cached filter result for user {}", event.user_id)
);
```

## Filter Middleware

### Filter Chains

Create middleware-style filter chains:

```rust
trait FilterMiddleware<T> {
    fn filter(&self, event: &T, next: &dyn Fn(&T) -> bool) -> bool;
}

struct LoggingFilterMiddleware;

impl<T: std::fmt::Debug> FilterMiddleware<T> for LoggingFilterMiddleware {
    fn filter(&self, event: &T, next: &dyn Fn(&T) -> bool) -> bool {
        println!("Filtering event: {:?}", event);
        let result = next(event);
        println!("Filter result: {}", result);
        result
    }
}

struct TimingFilterMiddleware;

impl<T> FilterMiddleware<T> for TimingFilterMiddleware {
    fn filter(&self, event: &T, next: &dyn Fn(&T) -> bool) -> bool {
        let start = std::time::Instant::now();
        let result = next(event);
        let duration = start.elapsed();
        println!("Filter took {:?}", duration);
        result
    }
}
```

### Filter Composition

Compose filters in reusable ways:

```rust
struct FilterComposer<T> {
    filters: Vec<Box<dyn Fn(&T) -> bool + Send + Sync>>,
}

impl<T> FilterComposer<T> {
    fn new() -> Self {
        Self { filters: Vec::new() }
    }
    
    fn add_filter<F>(mut self, filter: F) -> Self 
    where 
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        self.filters.push(Box::new(filter));
        self
    }
    
    fn build_and(self) -> impl Fn(&T) -> bool {
        move |event| self.filters.iter().all(|f| f(event))
    }
    
    fn build_or(self) -> impl Fn(&T) -> bool {
        move |event| self.filters.iter().any(|f| f(event))
    }
}

// Usage
let composed_filter = FilterComposer::new()
    .add_filter(|event: &UserAction| event.user_id > 1000)
    .add_filter(|event: &UserAction| event.action != "test")
    .add_filter(|event: &UserAction| is_business_hours(event))
    .build_and();

bus.on_filtered::<UserAction>(
    Filter::custom(composed_filter),
    |event| println!("Composed filter passed")
);
```

## Global Filters

### Bus-Level Filters

Apply filters at the bus level:

```rust
// Global filter for all events
let mut bus = EventBus::builder()
    .with_global_filter(Filter::custom(|event: &dyn Event| {
        // Filter out test events globally
        !event.event_type_name().starts_with("Test")
    }))
    .build();

// All handlers will only receive non-test events
bus.on::<UserAction>(|event| {
    println!("Production event: {}", event.action);
});
```

### Filter Hierarchies

Create hierarchical filter systems:

```rust
// Create filtered sub-buses
let production_bus = bus.create_filtered_sub_bus(
    Filter::custom(|event: &dyn Event| {
        !event.event_type_name().contains("Debug")
    })
);

let admin_bus = production_bus.create_filtered_sub_bus(
    Filter::custom(|event: &dyn Event| {
        // Only admin events
        event.metadata().category() == Some("admin")
    })
);

// Handlers on admin_bus only receive admin, non-debug events
admin_bus.on::<AdminAction>(|event| {
    println!("Admin action: {}", event.action);
});
```

## Filter Testing

### Testing Filters

Test filter behavior in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_id_filter() {
        let filter = Filter::field(|event: &UserAction| event.user_id > 1000);
        
        let high_id_event = UserAction {
            user_id: 1001,
            action: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        
        let low_id_event = UserAction {
            user_id: 999,
            action: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        
        assert!(filter.evaluate(&high_id_event));
        assert!(!filter.evaluate(&low_id_event));
    }
    
    #[test]
    fn test_combined_filter() {
        let filter = Filter::all_of([
            Filter::field(|event: &UserAction| event.user_id > 1000),
            Filter::field(|event: &UserAction| event.action == "purchase"),
        ]);
        
        let valid_event = UserAction {
            user_id: 1001,
            action: "purchase".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        
        let invalid_event = UserAction {
            user_id: 999,
            action: "purchase".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        
        assert!(filter.evaluate(&valid_event));
        assert!(!filter.evaluate(&invalid_event));
    }
}
```

### Integration Testing with Filters

Test filtered handlers in the context of an event bus:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use eventrs::testing::*;
    
    #[test]
    fn test_filtered_handler() {
        let mut bus = TestEventBus::new();
        let captured_events = Arc::new(Mutex::new(Vec::new()));
        
        let events = captured_events.clone();
        bus.on_filtered::<UserAction>(
            Filter::field(|event| event.user_id > 1000),
            move |event| {
                events.lock().unwrap().push(event);
            }
        );
        
        // Emit events
        bus.emit(UserAction {
            user_id: 1001,  // Should pass filter
            action: "test1".to_string(),
            timestamp: std::time::SystemTime::now(),
        });
        
        bus.emit(UserAction {
            user_id: 999,   // Should be filtered out
            action: "test2".to_string(),
            timestamp: std::time::SystemTime::now(),
        });
        
        // Verify only filtered events were handled
        let events = captured_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, 1001);
    }
}
```

## Best Practices

### Filter Design
1. **Keep filters simple**: Complex filters impact performance
2. **Use appropriate filter types**: Field filters are faster than custom predicates
3. **Combine filters efficiently**: Use Filter::all_of() and Filter::any_of()
4. **Cache expensive computations**: Don't repeat expensive operations

### Performance
1. **Order filters by selectivity**: Most selective filters first
2. **Avoid I/O in filters**: Filters should be CPU-only operations
3. **Use static filters when possible**: Avoid dynamic allocations
4. **Profile filter performance**: Measure filter execution time

### Maintainability
1. **Name filters descriptively**: Use clear, meaningful names
2. **Document filter logic**: Explain complex filter conditions
3. **Test filters thoroughly**: Unit test filter behavior
4. **Use type-safe filters**: Leverage Rust's type system

### Error Handling
1. **Handle filter exceptions**: Filters should not panic
2. **Provide fallback behavior**: When filters fail
3. **Log filter errors**: For debugging purposes
4. **Validate filter configuration**: At startup when possible

Event filtering is a powerful feature that enables fine-grained control over event processing. Well-designed filters make your event system more efficient and easier to maintain.