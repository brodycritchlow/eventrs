//! Event metadata system for EventRS.
//!
//! This module provides comprehensive metadata support for events, allowing
//! rich context information to be attached to events for debugging, filtering,
//! monitoring, and audit purposes.

use crate::priority::Priority;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

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
/// use eventrs::{EventMetadata, Priority, EventSource, SourceType};
/// use std::time::SystemTime;
/// 
/// let metadata = EventMetadata::new()
///     .with_priority(Priority::High)
///     .with_enhanced_source(EventSource::new("user_service", SourceType::Service))
///     .with_category("authentication")
///     .with_tags(vec!["security", "audit"])
///     .with_correlation_id("req-123")
///     .with_custom_field("user_ip", "192.168.1.100");
/// ```
#[derive(Debug, Clone)]
pub struct EventMetadata {
    /// Enhanced timestamp information.
    timestamps: EventTimestamps,
    
    /// Legacy timestamp field for backward compatibility.
    timestamp: Option<SystemTime>,
    
    /// Processing priority for this event.
    priority: Priority,
    
    /// Enhanced source information.
    enhanced_source: Option<EventSource>,
    
    /// Legacy source field for backward compatibility.
    source: Option<String>,
    
    /// Event signature for integrity verification.
    signature: Option<EventSignature>,
    
    /// Event provenance and audit trail.
    provenance: EventProvenance,
    
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
#[derive(Default)]
pub enum SecurityLevel {
    /// Public information, no restrictions.
    #[default]
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

/// Enhanced timestamp information for events.
#[derive(Debug, Clone)]
pub struct EventTimestamps {
    /// When the event was originally created.
    pub created_at: SystemTime,
    
    /// When the event was first received by the event bus.
    pub received_at: Option<SystemTime>,
    
    /// When the event processing started.
    pub processing_started_at: Option<SystemTime>,
    
    /// When the event processing completed.
    pub processing_completed_at: Option<SystemTime>,
    
    /// How long the event took to process.
    pub processing_duration: Option<Duration>,
    
    /// Custom timestamps for application-specific events.
    pub custom_timestamps: HashMap<String, SystemTime>,
}

impl EventTimestamps {
    /// Creates new timestamps with creation time set to now.
    pub fn new() -> Self {
        Self {
            created_at: SystemTime::now(),
            received_at: None,
            processing_started_at: None,
            processing_completed_at: None,
            processing_duration: None,
            custom_timestamps: HashMap::new(),
        }
    }
    
    /// Creates timestamps with a specific creation time.
    pub fn with_created_at(created_at: SystemTime) -> Self {
        Self {
            created_at,
            received_at: None,
            processing_started_at: None,
            processing_completed_at: None,
            processing_duration: None,
            custom_timestamps: HashMap::new(),
        }
    }
    
    /// Marks when the event was received.
    pub fn mark_received(&mut self) {
        self.received_at = Some(SystemTime::now());
    }
    
    /// Marks when processing started.
    pub fn mark_processing_started(&mut self) {
        self.processing_started_at = Some(SystemTime::now());
    }
    
    /// Marks when processing completed and calculates duration.
    pub fn mark_processing_completed(&mut self) {
        let now = SystemTime::now();
        self.processing_completed_at = Some(now);
        
        if let Some(start) = self.processing_started_at {
            if let Ok(duration) = now.duration_since(start) {
                self.processing_duration = Some(duration);
            }
        }
    }
    
    /// Adds a custom timestamp.
    pub fn add_custom_timestamp<S: Into<String>>(&mut self, name: S, timestamp: SystemTime) {
        self.custom_timestamps.insert(name.into(), timestamp);
    }
    
    /// Gets the total age of the event (time since creation).
    pub fn age(&self) -> Result<Duration, std::time::SystemTimeError> {
        SystemTime::now().duration_since(self.created_at)
    }
}

impl Default for EventTimestamps {
    fn default() -> Self {
        Self::new()
    }
}

/// Event signature for integrity verification.
#[derive(Debug, Clone)]
pub struct EventSignature {
    /// The signature algorithm used.
    pub algorithm: String,
    
    /// The actual signature bytes (typically base64 encoded).
    pub signature: String,
    
    /// The public key or key identifier used for verification.
    pub key_id: Option<String>,
    
    /// When the signature was created.
    pub signed_at: SystemTime,
    
