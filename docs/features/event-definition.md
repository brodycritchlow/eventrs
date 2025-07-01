# Event Definition

Events are the core data structures that flow through EventRS. This document covers how to define, create, and work with events in your applications.

## Basic Event Definition

Events in EventRS are strongly typed Rust structs that implement the `Event` trait. The easiest way to create an event is using the derive macro:

```rust
use eventrs::Event;

#[derive(Event, Clone, Debug)]
struct UserLoggedIn {
    user_id: u64,
    username: String,
    timestamp: std::time::SystemTime,
}
```

### Required Traits

All events must implement these traits:

- **`Event`**: Core event trait (can be derived)
- **`Clone`**: Events must be cloneable for distribution to multiple handlers
- **`Debug`** (recommended): For debugging and logging

```rust
#[derive(Event, Clone, Debug)]
struct MyEvent {
    data: String,
}
```

## Event Trait Implementation

For advanced use cases, you can manually implement the `Event` trait:

```rust
use eventrs::{Event, EventMetadata};

struct CustomEvent {
    payload: Vec<u8>,
}

impl Clone for CustomEvent {
    fn clone(&self) -> Self {
        Self {
            payload: self.payload.clone(),
        }
    }
}

impl Event for CustomEvent {
    fn event_type_name() -> &'static str {
        "CustomEvent"
    }
    
    fn metadata(&self) -> EventMetadata {
        EventMetadata::new()
            .with_size(self.payload.len())
            .with_timestamp(std::time::SystemTime::now())
    }
}
```

## Event Types

### Simple Events

Simple events contain basic data and are optimized for performance:

```rust
#[derive(Event, Clone, Debug)]
struct ButtonClicked {
    button_id: String,
    position: (i32, i32),
}

#[derive(Event, Clone, Debug)]
struct TimerExpired {
    timer_id: u64,
    duration: std::time::Duration,
}
```

### Copy Events

For maximum performance, events that implement `Copy` avoid cloning overhead:

```rust
#[derive(Event, Copy, Clone, Debug)]
struct SimpleCounter {
    count: u32,
}

#[derive(Event, Copy, Clone, Debug)]
struct StatusCode {
    code: u16,
    success: bool,
}
```

### Complex Events

Events can contain complex data structures:

```rust
use std::collections::HashMap;

#[derive(Event, Clone, Debug)]
struct DataProcessed {
    data: Vec<u8>,
    metadata: HashMap<String, String>,
    processing_time: std::time::Duration,
}

#[derive(Event, Clone, Debug)]
struct HttpRequest {
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}
```

### Enum Events

Events can be enums for representing different event variants:

```rust
#[derive(Event, Clone, Debug)]
enum NetworkEvent {
    Connected { peer_id: String },
    Disconnected { peer_id: String, reason: String },
    DataReceived { peer_id: String, data: Vec<u8> },
    Error { error: String },
}

#[derive(Event, Clone, Debug)]
enum GameEvent {
    PlayerJoined(String),
    PlayerLeft(String),
    ScoreUpdated { player: String, score: u32 },
    GameOver { winner: Option<String> },
}
```

## Event Metadata

Events can include metadata for debugging, filtering, and monitoring:

```rust
use eventrs::{Event, EventMetadata, Priority};

#[derive(Event, Clone, Debug)]
struct ImportantEvent {
    message: String,
}

impl ImportantEvent {
    fn new(message: String) -> Self {
        Self { message }
    }
    
    fn metadata(&self) -> EventMetadata {
        EventMetadata::new()
            .with_priority(Priority::High)
            .with_source("user_action")
            .with_category("security")
            .with_tags(vec!["audit", "important"])
    }
}
```

### Built-in Metadata Fields

- **Priority**: Event execution priority
- **Source**: Where the event originated
- **Category**: Event classification
- **Tags**: Additional labels for filtering
- **Timestamp**: When the event was created
- **Size**: Memory usage estimate

## Event Hierarchies

Create event hierarchies using traits and generics:

```rust
// Base event trait
trait UserEvent: Event {
    fn user_id(&self) -> u64;
}

#[derive(Event, Clone, Debug)]
struct UserLoggedIn {
    user_id: u64,
    session_id: String,
}

impl UserEvent for UserLoggedIn {
    fn user_id(&self) -> u64 {
        self.user_id
    }
}

#[derive(Event, Clone, Debug)]
struct UserLoggedOut {
    user_id: u64,
    duration: std::time::Duration,
}

impl UserEvent for UserLoggedOut {
    fn user_id(&self) -> u64 {
        self.user_id
    }
}
```

## Event Validation

Add validation to ensure event data integrity:

