# Basic Usage Examples

This document provides practical examples of using EventRS for common event-driven programming patterns.

## Simple Event Handling

### Basic Event Definition and Handling

```rust
use eventrs::{Event, EventBus};

// Define a simple event
#[derive(Event, Clone, Debug)]
struct UserLoggedIn {
    user_id: u64,
    username: String,
    timestamp: std::time::SystemTime,
}

fn main() {
    let mut bus = EventBus::new();
    
    // Register a handler
    bus.on::<UserLoggedIn>(|event| {
        println!("User {} (ID: {}) logged in at {:?}", 
            event.username, 
            event.user_id, 
            event.timestamp
        );
    });
    
    // Emit an event
    bus.emit(UserLoggedIn {
        user_id: 123,
        username: "alice".to_string(),
        timestamp: std::time::SystemTime::now(),
    });
}
```

### Multiple Handlers for Same Event

```rust
use eventrs::{Event, EventBus};

#[derive(Event, Clone, Debug)]
struct OrderPlaced {
    order_id: u64,
    customer_id: u64,
    amount: f64,
    items: Vec<String>,
}

fn main() {
    let mut bus = EventBus::new();
    
    // Analytics handler
    bus.on::<OrderPlaced>(|event| {
        println!("📊 Analytics: Order {} for ${:.2}", event.order_id, event.amount);
        // Track order metrics
    });
    
    // Inventory handler
    bus.on::<OrderPlaced>(|event| {
        println!("📦 Inventory: Reserving items for order {}", event.order_id);
        for item in &event.items {
            println!("  - Reserving: {}", item);
        }
    });
    
    // Email notification handler
    bus.on::<OrderPlaced>(|event| {
        println!("📧 Email: Sending confirmation for order {} to customer {}", 
            event.order_id, 
            event.customer_id
        );
    });
    
    // Emit an order event
    bus.emit(OrderPlaced {
        order_id: 12345,
        customer_id: 678,
        amount: 99.99,
        items: vec!["Laptop".to_string(), "Mouse".to_string()],
    });
}
```

## Error Handling

### Fallible Handlers

```rust
use eventrs::{Event, EventBus};
use thiserror::Error;

#[derive(Event, Clone, Debug)]
struct PaymentProcessed {
    payment_id: u64,
    amount: f64,
    customer_id: u64,
}

#[derive(Error, Debug)]
enum PaymentError {
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Invalid payment method")]
    InvalidPaymentMethod,
    #[error("Database error: {0}")]
    DatabaseError(String),
}

fn main() {
    let mut bus = EventBus::new();
    
    // Fallible payment processor
    bus.on_fallible::<PaymentProcessed>(|event| -> Result<(), PaymentError> {
        println!("Processing payment {} for ${:.2}", event.payment_id, event.amount);
        
        // Validate payment
        if event.amount <= 0.0 {
            return Err(PaymentError::InvalidPaymentMethod);
        }
        
        if event.amount > 10000.0 {
            return Err(PaymentError::InsufficientFunds);
        }
        
        // Simulate database operation
        simulate_database_update(event.payment_id)?;
        
        println!("✅ Payment {} processed successfully", event.payment_id);
        Ok(())
    });
    
    // Error handler
    bus.on_error(|error, event_type| {
        eprintln!("❌ Error processing {}: {}", event_type, error);
    });
    
    // Test successful payment
    bus.emit(PaymentProcessed {
        payment_id: 1001,
        amount: 50.0,
        customer_id: 123,
    });
    
    // Test failed payment
    bus.emit(PaymentProcessed {
        payment_id: 1002,
        amount: 15000.0,  // Too high - will fail
        customer_id: 124,
    });
}

fn simulate_database_update(payment_id: u64) -> Result<(), PaymentError> {
    // Simulate potential database failure
    if payment_id % 10 == 0 {
        Err(PaymentError::DatabaseError("Connection timeout".to_string()))
    } else {
        Ok(())
    }
}
```

## Event Filtering

### Conditional Event Processing

