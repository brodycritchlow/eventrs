# Async Event Examples

This document demonstrates practical async event patterns using EventRS's `AsyncEventBus`.

## Basic Async Event Handling

### Simple Async Operations

```rust
use eventrs::{AsyncEventBus, Event};
use tokio;
use std::time::Duration;

#[derive(Event, Clone, Debug)]
struct EmailRequest {
    to: String,
    subject: String,
    body: String,
    priority: EmailPriority,
}

#[derive(Clone, Debug)]
enum EmailPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::new();
    
    // Async email sender
    bus.on::<EmailRequest>(|event| async move {
        println!("📧 Sending email to {}: {}", event.to, event.subject);
        
        // Simulate email sending delay
        let delay = match event.priority {
            EmailPriority::Urgent => Duration::from_millis(100),
            EmailPriority::High => Duration::from_millis(300),
            EmailPriority::Normal => Duration::from_millis(500),
            EmailPriority::Low => Duration::from_millis(1000),
        };
        
        tokio::time::sleep(delay).await;
        
        // Simulate potential failure
        if event.to.contains("invalid") {
            eprintln!("❌ Failed to send email to {}", event.to);
        } else {
            println!("✅ Email sent successfully to {}", event.to);
        }
    }).await;
    
    // Email analytics tracker
    bus.on::<EmailRequest>(|event| async move {
        println!("📊 Tracking email analytics for: {}", event.subject);
        
        // Simulate analytics API call
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        println!("📈 Analytics recorded for email to {}", event.to);
    }).await;
    
    // Send test emails
    let emails = vec![
        EmailRequest {
            to: "user1@example.com".to_string(),
            subject: "Welcome!".to_string(),
            body: "Welcome to our service".to_string(),
            priority: EmailPriority::Normal,
        },
        EmailRequest {
            to: "user2@example.com".to_string(),
            subject: "Urgent: Security Alert".to_string(),
            body: "Please verify your account".to_string(),
            priority: EmailPriority::Urgent,
        },
        EmailRequest {
            to: "invalid@domain".to_string(),
            subject: "Test".to_string(),
            body: "This will fail".to_string(),
            priority: EmailPriority::Low,
        },
    ];
    
    for email in emails {
        bus.emit(email).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

## Concurrent Processing

### Parallel Async Handlers

```rust
use eventrs::{AsyncEventBus, Event};
use tokio;
use std::time::{Duration, Instant};

#[derive(Event, Clone, Debug)]
struct DataProcessingRequest {
    request_id: u64,
    data: Vec<u8>,
    processing_type: ProcessingType,
}

#[derive(Clone, Debug)]
enum ProcessingType {
    Analysis,
    Transformation,
    Validation,
    Storage,
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::new();
    
    // Data analysis handler
    bus.on::<DataProcessingRequest>(|event| async move {
        if matches!(event.processing_type, ProcessingType::Analysis) {
            println!("🔍 Starting analysis for request {}", event.request_id);
            
            // Simulate CPU-intensive analysis
            tokio::task::spawn_blocking(move || {
                std::thread::sleep(Duration::from_millis(200));
                println!("✅ Analysis completed for request {}", event.request_id);
            }).await.unwrap();
        }
    }).await;
    
    // Data transformation handler
    bus.on::<DataProcessingRequest>(|event| async move {
        if matches!(event.processing_type, ProcessingType::Transformation) {
            println!("🔄 Starting transformation for request {}", event.request_id);
            
            // Simulate async transformation
            tokio::time::sleep(Duration::from_millis(150)).await;
            
            println!("✅ Transformation completed for request {}", event.request_id);
        }
    }).await;
    
    // Data validation handler
    bus.on::<DataProcessingRequest>(|event| async move {
        if matches!(event.processing_type, ProcessingType::Validation) {
            println!("✔️  Starting validation for request {}", event.request_id);
            
            // Simulate validation with potential failure
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            if event.data.len() < 10 {
                eprintln!("❌ Validation failed for request {}: insufficient data", event.request_id);
            } else {
                println!("✅ Validation passed for request {}", event.request_id);
            }
        }
    }).await;
    
