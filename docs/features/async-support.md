# Async Support

EventRS provides first-class support for asynchronous event handling with native async/await integration. This document covers all aspects of async event processing, from basic async handlers to advanced concurrent patterns.

## AsyncEventBus

The `AsyncEventBus` is the core component for async event handling:

```rust
use eventrs::{AsyncEventBus, Event};

#[derive(Event, Clone, Debug)]
struct DataProcessed {
    id: u64,
    data: Vec<u8>,
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::new();
    
    // Register async handler
    bus.on::<DataProcessed>(|event| async move {
        // Async processing
        let result = process_data_async(&event.data).await;
        save_to_database(event.id, result).await;
    }).await;
    
    // Emit event asynchronously
    bus.emit(DataProcessed {
        id: 123,
        data: vec![1, 2, 3, 4, 5],
    }).await;
}
```

## Async Handler Patterns

### Basic Async Handlers

Simple async handlers for I/O operations:

```rust
// Database operations
bus.on::<UserCreated>(|event| async move {
    let user = database::create_user(&event.username).await?;
    cache::set_user(user.id, &user).await?;
    Ok(())
}).await;

// HTTP API calls
bus.on::<OrderPlaced>(|event| async move {
    let response = http_client
        .post("/api/orders")
        .json(&event)
        .send()
        .await?;
    
    if response.status().is_success() {
        println!("Order {} processed", event.order_id);
    }
}).await;
```

### Concurrent Handler Execution

Execute multiple async operations concurrently:

```rust
bus.on::<UserLoggedIn>(|event| async move {
    // Execute multiple operations concurrently
    let (analytics_result, notification_result, audit_result) = tokio::join!(
        analytics::track_login(event.user_id),
        notifications::send_welcome_message(event.user_id),
        audit::log_login_event(event.user_id)
    );
    
    // Handle results
    if let Err(e) = analytics_result {
        eprintln!("Analytics failed: {}", e);
    }
    
    if let Err(e) = notification_result {
        eprintln!("Notification failed: {}", e);
    }
    
    if let Err(e) = audit_result {
        eprintln!("Audit logging failed: {}", e);
    }
}).await;
```

### Stream Processing

Process streams of data asynchronously:

```rust
use futures::StreamExt;

bus.on::<DataStreamStarted>(|event| async move {
    let mut stream = create_data_stream(event.source_id).await;
    
    while let Some(data) = stream.next().await {
        match data {
            Ok(chunk) => {
                process_data_chunk(chunk).await;
            }
            Err(e) => {
                eprintln!("Stream error: {}", e);
                break;
            }
        }
    }
}).await;
```

## Async Event Emission

### Basic Async Emission

Emit events and wait for handler completion:

```rust
// Emit and wait for all handlers to complete
bus.emit(DataProcessed {
    id: 456,
    data: vec![6, 7, 8, 9, 10],
}).await;

println!("All handlers completed");
```

### Fire-and-Forget Emission

Emit events without waiting:

```rust
// Emit without waiting (fire-and-forget)
bus.emit_and_forget(LogEvent {
    level: "INFO".to_string(),
    message: "User action completed".to_string(),
});

// Continue immediately
println!("Event emitted, continuing...");
```

### Batch Async Emission

Process multiple events efficiently:

```rust
let events = vec![
    DataProcessed { id: 1, data: vec![1] },
    DataProcessed { id: 2, data: vec![2] },
    DataProcessed { id: 3, data: vec![3] },
];

// Emit all events concurrently
bus.emit_batch_concurrent(events).await;

// Or emit sequentially
bus.emit_batch_sequential(events).await;
```

## Async Error Handling

### Fallible Async Handlers

Handle errors in async handlers:

```rust
use eventrs::AsyncHandlerResult;

bus.on_fallible::<DataProcessed>(|event| async move -> AsyncHandlerResult<()> {
    // Potentially failing async operation
    let processed = risky_async_operation(&event.data).await?;
    
    // Another potentially failing operation
    save_processed_data(event.id, processed).await?;
    
    Ok(())
}).await;
```

### Error Recovery Patterns

Implement sophisticated error recovery:

