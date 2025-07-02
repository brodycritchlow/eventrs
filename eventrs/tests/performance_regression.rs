//! Performance regression tests for EventRS
//!
//! These tests ensure that performance doesn't regress between versions
//! by establishing baseline performance expectations.

use eventrs::filter::{AnyEvent, PredicateAnyFilter};
use eventrs::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
struct PerfTestEvent {
    id: u64,
    data: String,
}

impl Event for PerfTestEvent {
    fn event_type_name() -> &'static str {
        "PerfTestEvent"
    }
}

#[derive(Clone, Debug)]
struct SimpleEvent {
    value: u32,
}

impl Event for SimpleEvent {
    fn event_type_name() -> &'static str {
        "SimpleEvent"
    }
}

/// Test that basic event emission maintains acceptable performance
#[test]
fn test_basic_emission_performance() {
    let mut bus = EventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    bus.on(move |_event: SimpleEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    let event_count = 10_000;
    let start = Instant::now();

    for i in 0..event_count {
        bus.emit(SimpleEvent { value: i }).unwrap();
    }

    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();

    // Ensure we can process at least 100,000 events per second
    assert!(
        events_per_second > 100_000.0,
        "Performance regression: {} events/sec < 100,000 events/sec",
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

    bus.on(move |_event: SimpleEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    let event_count = 5_000;
    let start = Instant::now();

    for i in 0..event_count {
        bus.emit(SimpleEvent { value: i }).unwrap();
    }

    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();

    // Thread-safe version should still process at least 50,000 events per second
    assert!(
        events_per_second > 50_000.0,
        "Performance regression in thread-safe bus: {} events/sec < 50,000 events/sec",
        events_per_second
    );

    assert_eq!(counter.load(Ordering::Relaxed), event_count);
}

/// Test that multiple handlers don't cause significant performance degradation
#[test]
fn test_multiple_handlers_performance() {
    let mut bus = EventBus::new();
    let counters: Vec<Arc<AtomicUsize>> = (0..10).map(|_| Arc::new(AtomicUsize::new(0))).collect();

    // Register 10 handlers
    for counter in &counters {
        let counter_clone = Arc::clone(counter);
        bus.on(move |_event: SimpleEvent| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });
    }

    let event_count = 1_000usize;
    let start = Instant::now();

    for i in 0..event_count {
        bus.emit(SimpleEvent { value: i }).unwrap();
    }

    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();

    // With 10 handlers, should still process at least 10,000 events per second
    assert!(
        events_per_second > 10_000.0,
        "Performance regression with multiple handlers: {} events/sec < 10,000 events/sec",
        events_per_second
    );

    // Verify all handlers were called
    for counter in &counters {
        assert_eq!(counter.load(Ordering::Relaxed), event_count);
    }
}

/// Test that filtering doesn't cause significant performance degradation
#[test]
fn test_filtering_performance() {
    let mut bus = EventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    bus.on(move |_event: SimpleEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    // Add a filter that allows 50% of events
    let filter = PredicateAnyFilter::new("even_filter", |event: &dyn AnyEvent| {
        if let Some(simple_event) = event.as_any().downcast_ref::<SimpleEvent>() {
            simple_event.value % 2 == 0
        } else {
            true
        }
    });
    bus.add_global_filter("even_filter", Box::new(filter));

    let event_count = 1_000usize;
    let start = Instant::now();

    for i in 0..event_count {
        bus.emit(SimpleEvent { value: i }).unwrap();
    }

    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();

    // Filtering should still allow at least 10,000 events per second
    assert!(
        events_per_second > 10_000.0,
        "Performance regression with filtering: {} events/sec < 10,000 events/sec",
        events_per_second
    );

    // Only even-valued events should have been processed
    assert_eq!(counter.load(Ordering::Relaxed), event_count / 2);
}

/// Test that batch processing provides better performance than individual emissions
#[test]
fn test_batch_processing_performance() {
    let bus = ThreadSafeEventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    bus.on(move |_event: SimpleEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    let batch_size = 1_000;
    let events: Vec<SimpleEvent> = (0..batch_size).map(|i| SimpleEvent { value: i }).collect();

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

    // Batch processing should be faster than individual emissions
    assert!(
        batch_duration < individual_duration,
        "Batch processing regression: batch took {:?}, individual took {:?}",
        batch_duration,
        individual_duration
    );

    assert_eq!(counter.load(Ordering::Relaxed), batch_size);
}

/// Test that priority handling doesn't cause significant performance degradation
#[test]
fn test_priority_handling_performance() {
    let mut bus = EventBus::new();
    let order = Arc::new(Mutex::new(Vec::new()));

    // Register handlers with different priorities
    for priority in [Priority::High, Priority::Normal, Priority::Low] {
        let order_clone = Arc::clone(&order);
        bus.on_with_priority(
            move |event: SimpleEvent| {
                order_clone.lock().unwrap().push(event.value);
            },
            priority,
        );
    }

    let event_count = 100;
    let start = Instant::now();

    for i in 0..event_count {
        bus.emit(SimpleEvent { value: i }).unwrap();
    }

    let duration = start.elapsed();
    let events_per_second = event_count as f64 / duration.as_secs_f64();

    // Priority handling should still process at least 1,000 events per second
    assert!(
        events_per_second > 1_000.0,
        "Performance regression with priority handling: {} events/sec < 1,000 events/sec",
        events_per_second
    );

    // Verify all events were processed (3 handlers × event_count)
    assert_eq!(order.lock().unwrap().len(), event_count * 3);
}

/// Test concurrent access performance
#[test]
fn test_concurrent_access_performance() {
    let bus = Arc::new(ThreadSafeEventBus::new());
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    bus.on(move |_event: SimpleEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    let thread_count = 4;
    let events_per_thread = 250;
    let total_events = thread_count * events_per_thread;

    let start = Instant::now();

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let bus_clone = Arc::clone(&bus);
            std::thread::spawn(move || {
                for i in 0..events_per_thread {
                    let event = SimpleEvent {
                        value: (thread_id * events_per_thread) + i,
                    };
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

    // Concurrent access should process at least 5,000 events per second
    assert!(
        events_per_second > 5_000.0,
        "Performance regression with concurrent access: {} events/sec < 5,000 events/sec",
        events_per_second
    );

    assert_eq!(counter.load(Ordering::Relaxed), total_events);
}

/// Test memory efficiency - ensure events don't cause excessive allocations
#[test]
fn test_memory_efficiency() {
    let mut bus = EventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    bus.on(move |_event: PerfTestEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    let event_count = 1_000usize;

    // Create events with varying data sizes
    let events: Vec<PerfTestEvent> = (0..event_count)
        .map(|i| PerfTestEvent {
            id: i,
            data: format!("Event data for test event number {}", i),
        })
        .collect();

    let start = Instant::now();

    for event in events {
        bus.emit(event).unwrap();
    }

    let duration = start.elapsed();

    // Ensure reasonable performance even with larger events
    assert!(
        duration < Duration::from_millis(100),
        "Memory efficiency regression: took {:?} to process {} events",
        duration,
        event_count
    );

    assert_eq!(counter.load(Ordering::Relaxed), event_count);
}

/// Test that the system can handle high-frequency bursts
#[test]
fn test_burst_handling() {
    let mut bus = EventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    bus.on(move |_event: SimpleEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    let burst_size = 10_000;
    let burst_count = 5;

    let start = Instant::now();

    // Emit multiple bursts
    for _burst in 0..burst_count {
        for i in 0..burst_size {
            bus.emit(SimpleEvent { value: i }).unwrap();
        }
    }

    let duration = start.elapsed();
    let total_events = burst_size * burst_count;
    let events_per_second = total_events as f64 / duration.as_secs_f64();

    // Should handle bursts at reasonable speed
    assert!(
        events_per_second > 50_000.0,
        "Burst handling regression: {} events/sec < 50,000 events/sec",
        events_per_second
    );

    assert_eq!(counter.load(Ordering::Relaxed), total_events);
}

/// Test middleware performance impact
#[test]
fn test_middleware_performance_impact() {
    // Test without middleware
    let mut bus_without = EventBus::new();
    let counter_without = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter_without);

    bus_without.on(move |_event: SimpleEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    let event_count = 1_000usize;
    let start = Instant::now();

    for i in 0..event_count {
        bus_without.emit(SimpleEvent { value: i as u32 }).unwrap();
    }

    let duration_without = start.elapsed();

    // Test with middleware
    let mut bus_with = EventBusBuilder::new()
        .with_middleware(Box::new(LoggingMiddleware::new("test".to_string())))
        .build();

    let counter_with = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter_with);

    bus_with.on(move |_event: SimpleEvent| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
    });

    let start = Instant::now();

    for i in 0..event_count {
        bus_with.emit(SimpleEvent { value: i as u32 }).unwrap();
    }

    let duration_with = start.elapsed();

    // Middleware should not increase execution time by more than 3x
    let performance_ratio = duration_with.as_nanos() as f64 / duration_without.as_nanos() as f64;
    assert!(
        performance_ratio < 3.0,
        "Middleware performance regression: {}x slower with middleware",
        performance_ratio
    );

    assert_eq!(counter_without.load(Ordering::Relaxed), event_count);
    assert_eq!(counter_with.load(Ordering::Relaxed), event_count);
}
