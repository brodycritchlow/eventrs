//! Event metadata system for EventRS.
//!
//! This module provides comprehensive metadata support for events, allowing
//! rich context information to be attached to events for debugging, filtering,
//! monitoring, and audit purposes.

use crate::priority::Priority;
use std::collections::HashMap;
use std::time::SystemTime;

/// Metadata associated with an event.
/// 
/// Event metadata provides additional context information that can be used
/// for debugging, filtering, monitoring, and audit purposes. Metadata is
/// separate from the event data itself and doesn't affect the core event
/// processing logic.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{EventMetadata, Priority};
/// use std::time::SystemTime;
/// 
/// let metadata = EventMetadata::new()
///     .with_timestamp(SystemTime::now())
///     .with_priority(Priority::High)
///     .with_source("user_service")
///     .with_category("authentication")
///     .with_tags(vec!["security", "audit"])
///     .with_correlation_id("req-123")
///     .with_custom_field("user_ip", "192.168.1.100");
/// ```
#[derive(Debug, Clone)]
pub struct EventMetadata {
    /// When the event was created or occurred.
    timestamp: Option<SystemTime>,
    
    /// Processing priority for this event.
    priority: Priority,
    
    /// Source system, service, or component that generated the event.
    source: Option<String>,
    
    /// Category or classification of the event.
    category: Option<String>,
    
    /// Tags for flexible event classification and filtering.
    tags: Vec<String>,
    
    /// Correlation ID for tracing related events across services.
    correlation_id: Option<String>,
    
    /// Trace ID for distributed tracing systems.
    trace_id: Option<String>,
    
    /// Span ID for distributed tracing systems.
    span_id: Option<String>,
    
    /// User ID associated with this event, if applicable.
    user_id: Option<String>,
    
    /// Session ID associated with this event, if applicable.
    session_id: Option<String>,
    
    /// Request ID for HTTP requests or API calls.
    request_id: Option<String>,
    
    /// Environment where the event occurred (prod, staging, dev, etc.).
    environment: Option<String>,
    
    /// Version of the application or service that generated the event.
    version: Option<String>,
    
    /// Custom fields for application-specific metadata.
    custom_fields: HashMap<String, String>,
    
    /// Estimated size of the event data in bytes.
    size_hint: Option<usize>,
    
    /// Security classification level.
    security_level: SecurityLevel,
    
    /// Whether this event contains sensitive data.
    contains_pii: bool,
    
    /// Retention policy for this event.
    retention_policy: Option<String>,
}

/// Security classification levels for events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// Public information, no restrictions.
    Public,
    
    /// Internal use only, not for external sharing.
    Internal,
    
    /// Confidential information, restricted access.
    Confidential,
    
    /// Restricted information, very limited access.
    Restricted,
    
    /// Top secret information, highest security level.
    TopSecret,
}

impl EventMetadata {
    /// Creates new empty metadata with default values.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventMetadata, Priority};
    /// 
    /// let metadata = EventMetadata::new();
    /// assert_eq!(metadata.priority(), Priority::Normal);
    /// assert!(!metadata.contains_pii());
    /// ```
    pub fn new() -> Self {
        Self {
            timestamp: None,
            priority: Priority::default(),
            source: None,
            category: None,
            tags: Vec::new(),
            correlation_id: None,
            trace_id: None,
            span_id: None,
            user_id: None,
            session_id: None,
            request_id: None,
            environment: None,
            version: None,
            custom_fields: HashMap::new(),
            size_hint: None,
            security_level: SecurityLevel::Public,
            contains_pii: false,
            retention_policy: None,
        }
    }
    
    /// Sets the timestamp for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// use std::time::SystemTime;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_timestamp(SystemTime::now());
    /// ```
    pub fn with_timestamp(mut self, timestamp: SystemTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
    
    /// Sets the priority for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventMetadata, Priority};
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_priority(Priority::High);
    /// ```
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Sets the source for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_source("user_service");
    /// ```
    pub fn with_source<S: Into<String>>(mut self, source: S) -> Self {
        self.source = Some(source.into());
        self
    }
    
    /// Sets the category for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_category("authentication");
    /// ```
    pub fn with_category<S: Into<String>>(mut self, category: S) -> Self {
        self.category = Some(category.into());
        self
    }
    
    /// Adds tags to this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_tags(vec!["security", "audit", "login"]);
    /// ```
    pub fn with_tags<I, S>(mut self, tags: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(|s| s.into()));
        self
    }
    
