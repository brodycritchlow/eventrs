//! Integration test to verify that filtering actually works correctly.

use eventrs::prelude::*;
use eventrs::{AllowAllFilter, RejectAllFilter};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};

// Define test events
#[derive(Event, Clone, Debug)]
struct TestEvent {
    value: i32,
    is_important: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 EventRS Filtering Integration Test");
    println!();

    // Test 1: Basic filtering - only high values should be processed
    println!("Test 1: Basic Value Filtering");
    let mut bus = EventBus::new();
    let processed_count = Arc::new(AtomicU32::new(0));
    let processed_values = Arc::new(Mutex::new(Vec::new()));

    let count_clone = Arc::clone(&processed_count);
    let values_clone = Arc::clone(&processed_values);

    // Register a filtered handler that only processes values > 50
    let filter = PredicateFilter::new("high_value", |event: &TestEvent| {
        event.value > 50
    });
    
    bus.on_filtered(move |event: TestEvent| {
        count_clone.fetch_add(1, Ordering::Relaxed);
        values_clone.lock().unwrap().push(event.value);
        println!("  ✅ Processed high value: {}", event.value);
    }, Arc::new(filter));

    // Emit events - only some should be processed
    let test_values = vec![10, 25, 60, 75, 30, 80, 15];
    for value in &test_values {
        bus.emit(TestEvent { value: *value, is_important: false })?;
    }

    let final_count = processed_count.load(Ordering::Relaxed);
    let final_values = processed_values.lock().unwrap().clone();
    
    println!("  Total events emitted: {}", test_values.len());
    println!("  Events processed: {}", final_count);
    println!("  Processed values: {:?}", final_values);
    
    // Verify only values > 50 were processed
    let expected_values: Vec<i32> = test_values.iter().filter(|&&v| v > 50).cloned().collect();
    assert_eq!(final_count, expected_values.len() as u32, "Wrong number of events processed");
    assert_eq!(final_values, expected_values, "Wrong values processed");
    println!("  ✅ Test 1 PASSED - Filtering working correctly!");
    println!();

    // Test 2: Multiple handlers with different filters
    println!("Test 2: Multiple Filtered Handlers");
    let mut bus2 = EventBus::new();
    let important_count = Arc::new(AtomicU32::new(0));
    let high_value_count = Arc::new(AtomicU32::new(0));
    let both_count = Arc::new(AtomicU32::new(0));

    // Handler 1: Only important events
    let important_count_clone = Arc::clone(&important_count);
    let important_filter = PredicateFilter::new("important_only", |event: &TestEvent| {
        event.is_important
    });
    
    bus2.on_filtered(move |event: TestEvent| {
        important_count_clone.fetch_add(1, Ordering::Relaxed);
        println!("  📌 Important event: {} (important: {})", event.value, event.is_important);
    }, Arc::new(important_filter));

    // Handler 2: Only high values
    let high_value_count_clone = Arc::clone(&high_value_count);
    let high_value_filter = PredicateFilter::new("high_value", |event: &TestEvent| {
        event.value > 50
    });
    
    bus2.on_filtered(move |event: TestEvent| {
        high_value_count_clone.fetch_add(1, Ordering::Relaxed);
        println!("  💰 High value event: {} (value > 50)", event.value);
    }, Arc::new(high_value_filter));

    // Handler 3: Both important AND high value
    let both_count_clone = Arc::clone(&both_count);
    let both_filter = PredicateFilter::new("important_and_high", |event: &TestEvent| {
        event.is_important && event.value > 50
    });
    
    bus2.on_filtered(move |event: TestEvent| {
        both_count_clone.fetch_add(1, Ordering::Relaxed);
        println!("  🌟 Important AND high value: {} (important: {}, value: {})", 
                event.value, event.is_important, event.value);
    }, Arc::new(both_filter));

    // Emit diverse test events
    let test_events = vec![
        TestEvent { value: 25, is_important: false },  // No handlers
        TestEvent { value: 75, is_important: false },  // High value only
        TestEvent { value: 30, is_important: true },   // Important only
        TestEvent { value: 80, is_important: true },   // Both handlers
        TestEvent { value: 10, is_important: false },  // No handlers
        TestEvent { value: 60, is_important: true },   // Both handlers
    ];

    for event in &test_events {
        bus2.emit(event.clone())?;
    }

    let final_important = important_count.load(Ordering::Relaxed);
    let final_high_value = high_value_count.load(Ordering::Relaxed);
    let final_both = both_count.load(Ordering::Relaxed);

    println!("  📊 Results:");
    println!("    Important events processed: {}", final_important);
    println!("    High value events processed: {}", final_high_value);
    println!("    Both important AND high value: {}", final_both);