```rust
use eventrs::{Event, EventBus, Filter};

#[derive(Event, Clone, Debug)]
struct UserAction {
    user_id: u64,
    action: String,
    ip_address: String,
    user_type: UserType,
}

#[derive(Clone, Debug, PartialEq)]
enum UserType {
    Free,
    Premium,
    Admin,
}

fn main() {
    let mut bus = EventBus::new();
    
    // Handler for premium users only
    bus.on_filtered::<UserAction>(
        Filter::field(|event| event.user_type == UserType::Premium),
        |event| {
            println!("🌟 Premium user {} performed: {}", event.user_id, event.action);
            // Enhanced analytics for premium users
        }
    );
    
    // Handler for admin actions only
    bus.on_filtered::<UserAction>(
        Filter::field(|event| event.user_type == UserType::Admin),
        |event| {
            println!("🔐 Admin action by user {}: {}", event.user_id, event.action);
            // Security audit for admin actions
        }
    );
    
    // Handler for suspicious activities
    bus.on_filtered::<UserAction>(
        Filter::all_of([
            Filter::field(|event: &UserAction| event.action.contains("delete")),
            Filter::field(|event: &UserAction| event.user_type != UserType::Admin),
        ]),
        |event| {
            println!("⚠️  Suspicious delete action by non-admin user {}", event.user_id);
            // Flag for security review
        }
    );
    
    // Emit various user actions
    bus.emit(UserAction {
        user_id: 1,
        action: "login".to_string(),
        ip_address: "192.168.1.1".to_string(),
        user_type: UserType::Free,
    });
    
    bus.emit(UserAction {
        user_id: 2,
        action: "purchase_premium".to_string(),
        ip_address: "192.168.1.2".to_string(),
        user_type: UserType::Premium,
    });
    
    bus.emit(UserAction {
        user_id: 3,
        action: "delete_user".to_string(),
        ip_address: "192.168.1.3".to_string(),
        user_type: UserType::Admin,
    });
    
    bus.emit(UserAction {
        user_id: 4,
        action: "delete_file".to_string(),
        ip_address: "192.168.1.4".to_string(),
        user_type: UserType::Free,  // This will trigger suspicious activity alert
    });
}
```

## Priority-Based Processing

### Handler Priority Management

```rust
use eventrs::{Event, EventBus, Priority};

#[derive(Event, Clone, Debug)]
struct SystemAlert {
    alert_id: u64,
    severity: AlertSeverity,
    message: String,
    timestamp: std::time::SystemTime,
}

#[derive(Clone, Debug, PartialEq)]
enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

fn main() {
    let mut bus = EventBus::new();
    
    // Critical priority - immediate response
    bus.on_with_priority::<SystemAlert>(Priority::Critical, |event| {
        if event.severity == AlertSeverity::Critical {
            println!("🚨 CRITICAL ALERT {}: {}", event.alert_id, event.message);
            // Page on-call engineer immediately
            page_on_call_engineer(&event.message);
        }
    });
    
    // High priority - security and error handling
    bus.on_with_priority::<SystemAlert>(Priority::High, |event| {
        match event.severity {
            AlertSeverity::Error | AlertSeverity::Critical => {
                println!("🔴 High priority processing for alert {}", event.alert_id);
                // Log to security system
                log_to_security_system(&event);
            }
            _ => {}
        }
    });
    
    // Normal priority - standard logging
    bus.on_with_priority::<SystemAlert>(Priority::Normal, |event| {
        println!("📝 Logging alert {}: {} ({})", 
            event.alert_id, 
            event.message, 
            format!("{:?}", event.severity)
        );
        // Standard application logging
    });
    
    // Low priority - metrics and analytics
    bus.on_with_priority::<SystemAlert>(Priority::Low, |event| {
        println!("📈 Analytics: Processing alert {} for metrics", event.alert_id);
        // Update metrics dashboard
        update_metrics_dashboard(&event);
    });
    
    // Emit different severity alerts
    bus.emit(SystemAlert {
        alert_id: 1,
        severity: AlertSeverity::Info,
        message: "System startup completed".to_string(),
        timestamp: std::time::SystemTime::now(),
    });
    
    bus.emit(SystemAlert {
        alert_id: 2,
        severity: AlertSeverity::Critical,
        message: "Database connection lost".to_string(),
        timestamp: std::time::SystemTime::now(),
    });
    
    bus.emit(SystemAlert {
        alert_id: 3,
        severity: AlertSeverity::Warning,
        message: "High memory usage detected".to_string(),
        timestamp: std::time::SystemTime::now(),
    });
}

fn page_on_call_engineer(message: &str) {
    println!("📱 Paging on-call engineer: {}", message);
}

fn log_to_security_system(event: &SystemAlert) {
    println!("🛡️  Security log: Alert {} - {}", event.alert_id, event.message);
}

fn update_metrics_dashboard(event: &SystemAlert) {
    println!("📊 Metrics updated for alert type: {:?}", event.severity);
}
```

## Stateful Event Processing

### Event Aggregation and State Management

