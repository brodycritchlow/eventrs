# Complex Scenarios

This document demonstrates advanced EventRS usage patterns for complex real-world scenarios.

## E-Commerce Order Processing Pipeline

### Multi-Stage Order Processing

```rust
use eventrs::{AsyncEventBus, Event, Filter, Priority, Middleware, MiddlewareContext};
use tokio;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};
use thiserror::Error;

// Event definitions
#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct OrderReceived {
    order_id: u64,
    customer_id: u64,
    items: Vec<OrderItem>,
    total_amount: f64,
    payment_method: PaymentMethod,
    shipping_address: Address,
    timestamp: SystemTime,
}

#[derive(Event, Clone, Debug)]
struct InventoryReserved {
    order_id: u64,
    reserved_items: Vec<ReservedItem>,
    expiry_time: SystemTime,
}

#[derive(Event, Clone, Debug)]
struct PaymentProcessed {
    order_id: u64,
    payment_id: String,
    amount: f64,
    status: PaymentStatus,
}

#[derive(Event, Clone, Debug)]
struct OrderFulfilled {
    order_id: u64,
    tracking_number: String,
    estimated_delivery: SystemTime,
}

#[derive(Event, Clone, Debug)]
struct OrderFailed {
    order_id: u64,
    failure_reason: String,
    stage: OrderStage,
    should_retry: bool,
}

// Supporting types
#[derive(Clone, Debug, Serialize, Deserialize)]
struct OrderItem {
    product_id: u64,
    quantity: u32,
    price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum PaymentMethod {
    CreditCard { last_four: String },
    PayPal { email: String },
    BankTransfer { account: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
    state: String,
    zip_code: String,
    country: String,
}

#[derive(Clone, Debug)]
struct ReservedItem {
    product_id: u64,
    quantity: u32,
    warehouse_location: String,
}

#[derive(Clone, Debug, PartialEq)]
enum PaymentStatus {
    Pending,
    Authorized,
    Captured,
    Failed,
    Refunded,
}

#[derive(Clone, Debug)]
enum OrderStage {
    Validation,
    InventoryReservation,
    PaymentProcessing,
    Fulfillment,
    Shipping,
}

#[derive(Error, Debug)]
enum OrderProcessingError {
    #[error("Insufficient inventory for product {product_id}")]
    InsufficientInventory { product_id: u64 },
    #[error("Payment failed: {reason}")]
    PaymentFailed { reason: String },
    #[error("Shipping unavailable to {location}")]
    ShippingUnavailable { location: String },
    #[error("Order validation failed: {message}")]
    ValidationFailed { message: String },
}

// Order state management
#[derive(Debug, Clone)]
struct OrderState {
    order_id: u64,
    stage: OrderStage,
    inventory_reserved: bool,
    payment_processed: bool,
    fulfilled: bool,
    last_updated: SystemTime,
}

type OrderStateStore = Arc<Mutex<HashMap<u64, OrderState>>>;

// Middleware for order tracking
struct OrderTrackingMiddleware {
    state_store: OrderStateStore,
}

impl OrderTrackingMiddleware {
    fn new() -> Self {
        Self {
            state_store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn get_state_store(&self) -> OrderStateStore {
        self.state_store.clone()
    }
}

impl<E: Event> Middleware<E> for OrderTrackingMiddleware {
    fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> eventrs::MiddlewareResult {
        // Extract order ID from event if possible
        let order_id = self.extract_order_id(event);
        
        if let Some(id) = order_id {
            println!("🔍 Tracking order {} - processing event: {}", id, event.event_type_name());
            
            // Update last activity time
            let mut states = self.state_store.lock().unwrap();
            if let Some(state) = states.get_mut(&id) {
                state.last_updated = SystemTime::now();
            }
        }
        
        context.next(event)
    }
}

impl OrderTrackingMiddleware {
    fn extract_order_id<E: Event>(&self, event: &E) -> Option<u64> {
        // In a real implementation, you'd use reflection or implement
        // a trait to extract order ID from different event types
        // For this example, we'll simulate it
        match event.event_type_name() {
            name if name.contains("Order") => Some(12345), // Simplified
            _ => None,
        }
    }
}

// Services
struct InventoryService;
struct PaymentService;
struct FulfillmentService;
struct NotificationService;

impl InventoryService {
    async fn check_availability(&self, items: &[OrderItem]) -> Result<Vec<ReservedItem>, OrderProcessingError> {
        println!("📦 Checking inventory for {} items", items.len());
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let mut reserved_items = Vec::new();
        
        for item in items {
            // Simulate inventory check
            if item.product_id % 10 == 0 {
                return Err(OrderProcessingError::InsufficientInventory { 
                    product_id: item.product_id 
                });
            }
            
            reserved_items.push(ReservedItem {
                product_id: item.product_id,
                quantity: item.quantity,
                warehouse_location: format!("WH-{}", item.product_id % 3 + 1),
            });
        }
        
        println!("✅ Inventory reserved for {} items", reserved_items.len());
        Ok(reserved_items)
    }
    
    async fn release_reservation(&self, order_id: u64) {
        println!("🔓 Releasing inventory reservation for order {}", order_id);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

impl PaymentService {
    async fn process_payment(&self, order_id: u64, amount: f64, method: &PaymentMethod) -> Result<String, OrderProcessingError> {
        println!("💳 Processing payment of ${:.2} for order {}", amount, order_id);
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Simulate payment processing with potential failures
        match method {
            PaymentMethod::CreditCard { last_four } => {
                if last_four == "0000" {
                    return Err(OrderProcessingError::PaymentFailed {
                        reason: "Invalid card number".to_string(),
                    });
                }
                Ok(format!("CC-{}-{}", order_id, chrono::Utc::now().timestamp()))
            }
            PaymentMethod::PayPal { email } => {
                if email.contains("invalid") {
                    return Err(OrderProcessingError::PaymentFailed {
                        reason: "PayPal account not found".to_string(),
                    });
                }
                Ok(format!("PP-{}-{}", order_id, chrono::Utc::now().timestamp()))
            }
            PaymentMethod::BankTransfer { .. } => {
                // Bank transfers take longer
                tokio::time::sleep(Duration::from_millis(300)).await;
                Ok(format!("BT-{}-{}", order_id, chrono::Utc::now().timestamp()))
            }
        }
    }
    
    async fn refund_payment(&self, payment_id: &str, amount: f64) {
        println!("💸 Processing refund of ${:.2} for payment {}", amount, payment_id);
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

impl FulfillmentService {
    async fn fulfill_order(&self, order_id: u64, items: &[ReservedItem], address: &Address) -> Result<String, OrderProcessingError> {
        println!("📋 Fulfilling order {} with {} items", order_id, items.len());
        
        // Check shipping availability
        if address.country != "US" && address.country != "CA" {
            return Err(OrderProcessingError::ShippingUnavailable {
                location: address.country.clone(),
            });
        }
        
        // Simulate fulfillment processing
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        let tracking_number = format!("TRK-{}-{}", order_id, chrono::Utc::now().timestamp());
        println!("✅ Order {} fulfilled with tracking number: {}", order_id, tracking_number);
        
        Ok(tracking_number)
    }
}

impl NotificationService {
    async fn send_order_confirmation(&self, order_id: u64, customer_id: u64) {
        println!("📧 Sending order confirmation for order {} to customer {}", order_id, customer_id);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    async fn send_shipping_notification(&self, order_id: u64, customer_id: u64, tracking_number: &str) {
        println!("📦 Sending shipping notification for order {} (tracking: {})", order_id, tracking_number);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    async fn send_failure_notification(&self, order_id: u64, customer_id: u64, reason: &str) {
        println!("❌ Sending failure notification for order {}: {}", order_id, reason);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::builder()
        .with_middleware(OrderTrackingMiddleware::new())
        .build();
    
    let inventory_service = Arc::new(InventoryService);
    let payment_service = Arc::new(PaymentService);
    let fulfillment_service = Arc::new(FulfillmentService);
    let notification_service = Arc::new(NotificationService);
    let order_states: OrderStateStore = Arc::new(Mutex::new(HashMap::new()));
    
    // Stage 1: Order Validation and Initial Processing
    let states = order_states.clone();
    bus.on_with_priority::<OrderReceived>(Priority::High, move |event| {
        let states = states.clone();
        async move {
            println!("🎯 Processing new order {} for customer {}", event.order_id, event.customer_id);
            
            // Initialize order state
            {
                let mut state_map = states.lock().unwrap();
                state_map.insert(event.order_id, OrderState {
                    order_id: event.order_id,
                    stage: OrderStage::Validation,
                    inventory_reserved: false,
                    payment_processed: false,
                    fulfilled: false,
                    last_updated: SystemTime::now(),
                });
            }
            
            // Validate order
            if event.items.is_empty() {
                println!("❌ Order {} validation failed: no items", event.order_id);
                return;
            }
            
            if event.total_amount <= 0.0 {
                println!("❌ Order {} validation failed: invalid amount", event.order_id);
                return;
            }
            
            println!("✅ Order {} validated successfully", event.order_id);
        }
    }).await;
    
    // Stage 2: Inventory Reservation
    let inventory = inventory_service.clone();
    let states = order_states.clone();
    bus.on::<OrderReceived>(move |event| {
        let inventory = inventory.clone();
        let states = states.clone();
        async move {
            match inventory.check_availability(&event.items).await {
                Ok(reserved_items) => {
                    // Update state
                    {
                        let mut state_map = states.lock().unwrap();
                        if let Some(state) = state_map.get_mut(&event.order_id) {
                            state.stage = OrderStage::InventoryReservation;
                            state.inventory_reserved = true;
                        }
                    }
                    
                    // Emit inventory reserved event
                    println!("📦 Inventory reserved for order {}", event.order_id);
                    
                    // In a real system, you'd emit this event to the bus
                    // bus.emit(InventoryReserved { ... }).await;
                }
                Err(e) => {
                    println!("❌ Inventory reservation failed for order {}: {}", event.order_id, e);
                    // Handle inventory failure - could retry or cancel order
                }
            }
        }
    }).await;
    
    // Stage 3: Payment Processing
    let payment = payment_service.clone();
    let states = order_states.clone();
    bus.on::<OrderReceived>(move |event| {
        let payment = payment.clone();
        let states = states.clone();
        async move {
            // Wait a bit to ensure inventory is processed first
            tokio::time::sleep(Duration::from_millis(200)).await;
            
            match payment.process_payment(event.order_id, event.total_amount, &event.payment_method).await {
                Ok(payment_id) => {
                    // Update state
                    {
                        let mut state_map = states.lock().unwrap();
                        if let Some(state) = state_map.get_mut(&event.order_id) {
                            state.stage = OrderStage::PaymentProcessing;
                            state.payment_processed = true;
                        }
                    }
                    
                    println!("💳 Payment processed for order {}: {}", event.order_id, payment_id);
                    
                    // In a real system, you'd emit PaymentProcessed event
                }
                Err(e) => {
                    println!("❌ Payment failed for order {}: {}", event.order_id, e);
                    
                    // Release inventory on payment failure
                    inventory_service.release_reservation(event.order_id).await;
                }
            }
        }
    }).await;
    
    // Stage 4: Order Fulfillment
    let fulfillment = fulfillment_service.clone();
    let states = order_states.clone();
    bus.on::<OrderReceived>(move |event| {
        let fulfillment = fulfillment.clone();
        let states = states.clone();
        async move {
            // Wait for payment processing
            tokio::time::sleep(Duration::from_millis(400)).await;
            
            // Check if payment was successful
            let payment_success = {
                let state_map = states.lock().unwrap();
                state_map.get(&event.order_id)
                    .map(|s| s.payment_processed)
                    .unwrap_or(false)
            };
            
            if !payment_success {
                println!("⏸️  Skipping fulfillment for order {} - payment not processed", event.order_id);
                return;
            }
            
            // Create mock reserved items for fulfillment
            let reserved_items: Vec<ReservedItem> = event.items.iter().map(|item| {
                ReservedItem {
                    product_id: item.product_id,
                    quantity: item.quantity,
                    warehouse_location: format!("WH-{}", item.product_id % 3 + 1),
                }
            }).collect();
            
            match fulfillment.fulfill_order(event.order_id, &reserved_items, &event.shipping_address).await {
                Ok(tracking_number) => {
                    // Update state
                    {
                        let mut state_map = states.lock().unwrap();
                        if let Some(state) = state_map.get_mut(&event.order_id) {
                            state.stage = OrderStage::Fulfillment;
                            state.fulfilled = true;
                        }
                    }
                    
                    println!("📦 Order {} fulfilled with tracking: {}", event.order_id, tracking_number);
                    
                    // In a real system, you'd emit OrderFulfilled event
                }
                Err(e) => {
                    println!("❌ Fulfillment failed for order {}: {}", event.order_id, e);
                    
                    // Handle fulfillment failure - refund payment, release inventory
                }
            }
        }
    }).await;
    
    // Stage 5: Notifications
    let notification = notification_service.clone();
    bus.on_with_priority::<OrderReceived>(Priority::Low, move |event| {
        let notification = notification.clone();
        async move {
            // Send order confirmation
            notification.send_order_confirmation(event.order_id, event.customer_id).await;
            
            // Wait a bit then send shipping notification
            tokio::time::sleep(Duration::from_millis(600)).await;
            notification.send_shipping_notification(
                event.order_id, 
                event.customer_id, 
                &format!("TRK-{}", event.order_id)
            ).await;
        }
    }).await;
    
    // Order status monitoring
    let states = order_states.clone();
    bus.on::<OrderReceived>(move |event| {
        let states = states.clone();
        async move {
            // Monitor order progress
            tokio::time::sleep(Duration::from_secs(1)).await;
            
            let state_map = states.lock().unwrap();
            if let Some(state) = state_map.get(&event.order_id) {
                println!("📊 Order {} status: stage={:?}, inventory_reserved={}, payment_processed={}, fulfilled={}", 
                    state.order_id,
                    state.stage,
                    state.inventory_reserved,
                    state.payment_processed,
                    state.fulfilled
                );
            }
        }
    }).await;
    
    // Test with sample orders
    let test_orders = vec![
        OrderReceived {
            order_id: 12345,
            customer_id: 1001,
            items: vec![
                OrderItem { product_id: 101, quantity: 2, price: 29.99 },
                OrderItem { product_id: 102, quantity: 1, price: 49.99 },
            ],
            total_amount: 109.97,
            payment_method: PaymentMethod::CreditCard { last_four: "1234".to_string() },
            shipping_address: Address {
                street: "123 Main St".to_string(),
                city: "Anytown".to_string(),
                state: "CA".to_string(),
                zip_code: "12345".to_string(),
                country: "US".to_string(),
            },
            timestamp: SystemTime::now(),
        },
        OrderReceived {
            order_id: 12346,
            customer_id: 1002,
            items: vec![
                OrderItem { product_id: 200, quantity: 1, price: 99.99 }, // This will fail inventory
            ],
            total_amount: 99.99,
            payment_method: PaymentMethod::PayPal { email: "user@example.com".to_string() },
            shipping_address: Address {
                street: "456 Oak Ave".to_string(),
                city: "Another City".to_string(),
                state: "NY".to_string(),
                zip_code: "67890".to_string(),
                country: "US".to_string(),
            },
            timestamp: SystemTime::now(),
        },
    ];
    
    println!("🚀 Starting e-commerce order processing pipeline...\n");
    
    for order in test_orders {
        println!("📥 Received order {}", order.order_id);
        bus.emit(order).await;
        
        // Small delay between orders
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("{'=':<50}");
    }
    
    // Wait for all processing to complete
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    println!("\n✅ Order processing pipeline completed");
    
    // Print final order states
    println!("\n📊 Final Order States:");
    let state_map = order_states.lock().unwrap();
    for (order_id, state) in state_map.iter() {
        println!("  Order {}: {:?}", order_id, state);
    }
}
```

