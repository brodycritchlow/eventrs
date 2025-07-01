# Middleware System

EventRS provides a powerful middleware system that allows you to intercept, modify, and augment event processing. This document covers middleware creation, chaining, and advanced middleware patterns.

## Basic Middleware

### Simple Middleware

Create basic middleware to intercept events:

```rust
use eventrs::{Middleware, MiddlewareContext, Event};

struct LoggingMiddleware;

impl<E: Event> Middleware<E> for LoggingMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        println!("Processing event: {:?}", event);
        
        // Continue to next middleware or handlers
        let result = context.next(event);
        
        match &result {
            Ok(_) => println!("Event processed successfully"),
            Err(e) => println!("Event processing failed: {}", e),
        }
        
        result
    }
}

// Register middleware
let mut bus = EventBus::builder()
    .with_middleware(LoggingMiddleware)
    .build();
```

### Conditional Middleware

Create middleware that conditionally processes events:

```rust
struct ConditionalMiddleware<F> {
    condition: F,
    name: String,
}

impl<F, E> Middleware<E> for ConditionalMiddleware<F>
where
    F: Fn(&E) -> bool + Send + Sync,
    E: Event,
{
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        if (self.condition)(event) {
            println!("Middleware {} active for event", self.name);
            
            // Apply middleware logic
            self.process_event(event)?;
            
            // Continue processing
            context.next(event)
        } else {
            // Skip this middleware
            context.next(event)
        }
    }
}

// Usage
let conditional_middleware = ConditionalMiddleware {
    condition: |event: &UserAction| event.user_id > 1000,
    name: "premium_user_middleware".to_string(),
};

bus.add_middleware(conditional_middleware);
```

## Pre/Post Processing Middleware

### Pre-processing Middleware

Modify events before they reach handlers:

```rust
struct EventEnrichmentMiddleware;

impl<E: Event> Middleware<E> for EventEnrichmentMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        // Clone and modify event
        let mut enriched_event = event.clone();
        
        // Add timestamp if not present
        if enriched_event.metadata().timestamp().is_none() {
            enriched_event.set_timestamp(std::time::SystemTime::now());
        }
        
        // Add source information
        enriched_event.set_source("enrichment_middleware");
        
        // Continue with enriched event
        context.next(&enriched_event)
    }
}
```

### Post-processing Middleware

Process events after handlers complete:

```rust
struct AuditMiddleware {
    audit_log: Arc<Mutex<Vec<AuditEntry>>>,
}

impl<E: Event> Middleware<E> for AuditMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        let start_time = std::time::Instant::now();
        
        // Execute handlers
        let result = context.next(event);
        
        let duration = start_time.elapsed();
        
        // Log audit information
        let audit_entry = AuditEntry {
            event_type: event.event_type_name().to_string(),
            timestamp: std::time::SystemTime::now(),
            duration,
            success: result.is_ok(),
            handler_count: context.handler_count(),
        };
        
        self.audit_log.lock().unwrap().push(audit_entry);
        
        result
    }
}
```

## Middleware Chains

### Sequential Middleware

Chain multiple middleware components:

```rust
let bus = EventBus::builder()
    .with_middleware(AuthenticationMiddleware)
    .with_middleware(AuthorizationMiddleware)
    .with_middleware(ValidationMiddleware)
    .with_middleware(LoggingMiddleware)
    .with_middleware(MetricsMiddleware)
    .build();

// Execution order:
// 1. AuthenticationMiddleware
// 2. AuthorizationMiddleware  
// 3. ValidationMiddleware
// 4. LoggingMiddleware
// 5. MetricsMiddleware
// 6. Event handlers
// 7. MetricsMiddleware (post-processing)
// 8. LoggingMiddleware (post-processing)
// 9. ValidationMiddleware (post-processing)
// 10. AuthorizationMiddleware (post-processing)
// 11. AuthenticationMiddleware (post-processing)
```

### Branching Middleware

Create middleware that can branch execution:

