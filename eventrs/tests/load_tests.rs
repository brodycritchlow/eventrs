//! Load tests for EventRS
//!
//! These tests verify that the system can handle high loads and stress conditions
//! without failures or significant performance degradation.

use eventrs::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
struct LoadTestEvent {
    id: u64,
    thread_id: usize,
    timestamp: std::time::SystemTime,
    payload: Vec<u8>,
}

impl Event for LoadTestEvent {
    fn event_type_name() -> &'static str {
        "LoadTestEvent"
    }
}

#[derive(Clone, Debug)]
struct HighVolumeEvent {
    sequence: u64,
    data: String,
}

impl Event for HighVolumeEvent {
    fn event_type_name() -> &'static str {
        "HighVolumeEvent"
    }
}

/// Test high-volume event processing
#[test]
#[ignore = "Load test - run with --ignored"]
fn test_high_volume_processing() {
    let bus = Arc::new(ThreadSafeEventBus::new());
    let processed_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Register handler
    let processed_clone = Arc::clone(&processed_count);
    bus.on(move |_event: HighVolumeEvent| {
        processed_clone.fetch_add(1, Ordering::Relaxed);
        // Simulate some processing work
        thread::sleep(Duration::from_micros(10));
    });

    let total_events = 100_000usize;
    let thread_count = 8usize;
    let events_per_thread = total_events / thread_count;

    let barrier = Arc::new(Barrier::new(thread_count));
    let start_time = Arc::new(Mutex::new(None));

    println!(
        "Starting high-volume load test with {} events across {} threads",
        total_events, thread_count
    );

    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let bus_clone = Arc::clone(&bus);
            let error_clone = Arc::clone(&error_count);
            let barrier_clone = Arc::clone(&barrier);
            let start_time_clone = Arc::clone(&start_time);

            thread::spawn(move || {
                // Wait for all threads to be ready
                barrier_clone.wait();

                // First thread records start time
                if thread_id == 0 {
                    *start_time_clone.lock().unwrap() = Some(Instant::now());
                }

                for i in 0..events_per_thread {
                    let event = HighVolumeEvent {
                        sequence: (thread_id * events_per_thread + i) as u64,
                        data: format!("Data from thread {} event {}", thread_id, i),
                    };

                    if let Err(_) = bus_clone.emit(event) {
                        error_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    let start = start_time.lock().unwrap().unwrap();
    let duration = start.elapsed();

    let processed = processed_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("High-volume test completed:");
    println!("  Events processed: {}", processed);
    println!("  Errors: {}", errors);
    println!("  Duration: {:?}", duration);
    println!(
        "  Events/sec: {:.2}",
        processed as f64 / duration.as_secs_f64()
    );

    // Assertions
    assert_eq!(
        errors, 0,
        "No errors should occur during high-volume processing"
    );
    assert_eq!(processed, total_events, "All events should be processed");

    // Should process at least 1000 events per second under load
    let events_per_second = processed as f64 / duration.as_secs_f64();
    assert!(
        events_per_second > 1000.0,
        "Processing rate too low: {:.2} events/sec",
        events_per_second
    );
}

/// Test sustained load over time
#[test]
#[ignore = "Load test - run with --ignored"]
fn test_sustained_load() {
    let bus = Arc::new(ThreadSafeEventBus::new());
    let processed_count = Arc::new(AtomicUsize::new(0));
    let running = Arc::new(AtomicBool::new(true));

    // Register handler
    let processed_clone = Arc::clone(&processed_count);
    bus.on(move |_event: LoadTestEvent| {
        processed_clone.fetch_add(1, Ordering::Relaxed);
    });

    let test_duration = Duration::from_secs(30); // 30 second sustained test
    let thread_count = 4usize;

    println!(
        "Starting sustained load test for {:?} with {} producer threads",
        test_duration, thread_count
    );

    let start_time = Instant::now();

    // Start producer threads
    let producers: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let bus_clone = Arc::clone(&bus);
            let running_clone = Arc::clone(&running);

            thread::spawn(move || {
                let mut event_id = 0u64;
                while running_clone.load(Ordering::Relaxed) {
                    let event = LoadTestEvent {
                        id: event_id,
                        thread_id,
                        timestamp: std::time::SystemTime::now(),
                        payload: vec![0u8; 64], // 64-byte payload
                    };

                    if bus_clone.emit(event).is_ok() {
                        event_id += 1;
                    }

                    // Small delay to prevent overwhelming the system
                    thread::sleep(Duration::from_micros(100));
                }

                event_id
            })
        })
        .collect();

    // Monitor progress
    let monitor_handle = {
        let processed_clone = Arc::clone(&processed_count);
        let running_clone = Arc::clone(&running);

        thread::spawn(move || {
            let mut last_count = 0;
            while running_clone.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(5));
                let current_count = processed_clone.load(Ordering::Relaxed);
                let rate = (current_count - last_count) as f64 / 5.0;
                println!(
                    "  Events processed: {}, Rate: {:.2} events/sec",
                    current_count, rate
                );
                last_count = current_count;
            }
        })
    };

    // Wait for test duration
    thread::sleep(test_duration);
    running.store(false, Ordering::Relaxed);

    // Wait for all threads to finish
    let mut total_produced = 0;
    for producer in producers {
        total_produced += producer.join().unwrap();
    }
    monitor_handle.join().unwrap();

    let actual_duration = start_time.elapsed();
    let processed = processed_count.load(Ordering::Relaxed);

    println!("Sustained load test completed:");
    println!("  Duration: {:?}", actual_duration);
    println!("  Events produced: {}", total_produced);
    println!("  Events processed: {}", processed);
    println!(
        "  Average rate: {:.2} events/sec",
        processed as f64 / actual_duration.as_secs_f64()
    );

    // All produced events should be processed
    assert_eq!(
        processed, total_produced,
        "All produced events should be processed"
    );

    // Should maintain reasonable processing rate
    let avg_rate = processed as f64 / actual_duration.as_secs_f64();
    assert!(
        avg_rate > 100.0,
        "Average processing rate too low: {:.2} events/sec",
        avg_rate
    );
}

