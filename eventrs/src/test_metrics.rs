//! Test file for metrics functionality

#[cfg(test)]
mod tests {
    use crate::Event;
    use crate::metrics::{EventBusMetrics, EmissionResult, HandlerResult, EmissionStats, SystemMetrics};
    use std::time::{Duration, SystemTime};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct TestEvent {
        pub value: i32,
    }

    impl Event for TestEvent {
        fn event_type_name() -> &'static str {
            "TestEvent"
        }
    }

    #[test]
    fn test_event_bus_metrics_creation() {
        let metrics_enabled = EventBusMetrics::new(true);
        assert!(metrics_enabled.is_enabled());
        
        let metrics_disabled = EventBusMetrics::new(false);
        assert!(!metrics_disabled.is_enabled());
        
        let metrics_default_enabled = EventBusMetrics::enabled();
        assert!(metrics_default_enabled.is_enabled());
        
        let metrics_default_disabled = EventBusMetrics::disabled();
        assert!(!metrics_default_disabled.is_enabled());
    }

    #[test]
    fn test_metrics_enable_disable() {
        let mut metrics = EventBusMetrics::new(false);
        assert!(!metrics.is_enabled());
        
        metrics.enable();
        assert!(metrics.is_enabled());
        
        metrics.disable();
        assert!(!metrics.is_enabled());
    }

    #[test]
    fn test_emission_token_and_recording() {
        let metrics = EventBusMetrics::enabled();
        let event_type = TypeId::of::<TestEvent>();
        
        // Test emission token creation
        let token = metrics.start_emission(event_type);
        
        // Create a sample emission result
        let handler_results = vec![
            HandlerResult::success(
                "handler1".to_string(),
                Duration::from_millis(10),
                "Normal".to_string(),
            ),
            HandlerResult::success(
                "handler2".to_string(),
                Duration::from_millis(15),
                "High".to_string(),
            ),
        ];
        
        let emission_result = EmissionResult::success(
            Duration::from_millis(30),
            2,
            handler_results,
            "TestEvent".to_string(),
            true,
        );
        
        // Record the emission
        metrics.record_emission_end(token, emission_result);
        
        // Verify metrics were recorded
        let emission_stats = metrics.get_emission_stats(event_type);
        assert!(emission_stats.is_some());
        
        let stats = emission_stats.unwrap();
        assert_eq!(stats.total_emissions, 1);
        assert_eq!(stats.successful_emissions, 1);
        assert_eq!(stats.failed_emissions, 0);
        assert_eq!(stats.handler_count, 2);
        assert!(stats.avg_emission_time_ms > 0.0);
    }

    #[test]
    fn test_handler_registration_tracking() {
        let metrics = EventBusMetrics::enabled();
        
        // Record handler registrations
        metrics.record_handler_registration("handler1".to_string());
        metrics.record_handler_registration("handler2".to_string());
        
        let system_metrics = metrics.get_system_metrics();
        assert_eq!(system_metrics.total_handlers_registered, 2);
        
        // Record handler unregistration
        metrics.record_handler_unregistration("handler1".to_string());
        
        let system_metrics = metrics.get_system_metrics();
        assert_eq!(system_metrics.total_handlers_registered, 1);
    }

    #[test]
    fn test_disabled_metrics_no_recording() {
        let metrics = EventBusMetrics::disabled();
        let event_type = TypeId::of::<TestEvent>();
        
        // Start emission with disabled metrics
        let token = metrics.start_emission(event_type);
        
        let emission_result = EmissionResult::success(
            Duration::from_millis(30),
            1,
            vec![HandlerResult::success(
                "handler1".to_string(),
                Duration::from_millis(10),
                "Normal".to_string(),
            )],
            "TestEvent".to_string(),
            false,
        );
        
        // Record emission
        metrics.record_emission_end(token, emission_result);
        
        // Verify no stats were recorded
        let emission_stats = metrics.get_emission_stats(event_type);
        assert!(emission_stats.is_none());
        
        let all_stats = metrics.get_all_emission_stats();
        assert!(all_stats.is_empty());
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = EventBusMetrics::enabled();
        let event_type = TypeId::of::<TestEvent>();
        
        // Record some data
        metrics.record_handler_registration("handler1".to_string());
        let token = metrics.start_emission(event_type);
        let emission_result = EmissionResult::success(
            Duration::from_millis(10),
            1,
            vec![HandlerResult::success(
                "handler1".to_string(),
                Duration::from_millis(10),
                "Normal".to_string(),
            )],
            "TestEvent".to_string(),
            false,
        );
        metrics.record_emission_end(token, emission_result);
        
        // Verify data exists
        assert!(metrics.get_emission_stats(event_type).is_some());
        assert_eq!(metrics.get_system_metrics().total_handlers_registered, 1);
        
        // Reset metrics
        metrics.reset();
        
        // Verify data is cleared
        assert!(metrics.get_emission_stats(event_type).is_none());
        assert_eq!(metrics.get_system_metrics().total_handlers_registered, 0);
        assert!(metrics.get_system_metrics().last_reset.is_some());
    }

    #[test]
    fn test_emission_result_creation() {
        // Test successful emission result
        let handler_results = vec![
            HandlerResult::success(
                "handler1".to_string(),
                Duration::from_millis(10),
                "Normal".to_string(),
            ),
            HandlerResult::failure(
                "handler2".to_string(),
                Duration::from_millis(5),
                "High".to_string(),
                "Handler failed".to_string(),
            ),
        ];
        
        let success_result = EmissionResult::success(
            Duration::from_millis(20),
            2,
            handler_results.clone(),
            "TestEvent".to_string(),
            true,
        );
        
        assert!(success_result.success);
        assert_eq!(success_result.handlers_executed, 2);
        assert_eq!(success_result.handlers_succeeded, 1);
        assert_eq!(success_result.handlers_failed, 1);
        assert_eq!(success_result.success_rate(), 0.5);
        assert!(success_result.avg_handler_execution_time() > Duration::ZERO);
        
        // Test failed emission result
        let failure_result = EmissionResult::failure(
            Duration::from_millis(5),
            "Event validation failed".to_string(),
            "TestEvent".to_string(),
        );
        
        assert!(!failure_result.success);
        assert_eq!(failure_result.handlers_executed, 0);
        assert_eq!(failure_result.errors.len(), 1);
        assert_eq!(failure_result.success_rate(), 0.0);
    }

    #[test]
    fn test_handler_result_creation() {
        // Test successful handler result
        let success_result = HandlerResult::success(
            "handler1".to_string(),
            Duration::from_millis(10),
            "Normal".to_string(),
        );
        
        assert!(success_result.success);
        assert_eq!(success_result.handler_id, "handler1");
        assert_eq!(success_result.priority, "Normal");
        assert!(success_result.error.is_none());
        assert!(!success_result.skipped_by_filter);
        
        // Test failed handler result
        let failure_result = HandlerResult::failure(
            "handler2".to_string(),
            Duration::from_millis(5),
            "High".to_string(),
            "Handler panic".to_string(),
        );
        
        assert!(!failure_result.success);
        assert_eq!(failure_result.handler_id, "handler2");
        assert!(failure_result.error.is_some());
        assert_eq!(failure_result.error.unwrap(), "Handler panic");
        
        // Test skipped handler result
        let skipped_result = HandlerResult::skipped(
            "handler3".to_string(),
            "Low".to_string(),
        );
        
        assert!(skipped_result.success);
        assert!(skipped_result.skipped_by_filter);
        assert_eq!(skipped_result.execution_time, Duration::ZERO);
    }

    #[test]
    fn test_metrics_report_generation() {
        let metrics = EventBusMetrics::enabled();
        let event_type = TypeId::of::<TestEvent>();
        
        // Add some data
        metrics.record_handler_registration("handler1".to_string());
        let token = metrics.start_emission(event_type);
        let emission_result = EmissionResult::success(
            Duration::from_millis(15),
            1,
            vec![HandlerResult::success(
                "handler1".to_string(),
                Duration::from_millis(15),
                "Normal".to_string(),
            )],
            "TestEvent".to_string(),
            false,
        );
        metrics.record_emission_end(token, emission_result);
        
        // Generate report
        let report = metrics.generate_report();
        
        assert_eq!(report.system_metrics.total_events_processed, 1);
        assert_eq!(report.system_metrics.total_handlers_registered, 1);
        assert_eq!(report.emission_stats.len(), 1);
        assert_eq!(report.handler_metrics.len(), 1);
        assert!(report.report_timestamp <= SystemTime::now());
    }

    #[test]
    fn test_handler_metrics_aggregation() {
        let metrics = EventBusMetrics::enabled();
        let event_type = TypeId::of::<TestEvent>();
        
        // Record multiple executions of the same handler
        for i in 1..=3 {
            let token = metrics.start_emission(event_type);
            let execution_time = Duration::from_millis(i * 10);
            let emission_result = EmissionResult::success(
                execution_time,
                1,
                vec![HandlerResult::success(
                    "handler1".to_string(),
                    execution_time,
                    "Normal".to_string(),
                )],
                "TestEvent".to_string(),
                false,
            );
            metrics.record_emission_end(token, emission_result);
        }
        
        // Get handler metrics
        let handler_metrics = metrics.get_handler_metrics("handler1");
        assert!(handler_metrics.is_some());
        
        let metrics_data = handler_metrics.unwrap();
        assert_eq!(metrics_data.invocation_count, 3);
        assert_eq!(metrics_data.successful_executions, 3);
        assert_eq!(metrics_data.failed_executions, 0);
        assert_eq!(metrics_data.min_execution_time, Duration::from_millis(10));
        assert_eq!(metrics_data.max_execution_time, Duration::from_millis(30));
        assert!(metrics_data.avg_execution_time >= Duration::from_millis(10));
        assert!(metrics_data.avg_execution_time <= Duration::from_millis(30));
    }
}