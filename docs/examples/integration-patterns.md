# Integration Patterns

This document demonstrates how to integrate EventRS with popular Rust frameworks and external systems.

## Web Framework Integration

### Axum Web Server Integration

```rust
use eventrs::{AsyncEventBus, Event, ThreadSafeEventBus};
use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

// Application events
#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct UserCreated {
    user_id: u64,
    email: String,
    username: String,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug)]
struct UserLoginAttempt {
    user_id: Option<u64>,
    email: String,
    success: bool,
    ip_address: String,
    user_agent: String,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug)]
struct ApiRequestReceived {
    method: String,
    path: String,
    user_id: Option<u64>,
    response_status: u16,
    duration_ms: u64,
    timestamp: std::time::SystemTime,
}

// Application state
#[derive(Clone)]
struct AppState {
    event_bus: Arc<ThreadSafeEventBus>,
    user_service: Arc<UserService>,
}

// User service
struct UserService {
    // In a real app, this would contain database connections, etc.
}

impl UserService {
    fn new() -> Self {
        Self {}
    }
    
    async fn create_user(&self, email: &str, username: &str) -> Result<u64, UserError> {
        // Simulate user creation
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        if email.contains("invalid") {
            return Err(UserError::InvalidEmail);
        }
        
        // Generate user ID
        let user_id = rand::random::<u64>() % 10000 + 1000;
        Ok(user_id)
    }
    
    async fn authenticate_user(&self, email: &str, password: &str) -> Result<u64, UserError> {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        if email == "admin@example.com" && password == "password123" {
            Ok(1)
        } else {
            Err(UserError::InvalidCredentials)
        }
    }
}

// Error types
#[derive(Debug)]
enum UserError {
    InvalidEmail,
    InvalidCredentials,
    UserNotFound,
}

// Request/Response types
#[derive(Deserialize)]
struct CreateUserRequest {
    email: String,
    username: String,
}

#[derive(Serialize)]
struct CreateUserResponse {
    user_id: u64,
    message: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    success: bool,
    user_id: Option<u64>,
    message: String,
}

// Middleware for request tracking
async fn track_request_middleware<B>(
    State(state): State<AppState>,
    request: axum::http::Request<B>,
    next: axum::middleware::Next<B>,
) -> Result<impl axum::response::IntoResponse, StatusCode> {
    let start_time = std::time::Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    
    // Extract user ID from headers (in a real app, this would be from auth tokens)
    let user_id = request.headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());
    
    let response = next.run(request).await;
    
    let duration = start_time.elapsed();
    let status = response.status().as_u16();
    
    // Emit request tracking event
    state.event_bus.emit(ApiRequestReceived {
        method,
        path,
        user_id,
        response_status: status,
        duration_ms: duration.as_millis() as u64,
        timestamp: std::time::SystemTime::now(),
    });
    
    Ok(response)
}

// Route handlers
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<ResponseJson<CreateUserResponse>, StatusCode> {
    match state.user_service.create_user(&payload.email, &payload.username).await {
        Ok(user_id) => {
            // Emit user created event
            state.event_bus.emit(UserCreated {
                user_id,
                email: payload.email.clone(),
                username: payload.username.clone(),
                timestamp: std::time::SystemTime::now(),
            });
            
            Ok(ResponseJson(CreateUserResponse {
                user_id,
                message: "User created successfully".to_string(),
            }))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> ResponseJson<LoginResponse> {
    let start_time = std::time::Instant::now();
    
    match state.user_service.authenticate_user(&payload.email, &payload.password).await {
        Ok(user_id) => {
            // Emit successful login event
            state.event_bus.emit(UserLoginAttempt {
                user_id: Some(user_id),
                email: payload.email,
                success: true,
                ip_address: "127.0.0.1".to_string(), // In real app, extract from request
                user_agent: "test-client".to_string(),
                timestamp: std::time::SystemTime::now(),
            });
            
            ResponseJson(LoginResponse {
                success: true,
                user_id: Some(user_id),
                message: "Login successful".to_string(),
            })
        }
        Err(_) => {
            // Emit failed login event
            state.event_bus.emit(UserLoginAttempt {
                user_id: None,
                email: payload.email,
                success: false,
                ip_address: "127.0.0.1".to_string(),
                user_agent: "test-client".to_string(),
                timestamp: std::time::SystemTime::now(),
            });
            
            ResponseJson(LoginResponse {
                success: false,
                user_id: None,
                message: "Invalid credentials".to_string(),
            })
        }
    }
}

async fn get_user(Path(user_id): Path<u64>) -> ResponseJson<serde_json::Value> {
    // Simulate user lookup
    ResponseJson(serde_json::json!({
        "user_id": user_id,
        "username": format!("user_{}", user_id),
        "email": format!("user{}@example.com", user_id)
    }))
}

#[tokio::main]
async fn main() {
    // Initialize event bus
    let event_bus = Arc::new(ThreadSafeEventBus::new());
    
    // Set up event handlers
    setup_event_handlers(event_bus.clone()).await;
    
    // Create application state
    let app_state = AppState {
        event_bus,
        user_service: Arc::new(UserService::new()),
    };
    
    // Build the application
    let app = Router::new()
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .route("/auth/login", post(login))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(axum::middleware::from_fn_with_state(
                    app_state.clone(),
                    track_request_middleware,
                ))
        )
        .with_state(app_state);
    
    println!("🚀 Starting web server on http://localhost:3000");
    
    // Start the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    
    // Simulate some API calls
    tokio::spawn(simulate_api_calls());
    
    axum::serve(listener, app).await.unwrap();
}

async fn setup_event_handlers(bus: Arc<ThreadSafeEventBus>) {
    // User creation handler
    bus.on::<UserCreated>(|event| {
        println!("📧 Sending welcome email to {} ({})", event.username, event.email);
        println!("📊 Recording user creation analytics for user {}", event.user_id);
    });
    
    // Login attempt handler
    bus.on::<UserLoginAttempt>(|event| {
        if event.success {
            println!("✅ Successful login: {} from {}", event.email, event.ip_address);
        } else {
            println!("❌ Failed login attempt: {} from {}", event.email, event.ip_address);
            // In a real app, you might implement rate limiting here
        }
    });
    
    // API request analytics
    bus.on::<ApiRequestReceived>(|event| {
        println!("📈 API: {} {} -> {} ({}ms)", 
            event.method, 
            event.path, 
            event.response_status, 
            event.duration_ms
        );
        
        if event.duration_ms > 1000 {
            println!("⚠️  Slow request detected: {} {} took {}ms", 
                event.method, 
                event.path, 
                event.duration_ms
            );
        }
    });
}

async fn simulate_api_calls() {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    
    let client = reqwest::Client::new();
    
    println!("\n🧪 Running API simulation...\n");
    
    // Create a user
    let create_response = client
        .post("http://127.0.0.1:3000/users")
        .json(&serde_json::json!({
            "email": "alice@example.com",
            "username": "alice"
        }))
        .send()
        .await;
    
    if let Ok(response) = create_response {
        println!("Create user response: {}", response.status());
    }
    
    // Successful login
    let login_response = client
        .post("http://127.0.0.1:3000/auth/login")
        .json(&serde_json::json!({
            "email": "admin@example.com",
            "password": "password123"
        }))
        .send()
        .await;
    
    if let Ok(response) = login_response {
        println!("Login response: {}", response.status());
    }
    
    // Failed login
    let failed_login_response = client
        .post("http://127.0.0.1:3000/auth/login")
        .json(&serde_json::json!({
            "email": "hacker@evil.com",
            "password": "wrongpassword"
        }))
        .send()
        .await;
    
    if let Ok(response) = failed_login_response {
        println!("Failed login response: {}", response.status());
    }
    
    // Get user
    let get_response = client
        .get("http://127.0.0.1:3000/users/123")
        .send()
        .await;
    
    if let Ok(response) = get_response {
        println!("Get user response: {}", response.status());
    }
}
```

