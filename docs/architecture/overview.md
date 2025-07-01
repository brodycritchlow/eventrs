# Architecture Overview

EventRS is built on a foundation of type safety, performance, and flexibility. This document outlines the core architectural concepts and design decisions that make EventRS both powerful and efficient.

## Core Components

### Event Bus

The **Event Bus** is the central component that manages event distribution. EventRS provides two main bus implementations:

- **`EventBus`** - Synchronous event handling
- **`AsyncEventBus`** - Asynchronous event handling with `async/await` support

```rust
// Synchronous bus
let mut sync_bus = EventBus::new();

// Asynchronous bus  
let mut async_bus = AsyncEventBus::new();
```

### Events

Events are the data structures that flow through the system. They must implement the `Event` trait, which can be derived:

```rust
#[derive(Event, Clone, Debug)]
struct UserAction {
    user_id: u64,
    action: String,
    timestamp: SystemTime,
}
```

**Key Properties:**
- **Type Safety**: Events are strongly typed at compile time
- **Zero-Cost**: No runtime type checking or boxing
- **Cloneable**: Events must be cloneable for distribution to multiple handlers

### Handlers

Handlers are functions or closures that process events. EventRS supports multiple handler types:

- **Sync Handlers**: `Fn(Event) -> ()`
- **Async Handlers**: `Fn(Event) -> Future<Output = ()>`
- **Fallible Handlers**: `Fn(Event) -> Result<(), Error>`

```rust
// Sync handler
bus.on::<UserAction>(|event| {
    println!("User {} performed: {}", event.user_id, event.action);
});

// Async handler
bus.on::<UserAction>(|event| async move {
    save_to_database(event).await;
});
```

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Application Layer                    │
├─────────────────────────────────────────────────────────────┤
│                          Event Bus                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Router    │  │ Middleware  │  │    Handler Store    │  │
│  │             │  │   Chain     │  │                     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                      Core Abstractions                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Events    │  │  Handlers   │  │      Filters        │  │
│  │   (Types)   │  │ (Functions) │  │   (Predicates)      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Runtime Components                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Executor    │  │ Scheduler   │  │   Memory Pool       │  │
│  │ (Sync/Async)│  │ (Priority)  │  │   (Allocation)      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Event Flow

The event lifecycle follows this pattern:

1. **Event Creation**: Application creates an event instance
2. **Emission**: Event is emitted to the bus via `emit()`
3. **Routing**: Bus determines which handlers should receive the event
4. **Filtering**: Event passes through registered filters
5. **Middleware**: Pre-processing middleware transforms the event
6. **Execution**: Handlers execute in priority order
7. **Post-processing**: Post-processing middleware runs
8. **Completion**: Event handling completes

```
Event Created → Bus.emit() → Routing → Filtering → Middleware
                                                       ↓
Completion ← Post-processing ← Handler Execution ← Pre-processing
```

## Thread Safety Model

EventRS provides thread safety through multiple strategies:

### Single-Threaded Optimization
- **Lock-free**: No synchronization overhead for single-threaded use
- **Direct dispatch**: Handlers called directly without queuing

### Multi-threaded Safety
- **Arc<Mutex<>>**: Shared bus instances use atomic reference counting
- **Channel-based**: Optional channel-based event distribution
- **Thread-local**: Per-thread event buses for high-performance scenarios

```rust
// Single-threaded (no locks)
let mut bus = EventBus::new();

// Multi-threaded (thread-safe)
let bus = Arc::new(Mutex::new(EventBus::new()));

// Channel-based distribution
let (sender, receiver) = EventBus::channel();
```

## Memory Management

EventRS minimizes allocations through several techniques:

### Zero-Copy Event Distribution
- Events are cloned only when necessary
- Handlers receive references when possible
- Smart pointer optimization for large events

### Handler Storage Optimization
- **Type erasure**: Handlers stored without generic overhead
- **Inline storage**: Small handlers stored inline
- **Arena allocation**: Batch allocation for handler collections

### Event Queuing
- **Ring buffers**: Fixed-size, allocation-free queuing
- **Batch processing**: Process multiple events per allocation
- **Memory pools**: Reuse event containers

## Performance Characteristics

### Time Complexity
- **Event Emission**: O(h) where h = number of handlers for event type
- **Handler Registration**: O(1) amortized
- **Event Filtering**: O(f) where f = number of active filters

### Space Complexity
- **Handler Storage**: O(h) where h = total registered handlers
- **Event Buffering**: O(b) where b = buffer size (configurable)
- **Type Information**: O(t) where t = number of unique event types

### Optimizations
- **Compile-time dispatch**: Zero-cost handler calling
- **Inline handlers**: Small handlers inlined at call site
- **Branch prediction**: Hot paths optimized for common cases

## Extensibility Points

EventRS provides several extension mechanisms:

### Custom Event Buses
Implement `EventBusImpl` trait for custom routing logic:

```rust
trait EventBusImpl {
    fn emit<E: Event>(&mut self, event: E);
    fn on<E: Event, H: Handler<E>>(&mut self, handler: H);
}
```

### Middleware System
Create custom middleware for cross-cutting concerns:

```rust
trait Middleware {
    fn pre_process<E: Event>(&self, event: &mut E) -> Result<(), Error>;
    fn post_process<E: Event>(&self, event: &E, result: &HandlerResult);
}
```

### Custom Executors
Implement custom execution strategies:

```rust
trait Executor {
    fn execute<E: Event>(&self, event: E, handlers: &[Handler<E>]);
}
```

## Design Principles

1. **Type Safety First**: Compile-time guarantees over runtime flexibility
2. **Zero-Cost Abstractions**: Pay only for what you use
3. **Performance by Default**: Fast path optimized for common cases
4. **Composability**: Small, focused components that work together
5. **Extensibility**: Clear extension points without sacrificing performance
6. **Memory Efficiency**: Minimal allocations and memory overhead

This architecture enables EventRS to provide both high performance and rich functionality while maintaining Rust's safety guarantees.