```rust
struct RoutingMiddleware;

impl<E: Event> Middleware<E> for RoutingMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        match event.metadata().category() {
            Some("critical") => {
                // Route to critical path
                context.route_to("critical_handlers")?;
            }
            Some("analytics") => {
                // Route to analytics path
                context.route_to("analytics_handlers")?;
            }
            _ => {
                // Default path
                context.next(event)?;
            }
        }
        
        Ok(())
    }
}
```

## Specialized Middleware

### Authentication Middleware

Verify event authenticity:

```rust
struct AuthenticationMiddleware {
    secret_key: String,
}

impl AuthenticationMiddleware {
    fn verify_signature(&self, event: &dyn Event) -> bool {
        if let Some(signature) = event.metadata().signature() {
            // Verify HMAC signature
            let expected = self.compute_hmac(event);
            signature == expected
        } else {
            false
        }
    }
    
    fn compute_hmac(&self, event: &dyn Event) -> String {
        // HMAC computation logic
        format!("hmac_{}_{}", self.secret_key, event.event_type_name())
    }
}

impl<E: Event> Middleware<E> for AuthenticationMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        if self.verify_signature(event) {
            context.next(event)
        } else {
            Err(MiddlewareError::AuthenticationFailed)
        }
    }
}
```

### Rate Limiting Middleware

Control event processing rate:

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct RateLimitingMiddleware {
    limits: Arc<RwLock<HashMap<String, RateLimit>>>,
    default_limit: u32,
    window_duration: Duration,
}

struct RateLimit {
    count: u32,
    window_start: Instant,
}

impl<E: Event> Middleware<E> for RateLimitingMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        let key = self.get_rate_limit_key(event);
        
        if self.check_rate_limit(&key)? {
            context.next(event)
        } else {
            Err(MiddlewareError::RateLimitExceeded)
        }
    }
}

impl RateLimitingMiddleware {
    fn check_rate_limit(&self, key: &str) -> Result<bool, MiddlewareError> {
        let mut limits = self.limits.write().unwrap();
        let now = Instant::now();
        
        let rate_limit = limits.entry(key.to_string()).or_insert(RateLimit {
            count: 0,
            window_start: now,
        });
        
        // Reset window if expired
        if now.duration_since(rate_limit.window_start) > self.window_duration {
            rate_limit.count = 0;
            rate_limit.window_start = now;
        }
        
        // Check limit
        if rate_limit.count < self.default_limit {
            rate_limit.count += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn get_rate_limit_key(&self, event: &dyn Event) -> String {
        // Create rate limit key based on event properties
        format!("{}_{}", 
            event.event_type_name(),
            event.metadata().source().unwrap_or("unknown")
        )
    }
}
```

### Caching Middleware

Cache event processing results:

```rust
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

struct CachingMiddleware<E> {
    cache: Arc<RwLock<HashMap<u64, CacheEntry>>>,
    ttl: Duration,
    _phantom: PhantomData<E>,
}

struct CacheEntry {
    result: MiddlewareResult,
    timestamp: Instant,
}

impl<E: Event + Hash> Middleware<E> for CachingMiddleware<E> {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        let cache_key = self.compute_cache_key(event);
        
        // Check cache first
        if let Some(cached_result) = self.get_cached_result(cache_key) {
            return cached_result;
        }
        
        // Process event
        let result = context.next(event);
        
        // Cache result
        self.cache_result(cache_key, &result);
        
        result
    }
}

impl<E: Event + Hash> CachingMiddleware<E> {
    fn compute_cache_key(&self, event: &E) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        event.hash(&mut hasher);
        hasher.finish()
    }
    
    fn get_cached_result(&self, key: u64) -> Option<MiddlewareResult> {
        let cache = self.cache.read().unwrap();
        if let Some(entry) = cache.get(&key) {
            if entry.timestamp.elapsed() < self.ttl {
                return Some(entry.result.clone());
            }
        }
        None
    }
    
    fn cache_result(&self, key: u64, result: &MiddlewareResult) {
        let mut cache = self.cache.write().unwrap();
        cache.insert(key, CacheEntry {
            result: result.clone(),
            timestamp: Instant::now(),
        });
    }
}
```

## Async Middleware

### Async Middleware Implementation

Create middleware for async event processing:

```rust
use eventrs::AsyncMiddleware;