## Database Integration with SQLx

### PostgreSQL Event Store

```rust
use eventrs::{AsyncEventBus, Event};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use tokio;
use uuid::Uuid;

// Events with database persistence
#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct OrderCreated {
    order_id: Uuid,
    customer_id: u64,
    total_amount: f64,
    items: Vec<OrderItem>,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct PaymentProcessed {
    order_id: Uuid,
    payment_id: String,
    amount: f64,
    status: String,
    timestamp: std::time::SystemTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct OrderItem {
    product_id: u64,
    quantity: u32,
    price: f64,
}

// Event store for persisting events
struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        
        // Create events table if it doesn't exist
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS events (
                id SERIAL PRIMARY KEY,
                event_id UUID NOT NULL DEFAULT gen_random_uuid(),
                event_type VARCHAR NOT NULL,
                aggregate_id UUID,
                event_data JSONB NOT NULL,
                metadata JSONB,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                version INTEGER NOT NULL DEFAULT 1
            )
        "#)
        .execute(&pool)
        .await?;
        
        // Create index for faster queries
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_aggregate_id ON events(aggregate_id)")
            .execute(&pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)")
            .execute(&pool)
            .await?;
        
        Ok(Self { pool })
    }
    
    async fn store_event<E: Event + Serialize>(
        &self,
        event: &E,
        aggregate_id: Option<Uuid>,
    ) -> Result<Uuid, sqlx::Error> {
        let event_data = serde_json::to_value(event).unwrap();
        let event_type = event.event_type_name();
        
        let metadata = serde_json::json!({
            "source": "eventrs",
            "version": "1.0"
        });
        
        let result = sqlx::query(r#"
            INSERT INTO events (event_type, aggregate_id, event_data, metadata)
            VALUES ($1, $2, $3, $4)
            RETURNING event_id
        "#)
        .bind(event_type)
        .bind(aggregate_id)
        .bind(event_data)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;
        
        let event_id: Uuid = result.get("event_id");
        Ok(event_id)
    }
    
    async fn get_events_by_aggregate(&self, aggregate_id: Uuid) -> Result<Vec<StoredEvent>, sqlx::Error> {
        let rows = sqlx::query(r#"
            SELECT event_id, event_type, aggregate_id, event_data, metadata, timestamp, version
            FROM events
            WHERE aggregate_id = $1
            ORDER BY timestamp ASC
        "#)
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await?;
        
        let events = rows.into_iter().map(|row| StoredEvent {
            event_id: row.get("event_id"),
            event_type: row.get("event_type"),
            aggregate_id: row.get("aggregate_id"),
            event_data: row.get("event_data"),
            metadata: row.get("metadata"),
            timestamp: row.get("timestamp"),
            version: row.get("version"),
        }).collect();
        
        Ok(events)
    }
    
    async fn get_events_by_type(&self, event_type: &str) -> Result<Vec<StoredEvent>, sqlx::Error> {
        let rows = sqlx::query(r#"
            SELECT event_id, event_type, aggregate_id, event_data, metadata, timestamp, version
            FROM events
            WHERE event_type = $1
            ORDER BY timestamp ASC
        "#)
        .bind(event_type)
        .fetch_all(&self.pool)
        .await?;
        
        let events = rows.into_iter().map(|row| StoredEvent {
            event_id: row.get("event_id"),
            event_type: row.get("event_type"),
            aggregate_id: row.get("aggregate_id"),
            event_data: row.get("event_data"),
            metadata: row.get("metadata"),
            timestamp: row.get("timestamp"),
            version: row.get("version"),
        }).collect();
        
        Ok(events)
    }
}

#[derive(Debug)]
struct StoredEvent {
    event_id: Uuid,
    event_type: String,
    aggregate_id: Option<Uuid>,
    event_data: serde_json::Value,
    metadata: Option<serde_json::Value>,
    timestamp: chrono::DateTime<chrono::Utc>,
    version: i32,
}

// Order aggregate for event sourcing
struct OrderAggregate {
    id: Uuid,
    customer_id: u64,
    total_amount: f64,
    status: OrderStatus,
    items: Vec<OrderItem>,
    payments: Vec<PaymentInfo>,
    version: u32,
}

#[derive(Debug, Clone)]
enum OrderStatus {
    Created,
    PaymentPending,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

#[derive(Debug, Clone)]
struct PaymentInfo {
    payment_id: String,
    amount: f64,
    status: String,
}

impl OrderAggregate {
    fn new(id: Uuid) -> Self {
        Self {
            id,
            customer_id: 0,
            total_amount: 0.0,
            status: OrderStatus::Created,
            items: Vec::new(),
            payments: Vec::new(),
            version: 0,
        }
    }
    
    fn apply_event(&mut self, event: &StoredEvent) -> Result<(), String> {
        match event.event_type.as_str() {
            "OrderCreated" => {
                let order_created: OrderCreated = serde_json::from_value(event.event_data.clone())
                    .map_err(|e| format!("Failed to deserialize OrderCreated: {}", e))?;
                
                self.customer_id = order_created.customer_id;
                self.total_amount = order_created.total_amount;
                self.items = order_created.items;
                self.status = OrderStatus::Created;
            }
            "PaymentProcessed" => {
                let payment_processed: PaymentProcessed = serde_json::from_value(event.event_data.clone())
                    .map_err(|e| format!("Failed to deserialize PaymentProcessed: {}", e))?;
                
                self.payments.push(PaymentInfo {
                    payment_id: payment_processed.payment_id,
                    amount: payment_processed.amount,
                    status: payment_processed.status.clone(),
                });
                
                if payment_processed.status == "completed" {
                    self.status = OrderStatus::Paid;
                }
            }
            _ => return Err(format!("Unknown event type: {}", event.event_type)),
        }
        
        self.version += 1;
        Ok(())
    }
    
    async fn load_from_events(id: Uuid, event_store: &PostgresEventStore) -> Result<Self, String> {
        let mut aggregate = Self::new(id);
        
        let events = event_store.get_events_by_aggregate(id).await
            .map_err(|e| format!("Failed to load events: {}", e))?;
        
        for event in events {
            aggregate.apply_event(&event)?;
        }
        
        Ok(aggregate)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Database URL - in a real app, this would come from config
    let database_url = "postgresql://username:password@localhost/eventstore";
    
    // Initialize event store
    let event_store = Arc::new(
        PostgresEventStore::new(database_url).await
            .unwrap_or_else(|_| {
                // For demo purposes, create a mock store if DB is not available
                println!("⚠️  Could not connect to PostgreSQL, using mock store");
                std::process::exit(1);
            })
    );
    
    // Initialize event bus
    let mut bus = AsyncEventBus::new();
    
    // Set up event persistence handlers
    let store_clone = event_store.clone();
    bus.on::<OrderCreated>(move |event| {
        let store = store_clone.clone();
        async move {
            match store.store_event(&event, Some(event.order_id)).await {
                Ok(event_id) => {
                    println!("✅ OrderCreated event stored with ID: {}", event_id);
                }
                Err(e) => {
                    eprintln!("❌ Failed to store OrderCreated event: {}", e);
                }
            }
        }
    }).await;
    
    let store_clone = event_store.clone();
    bus.on::<PaymentProcessed>(move |event| {
        let store = store_clone.clone();
        async move {
            match store.store_event(&event, Some(event.order_id)).await {
                Ok(event_id) => {
                    println!("✅ PaymentProcessed event stored with ID: {}", event_id);
                }
                Err(e) => {
                    eprintln!("❌ Failed to store PaymentProcessed event: {}", e);
                }
            }
        }
    }).await;
    
    // Business logic handlers
    bus.on::<OrderCreated>(|event| async move {
        println!("📦 Processing new order {} for customer {}", 
            event.order_id, 
            event.customer_id
        );
        
        // Simulate order processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        println!("📧 Sending order confirmation email");
    }).await;
    
    bus.on::<PaymentProcessed>(|event| async move {
        println!("💳 Payment {} processed for order {} ({})", 
            event.payment_id,
            event.order_id, 
            event.status
        );
        
        if event.status == "completed" {
            println!("✅ Order {} is now paid, proceeding to fulfillment", event.order_id);
        } else {
            println!("❌ Payment failed for order {}", event.order_id);
        }
    }).await;
    
    // Simulate business operations
    println!("🚀 Starting database integration demo...\n");
    
    // Create sample order
    let order_id = Uuid::new_v4();
    let order_created = OrderCreated {
        order_id,
        customer_id: 1001,
        total_amount: 99.99,
        items: vec![
            OrderItem { product_id: 201, quantity: 2, price: 29.99 },
            OrderItem { product_id: 202, quantity: 1, price: 39.99 },
        ],
        timestamp: std::time::SystemTime::now(),
    };
    
    bus.emit(order_created).await;
    
    // Wait a bit for processing
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    
    // Process payment
    let payment_processed = PaymentProcessed {
        order_id,
        payment_id: "PAY-12345".to_string(),
        amount: 99.99,
        status: "completed".to_string(),
        timestamp: std::time::SystemTime::now(),
    };
    
    bus.emit(payment_processed).await;
    
    // Wait for processing
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    
    // Demonstrate event sourcing - rebuild aggregate from events
    println!("\n🔄 Demonstrating event sourcing...");
    
    match OrderAggregate::load_from_events(order_id, &event_store).await {
        Ok(aggregate) => {
            println!("📊 Rebuilt order aggregate from events:");
            println!("  Order ID: {}", aggregate.id);
            println!("  Customer ID: {}", aggregate.customer_id);
            println!("  Total Amount: ${:.2}", aggregate.total_amount);
            println!("  Status: {:?}", aggregate.status);
            println!("  Items: {} items", aggregate.items.len());
            println!("  Payments: {} payments", aggregate.payments.len());
            println!("  Version: {}", aggregate.version);
        }
        Err(e) => {
            eprintln!("❌ Failed to rebuild aggregate: {}", e);
        }
    }
    
    // Query events by type
    println!("\n📋 Events stored in database:");
    
    if let Ok(order_events) = event_store.get_events_by_type("OrderCreated").await {
        println!("OrderCreated events: {}", order_events.len());
    }
    
    if let Ok(payment_events) = event_store.get_events_by_type("PaymentProcessed").await {
        println!("PaymentProcessed events: {}", payment_events.len());
    }
    
    println!("\n✅ Database integration demo completed");
    
    Ok(())
}
```