    // Storage handler
    bus.on::<DataProcessingRequest>(|event| async move {
        if matches!(event.processing_type, ProcessingType::Storage) {
            println!("💾 Starting storage for request {}", event.request_id);
            
            // Simulate database operation
            tokio::time::sleep(Duration::from_millis(300)).await;
            
            println!("✅ Data stored for request {}", event.request_id);
        }
    }).await;
    
    // Process multiple requests concurrently
    let requests = vec![
        DataProcessingRequest {
            request_id: 1,
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            processing_type: ProcessingType::Analysis,
        },
        DataProcessingRequest {
            request_id: 2,
            data: vec![11, 12, 13, 14, 15],
            processing_type: ProcessingType::Transformation,
        },
        DataProcessingRequest {
            request_id: 3,
            data: vec![21, 22], // This will fail validation
            processing_type: ProcessingType::Validation,
        },
        DataProcessingRequest {
            request_id: 4,
            data: vec![31, 32, 33, 34, 35, 36, 37, 38, 39, 40],
            processing_type: ProcessingType::Storage,
        },
    ];
    
    let start = Instant::now();
    
    // Emit all requests concurrently
    let futures: Vec<_> = requests.into_iter()
        .map(|req| bus.emit(req))
        .collect();
    
    futures::future::join_all(futures).await;
    
    println!("🏁 All requests processed in {:?}", start.elapsed());
}
```

## Database Operations

### Async Database Event Handlers

```rust
use eventrs::{AsyncEventBus, Event};
use tokio;
use std::time::Duration;
use thiserror::Error;

#[derive(Event, Clone, Debug)]
struct UserCreated {
    user_id: u64,
    username: String,
    email: String,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug)]
struct UserDeleted {
    user_id: u64,
    deleted_by: u64,
    reason: String,
}

#[derive(Error, Debug)]
enum DatabaseError {
    #[error("Connection timeout")]
    ConnectionTimeout,
    #[error("Constraint violation: {message}")]
    ConstraintViolation { message: String },
    #[error("Record not found")]
    NotFound,
}

// Mock database operations
struct DatabaseService;

impl DatabaseService {
    async fn save_user(&self, user_id: u64, username: &str, email: &str) -> Result<(), DatabaseError> {
        println!("💾 Saving user {} to database...", username);
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Simulate constraint violation for duplicate emails
        if email.contains("duplicate") {
            return Err(DatabaseError::ConstraintViolation {
                message: "Email already exists".to_string(),
            });
        }
        
        println!("✅ User {} saved to database", username);
        Ok(())
    }
    
    async fn delete_user(&self, user_id: u64) -> Result<(), DatabaseError> {
        println!("🗑️  Deleting user {} from database...", user_id);
        tokio::time::sleep(Duration::from_millis(80)).await;
        
        // Simulate user not found
        if user_id == 999 {
            return Err(DatabaseError::NotFound);
        }
        
        println!("✅ User {} deleted from database", user_id);
        Ok(())
    }
    