    /// Additional signature metadata.
    pub metadata: HashMap<String, String>,
}

impl EventSignature {
    /// Creates a new event signature.
    pub fn new<A: Into<String>, S: Into<String>>(algorithm: A, signature: S) -> Self {
        Self {
            algorithm: algorithm.into(),
            signature: signature.into(),
            key_id: None,
            signed_at: SystemTime::now(),
            metadata: HashMap::new(),
        }
    }
    
    /// Sets the key ID for this signature.
    pub fn with_key_id<K: Into<String>>(mut self, key_id: K) -> Self {
        self.key_id = Some(key_id.into());
        self
    }
    
    /// Sets when the signature was created.
    pub fn with_signed_at(mut self, signed_at: SystemTime) -> Self {
        self.signed_at = signed_at;
        self
    }
    
    /// Adds signature metadata.
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Enhanced source information for events.
#[derive(Debug, Clone)]
pub struct EventSource {
    /// Primary source identifier.
    pub name: String,
    
    /// Source hierarchy (e.g., ["system", "service", "component"]).
    pub hierarchy: Vec<String>,
    
    /// Source type classification.
    pub source_type: SourceType,
    
    /// Version of the source system.
    pub version: Option<String>,
    
    /// Instance ID of the source (for scaled services).
    pub instance_id: Option<String>,
    
    /// Additional source metadata.
    pub metadata: HashMap<String, String>,
}

/// Types of event sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceType {
    /// A user action or interaction.
    User,
    
    /// An automated system process.
    System,
    
    /// A microservice or application component.
    Service,
    
    /// An external API or webhook.
    External,
    
    /// A scheduled task or cron job.
    Scheduled,
    
    /// A message queue or event stream.
    Queue,
    
    /// A database trigger or change.
    Database,
    
    /// Custom source type.
    Custom(String),
}

impl EventSource {
    /// Creates a new event source.
    pub fn new<S: Into<String>>(name: S, source_type: SourceType) -> Self {
        Self {
            name: name.into(),
            hierarchy: vec![],
            source_type,
            version: None,
            instance_id: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Sets the source hierarchy.
    pub fn with_hierarchy<I, S>(mut self, hierarchy: I) -> Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.hierarchy = hierarchy.into_iter().map(|s| s.into()).collect();
        self
    }
    
    /// Sets the source version.
    pub fn with_version<V: Into<String>>(mut self, version: V) -> Self {
        self.version = Some(version.into());
        self
    }
    
    /// Sets the instance ID.
    pub fn with_instance_id<I: Into<String>>(mut self, instance_id: I) -> Self {
        self.instance_id = Some(instance_id.into());
        self
    }
    
    /// Adds source metadata.
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    /// Gets the full source path (hierarchy + name).
    pub fn full_path(&self) -> String {
        if self.hierarchy.is_empty() {
            self.name.clone()
        } else {
            format!("{}/{}", self.hierarchy.join("/"), self.name)
        }
    }
}

/// Event provenance tracking for audit trails.
#[derive(Debug, Clone)]
pub struct EventProvenance {
    /// Chain of events that led to this event.
    pub parent_events: Vec<String>,
    
    /// Transformations applied to create this event.
    pub transformations: Vec<EventTransformation>,
    
    /// Original event ID if this is a derived event.
    pub original_event_id: Option<String>,
    
    /// Derivation depth (0 for original events).
    pub derivation_depth: u32,
}

/// A transformation applied to an event.
#[derive(Debug, Clone)]
pub struct EventTransformation {
    /// Type of transformation.
    pub transformation_type: String,
    
    /// When the transformation was applied.
    pub applied_at: SystemTime,
    
    /// Source that applied the transformation.
    pub applied_by: String,
    
    /// Transformation parameters or metadata.
    pub parameters: HashMap<String, String>,
}

impl EventProvenance {
    /// Creates new empty provenance.
    pub fn new() -> Self {
        Self {
            parent_events: vec![],
            transformations: vec![],
            original_event_id: None,
            derivation_depth: 0,
        }
    }
    
    /// Creates provenance for a derived event.
    pub fn derived_from<I: Into<String>>(original_event_id: I, parent_depth: u32) -> Self {
        Self {
            parent_events: vec![],
            transformations: vec![],
            original_event_id: Some(original_event_id.into()),
            derivation_depth: parent_depth + 1,
        }
    }
    
