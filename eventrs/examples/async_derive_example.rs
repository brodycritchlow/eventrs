//! Example demonstrating the new AsyncEvent derive macro with advanced features.

use eventrs::{Event, AsyncEvent, AsyncEventMarker, EventBus, Priority};
use std::time::SystemTime;

// Basic event using the Event derive macro with attributes
#[derive(Event, Clone, Debug)]
#[event(priority = "high", category = "user", source = "auth_system")]
struct UserLoginEvent {
    user_id: u64,
    timestamp: SystemTime,
    ip_address: String,
}

// Advanced async event using the AsyncEvent derive macro
#[derive(AsyncEvent, Clone, Debug)]
#[event(async_validation, async_metadata, priority = "critical", source = "transaction_system")]
struct TransactionEvent {
    transaction_id: String,
    amount: f64,
    currency: String,
    user_id: u64,
    metadata: Vec<u8>,
}

impl TransactionEvent {
    // Async validation method - called automatically when async_validation is specified
    async fn validate_async(&self) -> Result<(), eventrs::EventValidationError> {
        // Simulate async validation (e.g., checking against external service)
        if self.amount <= 0.0 {
            return Err(eventrs::EventValidationError::InvalidValue {
                field: "amount".to_string(),
                value: self.amount.to_string(),
            });
        }
        
        if self.transaction_id.is_empty() {
            return Err(eventrs::EventValidationError::MissingField {
                field: "transaction_id".to_string(),
            });
        }
        
        // Simulate async database check
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        
        Ok(())
    }
    
    // Async metadata generation - called when async_metadata is specified
    async fn generate_metadata_async(&self) -> eventrs::EventMetadata {
        // Simulate fetching enriched metadata from external service
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        
        eventrs::EventMetadata::new()
            .with_priority(Priority::Critical)
            .with_source("transaction_processor")
            .with_category("financial")
            .with_timestamp(SystemTime::now())
            .with_custom_field("transaction_type", "payment")
            .with_custom_field("risk_score", "low")
    }
}

// Event with custom priority value
#[derive(Event, Clone, Debug)]
#[event(priority = "750", category = "system")]
struct CustomPriorityEvent {
    message: String,
    severity: u8,
}

fn main() {
    println!("EventRS AsyncEvent Derive Macro Example");
    println!("========================================");
    
    // Test basic event with attributes
    let login_event = UserLoginEvent {
        user_id: 12345,
        timestamp: SystemTime::now(),
        ip_address: "192.168.1.100".to_string(),
    };
    
    println!("Login Event:");
    println!("  Type: {}", UserLoginEvent::event_type_name());
    println!("  Metadata: {:?}", login_event.metadata());
    println!("  Size: {} bytes", login_event.size_hint());
    println!("  Log: {}", login_event.log_description());
    
    // Test async event
    let transaction_event = TransactionEvent {
        transaction_id: "txn_abc123".to_string(),
        amount: 99.99,
        currency: "USD".to_string(),
        user_id: 12345,
        metadata: vec![1, 2, 3, 4, 5],
    };
    
    println!("\nTransaction Event:");
    println!("  Type: {}", TransactionEvent::event_type_name());
    println!("  Metadata: {:?}", transaction_event.metadata());
    println!("  Size: {} bytes", transaction_event.size_hint());
    println!("  Is AsyncEvent: {}", is_async_event(&transaction_event));
    
    // Test custom priority event
    let custom_event = CustomPriorityEvent {
        message: "System warning".to_string(),
        severity: 3,
    };
    
    println!("\nCustom Priority Event:");
    println!("  Type: {}", CustomPriorityEvent::event_type_name());
    println!("  Metadata: {:?}", custom_event.metadata());
    println!("  Priority: {:?}", custom_event.metadata().priority());
    
    // Test with event bus
    let mut bus = EventBus::new();
    
    bus.on(|event: UserLoginEvent| {
        println!("Handling login for user: {}", event.user_id);
    });
    
    bus.on(|event: TransactionEvent| {
        println!("Processing transaction: {} for ${}", event.transaction_id, event.amount);
    });
    
    bus.on(|event: CustomPriorityEvent| {
        println!("System event: {} (severity: {})", event.message, event.severity);
    });
    
    // Emit events
    println!("\nEmitting events...");
    bus.emit(login_event).expect("Failed to emit login event");
    bus.emit(transaction_event).expect("Failed to emit transaction event");
    bus.emit(custom_event).expect("Failed to emit custom event");
    
    println!("\nAll events processed successfully!");
}

// Helper function to check if an event implements AsyncEventMarker
fn is_async_event<E: Event>(event: &E) -> bool {
    // In a real implementation, you could use std::any::Any for runtime checking
    // For this example, we'll check the type name
    E::event_type_name().contains("Transaction")
}

// Compile-time check that our events implement the correct traits
#[allow(dead_code)]
fn compile_time_checks() {
    // These should compile without issues
    fn accepts_event<E: Event>(_event: E) {}
    fn accepts_async_event<E: Event + AsyncEventMarker>(_event: E) {}
    
    accepts_event(UserLoginEvent {
        user_id: 1,
        timestamp: SystemTime::now(),
        ip_address: "test".to_string(),
    });
    
    accepts_event(TransactionEvent {
        transaction_id: "test".to_string(),
        amount: 1.0,
        currency: "USD".to_string(),
        user_id: 1,
        metadata: vec![],
    });
    
    accepts_async_event(TransactionEvent {
        transaction_id: "test".to_string(),
        amount: 1.0,
        currency: "USD".to_string(),
        user_id: 1,
        metadata: vec![],
    });
}