//! Core event trait and related functionality.
//!
//! This module defines the fundamental `Event` trait that all events in EventRS must implement.
//! The trait provides type-safe event handling with minimal runtime overhead.

use crate::metadata::EventMetadata;
use crate::error::EventValidationError;
use std::any::TypeId;

/// Core trait that all events must implement.
/// 
/// This trait provides the foundation for type-safe event handling in EventRS.
/// Events must be cloneable, thread-safe, and have a static lifetime to ensure
/// they can be safely shared across event handlers.
/// 
/// Most events should use `#[derive(Event)]` for automatic implementation.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::Event;
/// 
/// #[derive(Event, Clone, Debug)]
/// struct UserLoggedIn {
///     user_id: u64,
///     timestamp: std::time::SystemTime,
/// }
/// ```
/// 
/// # Manual Implementation
/// 
/// For advanced use cases, you can manually implement the trait:
/// 
/// ```rust
/// use eventrs::{Event, EventMetadata};
/// 
/// #[derive(Clone, Debug)]
/// struct CustomEvent {
///     data: Vec<u8>,
/// }
/// 
/// impl Event for CustomEvent {
///     fn event_type_name() -> &'static str {
///         "CustomEvent"
///     }
///     
///     fn metadata(&self) -> EventMetadata {
///         EventMetadata::new()
///             .with_timestamp(std::time::SystemTime::now())
///             .with_source("custom")
///     }
/// }
/// ```
pub trait Event: Clone + Send + Sync + 'static {
    /// Returns the name of the event type.
    /// 
    /// This is used for debugging, logging, and type identification.
    /// The default implementation uses the full type name including module path.
    fn event_type_name() -> &'static str
    where
        Self: Sized,
    {
        std::any::type_name::<Self>()
    }
    
    /// Returns a unique identifier for this event type.
    /// 
    /// This is used internally for efficient type-based routing.
    /// The default implementation uses the TypeId.
    fn event_type_id() -> TypeId
    where
        Self: Sized,
    {
        TypeId::of::<Self>()
    }
    
    /// Returns metadata associated with this event instance.
    /// 
    /// Override this method to provide custom metadata such as
    /// timestamps, priorities, source information, or custom fields.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{Event, EventMetadata, Priority};
    /// 
    /// #[derive(Event, Clone)]
    /// struct ImportantEvent {
    ///     message: String,
    /// }
    /// 
    /// impl ImportantEvent {
    ///     fn metadata(&self) -> EventMetadata {
    ///         EventMetadata::new()
    ///             .with_priority(Priority::High)
    ///             .with_source("critical_system")
    ///             .with_category("alert")
    ///     }
    /// }
    /// ```
    fn metadata(&self) -> EventMetadata {
        EventMetadata::default()
    }
    
    /// Returns the estimated size of this event in bytes.
    /// 
    /// Used for memory management, performance optimization, and metrics collection.
    /// The default implementation uses `std::mem::size_of` which provides the
    /// stack size but may not account for heap-allocated data.
    /// 
    /// Override this method for events with significant heap allocations:
    /// 
    /// ```rust
    /// use eventrs::Event;
    /// 
    /// #[derive(Event, Clone)]
    /// struct LargeEvent {
    ///     data: Vec<u8>,
    /// }
    /// 
    /// impl LargeEvent {
    ///     fn size_hint(&self) -> usize {
    ///         std::mem::size_of::<Self>() + self.data.len()
    ///     }
    /// }
    /// ```
    fn size_hint(&self) -> usize {
        std::mem::size_of::<Self>()
    }
    
    /// Validates the event data.
    /// 
    /// Override this method to implement custom validation logic.
    /// Events that fail validation may be rejected by the event bus
    /// depending on the configuration.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{Event, EventValidationError};
    /// 
    /// #[derive(Event, Clone)]
    /// struct EmailEvent {
    ///     to: String,
    ///     subject: String,
    /// }
    /// 
    /// impl EmailEvent {
    ///     fn validate(&self) -> Result<(), EventValidationError> {
    ///         if self.to.is_empty() {
    ///             return Err(EventValidationError::MissingField {
    ///                 field: "to".to_string(),
    ///             });
    ///         }
    ///         
    ///         if !self.to.contains('@') {
    ///             return Err(EventValidationError::InvalidValue {
    ///                 field: "to".to_string(),
    ///                 value: self.to.clone(),
    ///             });
    ///         }
    ///         
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn validate(&self) -> Result<(), EventValidationError> {
        Ok(())
    }
    
    /// Returns whether this event is considered expensive to clone.
    /// 
    /// Used for optimization decisions. Events marked as expensive
    /// may receive special handling to minimize cloning overhead.
    /// 
    /// The default implementation considers events with size > 1KB as expensive.
    fn is_expensive_to_clone(&self) -> bool {
        self.size_hint() > 1024
    }
    
    /// Returns a short description of this event for logging.
    /// 
    /// Override this method to provide meaningful log messages
    /// without exposing sensitive data.
    fn log_description(&self) -> String {
        format!("{} ({}B)", Self::event_type_name(), self.size_hint())
    }
}