```rust
use eventrs::{Event, EventError};

#[derive(Event, Clone, Debug)]
struct EmailSent {
    to: String,
    subject: String,
    body: String,
}

impl EmailSent {
    fn new(to: String, subject: String, body: String) -> Result<Self, EventError> {
        if to.is_empty() {
            return Err(EventError::InvalidData("Email address cannot be empty"));
        }
        
        if !to.contains('@') {
            return Err(EventError::InvalidData("Invalid email address format"));
        }
        
        Ok(Self { to, subject, body })
    }
}
```

## Event Serialization

Events can be serialized for persistence or network transmission:

```rust
use serde::{Deserialize, Serialize};

#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct ApiRequest {
    method: String,
    path: String,
    #[serde(with = "serde_json")]
    body: serde_json::Value,
}

// Convert to/from JSON
let event = ApiRequest {
    method: "POST".to_string(),
    path: "/api/users".to_string(),
    body: serde_json::json!({"name": "John"}),
};

let json = serde_json::to_string(&event)?;
let deserialized: ApiRequest = serde_json::from_str(&json)?;
```

## Generic Events

Create reusable generic events:

```rust
#[derive(Event, Clone, Debug)]
struct DataEvent<T> 
where 
    T: Clone + Debug + Send + Sync + 'static,
{
    data: T,
    source: String,
}

// Usage
type StringDataEvent = DataEvent<String>;
type NumberDataEvent = DataEvent<u64>;

let string_event = StringDataEvent {
    data: "Hello".to_string(),
    source: "input".to_string(),
};
```

## Event Builder Pattern

For complex events, use the builder pattern:

```rust
#[derive(Event, Clone, Debug)]
struct HttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
    timestamp: std::time::SystemTime,
}

impl HttpResponse {
    fn builder() -> HttpResponseBuilder {
        HttpResponseBuilder::default()
    }
}

#[derive(Default)]
struct HttpResponseBuilder {
    status: Option<u16>,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl HttpResponseBuilder {
    fn status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }
    
    fn header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
    
    fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }
    
    fn build(self) -> HttpResponse {
        HttpResponse {
            status: self.status.unwrap_or(200),
            headers: self.headers,
            body: self.body,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

// Usage
let response = HttpResponse::builder()
    .status(201)
    .header("Content-Type".to_string(), "application/json".to_string())
    .body(b"{\"success\": true}".to_vec())
    .build();
```

## Event Versioning

Handle event evolution with versioning:

```rust
#[derive(Event, Clone, Debug)]
enum UserEventV1 {
    LoggedIn { user_id: u64 },
    LoggedOut { user_id: u64 },
}

#[derive(Event, Clone, Debug)]
enum UserEventV2 {
    LoggedIn { user_id: u64, session_id: String },
    LoggedOut { user_id: u64, session_id: String, duration: std::time::Duration },
    PasswordChanged { user_id: u64 },
}

// Migration function
fn migrate_v1_to_v2(v1: UserEventV1) -> UserEventV2 {
    match v1 {
        UserEventV1::LoggedIn { user_id } => {
            UserEventV2::LoggedIn {
                user_id,
                session_id: generate_session_id(),
            }
        }
        UserEventV1::LoggedOut { user_id } => {
            UserEventV2::LoggedOut {
                user_id,
                session_id: "unknown".to_string(),
                duration: std::time::Duration::from_secs(0),
            }
        }
    }
}
```

## Best Practices

### Event Naming
- Use descriptive, past-tense names: `UserLoggedIn`, `OrderCreated`
- Include context: `PaymentProcessed` vs `ProcessingComplete`
- Avoid abbreviations: `UserAuthenticated` vs `UserAuth`

### Event Data
- Include all relevant information
- Avoid large payloads that impact performance
- Use references or IDs for large data structures
- Make events self-contained when possible

### Event Design
- Keep events immutable after creation
- Use builder pattern for complex events
- Include metadata for debugging and monitoring
- Design for forward compatibility

### Performance Considerations
- Prefer `Copy` types for simple events
- Minimize cloning overhead
- Use `Cow<str>` for string data when appropriate
- Consider memory layout for hot path events

## Testing Events

Test event creation and validation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_logged_in_creation() {
        let event = UserLoggedIn {
            user_id: 123,
            username: "john_doe".to_string(),
            timestamp: std::time::SystemTime::now(),
        };
        
        assert_eq!(event.user_id, 123);
        assert_eq!(event.username, "john_doe");
    }
    
    #[test]
    fn test_event_metadata() {
        let event = ImportantEvent::new("Test message".to_string());
        let metadata = event.metadata();
        
        assert_eq!(metadata.priority(), Priority::High);
        assert_eq!(metadata.source(), Some("user_action"));
    }
}
```

Events form the foundation of EventRS applications. Well-designed events make your system more maintainable, debuggable, and performant.