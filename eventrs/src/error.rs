//! Error types for EventRS.
//!
//! This module defines all error types that can occur during event processing,
//! handler execution, and bus operations.

use thiserror::Error;
use std::time::Duration;

/// Errors that can occur during event bus operations.
#[derive(Debug, Error)]
pub enum EventBusError {
    /// Handler execution failed.
    #[error("Handler execution failed: {0}")]
    HandlerError(#[from] HandlerError),
    
    /// Event validation failed.
    #[error("Event validation failed: {0}")]
    ValidationError(#[from] EventValidationError),
    
    /// Middleware processing failed.
    #[error("Middleware error: {0}")]
    MiddlewareError(#[from] MiddlewareError),
    
    /// Filter evaluation failed.
    #[error("Filter error: {0}")]
    FilterError(#[from] FilterError),
    
    /// The event bus is shutting down and cannot process new events.
    #[error("Bus is shutting down")]
    ShuttingDown,
    
    /// The event bus is not properly initialized.
    #[error("Bus is not initialized")]
    NotInitialized,
    
    /// Configuration error occurred.
    #[error("Configuration error: {message}")]
    ConfigurationError {
        /// The configuration error message.
        message: String,
    },
    
    /// A system resource has been exhausted.
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted {
        /// The name of the exhausted resource.
        resource: String,
    },
    
    /// The operation timed out.
    #[error("Operation timed out after {duration:?}")]
    Timeout {
        /// The timeout duration.
        duration: Duration,
    },
    
    /// A concurrency-related error occurred.
    #[error("Concurrency error: {message}")]
    ConcurrencyError {
        /// The concurrency error message.
        message: String,
    },
    
    /// An internal error occurred that shouldn't happen under normal circumstances.
    #[error("Internal error: {message}")]
    Internal {
        /// The internal error message.
        message: String,
    },
}

/// Errors that can occur during handler execution.
#[derive(Debug, Error)]
pub enum HandlerError {
    /// A handler panicked during execution.
    #[error("Handler panicked: {message}")]
    Panic {
        /// The panic message.
        message: String,
    },
    
    /// Handler execution exceeded the specified timeout.
    #[error("Handler timeout after {duration:?}")]
    Timeout {
        /// The timeout duration.
        duration: Duration,
    },
    
    /// Handler failed with a custom error.
    #[error("Handler failed with custom error: {0}")]
    Custom(Box<dyn std::error::Error + Send + Sync>),
    
    /// Handler registration failed.
    #[error("Handler registration failed: {reason}")]
    RegistrationFailed {
        /// The reason for registration failure.
        reason: String,
    },
    
    /// Handler was not found.
    #[error("Handler not found: {handler_id}")]
    NotFound {
        /// The handler ID that was not found.
        handler_id: String,
    },
    
    /// Handler is already registered.
    #[error("Handler already registered: {handler_id}")]
    AlreadyRegistered {
        /// The handler ID that is already registered.
        handler_id: String,
    },
    
    /// Handler execution was cancelled.
    #[error("Handler execution cancelled")]
    Cancelled,
    
    /// Too many handlers are registered for this event type.
    #[error("Too many handlers registered: {count} (max: {max})")]
    TooManyHandlers {
        /// Current handler count.
        count: usize,
        /// Maximum allowed handlers.
        max: usize,
    },
}

/// Errors that can occur during event validation.
#[derive(Debug, Error)]
pub enum EventValidationError {
    /// A required field is missing from the event.
    #[error("Required field missing: {field}")]
    MissingField {
        /// The name of the missing field.
        field: String,
    },
    
    /// A field has an invalid value.
    #[error("Invalid field value: {field} = {value}")]
    InvalidValue {
        /// The name of the field with invalid value.
        field: String,
        /// The invalid value.
        value: String,
    },
    
    /// The event data is in an invalid format.
    #[error("Invalid event format: {message}")]
    InvalidFormat {
        /// Description of the format error.
        message: String,
    },
    
    /// The event size exceeds the maximum allowed size.
    #[error("Event too large: {size} bytes (max: {max} bytes)")]
    TooLarge {
        /// The event size in bytes.
        size: usize,
        /// The maximum allowed size in bytes.
        max: usize,
    },
    
    /// A constraint violation occurred.
    #[error("Constraint violation: {constraint}")]
    ConstraintViolation {
        /// Description of the violated constraint.
        constraint: String,
    },
    
    /// Custom validation error with a specific message.
    #[error("Custom validation error: {message}")]
    Custom {
        /// The custom error message.
        message: String,
    },
}

/// Result type for middleware operations.
pub type MiddlewareResult = Result<(), MiddlewareError>;

/// Errors that can occur in middleware processing.
#[derive(Debug, Error)]
pub enum MiddlewareError {
    /// Authentication failed.
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    /// Authorization failed with a specific reason.
    #[error("Authorization failed: {reason}")]
    AuthorizationFailed {
        /// The reason for authorization failure.
        reason: String,
    },
    
    /// Rate limit has been exceeded.
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    /// Circuit breaker is open, preventing the operation.
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,
    
    /// The middleware chain was interrupted.
    #[error("Middleware chain interrupted")]
    ChainInterrupted,
    
    /// The middleware chain was short-circuited.
    #[error("Middleware chain short-circuited")]
    ChainShortCircuited,
    
    /// A middleware component failed to process the event.
    #[error("Middleware processing failed: {middleware}")]
    ProcessingFailed {
        /// The name of the failed middleware.
        middleware: String,
    },
    
    /// Middleware configuration is invalid.
    #[error("Invalid middleware configuration: {message}")]
    InvalidConfiguration {
        /// The configuration error message.
        message: String,
    },
    
    /// Custom middleware error with a specific message.
    #[error("Custom middleware error: {message}")]
    Custom {
        /// The custom error message.
        message: String,
    },
}

impl From<EventValidationError> for MiddlewareError {
    fn from(err: EventValidationError) -> Self {
        MiddlewareError::Custom {
            message: err.to_string(),
        }
    }
}

/// Errors that can occur during filter evaluation.
#[derive(Debug, Error)]
pub enum FilterError {
    /// Filter evaluation failed with a custom error.
    #[error("Filter evaluation failed: {message}")]
    EvaluationFailed {
        /// The error message.
        message: String,
    },
    
    /// Filter compilation failed (for regex or complex filters).
    #[error("Filter compilation failed: {pattern}")]
    CompilationFailed {
        /// The filter pattern that failed to compile.
        pattern: String,
    },
    
    /// Invalid filter configuration.
    #[error("Invalid filter configuration: {message}")]
    InvalidConfiguration {
        /// The configuration error message.
        message: String,
    },
    
    /// Filter type mismatch.
    #[error("Filter type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// The expected type name.
        expected: String,
        /// The actual type name.
        actual: String,
    },
    
    /// Custom filter error.
    #[error("Custom filter error: {message}")]
    Custom {
        /// The custom error message.
        message: String,
    },
}

/// Convenience type alias for EventBus operation results.
pub type EventBusResult<T> = Result<T, EventBusError>;

/// Convenience type alias for handler operation results.
pub type HandlerResult<T> = Result<T, HandlerError>;

/// Convenience type alias for validation results.
pub type ValidationResult<T> = Result<T, EventValidationError>;

/// Convenience type alias for filter results.
pub type FilterResult<T> = Result<T, FilterError>;

impl EventBusError {
    /// Creates a new configuration error.
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }
    
    /// Creates a new resource exhausted error.
    pub fn resource_exhausted<S: Into<String>>(resource: S) -> Self {
        Self::ResourceExhausted {
            resource: resource.into(),
        }
    }
    
    /// Creates a new timeout error.
    pub fn timeout(duration: Duration) -> Self {
        Self::Timeout { duration }
    }
    
    /// Creates a new concurrency error.
    pub fn concurrency<S: Into<String>>(message: S) -> Self {
        Self::ConcurrencyError {
            message: message.into(),
        }
    }
    
    /// Creates a new internal error.
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

impl HandlerError {
    /// Creates a new panic error.
    pub fn panic<S: Into<String>>(message: S) -> Self {
        Self::Panic {
            message: message.into(),
        }
    }
    
    /// Creates a new timeout error.
    pub fn timeout(duration: Duration) -> Self {
        Self::Timeout { duration }
    }
    
    /// Creates a new custom error from any error type.
    pub fn custom<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Custom(Box::new(error))
    }
    
    /// Creates a new registration failed error.
    pub fn registration_failed<S: Into<String>>(reason: S) -> Self {
        Self::RegistrationFailed {
            reason: reason.into(),
        }
    }
    
    /// Creates a new not found error.
    pub fn not_found<S: Into<String>>(handler_id: S) -> Self {
        Self::NotFound {
            handler_id: handler_id.into(),
        }
    }
    
    /// Creates a new already registered error.
    pub fn already_registered<S: Into<String>>(handler_id: S) -> Self {
        Self::AlreadyRegistered {
            handler_id: handler_id.into(),
        }
    }
    
    /// Creates a new too many handlers error.
    pub fn too_many_handlers(count: usize, max: usize) -> Self {
        Self::TooManyHandlers { count, max }
    }
}

impl EventValidationError {
    /// Creates a new missing field error.
    pub fn missing_field<S: Into<String>>(field: S) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }
    
    /// Creates a new invalid value error.
    pub fn invalid_value<S: Into<String>>(field: S, value: S) -> Self {
        Self::InvalidValue {
            field: field.into(),
            value: value.into(),
        }
    }
    
    /// Creates a new invalid format error.
    pub fn invalid_format<S: Into<String>>(message: S) -> Self {
        Self::InvalidFormat {
            message: message.into(),
        }
    }
    
    /// Creates a new too large error.
    pub fn too_large(size: usize, max: usize) -> Self {
        Self::TooLarge { size, max }
    }
    
    /// Creates a new constraint violation error.
    pub fn constraint_violation<S: Into<String>>(constraint: S) -> Self {
        Self::ConstraintViolation {
            constraint: constraint.into(),
        }
    }
    
    /// Creates a new custom validation error.
    pub fn custom<S: Into<String>>(message: S) -> Self {
        Self::Custom {
            message: message.into(),
        }
    }
}

impl MiddlewareError {
    /// Creates a new authorization failed error.
    pub fn authorization_failed<S: Into<String>>(reason: S) -> Self {
        Self::AuthorizationFailed {
            reason: reason.into(),
        }
    }
    
