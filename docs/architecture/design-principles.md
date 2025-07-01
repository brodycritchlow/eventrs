# Design Principles

EventRS is built on a foundation of carefully considered design principles that guide every architectural decision. These principles ensure the system is both powerful and maintainable while staying true to Rust's core values.

## 1. Type Safety First

**Principle**: Compile-time type safety over runtime flexibility

EventRS prioritizes compile-time guarantees over runtime dynamism. This means:

- **Events are strongly typed**: No `Any` types or runtime casting
- **Handlers are type-checked**: Mismatched handlers caught at compile time
- **Zero runtime type errors**: Impossible to register wrong handler types

```rust
// ✅ Compile-time type safety
#[derive(Event)]
struct UserLoggedIn { user_id: u64 }

bus.on::<UserLoggedIn>(|event| {
    // event.user_id is guaranteed to be u64
    println!("User {} logged in", event.user_id);
});

// ❌ This won't compile - type mismatch
bus.on::<UserLoggedIn>(|event: &UserLoggedOut| {
    // Compile error: expected UserLoggedIn, found UserLoggedOut
});
```

**Benefits**:
- Catch errors at compile time
- Better IDE support and refactoring
- Self-documenting code through types
- Zero runtime type checking overhead

## 2. Zero-Cost Abstractions

**Principle**: High-level abstractions with zero runtime overhead

Following Rust's zero-cost abstraction principle, EventRS ensures that abstractions compile down to optimal machine code:

- **Static dispatch**: All handler calls resolved at compile time
- **Monomorphization**: Generic code specialized for each type
- **Inline optimization**: Small handlers inlined automatically

```rust
// This high-level code...
bus.on::<UserAction>(|event| process_user_action(event));
bus.emit(UserAction { user_id: 123, action: "login".to_string() });

// ...compiles to direct function calls with no overhead
process_user_action(UserAction { user_id: 123, action: "login".to_string() });
```

**Performance guarantees**:
- Handler registration: O(1) amortized
- Event emission: Direct function calls when possible
- No heap allocation for simple events
- No runtime type information lookups

## 3. Memory Safety Without Garbage Collection

**Principle**: Leverage Rust's ownership system for safe, efficient memory management

EventRS uses Rust's ownership system to provide memory safety without runtime overhead:

- **Ownership-based lifetimes**: Clear ownership of events and handlers
- **Zero-copy when possible**: Minimize unnecessary clones
- **RAII cleanup**: Automatic resource cleanup on scope exit

```rust
// Memory-safe event handling
{
    let mut bus = EventBus::new();
    
    // Event is moved into the bus
    let event = UserAction { user_id: 123, action: "login".to_string() };
    bus.emit(event); // event moved, no longer accessible
    
    // Handlers automatically cleaned up when bus is dropped
} // bus dropped, all resources freed
```

## 4. Composability Over Monoliths

**Principle**: Small, focused components that combine to create complex behavior

EventRS favors composition of simple components over large, monolithic designs:

- **Modular middleware**: Chain multiple middleware components
- **Pluggable executors**: Swap execution strategies
- **Composable filters**: Combine simple filters into complex conditions

```rust
// Compose middleware
let bus = EventBus::builder()
    .with_middleware(LoggingMiddleware::new())
    .with_middleware(MetricsMiddleware::new())
    .with_middleware(ValidationMiddleware::new())
    .build();

// Compose filters
let filter = Filter::all_of([
    Filter::event_type::<UserAction>(),
    Filter::field(|u: &UserAction| u.user_id > 1000),
    Filter::custom(|_| is_business_hours()),
]);
```

## 5. Performance by Default

**Principle**: Optimize for the common case, provide options for edge cases

EventRS is designed to be fast by default while providing flexibility for special needs:

- **Fast path optimization**: Common operations are highly optimized
- **Lazy evaluation**: Work is deferred until necessary
- **Batch operations**: Efficient handling of multiple events

```rust
// Fast path: direct handler invocation
bus.on::<SimpleEvent>(|_| {}); // Inlined, zero overhead

// Optimized path: batch processing
bus.emit_batch([
    SimpleEvent { id: 1 },
    SimpleEvent { id: 2 },
    SimpleEvent { id: 3 },
]); // Single allocation, batch processing
```

## 6. Explicit Over Implicit

**Principle**: Make behavior explicit and predictable