struct AsyncLoggingMiddleware;

#[async_trait]
impl<E: Event> AsyncMiddleware<E> for AsyncLoggingMiddleware {
    async fn handle(&self, event: &E, context: &mut AsyncMiddlewareContext<E>) -> MiddlewareResult {
        println!("Async processing event: {:?}", event);
        
        // Async pre-processing
        self.log_to_database(event).await?;
        
        // Continue to next middleware or handlers
        let result = context.next(event).await;
        
        // Async post-processing
        match &result {
            Ok(_) => self.log_success(event).await?,
            Err(e) => self.log_error(event, e).await?,
        }
        
        result
    }
}

impl AsyncLoggingMiddleware {
    async fn log_to_database(&self, event: &dyn Event) -> Result<(), MiddlewareError> {
        // Async database logging
        database::log_event(event).await
            .map_err(|e| MiddlewareError::DatabaseError(e))
    }
    
    async fn log_success(&self, event: &dyn Event) -> Result<(), MiddlewareError> {
        // Async success logging
        analytics::track_success(event).await
            .map_err(|e| MiddlewareError::AnalyticsError(e))
    }
    
    async fn log_error(&self, event: &dyn Event, error: &MiddlewareError) -> Result<(), MiddlewareError> {
        // Async error logging
        error_tracking::report_error(event, error).await
            .map_err(|e| MiddlewareError::ErrorTrackingError(e))
    }
}
```

### Async Circuit Breaker Middleware

Implement circuit breaker pattern:

```rust
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

struct AsyncCircuitBreakerMiddleware {
    failure_count: AtomicU32,
    failure_threshold: u32,
    is_open: AtomicBool,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    recovery_timeout: Duration,
}

#[async_trait]
impl<E: Event> AsyncMiddleware<E> for AsyncCircuitBreakerMiddleware {
    async fn handle(&self, event: &E, context: &mut AsyncMiddlewareContext<E>) -> MiddlewareResult {
        // Check if circuit breaker is open
        if self.is_circuit_open().await {
            return Err(MiddlewareError::CircuitBreakerOpen);
        }
        
        // Process event
        let result = context.next(event).await;
        
        // Update circuit breaker state based on result
        match &result {
            Ok(_) => self.record_success().await,
            Err(_) => self.record_failure().await,
        }
        
        result
    }
}

impl AsyncCircuitBreakerMiddleware {
    async fn is_circuit_open(&self) -> bool {
        if !self.is_open.load(Ordering::SeqCst) {
            return false;
        }
        
        // Check if recovery timeout has elapsed
        let last_failure = self.last_failure_time.lock().unwrap();
        if let Some(last_failure_time) = *last_failure {
            if last_failure_time.elapsed() > self.recovery_timeout {
                // Try to close circuit
                self.is_open.store(false, Ordering::SeqCst);
                self.failure_count.store(0, Ordering::SeqCst);
                return false;
            }
        }
        
        true
    }
    
    async fn record_success(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.is_open.store(false, Ordering::SeqCst);
    }
    
    async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        
        if failures >= self.failure_threshold {
            self.is_open.store(true, Ordering::SeqCst);
            *self.last_failure_time.lock().unwrap() = Some(Instant::now());
        }
    }
}
```

## Middleware Configuration

### Dynamic Middleware Configuration

Configure middleware at runtime:

```rust
struct ConfigurableMiddleware {
    config: Arc<RwLock<MiddlewareConfig>>,
}

struct MiddlewareConfig {
    enabled: bool,
    log_level: String,
    rate_limit: u32,
    cache_ttl: Duration,
}

impl<E: Event> Middleware<E> for ConfigurableMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
        let config = self.config.read().unwrap();
        
        if !config.enabled {
            return context.next(event);
        }
        
        // Apply middleware based on configuration
        if config.log_level == "debug" {
            println!("Debug: Processing event {:?}", event);
        }
        
        // Rate limiting based on config
        if !self.check_rate_limit(config.rate_limit) {
            return Err(MiddlewareError::RateLimitExceeded);
        }
        
        context.next(event)
    }
}