    /// Creates a new processing failed error.
    pub fn processing_failed<S: Into<String>>(middleware: S) -> Self {
        Self::ProcessingFailed {
            middleware: middleware.into(),
        }
    }
    
    /// Creates a new invalid configuration error.
    pub fn invalid_configuration<S: Into<String>>(message: S) -> Self {
        Self::InvalidConfiguration {
            message: message.into(),
        }
    }
    
    /// Creates a new custom middleware error.
    pub fn custom<S: Into<String>>(message: S) -> Self {
        Self::Custom {
            message: message.into(),
        }
    }
}

impl FilterError {
    /// Creates a new evaluation failed error.
    pub fn evaluation_failed<S: Into<String>>(message: S) -> Self {
        Self::EvaluationFailed {
            message: message.into(),
        }
    }
    
    /// Creates a new compilation failed error.
    pub fn compilation_failed<S: Into<String>>(pattern: S) -> Self {
        Self::CompilationFailed {
            pattern: pattern.into(),
        }
    }
    
    /// Creates a new invalid configuration error.
    pub fn invalid_configuration<S: Into<String>>(message: S) -> Self {
        Self::InvalidConfiguration {
            message: message.into(),
        }
    }
    
    /// Creates a new type mismatch error.
    pub fn type_mismatch<S: Into<String>>(expected: S, actual: S) -> Self {
        Self::TypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }
    