```rust
use eventrs::{Event, EventBus};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Event, Clone, Debug)]
struct SensorReading {
    sensor_id: String,
    value: f64,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug)]
struct ThresholdExceeded {
    sensor_id: String,
    current_value: f64,
    threshold: f64,
    consecutive_violations: u32,
}

#[derive(Debug)]
struct SensorState {
    last_value: f64,
    violation_count: u32,
    threshold: f64,
}

fn main() {
    let mut bus = EventBus::new();
    
    // Shared state for sensor monitoring
    let sensor_states = Arc::new(Mutex::new(HashMap::<String, SensorState>::new()));
    
    // Sensor reading processor
    let states = sensor_states.clone();
    bus.on::<SensorReading>(move |event| {
        let mut states = states.lock().unwrap();
        
        let state = states.entry(event.sensor_id.clone()).or_insert(SensorState {
            last_value: 0.0,
            violation_count: 0,
            threshold: 100.0, // Default threshold
        });
        
        state.last_value = event.value;
        
        println!("📡 Sensor {}: {:.2}", event.sensor_id, event.value);
        
        // Check threshold
        if event.value > state.threshold {
            state.violation_count += 1;
            println!("⚠️  Threshold exceeded for sensor {} (violation #{})", 
                event.sensor_id, 
                state.violation_count
            );
            
            // Emit threshold exceeded event if consecutive violations
            if state.violation_count >= 3 {
                // Note: In real code, you'd emit this to the same bus
                println!("🚨 ALERT: Sensor {} has {} consecutive violations!", 
                    event.sensor_id, 
                    state.violation_count
                );
            }
        } else {
            state.violation_count = 0; // Reset violation count
        }
    });
    
    // Threshold exceeded handler
    bus.on::<ThresholdExceeded>(|event| {
        println!("🚨 THRESHOLD ALERT: Sensor {} exceeded threshold {:.2} with value {:.2} ({} consecutive violations)",
            event.sensor_id,
            event.threshold,
            event.current_value,
            event.consecutive_violations
        );
    });
    
    // Simulate sensor readings
    let sensor_readings = vec![
        SensorReading { sensor_id: "temp_01".to_string(), value: 95.0, timestamp: std::time::SystemTime::now() },
        SensorReading { sensor_id: "temp_01".to_string(), value: 105.0, timestamp: std::time::SystemTime::now() },
        SensorReading { sensor_id: "temp_01".to_string(), value: 110.0, timestamp: std::time::SystemTime::now() },
        SensorReading { sensor_id: "temp_01".to_string(), value: 115.0, timestamp: std::time::SystemTime::now() },
        SensorReading { sensor_id: "temp_02".to_string(), value: 85.0, timestamp: std::time::SystemTime::now() },
        SensorReading { sensor_id: "temp_01".to_string(), value: 90.0, timestamp: std::time::SystemTime::now() },
    ];
    
    for reading in sensor_readings {
        bus.emit(reading);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    
    // Print final states
    println!("\n📊 Final sensor states:");
    let states = sensor_states.lock().unwrap();
    for (sensor_id, state) in states.iter() {
        println!("  {}: value={:.2}, violations={}, threshold={:.2}", 
            sensor_id, 
            state.last_value, 
            state.violation_count, 
            state.threshold
        );
    }
}
```

## Event Transformation and Enrichment

### Event Processing Pipeline