/// Test memory stability under load
#[test]
#[ignore = "Load test - run with --ignored"]
fn test_memory_stability() {
    let bus = Arc::new(ThreadSafeEventBus::new());
    let processed_count = Arc::new(AtomicUsize::new(0));

    // Register handler that processes events quickly
    let processed_clone = Arc::clone(&processed_count);
    bus.on(move |_event: LoadTestEvent| {
        processed_clone.fetch_add(1, Ordering::Relaxed);
    });

    let batch_count = 100usize;
    let events_per_batch = 1000usize;
    let total_events = batch_count * events_per_batch;

    println!(
        "Starting memory stability test with {} batches of {} events",
        batch_count, events_per_batch
    );

    let start_time = Instant::now();

    for batch in 0..batch_count {
        let batch_start = Instant::now();

        // Process a batch of events
        for i in 0..events_per_batch {
            let event = LoadTestEvent {
                id: (batch * events_per_batch + i) as u64,
                thread_id: 0,
                timestamp: std::time::SystemTime::now(),
                payload: vec![42u8; 256], // 256-byte payload
            };

            bus.emit(event).unwrap();
        }

        let batch_duration = batch_start.elapsed();
        let processed_so_far = processed_count.load(Ordering::Relaxed);

        if batch % 10 == 0 {
            println!(
                "  Batch {}: {} events processed, batch took {:?}",
                batch, processed_so_far, batch_duration
            );
        }

        // Brief pause to allow cleanup
        thread::sleep(Duration::from_millis(10));
    }

    let total_duration = start_time.elapsed();
    let final_processed = processed_count.load(Ordering::Relaxed);

    println!("Memory stability test completed:");
    println!("  Total events: {}", total_events);
    println!("  Events processed: {}", final_processed);
    println!("  Duration: {:?}", total_duration);
    println!(
        "  Average rate: {:.2} events/sec",
        final_processed as f64 / total_duration.as_secs_f64()
    );

    assert_eq!(
        final_processed, total_events,
        "All events should be processed"
    );
}