/// Type-erased event wrapper for internal use.
/// 
/// This allows the event bus to store events of different types
/// in the same collection while maintaining type safety through
/// the type system.
#[derive(Debug)]
pub(crate) struct EventWrapper {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) data: Box<dyn std::any::Any + Send + Sync>,
    pub(crate) metadata: EventMetadata,
    pub(crate) size: usize,
}

impl EventWrapper {
    /// Creates a new event wrapper from a concrete event.
    pub(crate) fn new<E: Event>(event: E) -> Self {
        let metadata = event.metadata();
        let size = event.size_hint();
        
        Self {
            type_id: E::event_type_id(),
            type_name: E::event_type_name(),
            data: Box::new(event),
            metadata,
            size,
        }
    }
    
    /// Attempts to downcast the wrapped event to the specified type.
    pub(crate) fn downcast<E: Event>(&self) -> Option<&E> {
        if self.type_id == E::event_type_id() {
            self.data.downcast_ref::<E>()
        } else {
            None
        }
    }
    
    /// Returns whether this wrapper contains an event of the specified type.
    pub(crate) fn is_type<E: Event>(&self) -> bool {
        self.type_id == E::event_type_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::Priority;

    #[derive(Clone, Debug)]
    struct TestEvent {
        value: u32,
        data: Vec<u8>,
    }

    impl Event for TestEvent {
        fn event_type_name() -> &'static str {
            "TestEvent"
        }
        
        fn metadata(&self) -> EventMetadata {
            EventMetadata::new()
                .with_priority(Priority::High)
                .with_source("test")
        }
        
        fn size_hint(&self) -> usize {
            std::mem::size_of::<Self>() + self.data.len()
        }
        
        fn validate(&self) -> Result<(), EventValidationError> {
            if self.value == 0 {
                return Err(EventValidationError::InvalidValue {
                    field: "value".to_string(),
                    value: "0".to_string(),
                });
            }
            Ok(())
        }
    }

    #[test]
    fn test_event_type_name() {
        assert_eq!(TestEvent::event_type_name(), "TestEvent");
    }

    #[test]
    fn test_event_type_id() {
        let id1 = TestEvent::event_type_id();
        let id2 = TestEvent::event_type_id();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_event_metadata() {
        let event = TestEvent {
            value: 42,
            data: vec![1, 2, 3],
        };
        
        let metadata = event.metadata();
        assert_eq!(metadata.priority(), Priority::High);
        assert_eq!(metadata.source(), Some("test"));
    }

    #[test]
    fn test_event_size_hint() {
        let event = TestEvent {
            value: 42,
            data: vec![1, 2, 3, 4, 5],
        };
        
        let expected_size = std::mem::size_of::<TestEvent>() + 5;
        assert_eq!(event.size_hint(), expected_size);
    }

    #[test]
    fn test_event_validation() {
        let valid_event = TestEvent {
            value: 42,
            data: vec![],
        };
        assert!(valid_event.validate().is_ok());

        let invalid_event = TestEvent {
            value: 0,
            data: vec![],
        };
        assert!(invalid_event.validate().is_err());
    }

    #[test]
    fn test_expensive_clone_detection() {
        let small_event = TestEvent {
            value: 42,
            data: vec![1; 100],
        };
        assert!(!small_event.is_expensive_to_clone());

        let large_event = TestEvent {
            value: 42,
            data: vec![1; 2000],
        };
        assert!(large_event.is_expensive_to_clone());
    }

    #[test]
    fn test_event_wrapper() {
        let event = TestEvent {
            value: 42,
            data: vec![1, 2, 3],
        };
        
        let wrapper = EventWrapper::new(event.clone());
        
        assert_eq!(wrapper.type_id, TestEvent::event_type_id());
        assert_eq!(wrapper.type_name, "TestEvent");
        assert_eq!(wrapper.size, event.size_hint());
        
        let downcast = wrapper.downcast::<TestEvent>().unwrap();
        assert_eq!(downcast.value, 42);
        assert_eq!(downcast.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_log_description() {
        let event = TestEvent {
            value: 42,
            data: vec![1, 2, 3],
        };
        
        let description = event.log_description();
        assert!(description.contains("TestEvent"));
        assert!(description.contains("B)"));
    }
}