    /// Adds a single tag to this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_tag("security")
    ///     .with_tag("audit");
    /// ```
    pub fn with_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(tag.into());
        self
    }
    
    /// Sets a correlation ID for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_correlation_id("req-123-456");
    /// ```
    pub fn with_correlation_id<S: Into<String>>(mut self, id: S) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
    
    /// Sets a trace ID for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_trace_id("trace-abc-123");
    /// ```
    pub fn with_trace_id<S: Into<String>>(mut self, id: S) -> Self {
        self.trace_id = Some(id.into());
        self
    }
    
    /// Sets a span ID for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_span_id("span-def-456");
    /// ```
    pub fn with_span_id<S: Into<String>>(mut self, id: S) -> Self {
        self.span_id = Some(id.into());
        self
    }
    
    /// Sets a user ID for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_user_id("user-789");
    /// ```
    pub fn with_user_id<S: Into<String>>(mut self, id: S) -> Self {
        self.user_id = Some(id.into());
        self
    }
    
    /// Sets a session ID for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_session_id("session-xyz-789");
    /// ```
    pub fn with_session_id<S: Into<String>>(mut self, id: S) -> Self {
        self.session_id = Some(id.into());
        self
    }
    
    /// Sets a request ID for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_request_id("req-987-654");
    /// ```
    pub fn with_request_id<S: Into<String>>(mut self, id: S) -> Self {
        self.request_id = Some(id.into());
        self
    }
    
    /// Sets the environment for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_environment("production");
    /// ```
    pub fn with_environment<S: Into<String>>(mut self, environment: S) -> Self {
        self.environment = Some(environment.into());
        self
    }
    
    /// Sets the version for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_version("1.2.3");
    /// ```
    pub fn with_version<S: Into<String>>(mut self, version: S) -> Self {
        self.version = Some(version.into());
        self
    }
    
    /// Adds a custom field to this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_custom_field("client_ip", "192.168.1.100")
    ///     .with_custom_field("user_agent", "Mozilla/5.0...");
    /// ```
    pub fn with_custom_field<K, V>(mut self, key: K, value: V) -> Self 
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.custom_fields.insert(key.into(), value.into());
        self
    }
    
    /// Adds multiple custom fields to this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// use std::collections::HashMap;
    /// 
    /// let mut fields = HashMap::new();
    /// fields.insert("key1".to_string(), "value1".to_string());
    /// fields.insert("key2".to_string(), "value2".to_string());
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_custom_fields(fields);
    /// ```
    pub fn with_custom_fields<K, V>(mut self, fields: HashMap<K, V>) -> Self 
    where
        K: Into<String>,
        V: Into<String>,
    {
        for (key, value) in fields {
            self.custom_fields.insert(key.into(), value.into());
        }
        self
    }
    
    /// Sets the size hint for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_size_hint(1024);
    /// ```
    pub fn with_size_hint(mut self, size: usize) -> Self {
        self.size_hint = Some(size);
        self
    }
    
    /// Sets the security level for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventMetadata, SecurityLevel};
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_security_level(SecurityLevel::Confidential);
    /// ```
    pub fn with_security_level(mut self, level: SecurityLevel) -> Self {
        self.security_level = level;
        self
    }
    
    /// Marks this event as containing personally identifiable information (PII).
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_pii(true);
    /// ```
    pub fn with_pii(mut self, contains_pii: bool) -> Self {
        self.contains_pii = contains_pii;
        self
    }
    
    /// Sets the retention policy for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::EventMetadata;
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_retention_policy("30_days");
    /// ```
    pub fn with_retention_policy<S: Into<String>>(mut self, policy: S) -> Self {
        self.retention_policy = Some(policy.into());
        self
    }
    
    // Getter methods
    
    /// Returns the timestamp for this event.
    pub fn timestamp(&self) -> Option<SystemTime> {
        self.timestamp
    }
    
    /// Returns the priority for this event.
    pub fn priority(&self) -> Priority {
        self.priority
    }
    
    /// Returns the source for this event.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
    
    /// Returns the category for this event.
    pub fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }
    
    /// Returns the tags for this event.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
    
    /// Returns whether this event has the specified tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
    
    /// Returns the correlation ID for this event.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }
    
    /// Returns the trace ID for this event.
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_deref()
    }
    
    /// Returns the span ID for this event.
    pub fn span_id(&self) -> Option<&str> {
        self.span_id.as_deref()
    }
    
    /// Returns the user ID for this event.
    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }
    
    /// Returns the session ID for this event.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
    
    /// Returns the request ID for this event.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
    
    /// Returns the environment for this event.
    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }
    
    /// Returns the version for this event.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
    
    /// Returns a custom field value by key.
    pub fn custom_field(&self, key: &str) -> Option<&str> {
        self.custom_fields.get(key).map(|s| s.as_str())
    }
    
    /// Returns all custom fields.
    pub fn custom_fields(&self) -> &HashMap<String, String> {
        &self.custom_fields
    }
    
    /// Returns the size hint for this event.
    pub fn size_hint(&self) -> Option<usize> {
        self.size_hint
    }
    
    /// Returns the security level for this event.
    pub fn security_level(&self) -> SecurityLevel {
        self.security_level
    }
    
    /// Returns whether this event contains PII.
    pub fn contains_pii(&self) -> bool {
        self.contains_pii
    }
    
    /// Returns the retention policy for this event.
    pub fn retention_policy(&self) -> Option<&str> {
        self.retention_policy.as_deref()
    }
    
    /// Returns whether this metadata is empty (contains no meaningful data).
    pub fn is_empty(&self) -> bool {
        self.timestamp.is_none()
            && self.priority == Priority::default()
            && self.source.is_none()
            && self.category.is_none()
            && self.tags.is_empty()
            && self.correlation_id.is_none()
            && self.trace_id.is_none()
            && self.span_id.is_none()
            && self.user_id.is_none()
            && self.session_id.is_none()
            && self.request_id.is_none()
            && self.environment.is_none()
            && self.version.is_none()
            && self.custom_fields.is_empty()
            && self.size_hint.is_none()
            && self.security_level == SecurityLevel::Public
            && !self.contains_pii
            && self.retention_policy.is_none()
    }
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SecurityLevel {
    fn default() -> Self {
        SecurityLevel::Public
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SecurityLevel::Public => "Public",
            SecurityLevel::Internal => "Internal",
            SecurityLevel::Confidential => "Confidential",
            SecurityLevel::Restricted => "Restricted",
            SecurityLevel::TopSecret => "TopSecret",
        };
        write!(f, "{}", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_metadata_creation() {
        let metadata = EventMetadata::new();
        assert!(metadata.is_empty());
        assert_eq!(metadata.priority(), Priority::Normal);
        assert!(!metadata.contains_pii());
        assert_eq!(metadata.security_level(), SecurityLevel::Public);
    }

    #[test]
    fn test_metadata_builder_pattern() {
        let now = SystemTime::now();
        let metadata = EventMetadata::new()
            .with_timestamp(now)
            .with_priority(Priority::High)
            .with_source("test_service")
            .with_category("test")
            .with_tags(vec!["tag1", "tag2"])
            .with_correlation_id("corr-123")
            .with_trace_id("trace-456")
            .with_span_id("span-789")
            .with_user_id("user-123")
            .with_session_id("session-456")
            .with_request_id("req-789")
            .with_environment("test")
            .with_version("1.0.0")
            .with_custom_field("key1", "value1")
            .with_size_hint(1024)
            .with_security_level(SecurityLevel::Confidential)
            .with_pii(true)
            .with_retention_policy("30_days");

        assert_eq!(metadata.timestamp(), Some(now));
        assert_eq!(metadata.priority(), Priority::High);
        assert_eq!(metadata.source(), Some("test_service"));
        assert_eq!(metadata.category(), Some("test"));
        assert_eq!(metadata.tags(), &["tag1", "tag2"]);
        assert_eq!(metadata.correlation_id(), Some("corr-123"));
        assert_eq!(metadata.trace_id(), Some("trace-456"));
        assert_eq!(metadata.span_id(), Some("span-789"));
        assert_eq!(metadata.user_id(), Some("user-123"));
        assert_eq!(metadata.session_id(), Some("session-456"));
        assert_eq!(metadata.request_id(), Some("req-789"));
        assert_eq!(metadata.environment(), Some("test"));
        assert_eq!(metadata.version(), Some("1.0.0"));
        assert_eq!(metadata.custom_field("key1"), Some("value1"));
        assert_eq!(metadata.size_hint(), Some(1024));
        assert_eq!(metadata.security_level(), SecurityLevel::Confidential);
        assert!(metadata.contains_pii());
        assert_eq!(metadata.retention_policy(), Some("30_days"));
        assert!(!metadata.is_empty());
    }

    #[test]
    fn test_tags() {
        let metadata = EventMetadata::new()
            .with_tag("tag1")
            .with_tag("tag2")
            .with_tags(vec!["tag3", "tag4"]);

        assert_eq!(metadata.tags(), &["tag1", "tag2", "tag3", "tag4"]);
        assert!(metadata.has_tag("tag1"));
        assert!(metadata.has_tag("tag4"));
        assert!(!metadata.has_tag("tag5"));
    }

    #[test]
    fn test_custom_fields() {
        let mut fields = HashMap::new();
        fields.insert("key1".to_string(), "value1".to_string());
        fields.insert("key2".to_string(), "value2".to_string());

        let metadata = EventMetadata::new()
            .with_custom_field("key3", "value3")
            .with_custom_fields(fields);

        assert_eq!(metadata.custom_field("key1"), Some("value1"));
        assert_eq!(metadata.custom_field("key2"), Some("value2"));
        assert_eq!(metadata.custom_field("key3"), Some("value3"));
        assert_eq!(metadata.custom_field("nonexistent"), None);
        assert_eq!(metadata.custom_fields().len(), 3);
    }

    #[test]
    fn test_security_level() {
        assert!(SecurityLevel::TopSecret > SecurityLevel::Restricted);
        assert!(SecurityLevel::Restricted > SecurityLevel::Confidential);
        assert!(SecurityLevel::Confidential > SecurityLevel::Internal);
        assert!(SecurityLevel::Internal > SecurityLevel::Public);

        assert_eq!(SecurityLevel::default(), SecurityLevel::Public);
        assert_eq!(format!("{}", SecurityLevel::Confidential), "Confidential");
    }

    #[test]
    fn test_empty_metadata() {
        let metadata = EventMetadata::new();
        assert!(metadata.is_empty());

        let metadata_with_data = EventMetadata::new()
            .with_source("test");
        assert!(!metadata_with_data.is_empty());
    }
}