    async fn update_user_cache(&self, user_id: u64, operation: &str) -> Result<(), DatabaseError> {
        println!("🔄 Updating cache for user {} ({})", user_id, operation);
        tokio::time::sleep(Duration::from_millis(50)).await;
        println!("✅ Cache updated for user {}", user_id);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::new();
    let db = DatabaseService;
    
    // User creation handler
    bus.on_fallible::<UserCreated>(move |event| async move {
        // Primary database save
        db.save_user(event.user_id, &event.username, &event.email).await?;
        
        // Update cache
        db.update_user_cache(event.user_id, "created").await?;
        
        Ok::<(), DatabaseError>(())
    }).await;
    
    // User creation analytics
    bus.on::<UserCreated>(|event| async move {
        println!("📊 Recording user creation analytics for {}", event.username);
        
        // Simulate analytics API call
        tokio::time::sleep(Duration::from_millis(30)).await;
        
        println!("📈 Analytics recorded for user {}", event.username);
    }).await;
    
    // Welcome email for new users
    bus.on::<UserCreated>(|event| async move {
        println!("📧 Sending welcome email to {}", event.email);
        
        // Simulate email service call
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        if event.email.contains("@invalid") {
            eprintln!("❌ Failed to send welcome email to {}", event.email);
        } else {
            println!("✅ Welcome email sent to {}", event.email);
        }
    }).await;
    
    // User deletion handler
    let db_clone = DatabaseService;
    bus.on_fallible::<UserDeleted>(move |event| async move {
        println!("🔍 Verifying deletion permissions for user {}", event.user_id);
        
        // Delete from database
        db_clone.delete_user(event.user_id).await?;
        
        // Update cache
        db_clone.update_user_cache(event.user_id, "deleted").await?;
        
        Ok::<(), DatabaseError>(())
    }).await;
    
    // Audit logging for deletions
    bus.on::<UserDeleted>(|event| async move {
        println!("📝 Logging user deletion: user_id={}, deleted_by={}, reason='{}'", 
            event.user_id, 
            event.deleted_by, 
            event.reason
        );
        
        // Simulate audit log write
        tokio::time::sleep(Duration::from_millis(40)).await;
        
        println!("✅ Deletion audit logged");
    }).await;
    
    // Error handler for database operations
    bus.on_error(|error, event_type| async move {
        eprintln!("❌ Database error in {}: {}", event_type, error);
        
        // Could send to error tracking service
        println!("📤 Error reported to monitoring system");
    });
    
    // Test user operations
    println!("=== Creating Users ===");
    
    bus.emit(UserCreated {
        user_id: 1,
        username: "alice".to_string(),
        email: "alice@example.com".to_string(),
        timestamp: std::time::SystemTime::now(),
    }).await;
    
    bus.emit(UserCreated {
        user_id: 2,
        username: "bob".to_string(),
        email: "duplicate@example.com".to_string(), // This will fail
        timestamp: std::time::SystemTime::now(),
    }).await;
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    println!("\n=== Deleting Users ===");
    
    bus.emit(UserDeleted {
        user_id: 1,
        deleted_by: 10,
        reason: "User requested account deletion".to_string(),
    }).await;
    
    bus.emit(UserDeleted {
        user_id: 999, // This will fail - user not found
        deleted_by: 10,
        reason: "Cleanup invalid account".to_string(),
    }).await;
    
    tokio::time::sleep(Duration::from_millis(500)).await;
}
```

## Stream Processing

### Processing Event Streams

```rust
use eventrs::{AsyncEventBus, Event};
use tokio;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use std::time::Duration;

#[derive(Event, Clone, Debug)]
struct SensorData {
    sensor_id: String,
    temperature: f64,
    humidity: f64,
    pressure: f64,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug)]
struct AggregatedMetrics {
    sensor_id: String,
    avg_temperature: f64,
    avg_humidity: f64,
    sample_count: u32,
    time_window: Duration,
}

#[derive(Event, Clone, Debug)]
struct Alert {
    alert_type: String,
    sensor_id: String,
    message: String,
    severity: AlertSeverity,
}

#[derive(Clone, Debug)]
enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

