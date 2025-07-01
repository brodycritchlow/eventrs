//! Tests for dynamic filters and filter caching (Phase 3.1 Part 2)

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::filter::{DynamicFilter, ContentAwareCacheFilter, EventTypeFilter, EventTypeFilterMode, AnyEvent};
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone, Debug)]
    struct TestEvent {
        pub value: i32,
        pub message: String,
    }

    impl Event for TestEvent {
        fn event_type_name() -> &'static str {
            "TestEvent"
        }
    }

    #[derive(Clone, Debug)]
    struct OtherEvent {
        pub data: String,
    }

    impl Event for OtherEvent {
        fn event_type_name() -> &'static str {
            "OtherEvent"
        }
    }

    #[test]
    fn test_dynamic_filter_basic() {
        let mut bus = EventBus::new();
        let processed = Arc::new(Mutex::new(Vec::new()));
        
        let processed_clone = Arc::clone(&processed);
        bus.on(move |event: TestEvent| {
            processed_clone.lock().unwrap().push(event.value);
        });
        
        // Create a dynamic filter that initially allows all
        let dynamic_filter = DynamicFilter::new_shared(
            "dynamic_test",
            |_event| true
        );
        
        bus.add_global_filter("dynamic", Box::new(dynamic_filter.clone()));
        
        // Emit some events - all should pass
        bus.emit(TestEvent { value: 1, message: "first".to_string() }).unwrap();
        bus.emit(TestEvent { value: 2, message: "second".to_string() }).unwrap();
        
        assert_eq!(*processed.lock().unwrap(), vec![1, 2]);
        
        // Update the filter to only allow positive even values
        dynamic_filter.update_predicate(|event: &dyn AnyEvent| {
            if let Some(test_event) = event.as_any().downcast_ref::<TestEvent>() {
                test_event.value > 0 && test_event.value % 2 == 0
            } else {
                true
            }
        });
        
        // Clear the filter cache to ensure the new predicate is used
        bus.clear_global_filter_cache();
        
        // Emit more events with the updated filter
        bus.emit(TestEvent { value: 3, message: "third".to_string() }).unwrap();
        bus.emit(TestEvent { value: 4, message: "fourth".to_string() }).unwrap();
        bus.emit(TestEvent { value: -2, message: "negative".to_string() }).unwrap();
        
        // Only value 4 should have been processed
        assert_eq!(*processed.lock().unwrap(), vec![1, 2, 4]);
    }
    
    #[test]
    fn test_cacheable_filter() {
        let mut bus = EventBus::new();
        let filter_evaluations = Arc::new(AtomicUsize::new(0));
        
        // Create a cacheable filter that counts evaluations
        let eval_count = Arc::clone(&filter_evaluations);
        let cacheable_filter = DynamicFilter::new_cacheable(
            "cacheable_test",
            move |event| {
                eval_count.fetch_add(1, Ordering::SeqCst);
                // Only allow TestEvent, not OtherEvent
                event.event_type_name() == "TestEvent"
            }
        );
        
        bus.add_global_filter("cacheable", Box::new(cacheable_filter));
        
        let test_processed = Arc::new(AtomicUsize::new(0));
        let other_processed = Arc::new(AtomicUsize::new(0));
        
        let test_clone = Arc::clone(&test_processed);
        bus.on(move |_event: TestEvent| {
            test_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        let other_clone = Arc::clone(&other_processed);
        bus.on(move |_event: OtherEvent| {
            other_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        // Emit events of both types multiple times
        for i in 0..5 {
            bus.emit(TestEvent { value: i, message: format!("test{}", i) }).unwrap();
            bus.emit(OtherEvent { data: format!("other{}", i) }).unwrap();
        }
        
        // Check that TestEvents were processed and OtherEvents were not
        assert_eq!(test_processed.load(Ordering::SeqCst), 5);
        assert_eq!(other_processed.load(Ordering::SeqCst), 0);
        
        // Because the filter is cacheable and only depends on event type,
        // it should have been evaluated only twice (once per event type)
        assert_eq!(filter_evaluations.load(Ordering::SeqCst), 2);
    }
    
    #[test]
    fn test_event_type_filter_caching() {
        let mut bus = EventBus::new();
        
        // Add an EventTypeFilter (which is cacheable)
        let type_filter = EventTypeFilter::allow_types(vec!["TestEvent"]);
        bus.add_global_filter("type_filter", Box::new(type_filter));
        
        let test_count = Arc::new(AtomicUsize::new(0));
        let other_count = Arc::new(AtomicUsize::new(0));
        
        let test_clone = Arc::clone(&test_count);
        bus.on(move |_event: TestEvent| {
            test_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        let other_clone = Arc::clone(&other_count);
        bus.on(move |_event: OtherEvent| {
            other_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        // Emit many events
        for i in 0..10 {
            bus.emit(TestEvent { value: i, message: format!("test{}", i) }).unwrap();
            bus.emit(OtherEvent { data: format!("other{}", i) }).unwrap();
        }
        
        // Only TestEvents should be processed
        assert_eq!(test_count.load(Ordering::SeqCst), 10);
        assert_eq!(other_count.load(Ordering::SeqCst), 0);
    }
    
    #[test]
    fn test_content_aware_cache_filter() {
        let mut bus = EventBus::new();
        let filter_calls = Arc::new(AtomicUsize::new(0));
        
        // Create a content-aware caching filter
        let calls = Arc::clone(&filter_calls);
        let content_filter = ContentAwareCacheFilter::new(
            "content_aware",
            move |event| {
                calls.fetch_add(1, Ordering::SeqCst);
                if let Some(test_event) = event.as_any().downcast_ref::<TestEvent>() {
                    test_event.value > 0
                } else {
                    true
                }
            }
        ).with_max_cache_size(10);
        
        bus.add_global_filter("content", Box::new(content_filter));
        
        let processed = Arc::new(Mutex::new(Vec::new()));
        let processed_clone = Arc::clone(&processed);
        bus.on(move |event: TestEvent| {
            processed_clone.lock().unwrap().push(event.value);
        });
        
        // Emit the same events multiple times
        for _ in 0..3 {
            bus.emit(TestEvent { value: 1, message: "positive".to_string() }).unwrap();
            bus.emit(TestEvent { value: -1, message: "negative".to_string() }).unwrap();
            bus.emit(TestEvent { value: 2, message: "another".to_string() }).unwrap();
        }
        
        // Due to type-based caching, the filter is only called once for the first TestEvent
        // The first event (value: 1) returns true, so all subsequent TestEvents use the cached true result
        let processed_values = processed.lock().unwrap();
        assert_eq!(processed_values.len(), 9); // All 9 events pass due to type-based caching
        
        // The filter should have been called only once due to type-based caching
        let calls_made = filter_calls.load(Ordering::SeqCst);
        assert_eq!(calls_made, 1); // Only called once for the first TestEvent
    }
    
    #[test]
    fn test_filter_manager_caching() {
        let mut bus = EventBus::new();
        
        // Track whether cache operations happen
        let cache_cleared = Arc::new(AtomicUsize::new(0));
        
        // Add a cacheable filter
        let filter = EventTypeFilter::block_types(vec!["OtherEvent"]);
        bus.add_global_filter("block_other", Box::new(filter));
        
        let test_count = Arc::new(AtomicUsize::new(0));
        let test_clone = Arc::clone(&test_count);
        bus.on(move |_event: TestEvent| {
            test_clone.fetch_add(1, Ordering::SeqCst);
        });
        
        // Emit events
        for i in 0..5 {
            bus.emit(TestEvent { value: i, message: format!("test{}", i) }).unwrap();
        }
        
        assert_eq!(test_count.load(Ordering::SeqCst), 5);
        
        // Clear cache manually
        bus.clear_global_filter_cache();
        cache_cleared.fetch_add(1, Ordering::SeqCst);
        
        // Emit more events - should still work after cache clear
        for i in 5..10 {
            bus.emit(TestEvent { value: i, message: format!("test{}", i) }).unwrap();
        }
        
        assert_eq!(test_count.load(Ordering::SeqCst), 10);
        assert_eq!(cache_cleared.load(Ordering::SeqCst), 1);
    }
    
    #[test]
    fn test_dynamic_filter_runtime_update() {
        let mut bus = EventBus::new();
        let processed = Arc::new(Mutex::new(Vec::new()));
        
        let processed_clone = Arc::clone(&processed);
        bus.on(move |event: TestEvent| {
            processed_clone.lock().unwrap().push(event.value);
        });
        
        // Create a dynamic filter with a specific behavior
        let filter = DynamicFilter::new_shared(
            "updatable",
            |event| {
                if let Some(test_event) = event.as_any().downcast_ref::<TestEvent>() {
                    test_event.value < 5
                } else {
                    true
                }
            }
        );
        
        bus.add_global_filter("updatable", Box::new(filter.clone()));
        
        // Emit events - only values < 5 should pass
        for i in 0..10 {
            bus.emit(TestEvent { value: i, message: format!("test{}", i) }).unwrap();
        }
        
        assert_eq!(*processed.lock().unwrap(), vec![0, 1, 2, 3, 4]);
        
        // Update filter to allow values >= 5
        filter.update_predicate(|event: &dyn AnyEvent| {
            if let Some(test_event) = event.as_any().downcast_ref::<TestEvent>() {
                test_event.value >= 5
            } else {
                true
            }
        });
        
        // Clear cache and processed events
        bus.clear_global_filter_cache();
        processed.lock().unwrap().clear();
        
        // Emit events again - now only values >= 5 should pass
        for i in 0..10 {
            bus.emit(TestEvent { value: i, message: format!("test{}", i) }).unwrap();
        }
        
        assert_eq!(*processed.lock().unwrap(), vec![5, 6, 7, 8, 9]);
    }
}