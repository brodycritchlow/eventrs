# EventRS - High-Performance Event System for Rust

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/username/eventrs/actions)

EventRS is a high-performance, type-safe event system for Rust applications. It provides zero-cost abstractions for event-driven programming with support for both synchronous and asynchronous event handling, thread-safe event buses, and sophisticated event filtering capabilities.

## 🚀 Features

- **Type-Safe Events**: Compile-time type checking for all events and handlers
- **Zero-Cost Abstractions**: Minimal runtime overhead with compile-time optimizations
- **Async/Sync Support**: Handle events both synchronously and asynchronously ⚠️
- **Thread-Safe**: Built-in support for concurrent event handling
- **Event Filtering**: Sophisticated filtering system with custom predicates ⚠️
- **Priority System**: Control event execution order with priority-based handling ✅
- **Middleware Support**: Intercept and modify events with middleware chains ⚠️
- **Performance Optimized**: Designed for <10ns event processing latency

> **Legend**: ✅ Implemented | ⚠️ In Development | ❌ Planned

## 🏃 Quick Start

Add EventRS to your `Cargo.toml`:

```toml
[dependencies]
eventrs = { path = "./eventrs" }
```

### Basic Usage

```rust
use eventrs::prelude::*;

// Define your event
#[derive(Clone, Debug)]
struct UserLoggedIn {
    user_id: u64,
    username: String,
    timestamp: std::time::SystemTime,
}

impl Event for UserLoggedIn {}

// Create an event bus
let mut bus = EventBus::new();

// Register a handler
bus.on(|event: UserLoggedIn| {
    println!("User '{}' logged in!", event.username);
});

// Emit an event
bus.emit(UserLoggedIn {
    user_id: 123,
    username: "alice".to_string(),
    timestamp: std::time::SystemTime::now(),
}).expect("Failed to emit event");
```

### Priority-Based Handlers

```rust
use eventrs::prelude::*;

let mut bus = EventBus::new();

// High priority handler (executes first)
bus.on_with_priority(|event: UserLoggedIn| {
    println!("Security audit: User {} logged in", event.user_id);
}, Priority::High);

// Normal priority handler
bus.on(|event: UserLoggedIn| {
    println!("Welcome back, {}!", event.username);
});
```

## 📁 Project Structure

```
EventRSProposal/
├── eventrs/                 # Main library crate
│   ├── src/
│   │   ├── lib.rs           # Library entry point
│   │   ├── event.rs         # Event trait and core types
│   │   ├── event_bus.rs     # Synchronous event bus
│   │   ├── handler.rs       # Handler traits and implementations
│   │   ├── priority.rs      # Priority system
│   │   ├── metadata.rs      # Event metadata
│   │   └── error.rs         # Error types
│   ├── examples/            # Usage examples
│   └── tests/               # Integration tests
├── eventrs-derive/          # Derive macro crate
├── docs/                    # Comprehensive documentation
│   ├── architecture/        # Architecture documentation
│   ├── features/           # Feature documentation
│   ├── api/                # API reference
│   └── examples/           # Example documentation
└── README.md
```

## 🔧 Current Implementation Status

### ✅ **Implemented & Working**
- Core Event trait system with derive macro support
- Synchronous EventBus with handler registration and execution
- Priority-based handler execution
- Comprehensive error handling
- Event metadata system with rich context support
- Thread-safe handler storage
- Configuration system

### ⚠️ **In Development**
- AsyncEventBus for non-blocking event processing
- Event filtering system with predicates
- Middleware chain processing
- Concurrent handler execution
- Performance benchmarks

### 📋 **Planned Features**
- Advanced async/await integration
- Event sourcing capabilities
- Distributed event processing
- Performance monitoring and metrics
- Integration with popular frameworks

## 📖 Documentation

Comprehensive documentation is available in the [`docs/`](./docs/) directory:

- [Architecture Overview](./docs/architecture/overview.md)
- [Core Features](./docs/features/)
- [API Reference](./docs/api/)
- [Examples](./docs/examples/)

## 🧪 Examples

Run the basic example:

```bash
cargo run --example basic_example
```

Expected output:
```
🚀 EventRS Basic Example

📡 Emitting events...

👤 User 'alice' (ID: 123) logged in at SystemTime { ... }
💰 High-value order detected: $149.99
🛒 Order 456 created by user 123 for $149.99

✅ All events processed successfully!
📊 Total handlers registered: 3
```

## 🔬 Testing

Run all tests:

```bash
cargo test
```

Run documentation tests:

```bash
cargo test --doc
```

## ⚡ Performance Goals

EventRS is designed with performance as a first-class concern:

- **Event Emission**: Target <10ns per event
- **Handler Registration**: O(1) amortized complexity
- **Memory Overhead**: Minimal allocations in hot paths
- **Zero-Cost Abstractions**: Pay only for features you use

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guidelines](./docs/contributing/) for details on:

- Development setup
- Code style and standards
- Testing requirements
- Pull request process

## 📄 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🎯 Roadmap

See our [project documentation](./docs/) for detailed implementation plans and roadmap.

---

**Status**: Early Development - Core functionality working, advanced features in progress.