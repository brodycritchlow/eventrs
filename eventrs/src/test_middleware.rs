//! Test file for middleware functionality

#[cfg(test)]
mod tests {
    use crate::{Event, Middleware, MiddlewareChain, MiddlewareContext, MiddlewareResult};
    use crate::{LoggingMiddleware, MetricsMiddleware, ValidationMiddleware};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug)]
    struct TestEvent {
        pub value: i32,
        pub name: String,
    }

    impl Event for TestEvent {
        fn event_type_name() -> &'static str {
            "TestEvent"
        }
    }

    struct CountingMiddleware {
        counter: Arc<Mutex<usize>>,
    }

    impl CountingMiddleware {
        fn new(counter: Arc<Mutex<usize>>) -> Self {
            Self { counter }
        }
    }

    impl<E: Event> Middleware<E> for CountingMiddleware {
        fn handle(&self, event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
            let mut count = self.counter.lock().unwrap();
            *count += 1;
            context.next(event)
        }

        fn middleware_name(&self) -> &'static str {
            "CountingMiddleware"
        }
    }

    struct ShortCircuitMiddleware;

    impl<E: Event> Middleware<E> for ShortCircuitMiddleware {
        fn handle(&self, _event: &E, context: &mut MiddlewareContext<E>) -> MiddlewareResult {
            context.short_circuit();
            Ok(())
        }

        fn middleware_name(&self) -> &'static str {
            "ShortCircuitMiddleware"
        }
    }

    #[test]
    fn test_middleware_chain_creation() {
        let chain: MiddlewareChain<TestEvent> = MiddlewareChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_middleware_chain_add() {
        let mut chain: MiddlewareChain<TestEvent> = MiddlewareChain::new();
        let result = chain.add(LoggingMiddleware::default());
        assert!(result.is_ok());
        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_built_in_middleware_creation() {
        let _logging = LoggingMiddleware::new("Test".to_string());
        let _validation = ValidationMiddleware::new();
        let _metrics = MetricsMiddleware::new("TestMetrics".to_string());

        // Test default implementations
        let _logging_default = LoggingMiddleware::default();
        let _validation_default = ValidationMiddleware::default();
        let _metrics_default = MetricsMiddleware::default();
    }

    #[test]
    fn test_middleware_context_metadata() {
        let mut context: MiddlewareContext<TestEvent> = MiddlewareContext::new(1);

        // Test metadata storage and retrieval
        context.set_metadata("test_key".to_string(), 42i32);
        let value = context.get_metadata::<i32>("test_key");
        assert_eq!(value, Some(&42));

        // Test wrong type
        let wrong_type = context.get_metadata::<String>("test_key");
        assert_eq!(wrong_type, None);

        // Test missing key
        let missing = context.get_metadata::<i32>("missing_key");
        assert_eq!(missing, None);
    }

    #[test]
    fn test_middleware_context_short_circuit() {
        let mut context: MiddlewareContext<TestEvent> = MiddlewareContext::new(1);

        assert!(!context.is_short_circuited());
        context.short_circuit();
        assert!(context.is_short_circuited());
    }

    #[test]
    fn test_middleware_context_chain_info() {
        let context: MiddlewareContext<TestEvent> = MiddlewareContext::new(5);

        assert_eq!(context.current_position(), 0);
        assert_eq!(context.chain_length(), 5);
        assert!(context.execution_metrics().is_empty());
    }

    #[test]
    fn test_middleware_next_when_short_circuited() {
        let mut context: MiddlewareContext<TestEvent> = MiddlewareContext::new(1);
        let event = TestEvent {
            value: 42,
            name: "test".to_string(),
        };

        context.short_circuit();
        let result = context.next(&event);
        assert!(result.is_ok()); // Should return Ok but not continue processing
    }
}
