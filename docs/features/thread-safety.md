# Thread Safety

EventRS provides comprehensive thread safety features that enable safe concurrent event processing across multiple threads. This document covers thread-safe event buses, concurrent handler execution, and synchronization strategies.

## Thread-Safe Event Bus

### Shared Event Bus

Create thread-safe event buses that can be shared across threads:

```rust
use eventrs::{ThreadSafeEventBus, Event};
use std::sync::Arc;
use std::thread;

#[derive(Event, Clone, Debug)]
struct LogEvent {
    level: String,
    message: String,
    thread_id: String,
}

// Create thread-safe event bus
let bus = Arc::new(ThreadSafeEventBus::new());

// Register handler (thread-safe)
bus.on::<LogEvent>(|event| {
    println!("[{}] {}: {}", event.thread_id, event.level, event.message);
});

// Share bus across threads
let handles: Vec<_> = (0..4).map(|i| {
    let bus = bus.clone();
    thread::spawn(move || {
        bus.emit(LogEvent {
            level: "INFO".to_string(),
            message: format!("Message from thread {}", i),
            thread_id: format!("thread-{}", i),
        });
    })
}).collect();

// Wait for all threads to complete
for handle in handles {
    handle.join().unwrap();
}
```

### Lock-Free Single Producer

For high-performance scenarios with a single producer:

```rust
use eventrs::LockFreeEventBus;
use crossbeam_channel::{unbounded, Receiver};

// Create lock-free bus (single producer, multiple consumers)
let (bus, event_receiver) = LockFreeEventBus::new();

// Producer thread
let producer_bus = bus.clone();
let producer_handle = thread::spawn(move || {
    for i in 0..1000 {
        producer_bus.emit(LogEvent {
            level: "INFO".to_string(),
            message: format!("Event {}", i),
            thread_id: "producer".to_string(),
        });
    }
});

// Consumer threads
let consumer_handles: Vec<_> = (0..3).map(|id| {
    let receiver = event_receiver.clone();
    thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            println!("Consumer {} received: {:?}", id, event);
        }
    })
}).collect();

producer_handle.join().unwrap();
for handle in consumer_handles {
    handle.join().unwrap();
}
```

## Concurrent Handler Execution

### Parallel Handler Processing

Execute handlers concurrently when safe:

```rust
use eventrs::{ConcurrentEventBus, ConcurrencyMode};

// Configure concurrent execution
let bus = ConcurrentEventBus::builder()
    .with_concurrency_mode(ConcurrencyMode::Parallel)
    .with_max_concurrent_handlers(8)
    .build();

// Register handlers that can run in parallel
bus.on::<LogEvent>(|event| {
    // Thread-safe handler - can run concurrently
    println!("Handler 1: {}", event.message);
});

bus.on::<LogEvent>(|event| {
    // Another thread-safe handler
    println!("Handler 2: {}", event.message);
});

// Emit event - handlers execute in parallel
bus.emit(LogEvent {
    level: "INFO".to_string(),
    message: "Parallel processing".to_string(),
    thread_id: "main".to_string(),
});
```

### Sequential Handler Processing

Ensure handlers execute sequentially when needed:

```rust
// Configure sequential execution for specific event types
bus.configure_concurrency::<LogEvent>(ConcurrencyMode::Sequential);

// Or use sequential handlers explicitly
bus.on_sequential::<LogEvent>(|event| {
    // This handler will not run concurrently with other handlers
    // for the same event
    update_shared_state(&event);
});
```

## Synchronization Primitives

### Mutex-Protected Shared State

Share state safely between handlers:

```rust
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Shared state protected by mutex
let shared_state = Arc::new(Mutex::new(HashMap::<String, u32>::new()));

// Handler that modifies shared state
let state = shared_state.clone();
bus.on::<LogEvent>(move |event| {
    let mut state = state.lock().unwrap();
    let counter = state.entry(event.level.clone()).or_insert(0);
    *counter += 1;
    println!("Log level {} count: {}", event.level, counter);
});

// Another handler that reads shared state
let state = shared_state.clone();
bus.on::<LogEvent>(move |event| {
    let state = state.lock().unwrap();
    let total: u32 = state.values().sum();
    println!("Total events processed: {}", total);
});
```

### RwLock for Read-Heavy Workloads

Use RwLock when reads are more common than writes:

```rust
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

// Configuration that's read frequently, written rarely
let config = Arc::new(RwLock::new(HashMap::<String, String>::new()));

// Handler that reads configuration
let config = config.clone();
bus.on::<LogEvent>(move |event| {
    let config = config.read().unwrap();
    if let Some(log_level) = config.get("log_level") {
        if should_log(&event.level, log_level) {
            println!("Logging: {}", event.message);
        }
    }
});

// Handler that updates configuration (rare)
let config = config.clone();
bus.on::<ConfigUpdateEvent>(move |event| {
    let mut config = config.write().unwrap();
    config.insert(event.key.clone(), event.value.clone());
    println!("Configuration updated: {} = {}", event.key, event.value);
});
```