// Simulated sensor data stream
async fn create_sensor_stream() -> ReceiverStream<SensorData> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    
    tokio::spawn(async move {
        let sensor_ids = vec!["temp_01", "temp_02", "temp_03"];
        let mut counter = 0;
        
        loop {
            for sensor_id in &sensor_ids {
                let data = SensorData {
                    sensor_id: sensor_id.to_string(),
                    temperature: 20.0 + (counter as f64 * 0.5) + (rand::random::<f64>() * 10.0),
                    humidity: 50.0 + (rand::random::<f64>() * 30.0),
                    pressure: 1013.25 + (rand::random::<f64>() * 50.0),
                    timestamp: std::time::SystemTime::now(),
                };
                
                if tx.send(data).await.is_err() {
                    return;
                }
                
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            counter += 1;
            
            if counter > 20 {
                break;
            }
        }
    });
    
    ReceiverStream::new(rx)
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::new();
    
    // Real-time sensor data processor
    bus.on::<SensorData>(|event| async move {
        println!("📡 Sensor {}: temp={:.1}°C, humidity={:.1}%, pressure={:.1}hPa", 
            event.sensor_id,
            event.temperature,
            event.humidity,
            event.pressure
        );
        
        // Check for alerts
        if event.temperature > 35.0 {
            println!("🌡️  High temperature alert for sensor {}: {:.1}°C", 
                event.sensor_id, 
                event.temperature
            );
        }
        
        if event.humidity < 20.0 || event.humidity > 90.0 {
            println!("💧 Humidity alert for sensor {}: {:.1}%", 
                event.sensor_id, 
                event.humidity
            );
        }
    }).await;
    
    // Data aggregation handler
    bus.on::<SensorData>(|event| async move {
        // In a real implementation, you'd maintain sliding windows per sensor
        println!("📊 Aggregating data for sensor {}", event.sensor_id);
        
        // Simulate aggregation processing
        tokio::time::sleep(Duration::from_millis(10)).await;
    }).await;
    
    // Data storage handler
    bus.on::<SensorData>(|event| async move {
        // Simulate database write
        println!("💾 Storing sensor data: {} at {:?}", 
            event.sensor_id, 
            event.timestamp
        );
        
        tokio::time::sleep(Duration::from_millis(20)).await;
    }).await;
    
    // Alert handler
    bus.on::<Alert>(|event| async move {
        let severity_emoji = match event.severity {
            AlertSeverity::Info => "ℹ️",
            AlertSeverity::Warning => "⚠️",
            AlertSeverity::Critical => "🚨",
        };
        
        println!("{} ALERT [{}]: {} - {}", 
            severity_emoji,
            format!("{:?}", event.severity).to_uppercase(),
            event.alert_type,
            event.message
        );
        
        // Simulate alert notification
        if matches!(event.severity, AlertSeverity::Critical) {
            println!("📱 Sending push notification for critical alert");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }).await;
    
    // Create and process sensor data stream
    let sensor_stream = create_sensor_stream().await;
    
    println!("🚀 Starting sensor data processing...\n");
    
    // Process stream events
    bus.emit_stream(sensor_stream.map(|sensor_data| {
        // You could transform the data here if needed
        sensor_data
    })).await;
    
    println!("\n✅ Stream processing completed");
    
    // Simulate some aggregated metrics
    bus.emit(AggregatedMetrics {
        sensor_id: "temp_01".to_string(),
        avg_temperature: 28.5,
        avg_humidity: 65.2,
        sample_count: 20,
        time_window: Duration::from_secs(60),
    }).await;
    
    // Simulate some alerts
    bus.emit(Alert {
        alert_type: "temperature_threshold".to_string(),
        sensor_id: "temp_02".to_string(),
        message: "Temperature exceeded 35°C threshold".to_string(),
        severity: AlertSeverity::Warning,
    }).await;
    
    bus.emit(Alert {
        alert_type: "sensor_offline".to_string(),
        sensor_id: "temp_03".to_string(),
        message: "Sensor has not reported data for 5 minutes".to_string(),
        severity: AlertSeverity::Critical,
    }).await;
}
```

## Error Handling and Resilience

### Retry Logic and Circuit Breaker

```rust
use eventrs::{AsyncEventBus, Event};
use tokio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Event, Clone, Debug)]
struct ApiRequest {
    request_id: u64,
    endpoint: String,
    payload: String,
    retry_count: u32,
}

#[derive(Error, Debug)]
enum ApiError {
    #[error("Network timeout")]
    Timeout,
    #[error("Server error: {status}")]
    ServerError { status: u16 },
    #[error("Rate limited")]
    RateLimited,
    #[error("Service unavailable")]
    ServiceUnavailable,
}

// Mock API service with failure simulation
struct ApiService {
    failure_count: Arc<AtomicU32>,
}

impl ApiService {
    fn new() -> Self {
        Self {
            failure_count: Arc::new(AtomicU32::new(0)),
        }
    }
    
    async fn make_request(&self, endpoint: &str, payload: &str) -> Result<String, ApiError> {
        println!("🌐 Making API request to {}", endpoint);
        
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Simulate various failure scenarios
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst);
        