// Update configuration at runtime
fn update_middleware_config(middleware: &ConfigurableMiddleware, new_config: MiddlewareConfig) {
    *middleware.config.write().unwrap() = new_config;
}
```

### Conditional Middleware Loading

Load middleware based on conditions:

```rust
struct MiddlewareBuilder {
    middleware_stack: Vec<Box<dyn Middleware<dyn Event>>>,
}

impl MiddlewareBuilder {
    fn new() -> Self {
        Self {
            middleware_stack: Vec::new(),
        }
    }
    
    fn add_if<M, F>(mut self, condition: F, middleware: M) -> Self
    where
        M: Middleware<dyn Event> + 'static,
        F: FnOnce() -> bool,
    {
        if condition() {
            self.middleware_stack.push(Box::new(middleware));
        }
        self
    }
    
    fn build(self) -> Vec<Box<dyn Middleware<dyn Event>>> {
        self.middleware_stack
    }
}

// Usage
let middleware = MiddlewareBuilder::new()
    .add_if(|| cfg!(debug_assertions), LoggingMiddleware)
    .add_if(|| std::env::var("ENABLE_METRICS").is_ok(), MetricsMiddleware)
    .add_if(|| is_production_environment(), AuthenticationMiddleware)
    .build();

let bus = EventBus::builder()
    .with_middleware_stack(middleware)
    .build();
```

## Testing Middleware

### Unit Testing Middleware

Test middleware in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use eventrs::testing::*;
    
    #[test]
    fn test_logging_middleware() {
        let middleware = LoggingMiddleware;
        let mut context = MockMiddlewareContext::new();
        
        // Setup expectations
        context.expect_next()
            .times(1)
            .returning(|_| Ok(()));
        
        let event = TestEvent { id: 123 };
        let result = middleware.handle(&event, &mut context);
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_rate_limiting_middleware() {
        let middleware = RateLimitingMiddleware::new(5, Duration::from_secs(60));
        let mut context = MockMiddlewareContext::new();
        
        context.expect_next()
            .returning(|_| Ok(()));
        
        let event = TestEvent { id: 123 };
        
        // First 5 requests should succeed
        for _ in 0..5 {
            assert!(middleware.handle(&event, &mut context).is_ok());
        }
        
        // 6th request should be rate limited
        assert!(middleware.handle(&event, &mut context).is_err());
    }
}
```

### Integration Testing

Test middleware within the event bus:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_middleware_chain() {
        let mut execution_order = Vec::new();
        
        let bus = EventBus::builder()
            .with_middleware(RecordingMiddleware::new("first", &mut execution_order))
            .with_middleware(RecordingMiddleware::new("second", &mut execution_order))
            .build();
        
        bus.on::<TestEvent>(|_| {
            execution_order.push("handler".to_string());
        });
        
        bus.emit(TestEvent { id: 123 });
        
        assert_eq!(execution_order, vec![
            "first_pre",
            "second_pre", 
            "handler",
            "second_post",
            "first_post"
        ]);
    }
}
```

## Best Practices

### Middleware Design
1. **Keep middleware focused**: One responsibility per middleware
2. **Make middleware composable**: Should work well in chains
3. **Handle errors gracefully**: Don't let middleware failures break the chain
4. **Use appropriate middleware types**: Sync vs async based on operations

### Performance
1. **Minimize middleware overhead**: Keep processing lightweight
2. **Use caching wisely**: Cache expensive operations, not cheap ones
3. **Consider ordering**: Put most selective middleware first
4. **Profile middleware performance**: Measure impact on event processing

### Error Handling
1. **Define clear error types**: Use specific middleware error types
2. **Implement fallback behavior**: When middleware fails
3. **Log middleware errors**: For debugging and monitoring
4. **Test error scenarios**: Verify error handling works correctly

### Security
1. **Validate middleware inputs**: Don't trust event data
2. **Implement authentication**: For sensitive operations
3. **Use rate limiting**: Prevent abuse
4. **Audit middleware operations**: Log security-relevant actions

The middleware system in EventRS provides powerful capabilities for cross-cutting concerns, enabling clean separation of business logic from infrastructure concerns while maintaining high performance and flexibility.