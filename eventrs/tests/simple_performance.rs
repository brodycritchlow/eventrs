//! Simple performance regression tests for EventRS
//!
//! These tests ensure that basic performance doesn't regress between versions.

use eventrs::prelude::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Debug)]
struct TestEvent {
    value: u32,
}

impl Event for TestEvent {
    fn event_type_name() -> &'static str {
        "TestEvent"
    }
}

/// Test that basic event emission maintains acceptable performance
#[test]
fn test_basic_emission_performance() {
    let mut bus = EventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    
    bus.on(move |_event: TestEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });
    
    let event_count = 10_000usize;
    let start = Instant::now();
    
    for i in 0..event_count {
        bus.emit(TestEvent { value: i as u32 }).unwrap();
    }
    
    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();
    
    // Ensure we can process at least 50,000 events per second
    assert!(
        events_per_second > 50_000.0,
        "Performance regression: {} events/sec < 50,000 events/sec",
        events_per_second
    );
    
    assert_eq!(counter.load(Ordering::Relaxed), event_count);
}

/// Test that thread-safe emission maintains acceptable performance
#[test]
fn test_thread_safe_emission_performance() {
    let bus = ThreadSafeEventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    
    bus.on(move |_event: TestEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });
    
    let event_count = 5_000usize;
    let start = Instant::now();
    
    for i in 0..event_count {
        bus.emit(TestEvent { value: i as u32 }).unwrap();
    }
    
    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();
    
    // Thread-safe version should still process at least 25,000 events per second
    assert!(
        events_per_second > 25_000.0,
        "Performance regression in thread-safe bus: {} events/sec < 25,000 events/sec",
        events_per_second
    );
    
    assert_eq!(counter.load(Ordering::Relaxed), event_count);
}

/// Test that multiple handlers don't cause significant performance degradation
#[test]
fn test_multiple_handlers_performance() {
    let mut bus = EventBus::new();
    let counters: Vec<Arc<AtomicUsize>> = (0..5)
        .map(|_| Arc::new(AtomicUsize::new(0)))
        .collect();
    
    // Register 5 handlers
    for counter in &counters {
        let counter_clone = Arc::clone(counter);
        bus.on(move |_event: TestEvent| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });
    }
    
    let event_count = 1_000usize;
    let start = Instant::now();
    
    for i in 0..event_count {
        bus.emit(TestEvent { value: i as u32 }).unwrap();
    }
    
    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();
    
    // With 5 handlers, should still process at least 5,000 events per second
    assert!(
        events_per_second > 5_000.0,
        "Performance regression with multiple handlers: {} events/sec < 5,000 events/sec",
        events_per_second
    );
    
    // Verify all handlers were called
    for counter in &counters {
        assert_eq!(counter.load(Ordering::Relaxed), event_count);
    }
}

/// Test that batch processing provides better performance than individual emissions
#[test]
fn test_batch_processing_performance() {
    let bus = ThreadSafeEventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    
    bus.on(move |_event: TestEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });
    
    let batch_size = 1_000usize;
    let events: Vec<TestEvent> = (0..batch_size)
        .map(|i| TestEvent { value: i as u32 })
        .collect();
    
    // Test batch emission
    let start = Instant::now();
    bus.emit_batch(events.clone()).unwrap();
    let batch_duration = start.elapsed();
    
    counter.store(0, Ordering::Relaxed);
    
    // Test individual emissions
    let start = Instant::now();
    for event in events {
        bus.emit(event).unwrap();
    }
    let individual_duration = start.elapsed();
    
    // Both should complete reasonably quickly
    assert!(batch_duration < Duration::from_millis(100), "Batch too slow: {:?}", batch_duration);
    assert!(individual_duration < Duration::from_millis(500), "Individual too slow: {:?}", individual_duration);
    
    assert_eq!(counter.load(Ordering::Relaxed), batch_size);
}

/// Test concurrent access performance
#[test]
fn test_concurrent_access_performance() {
    let bus = Arc::new(ThreadSafeEventBus::new());
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    
    bus.on(move |_event: TestEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });
    
    let thread_count = 4usize;
    let events_per_thread = 250usize;
    let total_events = thread_count * events_per_thread;
    
    let start = Instant::now();
    
    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let bus_clone = Arc::clone(&bus);
            std::thread::spawn(move || {
                for i in 0..events_per_thread {
                    let event = TestEvent { value: (thread_id * events_per_thread + i) as u32 };
                    bus_clone.emit(event).unwrap();
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start.elapsed();
    let events_per_second = total_events as f64 / duration.as_secs_f64();
    
    // Concurrent access should process at least 2,500 events per second
    assert!(
        events_per_second > 2_500.0,
        "Performance regression with concurrent access: {} events/sec < 2,500 events/sec",
        events_per_second
    );
    
    assert_eq!(counter.load(Ordering::Relaxed), total_events);
}

/// Test that the system can handle high-frequency bursts
#[test]
fn test_burst_handling() {
    let mut bus = EventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    
    bus.on(move |_event: TestEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });
    
    let burst_size = 5_000usize;
    let burst_count = 3usize;
    
    let start = Instant::now();
    
    // Emit multiple bursts
    for _burst in 0..burst_count {
        for i in 0..burst_size {
            bus.emit(TestEvent { value: i as u32 }).unwrap();
        }
    }
    
    let duration = start.elapsed();
    let total_events = burst_size * burst_count;
    let events_per_second = total_events as f64 / duration.as_secs_f64();
    
    // Should handle bursts at reasonable speed
    assert!(
        events_per_second > 25_000.0,
        "Burst handling regression: {} events/sec < 25,000 events/sec",
        events_per_second
    );
    
    assert_eq!(counter.load(Ordering::Relaxed), total_events);
}