    /// Creates a new custom filter error.
    pub fn custom<S: Into<String>>(message: S) -> Self {
        Self::Custom {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let error = EventBusError::configuration("test config error");
        assert_eq!(error.to_string(), "Configuration error: test config error");
        
        let error = HandlerError::timeout(Duration::from_secs(5));
        assert_eq!(error.to_string(), "Handler timeout after 5s");
        
        let error = EventValidationError::missing_field("user_id");
        assert_eq!(error.to_string(), "Required field missing: user_id");
    }
    
    #[test]
    fn test_error_creation() {
        let timeout_error = EventBusError::timeout(Duration::from_millis(100));
        match timeout_error {
            EventBusError::Timeout { duration } => {
                assert_eq!(duration, Duration::from_millis(100));
            }
            _ => panic!("Expected timeout error"),
        }
        
        let validation_error = EventValidationError::invalid_value("age", "-5");
        match validation_error {
            EventValidationError::InvalidValue { field, value } => {
                assert_eq!(field, "age");
                assert_eq!(value, "-5");
            }
            _ => panic!("Expected invalid value error"),
        }
    }
    
    #[test]
    fn test_error_conversion() {
        let handler_error = HandlerError::panic("test panic");
        let bus_error: EventBusError = handler_error.into();
        
        match bus_error {
            EventBusError::HandlerError(HandlerError::Panic { message }) => {
                assert_eq!(message, "test panic");
            }
            _ => panic!("Expected handler error conversion"),
        }
    }
}