//! Tests for global filtering functionality (Phase 3.1)

#[cfg(test)]
mod tests {
    use crate::filter::{
        AllowAllAnyFilter, EventTypeFilter, EventTypeFilterMode, PredicateAnyFilter,
        RejectAllAnyFilter,
    };
    use crate::prelude::*;
    use std::sync::{Arc, Mutex};

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
    struct AdminEvent {
        pub user_id: u64,
        pub action: String,
    }

    impl Event for AdminEvent {
        fn event_type_name() -> &'static str {
            "AdminEvent"
        }
    }

    #[test]
    fn test_global_filter_basic() {
        let mut bus = EventBus::new();
        let processed_events = Arc::new(Mutex::new(Vec::new()));

        // Register a handler
        let events_clone = Arc::clone(&processed_events);
        bus.on(move |event: TestEvent| {
            events_clone.lock().unwrap().push(event.value);
        });

        // Add a global filter that only allows positive values
        let positive_filter = PredicateAnyFilter::new("positive_values", |event| {
            if let Some(test_event) = event.as_any().downcast_ref::<TestEvent>() {
                test_event.value > 0
            } else {
                true // Allow non-TestEvent events
            }
        });
        bus.add_global_filter("positive_values", Box::new(positive_filter));

        // Emit events
        bus.emit(TestEvent {
            value: 5,
            message: "positive".to_string(),
        })
        .unwrap();
        bus.emit(TestEvent {
            value: -3,
            message: "negative".to_string(),
        })
        .unwrap();
        bus.emit(TestEvent {
            value: 10,
            message: "positive".to_string(),
        })
        .unwrap();

        // Check that only positive events were processed
        let processed = processed_events.lock().unwrap();
        assert_eq!(*processed, vec![5, 10]);
    }

    #[test]
    fn test_global_filter_with_priority() {
        let mut bus = EventBus::new();
        let processed_events = Arc::new(Mutex::new(Vec::new()));

        let events_clone = Arc::clone(&processed_events);
        bus.on(move |event: TestEvent| {
            events_clone.lock().unwrap().push(event.value);
        });

        // Add filters with different priorities
        let low_priority_filter = PredicateAnyFilter::new("allow_all", |_event| true);
        bus.add_global_filter_with_priority("allow_all", Box::new(low_priority_filter), 1);

        let high_priority_filter = PredicateAnyFilter::new("block_negative", |event| {
            if let Some(test_event) = event.as_any().downcast_ref::<TestEvent>() {
                test_event.value >= 0
            } else {
                true
            }
        });
        bus.add_global_filter_with_priority("block_negative", Box::new(high_priority_filter), 10);

        // Emit events
        bus.emit(TestEvent {
            value: 5,
            message: "positive".to_string(),
        })
        .unwrap();
        bus.emit(TestEvent {
            value: -3,
            message: "negative".to_string(),
        })
        .unwrap();

        // Negative event should be blocked by high-priority filter
        let processed = processed_events.lock().unwrap();
        assert_eq!(*processed, vec![5]);
    }

    #[test]
    fn test_event_type_filter() {
        let mut bus = EventBus::new();
        let test_events = Arc::new(Mutex::new(Vec::new()));
        let admin_events = Arc::new(Mutex::new(Vec::new()));

        // Register handlers for both event types
        let test_clone = Arc::clone(&test_events);
        bus.on(move |event: TestEvent| {
            test_clone.lock().unwrap().push(event.value);
        });

        let admin_clone = Arc::clone(&admin_events);
        bus.on(move |event: AdminEvent| {
            admin_clone.lock().unwrap().push(event.user_id);
        });

        // Add a filter that only allows AdminEvent
        let admin_only_filter = EventTypeFilter::allow_types(vec!["AdminEvent"]);
        bus.add_global_filter("admin_only", Box::new(admin_only_filter));

        // Emit events of both types
        bus.emit(TestEvent {
            value: 42,
            message: "test".to_string(),
        })
        .unwrap();
        bus.emit(AdminEvent {
            user_id: 123,
            action: "login".to_string(),
        })
        .unwrap();
        bus.emit(TestEvent {
            value: 99,
            message: "test2".to_string(),
        })
        .unwrap();

        // Only AdminEvent should be processed
        assert!(test_events.lock().unwrap().is_empty());
        assert_eq!(*admin_events.lock().unwrap(), vec![123]);
    }

    #[test]
    fn test_allow_all_any_filter() {
        let mut bus = EventBus::new();
        let processed_events = Arc::new(Mutex::new(Vec::new()));

        let events_clone = Arc::clone(&processed_events);
        bus.on(move |event: TestEvent| {
            events_clone.lock().unwrap().push(event.value);
        });

        // Add allow-all filter
        bus.add_global_filter("allow_all", Box::new(AllowAllAnyFilter));

        // Emit events
        bus.emit(TestEvent {
            value: 1,
            message: "test".to_string(),
        })
        .unwrap();
        bus.emit(TestEvent {
            value: 2,
            message: "test".to_string(),
        })
        .unwrap();

        // All events should be processed
        let processed = processed_events.lock().unwrap();
        assert_eq!(*processed, vec![1, 2]);
    }

    #[test]
    fn test_reject_all_any_filter() {
        let mut bus = EventBus::new();
        let processed_events = Arc::new(Mutex::new(Vec::new()));

        let events_clone = Arc::clone(&processed_events);
        bus.on(move |event: TestEvent| {
            events_clone.lock().unwrap().push(event.value);
        });

        // Add reject-all filter
        bus.add_global_filter("reject_all", Box::new(RejectAllAnyFilter));

        // Emit events
        bus.emit(TestEvent {
            value: 1,
            message: "test".to_string(),
        })
        .unwrap();
        bus.emit(TestEvent {
            value: 2,
            message: "test".to_string(),
        })
        .unwrap();

        // No events should be processed
        let processed = processed_events.lock().unwrap();
        assert!(processed.is_empty());
    }

    #[test]
    fn test_global_filter_management() {
        let bus = EventBus::new();

        // Add some filters
        bus.add_global_filter("filter1", Box::new(AllowAllAnyFilter));
        bus.add_global_filter("filter2", Box::new(RejectAllAnyFilter));

        // Check filter count and list
        assert_eq!(bus.global_filter_count(), 2);
        let filter_ids = bus.list_global_filters();
        assert!(filter_ids.contains(&"filter1".to_string()));
        assert!(filter_ids.contains(&"filter2".to_string()));

        // Test filter enabled/disabled state
        assert!(bus.is_global_filter_enabled("filter1"));
        assert!(bus.is_global_filter_enabled("filter2"));

        // Disable a filter
        assert!(bus.set_global_filter_enabled("filter1", false));
        assert!(!bus.is_global_filter_enabled("filter1"));

        // Enable it again
        assert!(bus.set_global_filter_enabled("filter1", true));
        assert!(bus.is_global_filter_enabled("filter1"));

        // Remove a filter
        assert!(bus.remove_global_filter("filter1"));
        assert_eq!(bus.global_filter_count(), 1);
        assert!(!bus.is_global_filter_enabled("filter1"));

        // Try to remove non-existent filter
        assert!(!bus.remove_global_filter("nonexistent"));
    }

    #[test]
    fn test_global_filtering_enabled_disabled() {
        let mut bus = EventBus::new();
        let processed_events = Arc::new(Mutex::new(Vec::new()));

        let events_clone = Arc::clone(&processed_events);
        bus.on(move |event: TestEvent| {
            events_clone.lock().unwrap().push(event.value);
        });

        // Add a reject-all filter
        bus.add_global_filter("reject_all", Box::new(RejectAllAnyFilter));

        // Emit event - should be rejected
        bus.emit(TestEvent {
            value: 1,
            message: "test".to_string(),
        })
        .unwrap();
        assert!(processed_events.lock().unwrap().is_empty());

        // Disable global filtering
        bus.set_global_filtering_enabled(false);
        assert!(!bus.is_global_filtering_enabled());

        // Emit event - should now pass through
        bus.emit(TestEvent {
            value: 2,
            message: "test".to_string(),
        })
        .unwrap();
        assert_eq!(*processed_events.lock().unwrap(), vec![2]);

        // Re-enable global filtering
        bus.set_global_filtering_enabled(true);
        assert!(bus.is_global_filtering_enabled());

        // Emit event - should be rejected again
        bus.emit(TestEvent {
            value: 3,
            message: "test".to_string(),
        })
        .unwrap();
        assert_eq!(*processed_events.lock().unwrap(), vec![2]); // Still only the one that passed through
    }

    #[test]
    fn test_filter_manager_access() {
        let bus = EventBus::new();

        // Test direct access to filter manager
        let filter_manager = bus.global_filter_manager();

        // Add filter through manager
        let test_filter = PredicateAnyFilter::new("test", |_| true);
        filter_manager.add_filter("direct_filter", Box::new(test_filter));

        // Verify it was added
        assert_eq!(bus.global_filter_count(), 1);
        assert!(bus.is_global_filter_enabled("direct_filter"));
    }

    #[test]
    fn test_filter_cache_operations() {
        let bus = EventBus::new();

        // Add a filter
        bus.add_global_filter("test_filter", Box::new(AllowAllAnyFilter));

        // Clear cache (should not cause any issues)
        bus.clear_global_filter_cache();

        // Filter should still work
        assert!(bus.is_global_filtering_enabled());
        assert_eq!(bus.global_filter_count(), 1);
    }
}
