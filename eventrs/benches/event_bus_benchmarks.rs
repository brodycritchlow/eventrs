//! Performance benchmarks for EventRS
//!
//! This module provides comprehensive benchmarks to track performance
//! regressions and ensure optimal event processing throughput.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use eventrs::prelude::*;
use eventrs::{AnyEvent, PredicateAnyFilter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
struct BenchmarkEvent {
    value: u32,
    data: String,
}

impl Event for BenchmarkEvent {
    fn event_type_name() -> &'static str {
        "BenchmarkEvent"
    }
}

#[derive(Clone, Debug)]
struct SimpleEvent {
    id: u64,
}

impl Event for SimpleEvent {
    fn event_type_name() -> &'static str {
        "SimpleEvent"
    }
}

#[derive(Clone, Debug)]
struct ComplexEvent {
    id: u64,
    name: String,
    tags: Vec<String>,
    metadata: std::collections::HashMap<String, String>,
}

impl Event for ComplexEvent {
    fn event_type_name() -> &'static str {
        "ComplexEvent"
    }
}

fn benchmark_basic_emission(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_emission");

    for event_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("sync_event_bus", event_count),
            event_count,
            |b, &event_count| {
                let mut bus = EventBus::new();
                let counter = Arc::new(AtomicUsize::new(0));
                let counter_clone = Arc::clone(&counter);

                bus.on(move |_event: SimpleEvent| {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                });

                b.iter(|| {
                    for i in 0..event_count {
                        bus.emit(SimpleEvent { id: i }).unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn benchmark_thread_safe_emission(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_safe_emission");

    for event_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("thread_safe_event_bus", event_count),
            event_count,
            |b, &event_count| {
                let bus = ThreadSafeEventBus::new();
                let counter = Arc::new(AtomicUsize::new(0));
                let counter_clone = Arc::clone(&counter);

                bus.on(move |_event: SimpleEvent| {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                });

                b.iter(|| {
                    for i in 0..event_count {
                        bus.emit(SimpleEvent { id: i }).unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn benchmark_handler_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("handler_execution");

    for handler_count in [1, 5, 10, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("multiple_handlers", handler_count),
            handler_count,
            |b, &handler_count| {
                let mut bus = EventBus::new();

                for _ in 0..handler_count {
                    let counter = Arc::new(AtomicUsize::new(0));
                    bus.on(move |_event: SimpleEvent| {
                        counter.fetch_add(1, Ordering::Relaxed);
                    });
                }

                let event = SimpleEvent { id: 42 };

                b.iter(|| {
                    bus.emit(black_box(event.clone())).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn benchmark_complex_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_events");

    group.bench_function("complex_event_emission", |b| {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.on(move |_event: ComplexEvent| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        let event = ComplexEvent {
            id: 12345,
            name: "Test Event".to_string(),
            tags: vec!["benchmark".to_string(), "performance".to_string()],
            metadata,
        };

        b.iter(|| {
            bus.emit(black_box(event.clone())).unwrap();
        });
    });

    group.finish();
}

fn benchmark_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("filtering");

    group.bench_function("predicate_filter", |b| {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.on(move |_event: BenchmarkEvent| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        // Add a filter that only allows even values
        let filter = PredicateAnyFilter::new("even_filter", |event: &dyn AnyEvent| {
            if let Some(benchmark_event) = event.as_any().downcast_ref::<BenchmarkEvent>() {
                benchmark_event.value % 2 == 0
            } else {
                false
            }
        });
        bus.add_global_filter("even_filter", Box::new(filter));

        b.iter(|| {
            for i in 0..1000 {
                let event = BenchmarkEvent {
                    value: i,
                    data: format!("data_{}", i),
                };
                bus.emit(black_box(event)).unwrap();
            }
        });
    });

    group.finish();
}

fn benchmark_middleware(c: &mut Criterion) {
    let mut group = c.benchmark_group("middleware");

    group.bench_function("logging_middleware", |b| {
        let mut bus = EventBusBuilder::new()
            .with_middleware(Box::new(LoggingMiddleware::new("benchmark".to_string())))
            .build();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.on(move |_event: SimpleEvent| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        let event = SimpleEvent { id: 123 };

        b.iter(|| {
            bus.emit(black_box(event.clone())).unwrap();
        });
    });

    group.finish();
}

#[cfg(feature = "metrics")]
fn benchmark_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics");

    group.bench_function("metrics_collection", |b| {
        let mut bus = EventBusBuilder::new().with_metrics(true).build();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.on(move |_event: SimpleEvent| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        let event = SimpleEvent { id: 456 };

        b.iter(|| {
            bus.emit(black_box(event.clone())).unwrap();
        });
    });

    group.finish();
}

fn benchmark_batch_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processing");

    for batch_size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_emit", batch_size),
            batch_size,
            |b, &batch_size| {
                let bus = ThreadSafeEventBus::new();
                let counter = Arc::new(AtomicUsize::new(0));
                let counter_clone = Arc::clone(&counter);

                bus.on(move |_event: SimpleEvent| {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                });

                let events: Vec<SimpleEvent> =
                    (0..batch_size).map(|i| SimpleEvent { id: i }).collect();

                b.iter(|| {
                    bus.emit_batch(black_box(events.clone())).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn benchmark_priority_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_handling");

    group.bench_function("priority_execution", |b| {
        let mut bus = EventBus::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        let order1 = Arc::clone(&order);
        bus.on_with_priority(
            move |_event: SimpleEvent| {
                order1.lock().unwrap().push(1);
            },
            Priority::High,
        );

        let order2 = Arc::clone(&order);
        bus.on_with_priority(
            move |_event: SimpleEvent| {
                order2.lock().unwrap().push(2);
            },
            Priority::Normal,
        );

        let order3 = Arc::clone(&order);
        bus.on_with_priority(
            move |_event: SimpleEvent| {
                order3.lock().unwrap().push(3);
            },
            Priority::Low,
        );

        let event = SimpleEvent { id: 789 };

        b.iter(|| {
            order.lock().unwrap().clear();
            bus.emit(black_box(event.clone())).unwrap();
        });
    });

    group.finish();
}

fn benchmark_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");

    group.bench_function("concurrent_emission", |b| {
        let bus = Arc::new(ThreadSafeEventBus::new());
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.on(move |_event: SimpleEvent| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        b.iter(|| {
            let handles: Vec<_> = (0..4)
                .map(|thread_id| {
                    let bus_clone = Arc::clone(&bus);
                    std::thread::spawn(move || {
                        for i in 0..100 {
                            let event = SimpleEvent {
                                id: (thread_id * 100) + i,
                            };
                            bus_clone.emit(event).unwrap();
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.finish();
}

// Group all benchmarks
criterion_group!(
    benches,
    benchmark_basic_emission,
    benchmark_thread_safe_emission,
    benchmark_handler_execution,
    benchmark_complex_events,
    benchmark_filtering,
    benchmark_middleware,
    benchmark_batch_processing,
    benchmark_priority_handling,
    benchmark_concurrent_access
);

// Add metrics benchmark only if feature is enabled
#[cfg(feature = "metrics")]
criterion_group!(metrics_benches, benchmark_metrics);

#[cfg(feature = "metrics")]
criterion_main!(benches, metrics_benches);

#[cfg(not(feature = "metrics"))]
criterion_main!(benches);
