//! Basic EventRS usage example demonstrating core functionality.

use eventrs::prelude::*;

// Define events
#[derive(Clone, Debug)]
struct UserLoggedIn {
    user_id: u64,
    username: String,
    timestamp: std::time::SystemTime,
}

impl Event for UserLoggedIn {}

#[derive(Clone, Debug)]
struct OrderCreated {
    order_id: u64,
    user_id: u64,
    total: f64,
}

impl Event for OrderCreated {}

fn main() {
    println!("🚀 EventRS Basic Example");

    let mut bus = EventBus::new();

    bus.on(|event: UserLoggedIn| {
        println!(
            "👤 User '{}' (ID: {}) logged in at {:?}",
            event.username, event.user_id, event.timestamp
        );
    });

    bus.on(|event: OrderCreated| {
        println!(
            "🛒 Order {} created by user {} for ${:.2}",
            event.order_id, event.user_id, event.total
        );
    });

    bus.on_with_priority(
        |event: OrderCreated| {
            if event.total > 100.0 {
                println!("💰 High-value order detected: ${:.2}", event.total);
            }
        },
        Priority::High,
    );

    println!("\n📡 Emitting events...\n");

    let login_event = UserLoggedIn {
        user_id: 123,
        username: "alice".to_string(),
        timestamp: std::time::SystemTime::now(),
    };

    let order_event = OrderCreated {
        order_id: 456,
        user_id: 123,
        total: 149.99,
    };

    bus.emit(login_event).expect("Failed to emit login event");
    bus.emit(order_event).expect("Failed to emit order event");

    println!("\n✅ All events processed successfully!");
    println!(
        "📊 Total handlers registered: {}",
        bus.total_handler_count()
    );
}