```rust
bus.on::<CriticalOperation>(|event| async move {
    let mut attempts = 0;
    let max_attempts = 3;
    
    loop {
        attempts += 1;
        
        match perform_critical_operation(&event).await {
            Ok(result) => {
                println!("Operation succeeded on attempt {}", attempts);
                break;
            }
            Err(e) if attempts < max_attempts => {
                eprintln!("Operation failed (attempt {}): {}", attempts, e);
                
                // Exponential backoff
                let delay = Duration::from_millis(100 * 2_u64.pow(attempts - 1));
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                eprintln!("Operation failed permanently after {} attempts: {}", attempts, e);
                // Handle permanent failure
                report_critical_failure(&event, e).await;
                break;
            }
        }
    }
}).await;
```

## Concurrency Control

### Limiting Concurrent Handlers

Control concurrency to prevent resource exhaustion:

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

// Create semaphore to limit concurrency
let semaphore = Arc::new(Semaphore::new(10)); // Max 10 concurrent handlers

bus.on::<ResourceIntensiveEvent>(move |event| {
    let semaphore = semaphore.clone();
    async move {
        // Acquire permit before processing
        let _permit = semaphore.acquire().await.unwrap();
        
        // Process event (limited concurrency)
        process_intensive_operation(&event).await;
        
        // Permit automatically released when _permit is dropped
    }
}).await;
```

### Rate Limiting

Implement rate limiting for async handlers:

```rust
use tokio::sync::RwLock;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

struct RateLimiter {
    requests: Arc<RwLock<VecDeque<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    async fn check_rate_limit(&self) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.write().await;
        
        // Remove old requests outside the window
        while let Some(&front) = requests.front() {
            if now.duration_since(front) > self.window {
                requests.pop_front();
            } else {
                break;
            }
        }
        
        // Check if we can add another request
        if requests.len() < self.max_requests {
            requests.push_back(now);
            true
        } else {
            false
        }
    }
}

let rate_limiter = Arc::new(RateLimiter {
    requests: Arc::new(RwLock::new(VecDeque::new())),
    max_requests: 100,
    window: Duration::from_secs(60),
});

bus.on::<ApiRequest>(move |event| {
    let rate_limiter = rate_limiter.clone();
    async move {
        if rate_limiter.check_rate_limit().await {
            // Process request
            handle_api_request(&event).await;
        } else {
            // Rate limit exceeded
            println!("Rate limit exceeded for API request");
        }
    }
}).await;
```

## Advanced Async Patterns

### Actor-like Pattern

Implement actor-like behavior with async handlers:

```rust
use tokio::sync::mpsc;

struct UserActor {
    user_id: u64,
    receiver: mpsc::Receiver<UserMessage>,
    state: UserState,
}

impl UserActor {
    async fn run(mut self) {
        while let Some(message) = self.receiver.recv().await {
            match message {
                UserMessage::UpdateProfile(data) => {
                    self.state.profile = data;
                    self.save_state().await;
                }
                UserMessage::AddFriend(friend_id) => {
                    self.state.friends.push(friend_id);
                    self.save_state().await;
                }
            }
        }
    }
    
    async fn save_state(&self) {
        database::save_user_state(self.user_id, &self.state).await;
    }
}

// Use with EventRS
bus.on::<UserEvent>(|event| async move {
    let (sender, receiver) = mpsc::channel(100);
    let actor = UserActor {
        user_id: event.user_id,
        receiver,
        state: load_user_state(event.user_id).await,
    };
    
    // Spawn actor
    tokio::spawn(actor.run());
    
    // Send message to actor
    sender.send(UserMessage::from(event)).await;
}).await;
```

### Async Pipeline Processing

Create processing pipelines with async stages:

```rust
async fn async_pipeline<T, U, V>(
    input: T,
    stage1: impl Fn(T) -> BoxFuture<'static, U>,
    stage2: impl Fn(U) -> BoxFuture<'static, V>,
) -> V {
    let intermediate = stage1(input).await;
    stage2(intermediate).await
}