        match failures % 10 {
            0..=2 => {
                // First few requests fail with server errors
                Err(ApiError::ServerError { status: 500 })
            }
            3..=4 => {
                // Some requests are rate limited
                Err(ApiError::RateLimited)
            }
            5 => {
                // One request times out
                Err(ApiError::Timeout)
            }
            6..=7 => {
                // Service unavailable
                Err(ApiError::ServiceUnavailable)
            }
            _ => {
                // Finally succeed
                Ok(format!("Success: processed {} bytes", payload.len()))
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::new();
    let api_service = Arc::new(ApiService::new());
    
    // API request handler with retry logic
    let service = api_service.clone();
    bus.on_fallible::<ApiRequest>(move |event| {
        let service = service.clone();
        async move {
            let max_retries = 3;
            let mut last_error = None;
            
            for attempt in 0..=max_retries {
                if attempt > 0 {
                    let delay = Duration::from_millis(100 * 2_u64.pow(attempt));
                    println!("⏳ Retrying in {:?} (attempt {} of {})", delay, attempt + 1, max_retries + 1);
                    tokio::time::sleep(delay).await;
                }
                
                match service.make_request(&event.endpoint, &event.payload).await {
                    Ok(response) => {
                        println!("✅ Request {} succeeded: {}", event.request_id, response);
                        return Ok(());
                    }
                    Err(e) => {
                        println!("❌ Request {} attempt {} failed: {}", 
                            event.request_id, 
                            attempt + 1, 
                            e
                        );
                        
                        last_error = Some(e);
                        
                        // Don't retry on certain errors
                        match &last_error {
                            Some(ApiError::RateLimited) => {
                                // For rate limiting, wait longer
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            Err(last_error.unwrap())
        }
    }).await;
    
    // Request analytics
    bus.on::<ApiRequest>(|event| async move {
        println!("📊 Tracking API request {} to {}", event.request_id, event.endpoint);
        
        // Simulate analytics processing
        tokio::time::sleep(Duration::from_millis(20)).await;
    }).await;
    
    // Error tracking
    bus.on_error(|error, event_type| async move {
        eprintln!("📈 Error in {}: {}", event_type, error);
        
        // In a real app, you'd send this to an error tracking service
        println!("📤 Error reported to monitoring service");
    });
    
    // Circuit breaker state
    let circuit_open = Arc::new(AtomicU32::new(0));
    let last_failure_time = Arc::new(tokio::sync::Mutex::new(None::<Instant>));
    
    // Circuit breaker middleware
    let circuit_state = circuit_open.clone();
    let failure_time = last_failure_time.clone();
    bus.on::<ApiRequest>(move |event| {
        let circuit_state = circuit_state.clone();
        let failure_time = failure_time.clone();
        
        async move {
            let failures = circuit_state.load(Ordering::SeqCst);
            
            // Check if circuit is open
            if failures >= 5 {
                let mut last_failure = failure_time.lock().await;
                
                if let Some(last_fail_time) = *last_failure {
                    // Check if enough time has passed to try again
                    if last_fail_time.elapsed() < Duration::from_secs(10) {
                        println!("🚫 Circuit breaker OPEN - rejecting request {}", event.request_id);
                        return;
                    } else {
                        // Reset circuit breaker
                        println!("🔄 Circuit breaker attempting to close");
                        circuit_state.store(0, Ordering::SeqCst);
                        *last_failure = None;
                    }
                }
            }
            
            println!("🔓 Circuit breaker CLOSED - processing request {}", event.request_id);
        }
    }).await;
    
    // Generate test requests
    let requests = vec![
        ApiRequest {
            request_id: 1,
            endpoint: "/api/users".to_string(),
            payload: r#"{"action": "list"}"#.to_string(),
            retry_count: 0,
        },
        ApiRequest {
            request_id: 2,
            endpoint: "/api/orders".to_string(),
            payload: r#"{"action": "create", "item": "widget"}"#.to_string(),
            retry_count: 0,
        },
        ApiRequest {
            request_id: 3,
            endpoint: "/api/payments".to_string(),
            payload: r#"{"amount": 99.99, "currency": "USD"}"#.to_string(),
            retry_count: 0,
        },
        ApiRequest {
            request_id: 4,
            endpoint: "/api/notifications".to_string(),
            payload: r#"{"message": "Hello", "recipient": "user@example.com"}"#.to_string(),
            retry_count: 0,
        },
    ];
    
    println!("🚀 Starting API request processing with retry logic...\n");
    
    for request in requests {
        bus.emit(request).await;
        
        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(200)).await;
        println!("---");
    }
    
    println!("\n✅ All requests processed");
}
```

These async examples demonstrate EventRS's powerful async capabilities, including concurrent processing, database operations, stream processing, and resilient error handling patterns. The examples show how to build robust, scalable async event-driven systems using EventRS.