### Atomic Operations

Use atomic operations for simple shared counters:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// Atomic counter for event statistics
let event_counter = Arc::new(AtomicU64::new(0));

// Handler that increments counter
let counter = event_counter.clone();
bus.on::<LogEvent>(move |_| {
    counter.fetch_add(1, Ordering::SeqCst);
});

// Handler that reads counter
let counter = event_counter.clone();
bus.on::<StatsRequestEvent>(move |_| {
    let count = counter.load(Ordering::SeqCst);
    println!("Total events processed: {}", count);
});
```

## Channel-Based Communication

### MPSC Channels

Use channels for thread communication:

```rust
use crossbeam_channel::{unbounded, Sender, Receiver};

// Create channel for cross-thread communication
let (sender, receiver): (Sender<LogEvent>, Receiver<LogEvent>) = unbounded();

// Handler that sends events to another thread
bus.on::<LogEvent>(move |event| {
    if event.level == "ERROR" {
        sender.send(event).unwrap();
    }
});

// Separate thread for error processing
let error_processor = thread::spawn(move || {
    while let Ok(error_event) = receiver.recv() {
        process_error(&error_event);
        send_alert(&error_event);
    }
});
```

### Work Stealing

Implement work-stealing pattern for load balancing:

```rust
use crossbeam_deque::{Injector, Stealer, Worker};
use std::sync::Arc;

// Work-stealing setup
let injector = Arc::new(Injector::new());
let workers: Vec<Worker<LogEvent>> = (0..4).map(|_| Worker::new_fifo()).collect();
let stealers: Vec<Stealer<LogEvent>> = workers.iter().map(|w| w.stealer()).collect();

// Handler that distributes work
let injector = injector.clone();
bus.on::<LogEvent>(move |event| {
    injector.push(event);
});

// Worker threads
let worker_handles: Vec<_> = workers.into_iter().enumerate().map(|(id, worker)| {
    let stealers = stealers.clone();
    let injector = injector.clone();
    
    thread::spawn(move || {
        loop {
            // Try to get work from own queue
            if let Some(event) = worker.pop() {
                process_event(id, event);
                continue;
            }
            
            // Try to get work from global injector
            if let Some(event) = injector.steal() {
                process_event(id, event);
                continue;
            }
            
            // Try to steal work from other workers
            for stealer in &stealers {
                if let Some(event) = stealer.steal() {
                    process_event(id, event);
                    break;
                }
            }
            
            // No work available, sleep briefly
            thread::sleep(std::time::Duration::from_millis(1));
        }
    })
}).collect();
```

## Async Thread Safety

### Async Mutex

Use async-aware synchronization primitives:

```rust
use tokio::sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock};
use std::sync::Arc;

// Async-safe shared state
let shared_state = Arc::new(AsyncMutex::new(HashMap::<String, u32>::new()));

// Async handler with shared state
let state = shared_state.clone();
async_bus.on::<LogEvent>(move |event| {
    let state = state.clone();
    async move {
        let mut state = state.lock().await;
        let counter = state.entry(event.level.clone()).or_insert(0);
        *counter += 1;
        println!("Async log level {} count: {}", event.level, counter);
    }
}).await;
```

### Actor Pattern with Async

Implement actor pattern for thread-safe async operations:

```rust
use tokio::sync::mpsc;

// Actor for handling log events
struct LogActor {
    receiver: mpsc::Receiver<LogEvent>,
    state: HashMap<String, u32>,
}

impl LogActor {
    async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            self.handle_event(event).await;
        }
    }
    
    async fn handle_event(&mut self, event: LogEvent) {
        let counter = self.state.entry(event.level.clone()).or_insert(0);
        *counter += 1;
        
        // Async operations
        save_to_database(&event).await;
        update_metrics(&event).await;
    }
}

// Create actor and channel
let (sender, receiver) = mpsc::channel(1000);
let actor = LogActor {
    receiver,
    state: HashMap::new(),
};

// Spawn actor
tokio::spawn(actor.run());

// Handler that sends to actor
async_bus.on::<LogEvent>(move |event| {
    let sender = sender.clone();
    async move {
        sender.send(event).await.unwrap();
    }
}).await;
```

## Thread Pool Integration

### Custom Thread Pool

Integrate with custom thread pools:

```rust
use threadpool::ThreadPool;
use std::sync::Arc;