bus.on::<RawDataEvent>(|event| async move {
    let result = async_pipeline(
        event.data,
        |data| Box::pin(async move { parse_data(data).await }),
        |parsed| Box::pin(async move { transform_data(parsed).await }),
    ).await;
    
    // Emit processed result
    bus.emit(ProcessedDataEvent { result }).await;
}).await;
```

## Async Testing

### Testing Async Handlers

Test async handlers effectively:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_async_handler() {
        let mut bus = AsyncEventBus::new();
        let result = Arc::new(Mutex::new(None));
        
        let result_clone = result.clone();
        bus.on::<TestEvent>(move |event| {
            let result = result_clone.clone();
            async move {
                // Simulate async work
                tokio::time::sleep(Duration::from_millis(10)).await;
                
                // Store result
                *result.lock().unwrap() = Some(event.value * 2);
            }
        }).await;
        
        // Emit test event
        bus.emit(TestEvent { value: 21 }).await;
        
        // Verify result
        assert_eq!(*result.lock().unwrap(), Some(42));
    }
    
    #[tokio::test]
    async fn test_concurrent_handlers() {
        let mut bus = AsyncEventBus::new();
        let counter = Arc::new(AtomicU32::new(0));
        
        // Register multiple handlers
        for _ in 0..10 {
            let counter = counter.clone();
            bus.on::<TestEvent>(move |_| {
                let counter = counter.clone();
                async move {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            }).await;
        }
        
        // Emit event
        bus.emit(TestEvent { value: 1 }).await;
        
        // All handlers should have executed
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }
}
```

### Mock Async Dependencies

Mock async dependencies for testing:

```rust
#[cfg(test)]
mod tests {
    use mockall::*;
    
    #[automock]
    trait AsyncDatabaseService {
        async fn save_data(&self, id: u64, data: &[u8]) -> Result<(), DatabaseError>;
        async fn load_data(&self, id: u64) -> Result<Vec<u8>, DatabaseError>;
    }
    
    #[tokio::test]
    async fn test_handler_with_mock() {
        let mut mock_db = MockAsyncDatabaseService::new();
        mock_db
            .expect_save_data()
            .with(eq(123), eq(vec![1, 2, 3]))
            .times(1)
            .returning(|_, _| Ok(()));
        
        let mock_db = Arc::new(mock_db);
        let mut bus = AsyncEventBus::new();
        
        let db = mock_db.clone();
        bus.on::<DataEvent>(move |event| {
            let db = db.clone();
            async move {
                db.save_data(event.id, &event.data).await.unwrap();
            }
        }).await;
        
        bus.emit(DataEvent {
            id: 123,
            data: vec![1, 2, 3],
        }).await;
    }
}
```

## Performance Optimization

### Async Handler Optimization

Optimize async handlers for performance:

```rust
// Use spawn_local for CPU-bound work
bus.on::<CpuIntensiveEvent>(|event| async move {
    let result = tokio::task::spawn_local(async move {
        cpu_intensive_computation(event.data)
    }).await.unwrap();
    
    store_result(result).await;
}).await;

// Pool connections for I/O
lazy_static! {
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

bus.on::<HttpRequestEvent>(|event| async move {
    let response = HTTP_CLIENT
        .get(&event.url)
        .send()
        .await?;
    
    process_response(response).await
}).await;
```

### Memory Management

Manage memory efficiently in async handlers:

```rust
// Avoid holding large data across await points
bus.on::<LargeDataEvent>(|event| async move {
    // Process data in chunks
    for chunk in event.data.chunks(1024) {
        process_chunk(chunk).await;
        // Chunk is dropped here, freeing memory
    }
}).await;

// Use Arc for shared data
let shared_config = Arc::new(load_config());

bus.on::<ConfigurableEvent>(move |event| {
    let config = shared_config.clone();
    async move {
        process_with_config(&event, &config).await;
    }
}).await;
```

## Best Practices

### Async Handler Design
1. **Keep handlers focused**: One async operation per handler when possible
2. **Handle cancellation**: Use cancellation tokens for long-running operations
3. **Avoid blocking**: Never use blocking operations in async handlers
4. **Use appropriate concurrency**: Don't spawn unnecessary tasks

### Error Handling
1. **Handle all error cases**: Async operations can fail in many ways
2. **Implement retry logic**: For transient failures
3. **Use timeouts**: Prevent handlers from hanging indefinitely
4. **Log async errors**: For debugging distributed systems

### Performance
1. **Pool resources**: Reuse connections and expensive objects
2. **Batch operations**: Group related async operations
3. **Use appropriate task spawning**: spawn vs spawn_local vs spawn_blocking
4. **Monitor performance**: Track async handler execution times

### Testing
1. **Test async behavior**: Verify concurrent execution
2. **Test error conditions**: Simulate failures and timeouts
3. **Mock async dependencies**: For reliable testing
4. **Use proper test frameworks**: tokio-test, async-std test utilities

Async support in EventRS enables building highly concurrent, efficient event-driven systems. Proper use of async patterns can significantly improve application performance and responsiveness.