    /// Adds a parent event.
    pub fn add_parent_event<I: Into<String>>(&mut self, event_id: I) {
        self.parent_events.push(event_id.into());
    }
    
    /// Adds a transformation.
    pub fn add_transformation(&mut self, transformation: EventTransformation) {
        self.transformations.push(transformation);
    }
    
    /// Returns whether this is an original event.
    pub fn is_original(&self) -> bool {
        self.derivation_depth == 0 && self.original_event_id.is_none()
    }
}

impl Default for EventProvenance {
    fn default() -> Self {
        Self::new()
    }
}

impl EventTransformation {
    /// Creates a new transformation.
    pub fn new<T: Into<String>, A: Into<String>>(
        transformation_type: T, 
        applied_by: A
    ) -> Self {
        Self {
            transformation_type: transformation_type.into(),
            applied_at: SystemTime::now(),
            applied_by: applied_by.into(),
            parameters: HashMap::new(),
        }
    }
    
    /// Adds a parameter to the transformation.
    pub fn with_parameter<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }
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
            timestamps: EventTimestamps::new(),
            timestamp: None,
            priority: Priority::default(),
            enhanced_source: None,
            source: None,
            signature: None,
            provenance: EventProvenance::new(),
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
    
    /// Creates metadata with a custom creation timestamp.
    pub fn with_created_at(created_at: SystemTime) -> Self {
        Self {
            timestamps: EventTimestamps::with_created_at(created_at),
            timestamp: Some(created_at), // For backward compatibility
            priority: Priority::default(),
            enhanced_source: None,
            source: None,
            signature: None,
            provenance: EventProvenance::new(),
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
        self.timestamps.created_at = timestamp;
        self
    }
    
    /// Sets enhanced source information for this event.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventMetadata, EventSource, SourceType};
    /// 
    /// let source = EventSource::new("auth_service", SourceType::Service)
    ///     .with_version("1.2.3")
    ///     .with_instance_id("auth-001");
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_enhanced_source(source);
    /// ```
    pub fn with_enhanced_source(mut self, source: EventSource) -> Self {
        // Set legacy source field for backward compatibility
        self.source = Some(source.full_path());
        self.enhanced_source = Some(source);
        self
    }
    
    /// Sets an event signature for integrity verification.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventMetadata, EventSignature};
    /// 
    /// let signature = EventSignature::new("HMAC-SHA256", "base64_signature_here")
    ///     .with_key_id("key_123");
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_signature(signature);
    /// ```
    pub fn with_signature(mut self, signature: EventSignature) -> Self {
        self.signature = Some(signature);
        self
    }
    
    /// Sets event provenance information.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{EventMetadata, EventProvenance};
    /// 
    /// let provenance = EventProvenance::derived_from("original_event_123", 0);
    /// 
    /// let metadata = EventMetadata::new()
    ///     .with_provenance(provenance);
    /// ```
    pub fn with_provenance(mut self, provenance: EventProvenance) -> Self {
        self.provenance = provenance;
        self
    }
    
    /// Marks the event as received (sets received timestamp).
    pub fn mark_received(&mut self) {
        self.timestamps.mark_received();
    }
    
    /// Marks processing as started.
    pub fn mark_processing_started(&mut self) {
        self.timestamps.mark_processing_started();
    }
    
    /// Marks processing as completed.
    pub fn mark_processing_completed(&mut self) {
        self.timestamps.mark_processing_completed();
    }
    
    /// Adds a custom timestamp.
    pub fn add_custom_timestamp<S: Into<String>>(&mut self, name: S, timestamp: SystemTime) {
        self.timestamps.add_custom_timestamp(name, timestamp);
    }
    
    /// Adds a parent event to the provenance chain.
    pub fn add_parent_event<I: Into<String>>(&mut self, event_id: I) {
        self.provenance.add_parent_event(event_id);
    }
    
    /// Adds a transformation to the provenance trail.
    pub fn add_transformation(&mut self, transformation: EventTransformation) {
        self.provenance.add_transformation(transformation);
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
    
    /// Returns the enhanced source information.
    pub fn enhanced_source(&self) -> Option<&EventSource> {
        self.enhanced_source.as_ref()
    }
    
    /// Returns the event signature.
    pub fn signature(&self) -> Option<&EventSignature> {
        self.signature.as_ref()
    }
    
    /// Returns the event provenance information.
    pub fn provenance(&self) -> &EventProvenance {
        &self.provenance
    }
    
    /// Returns the enhanced timestamp information.
    pub fn timestamps(&self) -> &EventTimestamps {
        &self.timestamps
    }
    
    /// Returns the creation timestamp.
    pub fn created_at(&self) -> SystemTime {
        self.timestamps.created_at
    }
    
    /// Returns when the event was received.
    pub fn received_at(&self) -> Option<SystemTime> {
        self.timestamps.received_at
    }
    
    /// Returns when processing started.
    pub fn processing_started_at(&self) -> Option<SystemTime> {
        self.timestamps.processing_started_at
    }
    
    /// Returns when processing completed.
    pub fn processing_completed_at(&self) -> Option<SystemTime> {
        self.timestamps.processing_completed_at
    }
    
    /// Returns the processing duration.
    pub fn processing_duration(&self) -> Option<Duration> {
        self.timestamps.processing_duration
    }
    
    /// Returns the event age (time since creation).
    pub fn age(&self) -> Result<Duration, std::time::SystemTimeError> {
        self.timestamps.age()
    }
    
    /// Returns a custom timestamp by name.
    pub fn custom_timestamp(&self, name: &str) -> Option<SystemTime> {
        self.timestamps.custom_timestamps.get(name).copied()
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
    /// Note: This checks for meaningful metadata beyond the default creation timestamp.
    pub fn is_empty(&self) -> bool {
        self.timestamp.is_none()
            && self.priority == Priority::default()
            && self.enhanced_source.is_none()
            && self.source.is_none()
            && self.signature.is_none()
            && self.provenance.is_original()
            && self.provenance.parent_events.is_empty()
            && self.provenance.transformations.is_empty()
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
            && self.timestamps.custom_timestamps.is_empty()
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

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            SourceType::User => "User",
            SourceType::System => "System",
            SourceType::Service => "Service",
            SourceType::External => "External",
            SourceType::Scheduled => "Scheduled",
            SourceType::Queue => "Queue",
            SourceType::Database => "Database",
            SourceType::Custom(s) => s,
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
        // Note: Metadata with only a creation timestamp is still considered "empty"
        // in terms of meaningful business data
        assert!(metadata.is_empty());

        let metadata_with_data = EventMetadata::new()
            .with_source("test");
        assert!(!metadata_with_data.is_empty());
    }

    #[test]
    fn test_enhanced_timestamps() {
        let mut metadata = EventMetadata::new();
        
        // Test that creation timestamp is automatically set
        assert!(metadata.created_at() <= SystemTime::now());
        
        // Test marking processing events
        metadata.mark_received();
        assert!(metadata.received_at().is_some());
        
        metadata.mark_processing_started();
        assert!(metadata.processing_started_at().is_some());
        
        metadata.mark_processing_completed();
        assert!(metadata.processing_completed_at().is_some());
        assert!(metadata.processing_duration().is_some());
        
        // Test custom timestamps
        let custom_time = SystemTime::now();
        metadata.add_custom_timestamp("deployment", custom_time);
        assert_eq!(metadata.custom_timestamp("deployment"), Some(custom_time));
        
        // Test age calculation
        assert!(metadata.age().is_ok());
    }

    #[test]
    fn test_enhanced_source() {
        let source = EventSource::new("user_service", SourceType::Service)
            .with_hierarchy(vec!["platform", "auth"])
            .with_version("1.2.3")
            .with_instance_id("auth-001")
            .with_metadata("region", "us-west-2");
        
        let metadata = EventMetadata::new()
            .with_enhanced_source(source.clone());
        
        assert_eq!(metadata.enhanced_source().unwrap().name, "user_service");
        assert_eq!(metadata.enhanced_source().unwrap().source_type, SourceType::Service);
        assert_eq!(metadata.enhanced_source().unwrap().full_path(), "platform/auth/user_service");
        assert_eq!(metadata.enhanced_source().unwrap().version, Some("1.2.3".to_string()));
        assert_eq!(metadata.enhanced_source().unwrap().instance_id, Some("auth-001".to_string()));
        
        // Test backward compatibility
        assert_eq!(metadata.source(), Some("platform/auth/user_service"));
    }

    #[test]
    fn test_event_signature() {
        let signature = EventSignature::new("HMAC-SHA256", "base64_encoded_signature")
            .with_key_id("key_123")
            .with_metadata("algorithm_params", "salt=random123");
        
        let metadata = EventMetadata::new()
            .with_signature(signature);
        
        let sig = metadata.signature().unwrap();
        assert_eq!(sig.algorithm, "HMAC-SHA256");
        assert_eq!(sig.signature, "base64_encoded_signature");
        assert_eq!(sig.key_id, Some("key_123".to_string()));
        assert!(sig.signed_at <= SystemTime::now());
    }

    #[test]
    fn test_event_provenance() {
        let metadata = EventMetadata::new();
        
        // Test original event
        assert!(metadata.provenance().is_original());
        assert_eq!(metadata.provenance().derivation_depth, 0);
        
        // Test derived event
        let derived_provenance = EventProvenance::derived_from("original_event_123", 0);
        let mut derived_metadata = EventMetadata::new()
            .with_provenance(derived_provenance);
        
        assert!(!derived_metadata.provenance().is_original());
        assert_eq!(derived_metadata.provenance().derivation_depth, 1);
        assert_eq!(derived_metadata.provenance().original_event_id, Some("original_event_123".to_string()));
        
        // Test adding parent events and transformations
        derived_metadata.add_parent_event("parent_event_456");
        
        let transformation = EventTransformation::new("filter", "data_pipeline")
            .with_parameter("filter_type", "content_based");
        derived_metadata.add_transformation(transformation);
        
        assert_eq!(derived_metadata.provenance().parent_events.len(), 1);
        assert_eq!(derived_metadata.provenance().transformations.len(), 1);
        assert_eq!(derived_metadata.provenance().transformations[0].transformation_type, "filter");
    }

    #[test]
    fn test_source_types_and_display() {
        assert_eq!(format!("{}", SourceType::User), "User");
        assert_eq!(format!("{}", SourceType::Service), "Service");
        assert_eq!(format!("{}", SourceType::Custom("webhook".to_string())), "webhook");
        
        let user_source = EventSource::new("ui_component", SourceType::User);
        let service_source = EventSource::new("auth_service", SourceType::Service);
        
        assert_eq!(user_source.source_type, SourceType::User);
        assert_eq!(service_source.source_type, SourceType::Service);
    }

    #[test]
    fn test_event_transformation() {
        let transformation = EventTransformation::new("enrichment", "analytics_service")
            .with_parameter("fields_added", "user_segment,geo_location")
            .with_parameter("version", "2.1");
        
        assert_eq!(transformation.transformation_type, "enrichment");
        assert_eq!(transformation.applied_by, "analytics_service");
        assert_eq!(transformation.parameters.get("fields_added"), Some(&"user_segment,geo_location".to_string()));
        assert!(transformation.applied_at <= SystemTime::now());
    }

    #[test]
    fn test_metadata_with_all_enhanced_features() {
        let now = SystemTime::now();
        
        let source = EventSource::new("payment_service", SourceType::Service)
            .with_hierarchy(vec!["platform", "commerce"])
            .with_version("2.1.0")
            .with_instance_id("payment-003");
        
        let signature = EventSignature::new("ED25519", "signature_bytes_here")
            .with_key_id("payment_key_v2");
        
        let provenance = EventProvenance::derived_from("original_payment_event", 1);
        
        let metadata = EventMetadata::with_created_at(now)
            .with_priority(Priority::High)
            .with_enhanced_source(source)
            .with_signature(signature)
            .with_provenance(provenance)
            .with_category("payment")
            .with_tags(vec!["financial", "audit"])
            .with_correlation_id("pay_123")
            .with_user_id("user_456")
            .with_security_level(SecurityLevel::Confidential)
            .with_pii(true)
            .with_retention_policy("7_years");
        
        // Test enhanced features
        assert_eq!(metadata.created_at(), now);
        assert!(metadata.enhanced_source().is_some());
        assert!(metadata.signature().is_some());
        assert!(!metadata.provenance().is_original());
        assert_eq!(metadata.provenance().derivation_depth, 2);
        
        // Test backward compatibility
        assert_eq!(metadata.timestamp(), Some(now));
        assert_eq!(metadata.source(), Some("platform/commerce/payment_service"));
        
        // Test that it's not empty
        assert!(!metadata.is_empty());
    }
}