```rust
use eventrs::{Event, EventBus};
use serde::{Deserialize, Serialize};

#[derive(Event, Clone, Debug)]
struct RawHttpRequest {
    method: String,
    path: String,
    headers: std::collections::HashMap<String, String>,
    body: Option<String>,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug)]
struct EnrichedHttpRequest {
    method: String,
    path: String,
    user_agent: Option<String>,
    client_ip: Option<String>,
    user_id: Option<u64>,
    is_authenticated: bool,
    request_size: usize,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug)]
struct SecurityAlert {
    alert_type: String,
    severity: String,
    details: String,
    client_ip: String,
    timestamp: std::time::SystemTime,
}

fn main() {
    let mut bus = EventBus::new();
    
    // Stage 1: Raw request enrichment
    bus.on::<RawHttpRequest>(|event| {
        println!("🌐 Processing {} {}", event.method, event.path);
        
        // Extract information from headers
        let user_agent = event.headers.get("User-Agent").cloned();
        let client_ip = event.headers.get("X-Forwarded-For")
            .or_else(|| event.headers.get("X-Real-IP"))
            .cloned();
        
        // Simulate user authentication check
        let user_id = extract_user_id(&event.headers);
        let is_authenticated = user_id.is_some();
        
        let request_size = event.body.as_ref().map(|b| b.len()).unwrap_or(0);
        
        let enriched = EnrichedHttpRequest {
            method: event.method.clone(),
            path: event.path.clone(),
            user_agent,
            client_ip,
            user_id,
            is_authenticated,
            request_size,
            timestamp: event.timestamp,
        };
        
        // In a real implementation, you'd emit this to the same bus
        println!("✨ Enriched request: authenticated={}, size={} bytes", 
            enriched.is_authenticated, 
            enriched.request_size
        );
        
        // Check for security issues
        check_for_security_issues(&enriched);
    });
    
    // Stage 2: Security monitoring
    bus.on::<EnrichedHttpRequest>(|event| {
        // Monitor for suspicious patterns
        if event.request_size > 1_000_000 {
            println!("🚨 Large request detected: {} bytes from {:?}", 
                event.request_size, 
                event.client_ip
            );
        }
        
        if !event.is_authenticated && is_protected_path(&event.path) {
            println!("⚠️  Unauthenticated access to protected path: {}", event.path);
        }
        
        if is_suspicious_user_agent(&event.user_agent) {
            println!("🤖 Suspicious user agent detected: {:?}", event.user_agent);
        }
    });
    
    // Stage 3: Analytics and logging
    bus.on::<EnrichedHttpRequest>(|event| {
        println!("📊 Analytics: {} {} - user_id={:?}, size={}", 
            event.method, 
            event.path, 
            event.user_id, 
            event.request_size
        );
    });
    
    // Security alert handler
    bus.on::<SecurityAlert>(|event| {
        println!("🛡️  SECURITY ALERT [{}]: {} from {} - {}", 
            event.severity,
            event.alert_type, 
            event.client_ip, 
            event.details
        );
    });
    
    // Simulate incoming HTTP requests
    let requests = vec![
        RawHttpRequest {
            method: "GET".to_string(),
            path: "/api/public/health".to_string(),
            headers: {
                let mut headers = std::collections::HashMap::new();
                headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Chrome)".to_string());
                headers.insert("X-Forwarded-For".to_string(), "192.168.1.100".to_string());
                headers
            },
            body: None,
            timestamp: std::time::SystemTime::now(),
        },
        RawHttpRequest {
            method: "POST".to_string(),
            path: "/api/admin/users".to_string(),
            headers: {
                let mut headers = std::collections::HashMap::new();
                headers.insert("User-Agent".to_string(), "curl/7.68.0".to_string());
                headers.insert("Authorization".to_string(), "Bearer valid_token_123".to_string());
                headers.insert("X-Real-IP".to_string(), "10.0.0.5".to_string());
                headers
            },
            body: Some(r#"{"username":"newuser","email":"user@example.com"}"#.to_string()),
            timestamp: std::time::SystemTime::now(),
        },
        RawHttpRequest {
            method: "GET".to_string(),
            path: "/api/admin/secrets".to_string(),
            headers: {
                let mut headers = std::collections::HashMap::new();
                headers.insert("User-Agent".to_string(), "suspicious_bot/1.0".to_string());
                headers.insert("X-Forwarded-For".to_string(), "192.168.1.200".to_string());
                headers
            },
            body: None,
            timestamp: std::time::SystemTime::now(),
        },
    ];
    
    for request in requests {
        bus.emit(request);
        println!("---");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn extract_user_id(headers: &std::collections::HashMap<String, String>) -> Option<u64> {
    headers.get("Authorization")
        .and_then(|auth| {
            if auth.contains("valid_token") {
                Some(123) // Simulate extracted user ID
            } else {
                None
            }
        })
}

fn is_protected_path(path: &str) -> bool {
    path.starts_with("/api/admin/") || path.starts_with("/api/private/")
}

fn is_suspicious_user_agent(user_agent: &Option<String>) -> bool {
    user_agent.as_ref()
        .map(|ua| ua.contains("bot") || ua.contains("crawler") || ua.contains("suspicious"))
        .unwrap_or(false)
}

fn check_for_security_issues(request: &EnrichedHttpRequest) {
    if !request.is_authenticated && is_protected_path(&request.path) {
        if let Some(ip) = &request.client_ip {
            println!("🚨 Security issue: Unauthenticated access to {} from {}", 
                request.path, 
                ip
            );
        }
    }
}
```

These examples demonstrate common EventRS usage patterns including basic event handling, error management, filtering, priority processing, stateful operations, and event transformation pipelines. Each example is self-contained and shows practical, real-world applications of event-driven programming with EventRS.