/// Test concurrent handler registration under load
#[test]
#[ignore = "Load test - run with --ignored"]
fn test_concurrent_handler_registration() {
    let bus = Arc::new(ThreadSafeEventBus::new());
    let processed_counts: Arc<Mutex<Vec<Arc<AtomicUsize>>>> = Arc::new(Mutex::new(Vec::new()));
    let registration_count = Arc::new(AtomicUsize::new(0));

    let handler_count = 50usize;
    let events_per_handler = 100usize;

    println!(
        "Starting concurrent handler registration test with {} handlers",
        handler_count
    );

    // Register handlers concurrently
    let registration_handles: Vec<_> = (0..handler_count)
        .map(|handler_id| {
            let bus_clone = Arc::clone(&bus);
            let counts_clone = Arc::clone(&processed_counts);
            let reg_count_clone = Arc::clone(&registration_count);

            thread::spawn(move || {
                let counter = Arc::new(AtomicUsize::new(0));
                counts_clone.lock().unwrap().push(Arc::clone(&counter));

                bus_clone.on(move |_event: LoadTestEvent| {
                    counter.fetch_add(1, Ordering::Relaxed);
                });

                reg_count_clone.fetch_add(1, Ordering::Relaxed);

                // Emit some events after registering
                for i in 0..events_per_handler {
                    let event = LoadTestEvent {
                        id: (handler_id * events_per_handler + i) as u64,
                        thread_id: handler_id,
                        timestamp: std::time::SystemTime::now(),
                        payload: vec![handler_id as u8; 32],
                    };

                    bus_clone.emit(event).unwrap();
                }
            })
        })
        .collect();

    // Wait for all handlers to register and emit events
    for handle in registration_handles {
        handle.join().unwrap();
    }

    // Wait a bit for all events to be processed
    thread::sleep(Duration::from_millis(100));

    let registered = registration_count.load(Ordering::Relaxed);
    let counts = processed_counts.lock().unwrap();
    let total_processed: usize = counts
        .iter()
        .map(|counter| counter.load(Ordering::Relaxed))
        .sum();

    println!("Concurrent registration test completed:");
    println!("  Handlers registered: {}", registered);
    println!("  Total events processed: {}", total_processed);

    assert_eq!(
        registered, handler_count,
        "All handlers should register successfully"
    );

    // Each event should be processed by all handlers
    let expected_total = handler_count * handler_count * events_per_handler;
    assert_eq!(
        total_processed, expected_total,
        "Each event should be processed by all handlers"
    );
}

/// Test error resilience under load
#[test]
#[ignore = "Load test - run with --ignored"]
fn test_error_resilience() {
    let bus = Arc::new(ThreadSafeEventBus::new());
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Register a handler that sometimes panics
    let success_clone = Arc::clone(&success_count);
    let error_clone = Arc::clone(&error_count);

    bus.on(move |event: LoadTestEvent| {
        if event.id % 10 == 0 {
            // Simulate an error condition
            error_clone.fetch_add(1, Ordering::Relaxed);
            panic!("Simulated handler error for event {}", event.id);
        } else {
            success_clone.fetch_add(1, Ordering::Relaxed);
        }
    });

    let total_events = 1000usize;

    println!(
        "Starting error resilience test with {} events (10% error rate)",
        total_events
    );

    let start_time = Instant::now();

    for i in 0..total_events {
        let event = LoadTestEvent {
            id: i,
            thread_id: 0,
            timestamp: std::time::SystemTime::now(),
            payload: vec![0u8; 16],
        };

        // Bus should continue operating even if handlers panic
        let _ = bus.emit(event); // Ignore the result as some handlers will panic
    }

    let duration = start_time.elapsed();

    // Wait for all events to be processed
    thread::sleep(Duration::from_millis(100));

    let successes = success_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("Error resilience test completed:");
    println!("  Duration: {:?}", duration);
    println!("  Successful handlers: {}", successes);
    println!("  Error handlers: {}", errors);
    println!("  Total: {}", successes + errors);

    // The bus should continue operating and process successful events
    assert!(
        successes > 0,
        "Some events should be processed successfully"
    );
    assert_eq!(
        successes + errors,
        total_events,
        "All events should reach handlers"
    );

    // Should maintain reasonable processing speed even with errors
    let events_per_second = total_events as f64 / duration.as_secs_f64();
    assert!(
        events_per_second > 1000.0,
        "Processing should remain fast despite errors"
    );
}

/// Test scalability with increasing load
#[test]
#[ignore = "Load test - run with --ignored"]
fn test_scalability() {
    let bus = Arc::new(ThreadSafeEventBus::new());
    let processed_count = Arc::new(AtomicUsize::new(0));

    let processed_clone = Arc::clone(&processed_count);
    bus.on(move |_event: LoadTestEvent| {
        processed_clone.fetch_add(1, Ordering::Relaxed);
    });

    // Test with increasing load levels
    let load_levels = vec![1000, 5000, 10000, 25000];

    println!("Starting scalability test with increasing loads");

    for &event_count in &load_levels {
        processed_count.store(0, Ordering::Relaxed);

        let start_time = Instant::now();

        for i in 0..event_count {
            let event = LoadTestEvent {
                id: i,
                thread_id: 0,
                timestamp: std::time::SystemTime::now(),
                payload: vec![0u8; 64],
            };

            bus.emit(event).unwrap();
        }

        let duration = start_time.elapsed();
        let processed = processed_count.load(Ordering::Relaxed);
        let events_per_second = processed as f64 / duration.as_secs_f64();

        println!(
            "  Load level {}: {:.2} events/sec",
            event_count, events_per_second
        );

        assert_eq!(processed, event_count, "All events should be processed");

        // Performance should not degrade significantly with higher loads
        assert!(
            events_per_second > 1000.0,
            "Performance degradation at load level {}: {:.2} events/sec",
            event_count,
            events_per_second
        );
    }
}
