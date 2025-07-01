# EventRS - High-Performance Event System for Rust

[![Crates.io](https://img.shields.io/crates/v/eventrs.svg)](https://crates.io/crates/eventrs)
[![Documentation](https://docs.rs/eventrs/badge.svg)](https://docs.rs/eventrs)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/username/eventrs/workflows/CI/badge.svg)](https://github.com/username/eventrs/actions)

EventRS is a high-performance, type-safe event system for Rust applications. It provides zero-cost abstractions for event-driven programming with support for both synchronous and asynchronous event handling, thread-safe event buses, and sophisticated event filtering capabilities.

## 🚀 Features

- **Type-Safe Events**: Compile-time type checking for all events and handlers
- **Zero-Cost Abstractions**: Minimal runtime overhead with compile-time optimizations
- **Async/Sync Support**: Handle events both synchronously and asynchronously
- **Thread-Safe**: Built-in support for concurrent event handling
- **Event Filtering**: Sophisticated filtering system with custom predicates
- **Priority System**: Control event execution order with priority-based handling
- **Middleware Support**: Intercept and modify events with middleware chains
- **Performance Optimized**: Benchmarked against other Rust event systems

## 🏃 Quick Start

Add EventRS to your `Cargo.toml`:

```toml
[dependencies]
eventrs = "0.1.0"
```

### Basic Usage

```rust
use eventrs::{Event, EventBus, Handler};

// Define your event
#[derive(Event, Clone)]
struct UserLoggedIn {
    user_id: u64,
    timestamp: std::time::SystemTime,
}

// Create an event bus
let mut bus = EventBus::new();

// Register a handler
bus.on::<UserLoggedIn>(|event| {
    println!("User {} logged in at {:?}", event.user_id, event.timestamp);
});

// Emit an event
bus.emit(UserLoggedIn {
    user_id: 123,
    timestamp: std::time::SystemTime::now(),
});
```

### Async Events

```rust
use eventrs::{AsyncEventBus, Event};
use tokio;

#[derive(Event, Clone)]
struct DataProcessed {
    data: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::new();
    
    bus.on::<DataProcessed>(|event| async move {
        // Async processing
        process_data(&event.data).await;
    }).await;
    
    bus.emit(DataProcessed {
        data: vec![1, 2, 3, 4, 5],
    }).await;
}
```

## 📖 Documentation

- **[Getting Started](guides/getting-started.md)** - Installation and basic setup
- **[Architecture Overview](architecture/overview.md)** - System design and concepts
- **[Feature Guide](features/)** - Detailed feature documentation
- **[API Reference](api/)** - Complete API documentation
- **[Examples](examples/)** - Practical usage examples
- **[Performance](benchmarks/performance-overview.md)** - Benchmarks and optimization

## 🎯 Use Cases

EventRS is perfect for:

- **Web Applications**: Handle HTTP requests, user actions, and system events
- **Game Development**: Manage game state changes, player actions, and system events
- **IoT Systems**: Process sensor data and device state changes
- **Microservices**: Implement event-driven architecture patterns
- **Real-time Applications**: Handle streaming data and live updates

## 🔥 Performance

EventRS is designed for high performance with minimal overhead:

- **< 10ns** per event emission (simple events)
- **Zero allocations** for basic event handling
- **Lock-free** for single-threaded scenarios
- **Efficient batching** for high-throughput scenarios

See our [benchmarks](benchmarks/performance-overview.md) for detailed performance analysis.

## 🆚 Comparison

| Feature | EventRS | tokio-events | bus | crossbeam-channel |
|---------|---------|--------------|-----|-------------------|
| Type Safety | ✅ | ❌ | ❌ | ✅ |
| Async Support | ✅ | ✅ | ❌ | ❌ |
| Zero-Cost | ✅ | ❌ | ✅ | ❌ |
| Filtering | ✅ | ❌ | ❌ | ❌ |
| Middleware | ✅ | ❌ | ❌ | ❌ |
| Thread-Safe | ✅ | ✅ | ✅ | ✅ |

## 🛠️ Development Status

EventRS is currently in active development. See our [roadmap](roadmap.md) for planned features and milestones.

### Current Version: 0.1.0 (Alpha)

- ⚠️ **Alpha Software**: API may change before 1.0 release
- 🔄 **Active Development**: Regular updates and improvements
- 📝 **Feedback Welcome**: Issues and suggestions appreciated

## 🤝 Contributing

We welcome contributions! Please see our [contributing guide](contributing/development.md) for details on:

- Setting up the development environment
- Running tests and benchmarks
- Submitting issues and pull requests
- Code style and conventions

## 📄 License

EventRS is dual-licensed under the MIT and Apache 2.0 licenses.

## 🔗 Links

- [Crates.io](https://crates.io/crates/eventrs)
- [Documentation](https://docs.rs/eventrs)
- [GitHub](https://github.com/username/eventrs)
- [Issues](https://github.com/username/eventrs/issues)
- [Discussions](https://github.com/username/eventrs/discussions)

---

*Built with ❤️ for the Rust community*