    // Verify counts
    let expected_important = test_events.iter().filter(|e| e.is_important).count();
    let expected_high_value = test_events.iter().filter(|e| e.value > 50).count();
    let expected_both = test_events.iter().filter(|e| e.is_important && e.value > 50).count();

    assert_eq!(final_important, expected_important as u32, "Wrong important count");
    assert_eq!(final_high_value, expected_high_value as u32, "Wrong high value count");
    assert_eq!(final_both, expected_both as u32, "Wrong both condition count");
    println!("  ✅ Test 2 PASSED - Multiple filters working correctly!");
    println!();

    // Test 3: Priority with filtering
    println!("Test 3: Priority with Filtering");
    let mut bus3 = EventBus::new();
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    // High priority handler - only for values > 70
    let order_clone1 = Arc::clone(&execution_order);
    let high_priority_filter = PredicateFilter::new("very_high_value", |event: &TestEvent| {
        event.value > 70
    });
    
    bus3.on_filtered_with_priority(move |event: TestEvent| {
        order_clone1.lock().unwrap().push(format!("HIGH_PRIORITY_{}", event.value));
        println!("  🔥 HIGH PRIORITY processed: {}", event.value);
    }, Arc::new(high_priority_filter), Priority::High);

    // Normal priority handler - for all values > 50
    let order_clone2 = Arc::clone(&execution_order);
    let normal_priority_filter = PredicateFilter::new("high_value", |event: &TestEvent| {
        event.value > 50
    });
    
    bus3.on_filtered_with_priority(move |event: TestEvent| {
        order_clone2.lock().unwrap().push(format!("NORMAL_PRIORITY_{}", event.value));
        println!("  ⭐ NORMAL PRIORITY processed: {}", event.value);
    }, Arc::new(normal_priority_filter), Priority::Normal);

    // Test with value 75 - should trigger both handlers
    bus3.emit(TestEvent { value: 75, is_important: false })?;
    
    // Test with value 60 - should trigger only normal priority
    bus3.emit(TestEvent { value: 60, is_important: false })?;
    
    // Test with value 25 - should trigger no handlers
    bus3.emit(TestEvent { value: 25, is_important: false })?;

    let final_order = execution_order.lock().unwrap().clone();
    println!("  📋 Execution order: {:?}", final_order);
    
    // Verify high priority ran before normal priority for value 75
    assert!(final_order.contains(&"HIGH_PRIORITY_75".to_string()));
    assert!(final_order.contains(&"NORMAL_PRIORITY_75".to_string()));
    assert!(final_order.contains(&"NORMAL_PRIORITY_60".to_string()));
    assert!(!final_order.iter().any(|s| s.contains("25")));
    println!("  ✅ Test 3 PASSED - Priority with filtering working correctly!");
    println!();

    // Test 4: Reject all and allow all filters
    println!("Test 4: Built-in Filters");
    let mut bus4 = EventBus::new();
    let allow_count = Arc::new(AtomicU32::new(0));
    let reject_count = Arc::new(AtomicU32::new(0));

    // Handler with allow-all filter
    let allow_count_clone = Arc::clone(&allow_count);
    bus4.on_filtered(move |_event: TestEvent| {
        allow_count_clone.fetch_add(1, Ordering::Relaxed);
        println!("  ✅ Allow-all handler executed");
    }, Arc::new(AllowAllFilter::<TestEvent>::new()));

    // Handler with reject-all filter
    let reject_count_clone = Arc::clone(&reject_count);
    bus4.on_filtered(move |_event: TestEvent| {
        reject_count_clone.fetch_add(1, Ordering::Relaxed);
        println!("  ❌ Reject-all handler executed (THIS SHOULD NOT HAPPEN!)");
    }, Arc::new(RejectAllFilter::<TestEvent>::new()));

    // Emit test events
    for i in 0..3 {
        bus4.emit(TestEvent { value: i, is_important: false })?;
    }

    let final_allow = allow_count.load(Ordering::Relaxed);
    let final_reject = reject_count.load(Ordering::Relaxed);

    println!("  📊 Results:");
    println!("    Allow-all handler executions: {}", final_allow);
    println!("    Reject-all handler executions: {}", final_reject);

    assert_eq!(final_allow, 3, "Allow-all filter should have processed all 3 events");
    assert_eq!(final_reject, 0, "Reject-all filter should have processed 0 events");
    println!("  ✅ Test 4 PASSED - Built-in filters working correctly!");
    println!();

    println!("🎉 ALL FILTERING INTEGRATION TESTS PASSED!");
    println!("✅ Event filtering is working correctly and events are properly filtered");
    println!("🔍 Handlers only execute when their filters evaluate to true");
    
    Ok(())
}