// Create thread pool
let pool = Arc::new(ThreadPool::new(8));

// Handler that executes in thread pool
let pool = pool.clone();
bus.on::<LogEvent>(move |event| {
    let pool = pool.clone();
    pool.execute(move || {
        // CPU-intensive work in thread pool
        process_log_event_intensive(&event);
    });
});
```

### Rayon Integration

Use Rayon for parallel processing:

```rust
use rayon::prelude::*;

// Handler that processes events in parallel
bus.on::<BatchLogEvent>(|event| {
    event.logs.par_iter().for_each(|log| {
        process_single_log(log);
    });
});
```

## Testing Thread Safety

### Concurrent Testing

Test thread safety with concurrent access:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    
    #[test]
    fn test_concurrent_event_emission() {
        let bus = Arc::new(ThreadSafeEventBus::new());
        let counter = Arc::new(AtomicU64::new(0));
        
        // Register handler
        let counter_clone = counter.clone();
        bus.on::<LogEvent>(move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        // Synchronize thread start
        let barrier = Arc::new(Barrier::new(10));
        
        // Spawn threads that emit events concurrently
        let handles: Vec<_> = (0..10).map(|i| {
            let bus = bus.clone();
            let barrier = barrier.clone();
            
            thread::spawn(move || {
                barrier.wait();
                
                for j in 0..100 {
                    bus.emit(LogEvent {
                        level: "INFO".to_string(),
                        message: format!("Thread {} Event {}", i, j),
                        thread_id: format!("thread-{}", i),
                    });
                }
            })
        }).collect();
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify all events were processed
        assert_eq!(counter.load(Ordering::SeqCst), 1000);
    }
    
    #[test]
    fn test_handler_registration_thread_safety() {
        let bus = Arc::new(ThreadSafeEventBus::new());
        
        // Register handlers from multiple threads
        let handles: Vec<_> = (0..5).map(|i| {
            let bus = bus.clone();
            thread::spawn(move || {
                bus.on::<LogEvent>(move |event| {
                    println!("Handler {} processed: {}", i, event.message);
                });
            })
        }).collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Emit event to test all handlers work
        bus.emit(LogEvent {
            level: "INFO".to_string(),
            message: "Test message".to_string(),
            thread_id: "test".to_string(),
        });
    }
}
```

### Race Condition Testing

Test for race conditions:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    #[test]
    fn test_no_race_conditions() {
        let bus = Arc::new(ThreadSafeEventBus::new());
        let shared_flag = Arc::new(AtomicBool::new(false));
        let race_detected = Arc::new(AtomicBool::new(false));
        
        // Handler that might race
        let flag = shared_flag.clone();
        let race_flag = race_detected.clone();
        bus.on::<LogEvent>(move |_| {
            // Try to detect race condition
            if flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                // Simulate work
                thread::sleep(std::time::Duration::from_nanos(100));
                
                // Check if another thread modified the flag
                if !flag.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                    race_flag.store(true, Ordering::SeqCst);
                }
            }
        });
        
        // Emit many events concurrently
        let handles: Vec<_> = (0..100).map(|i| {
            let bus = bus.clone();
            thread::spawn(move || {
                bus.emit(LogEvent {
                    level: "INFO".to_string(),
                    message: format!("Race test {}", i),
                    thread_id: format!("thread-{}", i),
                });
            })
        }).collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify no race condition was detected
        assert!(!race_detected.load(Ordering::SeqCst));
    }
}
```

## Best Practices

### Thread Safety Design
1. **Choose appropriate synchronization**: Mutex vs RwLock vs Atomic
2. **Minimize lock contention**: Keep critical sections small
3. **Avoid deadlocks**: Always acquire locks in the same order
4. **Use channels for communication**: Instead of shared mutable state

### Performance
1. **Prefer lock-free algorithms**: When possible for hot paths
2. **Use atomic operations**: For simple counters and flags
3. **Consider work-stealing**: For load balancing across threads
4. **Profile lock contention**: Identify synchronization bottlenecks

### Error Handling
1. **Handle poisoned mutexes**: When threads panic while holding locks
2. **Use timeouts**: For lock acquisition to avoid deadlocks
3. **Implement graceful degradation**: When synchronization fails
4. **Monitor thread health**: Detect and recover from thread failures

### Testing
1. **Test concurrent scenarios**: Multiple threads accessing shared resources
2. **Use stress testing**: High load to expose race conditions
3. **Test error conditions**: Thread panics, lock failures
4. **Use thread sanitizers**: Tools like ThreadSanitizer to detect issues

Thread safety in EventRS enables building robust, concurrent event-driven systems that can scale across multiple CPU cores while maintaining data integrity and system reliability.