## IoT Sensor Network Management

### Distributed Sensor Monitoring System

```rust
use eventrs::{AsyncEventBus, Event, Filter, Priority, ThreadSafeEventBus};
use tokio;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, Instant};
use serde::{Serialize, Deserialize};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

// Event definitions for IoT system
#[derive(Event, Clone, Debug, Serialize, Deserialize)]
struct SensorReading {
    sensor_id: String,
    sensor_type: SensorType,
    location: SensorLocation,
    value: f64,
    unit: String,
    timestamp: SystemTime,
    battery_level: Option<f32>,
    signal_strength: Option<i32>,
}

#[derive(Event, Clone, Debug)]
struct SensorAlert {
    sensor_id: String,
    alert_type: AlertType,
    severity: AlertSeverity,
    message: String,
    threshold_value: Option<f64>,
    current_value: f64,
    timestamp: SystemTime,
}

#[derive(Event, Clone, Debug)]
struct SensorOffline {
    sensor_id: String,
    last_seen: SystemTime,
    expected_interval: Duration,
    location: SensorLocation,
}

#[derive(Event, Clone, Debug)]
struct SensorBatteryLow {
    sensor_id: String,
    current_level: f32,
    threshold: f32,
    estimated_time_remaining: Duration,
}

#[derive(Event, Clone, Debug)]
struct AggregatedMetrics {
    location: SensorLocation,
    metric_type: String,
    time_window: Duration,
    average: f64,
    minimum: f64,
    maximum: f64,
    sample_count: u32,
    timestamp: SystemTime,
}

// Supporting types
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
enum SensorType {
    Temperature,
    Humidity,
    Pressure,
    AirQuality,
    Motion,
    Light,
    Sound,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
struct SensorLocation {
    building: String,
    floor: u32,
    room: String,
    zone: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum AlertType {
    ThresholdExceeded,
    ThresholdBelow,
    RapidChange,
    SensorMalfunction,
    CommunicationLoss,
}

#[derive(Clone, Debug, PartialEq, Ord, PartialOrd, Eq)]
enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

// Sensor management system
#[derive(Debug, Clone)]
struct SensorMetadata {
    sensor_id: String,
    sensor_type: SensorType,
    location: SensorLocation,
    last_reading: Option<SystemTime>,
    battery_level: Option<f32>,
    signal_strength: Option<i32>,
    is_active: bool,
    reading_interval: Duration,
    thresholds: SensorThresholds,
}

#[derive(Debug, Clone)]
struct SensorThresholds {
    min_value: Option<f64>,
    max_value: Option<f64>,
    critical_min: Option<f64>,
    critical_max: Option<f64>,
    rapid_change_threshold: Option<f64>,
}

#[derive(Debug)]
struct SensorRegistry {
    sensors: RwLock<HashMap<String, SensorMetadata>>,
    reading_history: RwLock<HashMap<String, VecDeque<SensorReading>>>,
}

impl SensorRegistry {
    fn new() -> Self {
        Self {
            sensors: RwLock::new(HashMap::new()),
            reading_history: RwLock::new(HashMap::new()),
        }
    }
    
    fn register_sensor(&self, metadata: SensorMetadata) {
        let mut sensors = self.sensors.write().unwrap();
        sensors.insert(metadata.sensor_id.clone(), metadata);
    }
    
    fn update_sensor_reading(&self, reading: &SensorReading) {
        // Update sensor metadata
        {
            let mut sensors = self.sensors.write().unwrap();
            if let Some(sensor) = sensors.get_mut(&reading.sensor_id) {
                sensor.last_reading = Some(reading.timestamp);
                sensor.battery_level = reading.battery_level;
                sensor.signal_strength = reading.signal_strength;
                sensor.is_active = true;
            }
        }
        
        // Store reading history (keep last 100 readings per sensor)
        {
            let mut history = self.reading_history.write().unwrap();
            let sensor_history = history.entry(reading.sensor_id.clone()).or_insert_with(VecDeque::new);
            
            sensor_history.push_back(reading.clone());
            if sensor_history.len() > 100 {
                sensor_history.pop_front();
            }
        }
    }
    
    fn get_sensor_metadata(&self, sensor_id: &str) -> Option<SensorMetadata> {
        let sensors = self.sensors.read().unwrap();
        sensors.get(sensor_id).cloned()
    }
    
    fn get_recent_readings(&self, sensor_id: &str, count: usize) -> Vec<SensorReading> {
        let history = self.reading_history.read().unwrap();
        history.get(sensor_id)
            .map(|readings| readings.iter().rev().take(count).cloned().collect())
            .unwrap_or_default()
    }
    
    fn get_offline_sensors(&self, timeout: Duration) -> Vec<SensorMetadata> {
        let sensors = self.sensors.read().unwrap();
        let now = SystemTime::now();
        
        sensors.values()
            .filter(|sensor| {
                if let Some(last_reading) = sensor.last_reading {
                    now.duration_since(last_reading).unwrap_or(Duration::ZERO) > timeout
                } else {
                    true // Never received a reading
                }
            })
            .cloned()
            .collect()
    }
}

// Analytics service for sensor data
struct SensorAnalytics {
    registry: Arc<SensorRegistry>,
}

impl SensorAnalytics {
    fn new(registry: Arc<SensorRegistry>) -> Self {
        Self { registry }
    }
    
    async fn analyze_reading(&self, reading: &SensorReading) -> Vec<SensorAlert> {
        let mut alerts = Vec::new();
        
        if let Some(metadata) = self.registry.get_sensor_metadata(&reading.sensor_id) {
            // Check threshold violations
            if let Some(max_threshold) = metadata.thresholds.max_value {
                if reading.value > max_threshold {
                    alerts.push(SensorAlert {
                        sensor_id: reading.sensor_id.clone(),
                        alert_type: AlertType::ThresholdExceeded,
                        severity: AlertSeverity::Warning,
                        message: format!("Value {:.2} exceeds threshold {:.2}", reading.value, max_threshold),
                        threshold_value: Some(max_threshold),
                        current_value: reading.value,
                        timestamp: reading.timestamp,
                    });
                }
            }
            
            if let Some(critical_max) = metadata.thresholds.critical_max {
                if reading.value > critical_max {
                    alerts.push(SensorAlert {
                        sensor_id: reading.sensor_id.clone(),
                        alert_type: AlertType::ThresholdExceeded,
                        severity: AlertSeverity::Critical,
                        message: format!("CRITICAL: Value {:.2} exceeds critical threshold {:.2}", reading.value, critical_max),
                        threshold_value: Some(critical_max),
                        current_value: reading.value,
                        timestamp: reading.timestamp,
                    });
                }
            }
            
            // Check for rapid changes
            if let Some(rapid_threshold) = metadata.thresholds.rapid_change_threshold {
                let recent_readings = self.registry.get_recent_readings(&reading.sensor_id, 2);
                if recent_readings.len() >= 2 {
                    let previous_value = recent_readings[1].value;
                    let change = (reading.value - previous_value).abs();
                    
                    if change > rapid_threshold {
                        alerts.push(SensorAlert {
                            sensor_id: reading.sensor_id.clone(),
                            alert_type: AlertType::RapidChange,
                            severity: AlertSeverity::Warning,
                            message: format!("Rapid change detected: {:.2} -> {:.2} (change: {:.2})", 
                                previous_value, reading.value, change),
                            threshold_value: Some(rapid_threshold),
                            current_value: change,
                            timestamp: reading.timestamp,
                        });
                    }
                }
            }
            
            // Check battery level
            if let Some(battery_level) = reading.battery_level {
                if battery_level < 0.2 {
                    alerts.push(SensorAlert {
                        sensor_id: reading.sensor_id.clone(),
                        alert_type: AlertType::SensorMalfunction,
                        severity: AlertSeverity::Warning,
                        message: format!("Low battery: {:.1}%", battery_level * 100.0),
                        threshold_value: Some(0.2),
                        current_value: battery_level as f64,
                        timestamp: reading.timestamp,
                    });
                }
            }
        }
        
        alerts
    }
    
    async fn calculate_aggregated_metrics(&self, location: &SensorLocation, time_window: Duration) -> Vec<AggregatedMetrics> {
        let sensors = self.registry.sensors.read().unwrap();
        let history = self.registry.reading_history.read().unwrap();
        
        let mut metrics = Vec::new();
        let cutoff_time = SystemTime::now() - time_window;
        
        // Group sensors by type at this location
        let mut sensor_groups: HashMap<SensorType, Vec<String>> = HashMap::new();
        
        for (sensor_id, metadata) in sensors.iter() {
            if metadata.location == *location {
                sensor_groups.entry(metadata.sensor_type.clone())
                    .or_insert_with(Vec::new)
                    .push(sensor_id.clone());
            }
        }
        
        // Calculate metrics for each sensor type
        for (sensor_type, sensor_ids) in sensor_groups {
            let mut values = Vec::new();
            
            for sensor_id in sensor_ids {
                if let Some(readings) = history.get(sensor_id) {
                    for reading in readings.iter().rev() {
                        if reading.timestamp >= cutoff_time {
                            values.push(reading.value);
                        } else {
                            break;
                        }
                    }
                }
            }
            
            if !values.is_empty() {
                let sum: f64 = values.iter().sum();
                let average = sum / values.len() as f64;
                let minimum = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let maximum = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                
                metrics.push(AggregatedMetrics {
                    location: location.clone(),
                    metric_type: format!("{:?}", sensor_type),
                    time_window,
                    average,
                    minimum,
                    maximum,
                    sample_count: values.len() as u32,
                    timestamp: SystemTime::now(),
                });
            }
        }
        
        metrics
    }
}

#[tokio::main]
async fn main() {
    let mut bus = AsyncEventBus::new();
    let registry = Arc::new(SensorRegistry::new());
    let analytics = Arc::new(SensorAnalytics::new(registry.clone()));
    
    // Register some sample sensors
    let locations = vec![
        SensorLocation { building: "HQ".to_string(), floor: 1, room: "Lobby".to_string(), zone: None },
        SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-A".to_string(), zone: Some("North".to_string()) },
        SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-B".to_string(), zone: Some("South".to_string()) },
    ];
    
    for (i, location) in locations.iter().enumerate() {
        for (j, sensor_type) in [SensorType::Temperature, SensorType::Humidity, SensorType::AirQuality].iter().enumerate() {
            let sensor_id = format!("{}-{}-{:02}", 
                match sensor_type {
                    SensorType::Temperature => "TEMP",
                    SensorType::Humidity => "HUM",
                    SensorType::AirQuality => "AQ",
                    _ => "UNKNOWN",
                },
                location.room.replace("-", ""),
                i * 3 + j + 1
            );
            
            let thresholds = match sensor_type {
                SensorType::Temperature => SensorThresholds {
                    min_value: Some(18.0),
                    max_value: Some(26.0),
                    critical_min: Some(10.0),
                    critical_max: Some(35.0),
                    rapid_change_threshold: Some(5.0),
                },
                SensorType::Humidity => SensorThresholds {
                    min_value: Some(30.0),
                    max_value: Some(70.0),
                    critical_min: Some(20.0),
                    critical_max: Some(80.0),
                    rapid_change_threshold: Some(20.0),
                },
                SensorType::AirQuality => SensorThresholds {
                    min_value: None,
                    max_value: Some(100.0),
                    critical_min: None,
                    critical_max: Some(200.0),
                    rapid_change_threshold: Some(50.0),
                },
                _ => SensorThresholds {
                    min_value: None,
                    max_value: None,
                    critical_min: None,
                    critical_max: None,
                    rapid_change_threshold: None,
                },
            };
            
            registry.register_sensor(SensorMetadata {
                sensor_id: sensor_id.clone(),
                sensor_type: sensor_type.clone(),
                location: location.clone(),
                last_reading: None,
                battery_level: None,
                signal_strength: None,
                is_active: false,
                reading_interval: Duration::from_secs(30),
                thresholds,
            });
        }
    }
    
    // Sensor reading processor
    let registry_clone = registry.clone();
    let analytics_clone = analytics.clone();
    bus.on_with_priority::<SensorReading>(Priority::High, move |reading| {
        let registry = registry_clone.clone();
        let analytics = analytics_clone.clone();
        async move {
            println!("📡 Sensor reading: {} = {:.2} {} (battery: {:?}%, signal: {:?}dBm)", 
                reading.sensor_id,
                reading.value,
                reading.unit,
                reading.battery_level.map(|b| (b * 100.0) as u32),
                reading.signal_strength
            );
            
            // Update registry
            registry.update_sensor_reading(&reading);
            
            // Analyze for alerts
            let alerts = analytics.analyze_reading(&reading).await;
            for alert in alerts {
                println!("🚨 ALERT [{}]: {} - {}", 
                    format!("{:?}", alert.severity).to_uppercase(),
                    alert.sensor_id,
                    alert.message
                );
                
                // In a real system, you'd emit these alerts to the bus
                // bus.emit(alert).await;
            }
        }
    }).await;
    
    // Offline sensor detection
    let registry_clone = registry.clone();
    bus.on::<SensorReading>(move |_| {
        let registry = registry_clone.clone();
        async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            
            let offline_sensors = registry.get_offline_sensors(Duration::from_secs(60));
            for sensor in offline_sensors {
                println!("⚠️  Sensor {} appears to be offline (last seen: {:?})", 
                    sensor.sensor_id,
                    sensor.last_reading
                );
            }
        }
    }).await;
    
    // Periodic aggregated metrics calculation
    let analytics_clone = analytics.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            let locations = vec![
                SensorLocation { building: "HQ".to_string(), floor: 1, room: "Lobby".to_string(), zone: None },
                SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-A".to_string(), zone: Some("North".to_string()) },
            ];
            
            for location in locations {
                let metrics = analytics_clone.calculate_aggregated_metrics(&location, Duration::from_secs(60)).await;
                
                for metric in metrics {
                    println!("📊 Metrics for {} {}: avg={:.2}, min={:.2}, max={:.2} ({} samples)", 
                        metric.location.room,
                        metric.metric_type,
                        metric.average,
                        metric.minimum,
                        metric.maximum,
                        metric.sample_count
                    );
                }
            }
        }
    });
    
    // Simulate sensor readings
    println!("🚀 Starting IoT sensor network simulation...\n");
    
    let sensor_ids = vec![
        "TEMP-Lobby-01", "HUM-Lobby-02", "AQ-Lobby-03",
        "TEMP-OfficeA-04", "HUM-OfficeA-05", "AQ-OfficeA-06",
        "TEMP-OfficeB-07", "HUM-OfficeB-08", "AQ-OfficeB-09",
    ];
    
    let locations = vec![
        SensorLocation { building: "HQ".to_string(), floor: 1, room: "Lobby".to_string(), zone: None },
        SensorLocation { building: "HQ".to_string(), floor: 1, room: "Lobby".to_string(), zone: None },
        SensorLocation { building: "HQ".to_string(), floor: 1, room: "Lobby".to_string(), zone: None },
        SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-A".to_string(), zone: Some("North".to_string()) },
        SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-A".to_string(), zone: Some("North".to_string()) },
        SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-A".to_string(), zone: Some("North".to_string()) },
        SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-B".to_string(), zone: Some("South".to_string()) },
        SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-B".to_string(), zone: Some("South".to_string()) },
        SensorLocation { building: "HQ".to_string(), floor: 2, room: "Office-B".to_string(), zone: Some("South".to_string()) },
    ];
    
    let sensor_types = vec![
        SensorType::Temperature, SensorType::Humidity, SensorType::AirQuality,
        SensorType::Temperature, SensorType::Humidity, SensorType::AirQuality,
        SensorType::Temperature, SensorType::Humidity, SensorType::AirQuality,
    ];
    
    // Generate readings for 30 seconds
    let start_time = Instant::now();
    let mut reading_counter = 0;
    
    while start_time.elapsed() < Duration::from_secs(30) {
        for (i, sensor_id) in sensor_ids.iter().enumerate() {
            let base_value = match sensor_types[i] {
                SensorType::Temperature => 22.0 + (reading_counter as f64 * 0.1).sin() * 3.0,
                SensorType::Humidity => 55.0 + (reading_counter as f64 * 0.15).cos() * 15.0,
                SensorType::AirQuality => 50.0 + (reading_counter as f64 * 0.05).sin() * 30.0,
                _ => 0.0,
            };
            
            // Add some random variation
            let value = base_value + (rand::random::<f64>() - 0.5) * 2.0;
            
            // Occasionally generate extreme values to trigger alerts
            let value = if reading_counter % 20 == 0 && i == 0 {
                40.0 // Temperature spike
            } else if reading_counter % 25 == 0 && i == 2 {
                150.0 // Air quality spike
            } else {
                value
            };
            
            let unit = match sensor_types[i] {
                SensorType::Temperature => "°C",
                SensorType::Humidity => "%",
                SensorType::AirQuality => "AQI",
                _ => "units",
            };
            
            let reading = SensorReading {
                sensor_id: sensor_id.to_string(),
                sensor_type: sensor_types[i].clone(),
                location: locations[i].clone(),
                value,
                unit: unit.to_string(),
                timestamp: SystemTime::now(),
                battery_level: Some(0.8 - (reading_counter as f32 * 0.001)),
                signal_strength: Some(-40 + (rand::random::<i32>() % 20)),
            };
            
            bus.emit(reading).await;
        }
        
        reading_counter += 1;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    
    println!("\n✅ IoT sensor network simulation completed");
    
    // Wait a bit for final processing
    tokio::time::sleep(Duration::from_secs(2)).await;
}
```

These complex scenarios demonstrate EventRS's capability to handle sophisticated real-world applications involving multiple stages, state management, error handling, and coordination between different system components. The examples show how EventRS can be used to build robust, scalable systems for e-commerce and IoT applications.