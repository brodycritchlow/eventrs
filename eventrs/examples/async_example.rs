//! Async EventBus example demonstrating asynchronous event handling.

use eventrs::prelude::*;
use std::time::SystemTime;
use tokio::time::{sleep, Duration};

// Define async-compatible events
#[derive(Event, Clone, Debug)]
struct AsyncUserLoggedIn {
    user_id: u64,
    username: String,
    timestamp: SystemTime,
}

#[derive(Event, Clone, Debug)]
struct AsyncOrderCreated {
    order_id: u64,
    user_id: u64,
    amount: f64,
    timestamp: SystemTime,
}

#[derive(Event, Clone, Debug)]
struct AsyncDataProcessed {
    data_id: u64,
    bytes_processed: usize,
    processing_time_ms: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 EventRS Async Example");
    println!();

    // Create an async event bus with custom configuration
    let config = AsyncEventBusConfig {
        concurrent_execution: true,
        max_concurrent_handlers: Some(50),
        use_priority_ordering: true,
        ..Default::default()
    };

    let mut bus = AsyncEventBus::with_config(config);

    println!("📋 Registering async handlers...");

    // Register async handlers with different priorities

    // High priority security handler
    bus.on_with_priority(
        |event: AsyncUserLoggedIn| async move {
            println!(
                "🔒 [HIGH PRIORITY] Security audit: User '{}' (ID: {}) logged in",
                event.username, event.user_id
            );

            // Simulate async security check
            sleep(Duration::from_millis(5)).await;
            println!("   ✅ Security check passed for user {}", event.user_id);
        },
        Priority::High,
    )
    .await;

    // Normal priority welcome handler
    bus.on(|event: AsyncUserLoggedIn| async move {
        println!(
            "👋 Welcome back, {}! You logged in at {:?}",
            event.username, event.timestamp
        );

        // Simulate async database update
        sleep(Duration::from_millis(10)).await;
        println!("   📝 User session updated in database");
    })
    .await;

    // Order processing handlers
    bus.on_with_priority(
        |event: AsyncOrderCreated| async move {
            println!(
                "💰 [HIGH PRIORITY] Processing high-value order: ${:.2} (Order: {})",
                event.amount, event.order_id
            );

            // Simulate async payment processing
            sleep(Duration::from_millis(20)).await;
            println!("   💳 Payment processed successfully");
        },
        Priority::High,
    )
    .await;

    bus.on(|event: AsyncOrderCreated| async move {
        println!(
            "📦 Order {} created by user {} for ${:.2}",
            event.order_id, event.user_id, event.amount
        );

        // Simulate async inventory update
        sleep(Duration::from_millis(15)).await;
        println!("   📊 Inventory updated");
    })
    .await;

    // Data processing handler
    bus.on(|event: AsyncDataProcessed| async move {
        println!(
            "🔄 Data processing completed: {} bytes in {}ms (ID: {})",
            event.bytes_processed, event.processing_time_ms, event.data_id
        );

        // Simulate async analytics update
        sleep(Duration::from_millis(8)).await;
        println!("   📈 Analytics metrics updated");
    })
    .await;

    println!(
        "📊 Total handlers registered: {}",
        bus.total_handler_count().await
    );
    println!();

    println!("📡 Emitting async events...");
    println!();

    // Emit events asynchronously
    let start_time = std::time::Instant::now();

    // User login event
    bus.emit(AsyncUserLoggedIn {
        user_id: 456,
        username: "bob".to_string(),
        timestamp: SystemTime::now(),
    })
    .await?;

    // Order creation event
    bus.emit(AsyncOrderCreated {
        order_id: 789,
        user_id: 456,
        amount: 299.99,
        timestamp: SystemTime::now(),
    })
    .await?;

    // Data processing event
    bus.emit(AsyncDataProcessed {
        data_id: 101,
        bytes_processed: 1024 * 1024, // 1MB
        processing_time_ms: 250,
    })
    .await?;

    // Wait a bit for all async handlers to complete
    sleep(Duration::from_millis(100)).await;

    let elapsed = start_time.elapsed();
    println!();
    println!("⏱️  All events processed in {:?}", elapsed);
    println!("✅ Async event processing completed successfully!");

    // Demonstrate sequential vs concurrent execution
    println!();
    println!("🔄 Testing sequential vs concurrent execution...");

    // Create a new bus with many handlers to demonstrate concurrency benefits
    let concurrent_config = AsyncEventBusConfig {
        concurrent_execution: true,
        concurrent_threshold: 3, // Lower threshold for testing
        ..Default::default()
    };
    let mut concurrent_bus = AsyncEventBus::with_config(concurrent_config);

    // Add multiple handlers that take some time to execute
    for i in 0..6 {
        concurrent_bus
            .on(move |event: AsyncDataProcessed| async move {
                println!(
                    "   🔄 Concurrent handler {}: {} bytes",
                    i, event.bytes_processed
                );
                sleep(Duration::from_millis(20)).await; // Simulate work
            })
            .await;
    }

    // Test with concurrent execution
    let start = std::time::Instant::now();
    concurrent_bus
        .emit(AsyncDataProcessed {
            data_id: 200,
            bytes_processed: 512,
            processing_time_ms: 50,
        })
        .await?;
    // Wait for all handlers to complete
    sleep(Duration::from_millis(100)).await;
    let concurrent_time = start.elapsed();

    // Create a bus with sequential execution
    let sequential_config = AsyncEventBusConfig {
        concurrent_execution: false,
        ..Default::default()
    };
    let mut sequential_bus = AsyncEventBus::with_config(sequential_config);

    // Add the same number of handlers
    for i in 0..6 {
        sequential_bus
            .on(move |event: AsyncDataProcessed| async move {
                println!(
                    "   🔄 Sequential handler {}: {} bytes",
                    i, event.bytes_processed
                );
                sleep(Duration::from_millis(20)).await; // Same work
            })
            .await;
    }

    let start = std::time::Instant::now();
    sequential_bus
        .emit(AsyncDataProcessed {
            data_id: 300,
            bytes_processed: 512,
            processing_time_ms: 50,
        })
        .await?;
    let sequential_time = start.elapsed();

    println!();
    println!("📊 Performance comparison:");
    println!(
        "   Concurrent execution (6 handlers): {:?}",
        concurrent_time
    );
    println!(
        "   Sequential execution (6 handlers): {:?}",
        sequential_time
    );

    if concurrent_time < sequential_time {
        let speedup = sequential_time.as_millis() as f64 / concurrent_time.as_millis() as f64;
        println!("   🚀 Concurrent execution is {:.2}x faster!", speedup);
    } else {
        println!("   ⚠️ Sequential execution was faster for this workload");
    }

    // Demonstrate graceful shutdown
    println!();
    println!("🛑 Demonstrating graceful shutdown...");
    bus.shutdown().await?;

    // This should fail
    match bus
        .emit(AsyncUserLoggedIn {
            user_id: 999,
            username: "test".to_string(),
            timestamp: SystemTime::now(),
        })
        .await
    {
        Err(EventBusError::ShuttingDown) => {
            println!("✅ Event correctly rejected after shutdown");
        }
        _ => {
            println!("❌ Expected shutdown error");
        }
    }

    println!();
    println!("🎉 Async example completed successfully!");

    Ok(())
}
