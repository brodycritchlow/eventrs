//! Test file for EventBusBuilder functionality

#[cfg(test)]
mod tests {
    use crate::{ErrorHandling, EventBus, Priority};

    #[test]
    fn test_event_bus_builder() {
        let bus = EventBus::builder()
            .with_capacity(1000)
            .with_metrics(true)
            .with_error_handling(ErrorHandling::StopOnFirstError)
            .with_default_priority(Priority::High)
            .with_validation(false)
            .with_priority_ordering(true)
            .build();

        let config = bus.config();
        assert_eq!(config.initial_capacity, 1000);
        assert_eq!(config.enable_metrics, true);
        assert_eq!(config.error_handling, ErrorHandling::StopOnFirstError);
        assert_eq!(config.default_handler_priority, Priority::High);
        assert_eq!(config.validate_events, false);
        assert_eq!(config.use_priority_ordering, true);
    }

    #[test]
    fn test_event_bus_builder_defaults() {
        let bus = EventBus::builder().build();
        let config = bus.config();

        // Check default values
        assert_eq!(config.initial_capacity, 64);
        assert_eq!(config.enable_metrics, false);
        assert_eq!(config.error_handling, ErrorHandling::ContinueOnError);
        assert_eq!(config.default_handler_priority, Priority::Normal);
        assert_eq!(config.validate_events, true);
        assert_eq!(config.use_priority_ordering, true);
    }
}