## Message Queue Integration

### Redis Pub/Sub Integration

```rust
use eventrs::{AsyncEventBus, Event, ThreadSafeEventBus};
use redis::{Client, Commands, Connection, PubSubCommands};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio;
use tokio::sync::mpsc;

// Events that can be distributed across services
#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct UserRegistered {
    user_id: u64,
    email: String,
    username: String,
    service: String,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct OrderProcessed {
    order_id: u64,
    user_id: u64,
    amount: f64,
    status: String,
    service: String,
    timestamp: std::time::SystemTime,
}

#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct NotificationSent {
    user_id: u64,
    notification_type: String,
    channel: String,
    success: bool,
    service: String,
    timestamp: std::time::SystemTime,
}

// Redis event distributor
struct RedisEventDistributor {
    client: Client,
    service_name: String,
}

impl RedisEventDistributor {
    fn new(redis_url: &str, service_name: String) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;
        
        Ok(Self {
            client,
            service_name,
        })
    }
    
    async fn publish_event<E: Event + Serialize>(&self, event: &E) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_connection()?;
        
        // Serialize event to JSON
        let event_data = serde_json::to_string(event).unwrap();
        
        // Create message with metadata
        let message = EventMessage {
            event_type: event.event_type_name().to_string(),
            source_service: self.service_name.clone(),
            data: event_data,
            timestamp: std::time::SystemTime::now(),
        };
        
        let message_json = serde_json::to_string(&message).unwrap();
        
        // Publish to Redis
        let channel = format!("events:{}", event.event_type_name());
        conn.publish(&channel, message_json)?;
        
        println!("📤 Published {} to Redis channel {}", event.event_type_name(), channel);
        
        Ok(())
    }
    
    async fn subscribe_to_events<F>(&self, event_types: Vec<&str>, handler: F) -> Result<(), redis::RedisError>
    where
        F: Fn(EventMessage) + Send + 'static,
    {
        let mut conn = self.client.get_connection()?;
        let mut pubsub = conn.as_pubsub();
        
        // Subscribe to channels
        for event_type in event_types {
            let channel = format!("events:{}", event_type);
            pubsub.subscribe(&channel)?;
            println!("🔔 Subscribed to Redis channel: {}", channel);
        }
        
        // Process messages
        loop {
            let msg = pubsub.get_message()?;
            let payload: String = msg.get_payload()?;
            
            if let Ok(event_message) = serde_json::from_str::<EventMessage>(&payload) {
                // Don't process our own events
                if event_message.source_service != self.service_name {
                    handler(event_message);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct EventMessage {
    event_type: String,
    source_service: String,
    data: String,
    timestamp: std::time::SystemTime,
}

// Service simulation
struct MicroserviceSimulator {
    service_name: String,
    event_bus: Arc<ThreadSafeEventBus>,
    redis_distributor: Arc<RedisEventDistributor>,
}

impl MicroserviceSimulator {
    fn new(
        service_name: String,
        redis_url: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let event_bus = Arc::new(ThreadSafeEventBus::new());
        let redis_distributor = Arc::new(RedisEventDistributor::new(redis_url, service_name.clone())?);
        
        Ok(Self {
            service_name,
            event_bus,
            redis_distributor,
        })
    }
    
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.setup_local_handlers().await;
        self.setup_redis_publishing().await;
        self.setup_redis_subscription().await;
        
        println!("🚀 Service '{}' started", self.service_name);
        Ok(())
    }
    
    async fn setup_local_handlers(&self) {
        match self.service_name.as_str() {
            "user-service" => {
                // User service handles user-related events locally
                self.event_bus.on::<UserRegistered>(|event| {
                    println!("👤 [user-service] Processing user registration: {} ({})", 
                        event.username, 
                        event.email
                    );
                });
            }
            "order-service" => {
                // Order service handles order events
                self.event_bus.on::<OrderProcessed>(|event| {
                    println!("🛒 [order-service] Processing order {}: ${:.2} ({})", 
                        event.order_id, 
                        event.amount, 
                        event.status
                    );
                });
            }
            "notification-service" => {
                // Notification service handles notifications
                self.event_bus.on::<NotificationSent>(|event| {
                    if event.success {
                        println!("📧 [notification-service] Notification sent to user {} via {}", 
                            event.user_id, 
                            event.channel
                        );
                    } else {
                        println!("❌ [notification-service] Failed to send notification to user {}", 
                            event.user_id
                        );
                    }
                });
            }
            _ => {}
        }
    }
    
    async fn setup_redis_publishing(&self) {
        let distributor = self.redis_distributor.clone();
        
        // Publish all local events to Redis
        self.event_bus.on::<UserRegistered>(move |event| {
            let distributor = distributor.clone();
            tokio::spawn(async move {
                if let Err(e) = distributor.publish_event(&event).await {
                    eprintln!("Failed to publish UserRegistered: {}", e);
                }
            });
        });
        
        let distributor = self.redis_distributor.clone();
        self.event_bus.on::<OrderProcessed>(move |event| {
            let distributor = distributor.clone();
            tokio::spawn(async move {
                if let Err(e) = distributor.publish_event(&event).await {
                    eprintln!("Failed to publish OrderProcessed: {}", e);
                }
            });
        });
        
        let distributor = self.redis_distributor.clone();
        self.event_bus.on::<NotificationSent>(move |event| {
            let distributor = distributor.clone();
            tokio::spawn(async move {
                if let Err(e) = distributor.publish_event(&event).await {
                    eprintln!("Failed to publish NotificationSent: {}", e);
                }
            });
        });
    }
    
    async fn setup_redis_subscription(&self) {
        let event_bus = self.event_bus.clone();
        let service_name = self.service_name.clone();
        let distributor = self.redis_distributor.clone();
        
        // Subscribe to events from other services
        tokio::spawn(async move {
            let subscription_events = match service_name.as_str() {
                "user-service" => vec!["OrderProcessed", "NotificationSent"],
                "order-service" => vec!["UserRegistered", "NotificationSent"],
                "notification-service" => vec!["UserRegistered", "OrderProcessed"],
                _ => vec![],
            };
            
            if !subscription_events.is_empty() {
                let _ = distributor.subscribe_to_events(subscription_events, move |message| {
                    println!("📥 [{}] Received {} from {}", 
                        service_name, 
                        message.event_type, 
                        message.source_service
                    );
                    
                    // Deserialize and emit locally based on event type
                    match message.event_type.as_str() {
                        "UserRegistered" => {
                            if let Ok(event) = serde_json::from_str::<UserRegistered>(&message.data) {
                                match service_name.as_str() {
                                    "order-service" => {
                                        println!("🛒 [order-service] User {} registered, setting up account", event.username);
                                    }
                                    "notification-service" => {
                                        println!("📧 [notification-service] Sending welcome email to {}", event.email);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        "OrderProcessed" => {
                            if let Ok(event) = serde_json::from_str::<OrderProcessed>(&message.data) {
                                match service_name.as_str() {
                                    "user-service" => {
                                        println!("👤 [user-service] Updating user {} order history", event.user_id);
                                    }
                                    "notification-service" => {
                                        println!("📧 [notification-service] Sending order confirmation to user {}", event.user_id);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        "NotificationSent" => {
                            if let Ok(event) = serde_json::from_str::<NotificationSent>(&message.data) {
                                match service_name.as_str() {
                                    "user-service" => {
                                        println!("👤 [user-service] Recording notification log for user {}", event.user_id);
                                    }
                                    "order-service" => {
                                        println!("🛒 [order-service] Customer {} notified about order", event.user_id);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }).await;
            }
        });
    }
    
    fn emit_user_registered(&self, user_id: u64, email: String, username: String) {
        self.event_bus.emit(UserRegistered {
            user_id,
            email,
            username,
            service: self.service_name.clone(),
            timestamp: std::time::SystemTime::now(),
        });
    }
    
    fn emit_order_processed(&self, order_id: u64, user_id: u64, amount: f64, status: String) {
        self.event_bus.emit(OrderProcessed {
            order_id,
            user_id,
            amount,
            status,
            service: self.service_name.clone(),
            timestamp: std::time::SystemTime::now(),
        });
    }
    
    fn emit_notification_sent(&self, user_id: u64, notification_type: String, channel: String, success: bool) {
        self.event_bus.emit(NotificationSent {
            user_id,
            notification_type,
            channel,
            success,
            service: self.service_name.clone(),
            timestamp: std::time::SystemTime::now(),
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Redis URL - in a real app, this would come from config
    let redis_url = "redis://127.0.0.1/";
    
    println!("🚀 Starting microservices with Redis integration...\n");
    
    // Create and start services
    let user_service = MicroserviceSimulator::new("user-service".to_string(), redis_url)
        .unwrap_or_else(|_| {
            println!("⚠️  Could not connect to Redis, exiting");
            std::process::exit(1);
        });
    
    let order_service = MicroserviceSimulator::new("order-service".to_string(), redis_url)?;
    let notification_service = MicroserviceSimulator::new("notification-service".to_string(), redis_url)?;
    
    // Start all services
    user_service.start().await?;
    order_service.start().await?;
    notification_service.start().await?;
    
    // Wait for services to initialize
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    
    println!("\n🧪 Simulating business operations...\n");
    
    // Simulate business flow
    // 1. User registers
    user_service.emit_user_registered(
        1001, 
        "alice@example.com".to_string(), 
        "alice".to_string()
    );
    
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // 2. User places an order
    order_service.emit_order_processed(
        5001, 
        1001, 
        99.99, 
        "completed".to_string()
    );
    
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // 3. Notifications are sent
    notification_service.emit_notification_sent(
        1001, 
        "welcome".to_string(), 
        "email".to_string(), 
        true
    );
    
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    
    notification_service.emit_notification_sent(
        1001, 
        "order_confirmation".to_string(), 
        "sms".to_string(), 
        true
    );
    
    // Keep running to see cross-service event processing
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    
    println!("\n✅ Microservices integration demo completed");
    
    Ok(())
}
```

These integration patterns demonstrate how EventRS can be seamlessly integrated with popular Rust frameworks and external systems. The examples show practical patterns for web applications, database persistence with event sourcing, and distributed system communication through message queues.