EventRS favors explicit APIs over "magic" behavior:

- **Explicit registration**: Handlers must be explicitly registered
- **Clear ownership**: Event ownership is always clear
- **Predictable execution**: Handler execution order is deterministic

```rust
// ✅ Explicit and clear
bus.on::<UserAction>(handler);
bus.emit(event);

// ❌ Avoid implicit behavior
// No automatic handler discovery
// No hidden global state
// No magic configuration
```

## 7. Fail Fast and Loud

**Principle**: Surface problems as early as possible

EventRS is designed to make problems visible quickly:

- **Compile-time errors**: Most issues caught during compilation
- **Explicit error handling**: Errors are returned, not hidden
- **Debug information**: Rich debugging support in development builds

```rust
// Compile-time error for type mismatches
bus.on::<UserLoggedIn>(|event: UserLoggedOut| {}); // Won't compile

// Explicit error handling
match bus.try_emit(event) {
    Ok(()) => println!("Event handled successfully"),
    Err(e) => eprintln!("Event handling failed: {}", e),
}
```

## 8. Async-First Design

**Principle**: Native async support, not an afterthought

EventRS is designed with async in mind from the ground up:

- **Native async handlers**: First-class async/await support
- **Non-blocking operations**: Async operations don't block the bus
- **Concurrent execution**: Multiple handlers can run concurrently

```rust
// Async is first-class, not bolted on
async_bus.on::<DataReceived>(|event| async move {
    let processed = process_data(event.data).await;
    save_to_database(processed).await;
});

await async_bus.emit(DataReceived { data: vec![1, 2, 3] });
```

## 9. Testability

**Principle**: Make testing easy and comprehensive

EventRS is designed to be easily testable:

- **Dependency injection**: Easy to mock components
- **Deterministic behavior**: Predictable execution for tests
- **Test utilities**: Built-in testing helpers

```rust
#[cfg(test)]
mod tests {
    use eventrs::testing::*;
    
    #[test]
    fn test_user_login_handler() {
        let mut bus = TestEventBus::new();
        let handler = MockHandler::new();
        
        bus.on::<UserLoggedIn>(handler.clone());
        bus.emit(UserLoggedIn { user_id: 123 });
        
        assert_eq!(handler.call_count(), 1);
        assert_eq!(handler.last_event().user_id, 123);
    }
}
```

## 10. Documentation and Discoverability

**Principle**: Self-documenting code with comprehensive documentation

EventRS prioritizes clear, discoverable APIs:

- **Self-documenting types**: Types that explain their purpose
- **Comprehensive documentation**: Every public API documented
- **Examples in docs**: Real-world usage examples

```rust
/// Handles user authentication events
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::*;
/// 
/// #[derive(Event)]
/// struct UserLoggedIn {
///     user_id: u64,
///     timestamp: SystemTime,
/// }
/// 
/// let mut bus = EventBus::new();
/// bus.on::<UserLoggedIn>(|event| {
///     println!("User {} logged in", event.user_id);
/// });
/// ```
pub struct AuthenticationHandler;
```

## Trade-offs and Decisions

### Chosen Trade-offs

1. **Compile-time safety vs Runtime flexibility**
   - **Chosen**: Compile-time safety
   - **Reason**: Catches more bugs, better performance

2. **Zero-cost abstractions vs Dynamic dispatch**
   - **Chosen**: Zero-cost abstractions
   - **Reason**: Better performance, clearer code

3. **Explicit APIs vs Convenience**
   - **Chosen**: Explicit APIs
   - **Reason**: Predictable behavior, easier debugging

### Future Considerations

These principles guide current development but may evolve:

- **Runtime flexibility**: May add optional dynamic features
- **Convenience APIs**: May add convenience wrappers over explicit APIs
- **Performance trade-offs**: May offer different performance profiles

## Principle Validation

Each major change to EventRS is evaluated against these principles:

1. Does it maintain type safety?
2. Does it preserve zero-cost abstractions?
3. Does it follow Rust's ownership model?
4. Is it composable with existing components?
5. Does it optimize for common cases?
6. Is the behavior explicit and predictable?
7. Does it fail fast when something goes wrong?
8. Does it support async naturally?
9. Is it easily testable?
10. Is it well-documented and discoverable?

These principles ensure EventRS remains a robust, performant, and maintainable event system that feels natural to Rust developers.