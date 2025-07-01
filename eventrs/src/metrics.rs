//! Metrics collection for EventRS.
//!
//! This module provides performance metrics and monitoring capabilities
//! for event buses, handlers, and overall system performance.
//!
//! # Examples
//!
//! ```rust
//! use eventrs::{EventBusMetrics, EmissionResult};
//! 
//! let metrics = EventBusMetrics::new();
//! 
//! // Metrics are automatically collected when events are emitted
//! // and can be accessed via the metrics instance
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::any::TypeId;

/// Comprehensive metrics collection for EventBus performance tracking.
/// 
/// EventBusMetrics provides detailed performance monitoring including:
/// - Event emission statistics
/// - Handler execution metrics
/// - Throughput and latency measurements
/// - Error tracking and analysis
#[derive(Debug, Clone)]
pub struct EventBusMetrics {
    /// Event emission statistics by event type
    emission_stats: Arc<RwLock<HashMap<TypeId, EmissionStats>>>,
    /// Handler execution metrics by handler ID
    handler_metrics: Arc<RwLock<HashMap<String, HandlerMetrics>>>,
    /// Overall system metrics
    system_metrics: Arc<Mutex<SystemMetrics>>,
    /// Whether metrics collection is enabled
    enabled: bool,
}

/// Statistics for event emissions of a specific type.
#[derive(Debug, Clone, Default)]
pub struct EmissionStats {
    /// Total number of events emitted
    pub total_emissions: u64,
    /// Total number of successful emissions
    pub successful_emissions: u64,
    /// Total number of failed emissions
    pub failed_emissions: u64,
    /// Average emission time in milliseconds
    pub avg_emission_time_ms: f64,
    /// Minimum emission time recorded
    pub min_emission_time: Duration,
    /// Maximum emission time recorded
    pub max_emission_time: Duration,
    /// Total time spent processing this event type
    pub total_processing_time: Duration,
    /// Last emission timestamp
    pub last_emission: Option<SystemTime>,
    /// Number of handlers that processed this event type
    pub handler_count: usize,
}

/// Metrics for individual handler performance.
#[derive(Debug, Clone, Default)]
pub struct HandlerMetrics {
    /// Handler identifier
    pub handler_id: String,
    /// Event type this handler processes
    pub event_type: String,
    /// Total number of invocations
    pub invocation_count: u64,
    /// Number of successful executions
    pub successful_executions: u64,
    /// Number of failed executions
    pub failed_executions: u64,
    /// Average execution time
    pub avg_execution_time: Duration,
    /// Minimum execution time
    pub min_execution_time: Duration,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Last execution timestamp
    pub last_execution: Option<SystemTime>,
    /// Handler priority
    pub priority: String,
}

/// System-wide metrics for the event bus.
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    /// Total number of events processed across all types
    pub total_events_processed: u64,
    /// Total number of handlers registered
    pub total_handlers_registered: usize,
    /// System uptime since metrics started
    pub uptime: Duration,
    /// Average events per second
    pub events_per_second: f64,
    /// Peak events per second
    pub peak_events_per_second: f64,
    /// Memory usage estimate (in bytes)
    pub estimated_memory_usage: usize,
    /// Start time of metrics collection
    pub start_time: SystemTime,
    /// Last reset timestamp
    pub last_reset: Option<SystemTime>,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            total_events_processed: 0,
            total_handlers_registered: 0,
            uptime: Duration::ZERO,
            events_per_second: 0.0,
            peak_events_per_second: 0.0,
            estimated_memory_usage: 0,
            start_time: SystemTime::now(),
            last_reset: None,
        }
    }
}

/// Detailed result of an event emission with performance metrics.
/// 
/// EmissionResult provides comprehensive information about the execution
/// of an event emission, including timing, handler results, and any errors.
#[derive(Debug, Clone)]
pub struct EmissionResult {
    /// Whether the emission was successful
    pub success: bool,
    /// Total time taken for the emission
    pub total_duration: Duration,
    /// Number of handlers that processed the event
    pub handlers_executed: usize,
    /// Number of handlers that succeeded
    pub handlers_succeeded: usize,
    /// Number of handlers that failed
    pub handlers_failed: usize,
    /// Detailed results for each handler
    pub handler_results: Vec<HandlerResult>,
    /// Any errors that occurred during emission
    pub errors: Vec<String>,
    /// Timestamp when emission started
    pub emission_start: SystemTime,
    /// Timestamp when emission completed
    pub emission_end: SystemTime,
    /// Event type that was emitted
    pub event_type: String,
    /// Whether the emission was processed with priority ordering
    pub used_priority_ordering: bool,
    /// Middleware execution metrics if applicable
    pub middleware_metrics: Option<Vec<MiddlewareExecutionMetric>>,
}

/// Result of a single handler execution.
#[derive(Debug, Clone)]
pub struct HandlerResult {
    /// Handler identifier
    pub handler_id: String,
    /// Whether the handler executed successfully
    pub success: bool,
    /// Time taken for handler execution
    pub execution_time: Duration,
    /// Handler priority
    pub priority: String,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Whether the handler was skipped due to filtering
    pub skipped_by_filter: bool,
}

/// Metrics for middleware execution within an emission.
#[derive(Debug, Clone)]
pub struct MiddlewareExecutionMetric {
    /// Middleware name
    pub middleware_name: String,
    /// Execution time
    pub execution_time: Duration,
    /// Whether middleware called next()
    pub called_next: bool,
    /// Any error that occurred
    pub error: Option<String>,
}

impl EventBusMetrics {
    /// Creates a new EventBusMetrics instance.
    /// 
    /// # Arguments
    /// * `enabled` - Whether to enable metrics collection
    /// 
    /// # Examples
    /// ```rust
    /// use eventrs::EventBusMetrics;
    /// 
    /// let metrics = EventBusMetrics::new(true);
    /// ```
    pub fn new(enabled: bool) -> Self {
        Self {
            emission_stats: Arc::new(RwLock::new(HashMap::new())),
            handler_metrics: Arc::new(RwLock::new(HashMap::new())),
            system_metrics: Arc::new(Mutex::new(SystemMetrics {
                start_time: SystemTime::now(),
                ..Default::default()
            })),
            enabled,
        }
    }
    
    /// Creates a new EventBusMetrics instance with metrics enabled.
    pub fn enabled() -> Self {
        Self::new(true)
    }
    
    /// Creates a new EventBusMetrics instance with metrics disabled.
    pub fn disabled() -> Self {
        Self::new(false)
    }
    
    /// Returns whether metrics collection is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Enables metrics collection.
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disables metrics collection.
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Records the start of an event emission.
    /// 
    /// Returns a token that should be used with `record_emission_end`.
    pub fn start_emission(&self, event_type: TypeId) -> EmissionToken {
        if !self.enabled {
            return EmissionToken::disabled();
        }
        
        EmissionToken {
            event_type,
            start_time: Instant::now(),
            start_timestamp: SystemTime::now(),
            enabled: true,
        }
    }
    
    /// Records the completion of an event emission.
    /// 
    /// # Arguments
    /// * `token` - Token returned from `start_emission`
    /// * `result` - The emission result
    pub fn record_emission_end(&self, token: EmissionToken, result: EmissionResult) {
        if !self.enabled || !token.enabled {
            return;
        }
        
        let duration = token.start_time.elapsed();
        
        // Update emission stats
        {
            let mut stats = self.emission_stats.write().unwrap();
            let emission_stats = stats.entry(token.event_type).or_default();
            
            emission_stats.total_emissions += 1;
            if result.success {
                emission_stats.successful_emissions += 1;
            } else {
                emission_stats.failed_emissions += 1;
            }
            
            // Update timing statistics
            let duration_ms = duration.as_secs_f64() * 1000.0;
            emission_stats.avg_emission_time_ms = 
                (emission_stats.avg_emission_time_ms * (emission_stats.total_emissions - 1) as f64 + duration_ms) 
                / emission_stats.total_emissions as f64;
            
            if emission_stats.total_emissions == 1 || duration < emission_stats.min_emission_time {
                emission_stats.min_emission_time = duration;
            }
            if duration > emission_stats.max_emission_time {
                emission_stats.max_emission_time = duration;
            }
            
            emission_stats.total_processing_time += duration;
            emission_stats.last_emission = Some(token.start_timestamp);
            emission_stats.handler_count = result.handlers_executed;
        }
        
        // Update handler metrics
        {
            let mut handler_metrics = self.handler_metrics.write().unwrap();
            for handler_result in &result.handler_results {
                let metrics = handler_metrics.entry(handler_result.handler_id.clone()).or_default();
                
                metrics.handler_id = handler_result.handler_id.clone();
                metrics.event_type = result.event_type.clone();
                metrics.priority = handler_result.priority.clone();
                metrics.invocation_count += 1;
                
                if handler_result.success {
                    metrics.successful_executions += 1;
                } else {
                    metrics.failed_executions += 1;
                }
                
                // Update timing
                let exec_time = handler_result.execution_time;
                if metrics.invocation_count == 1 {
                    metrics.avg_execution_time = exec_time;
                    metrics.min_execution_time = exec_time;
                    metrics.max_execution_time = exec_time;
                } else {
                    let total_nanos = metrics.avg_execution_time.as_nanos() * (metrics.invocation_count - 1) as u128 
                         + exec_time.as_nanos();
                    let avg_nanos = total_nanos / metrics.invocation_count as u128;
                    metrics.avg_execution_time = Duration::from_nanos(avg_nanos.min(u64::MAX as u128) as u64);
                    if exec_time < metrics.min_execution_time {
                        metrics.min_execution_time = exec_time;
                    }
                    if exec_time > metrics.max_execution_time {
                        metrics.max_execution_time = exec_time;
                    }
                }
                
                metrics.total_execution_time += exec_time;
                metrics.last_execution = Some(token.start_timestamp);
            }
        }
        
        // Update system metrics
        {
            let mut sys_metrics = self.system_metrics.lock().unwrap();
            sys_metrics.total_events_processed += 1;
            sys_metrics.uptime = sys_metrics.start_time.elapsed().unwrap_or(Duration::ZERO);
            
            // Calculate events per second
            let uptime_secs = sys_metrics.uptime.as_secs_f64();
            if uptime_secs > 0.0 {
                sys_metrics.events_per_second = sys_metrics.total_events_processed as f64 / uptime_secs;
                if sys_metrics.events_per_second > sys_metrics.peak_events_per_second {
                    sys_metrics.peak_events_per_second = sys_metrics.events_per_second;
                }
            }
        }
    }
    
    /// Records handler registration.
    pub fn record_handler_registration(&self, _handler_id: String) {
        if !self.enabled {
            return;
        }
        
        let mut sys_metrics = self.system_metrics.lock().unwrap();
        sys_metrics.total_handlers_registered += 1;
    }
    
    /// Records handler unregistration.
    pub fn record_handler_unregistration(&self, _handler_id: String) {
        if !self.enabled {
            return;
        }
        
        let mut sys_metrics = self.system_metrics.lock().unwrap();
        if sys_metrics.total_handlers_registered > 0 {
            sys_metrics.total_handlers_registered -= 1;
        }
    }
    
    /// Gets emission statistics for a specific event type.
    pub fn get_emission_stats(&self, event_type: TypeId) -> Option<EmissionStats> {
        if !self.enabled {
            return None;
        }
        
        let stats = self.emission_stats.read().unwrap();
        stats.get(&event_type).cloned()
    }
    
    /// Gets all emission statistics.
    pub fn get_all_emission_stats(&self) -> HashMap<TypeId, EmissionStats> {
        if !self.enabled {
            return HashMap::new();
        }
        
        self.emission_stats.read().unwrap().clone()
    }
    
    /// Gets handler metrics for a specific handler.
    pub fn get_handler_metrics(&self, handler_id: &str) -> Option<HandlerMetrics> {
        if !self.enabled {
            return None;
        }
        
        let metrics = self.handler_metrics.read().unwrap();
        metrics.get(handler_id).cloned()
    }
    
    /// Gets all handler metrics.
    pub fn get_all_handler_metrics(&self) -> HashMap<String, HandlerMetrics> {
        if !self.enabled {
            return HashMap::new();
        }
        
        self.handler_metrics.read().unwrap().clone()
    }
    
    /// Gets current system metrics.
    pub fn get_system_metrics(&self) -> SystemMetrics {
        if !self.enabled {
            return SystemMetrics::default();
        }
        
        let mut sys_metrics = self.system_metrics.lock().unwrap();
        sys_metrics.uptime = sys_metrics.start_time.elapsed().unwrap_or(Duration::ZERO);
        sys_metrics.clone()
    }
    
    /// Resets all metrics.
    pub fn reset(&self) {
        if !self.enabled {
            return;
        }
        
        {
            let mut stats = self.emission_stats.write().unwrap();
            stats.clear();
        }
        
        {
            let mut metrics = self.handler_metrics.write().unwrap();
            metrics.clear();
        }
        
        {
            let mut sys_metrics = self.system_metrics.lock().unwrap();
            *sys_metrics = SystemMetrics {
                start_time: SystemTime::now(),
                last_reset: Some(SystemTime::now()),
                ..Default::default()
            };
        }
    }
    
    /// Generates a summary report of all metrics.
    pub fn generate_report(&self) -> MetricsReport {
        MetricsReport {
            system_metrics: self.get_system_metrics(),
            emission_stats: self.get_all_emission_stats(),
            handler_metrics: self.get_all_handler_metrics(),
            report_timestamp: SystemTime::now(),
        }
    }
}

/// Token returned when starting an emission, used to track timing.
#[derive(Debug)]
pub struct EmissionToken {
    event_type: TypeId,
    start_time: Instant,
    start_timestamp: SystemTime,
    enabled: bool,
}

impl EmissionToken {
    fn disabled() -> Self {
        Self {
            event_type: TypeId::of::<()>(),
            start_time: Instant::now(),
            start_timestamp: UNIX_EPOCH,
            enabled: false,
        }
    }
}

/// Comprehensive metrics report.
#[derive(Debug, Clone)]
pub struct MetricsReport {
    /// System-wide metrics
    pub system_metrics: SystemMetrics,
    /// Emission statistics by event type
    pub emission_stats: HashMap<TypeId, EmissionStats>,
    /// Handler metrics by handler ID
    pub handler_metrics: HashMap<String, HandlerMetrics>,
    /// Timestamp when report was generated
    pub report_timestamp: SystemTime,
}

impl EmissionResult {
    /// Creates a new successful EmissionResult.
    pub fn success(
        total_duration: Duration,
        handlers_executed: usize,
        handler_results: Vec<HandlerResult>,
        event_type: String,
        used_priority_ordering: bool,
    ) -> Self {
        let handlers_succeeded = handler_results.iter().filter(|r| r.success).count();
        let handlers_failed = handlers_executed - handlers_succeeded;
        
        Self {
            success: true,
            total_duration,
            handlers_executed,
            handlers_succeeded,
            handlers_failed,
            handler_results,
            errors: Vec::new(),
            emission_start: SystemTime::now() - total_duration,
            emission_end: SystemTime::now(),
            event_type,
            used_priority_ordering,
            middleware_metrics: None,
        }
    }
    
    /// Creates a new failed EmissionResult.
    pub fn failure(
        total_duration: Duration,
        error: String,
        event_type: String,
    ) -> Self {
        Self {
            success: false,
            total_duration,
            handlers_executed: 0,
            handlers_succeeded: 0,
            handlers_failed: 0,
            handler_results: Vec::new(),
            errors: vec![error],
            emission_start: SystemTime::now() - total_duration,
            emission_end: SystemTime::now(),
            event_type,
            used_priority_ordering: false,
            middleware_metrics: None,
        }
    }
    
    /// Returns the success rate of handler executions.
    pub fn success_rate(&self) -> f64 {
        if self.handlers_executed == 0 {
            return 0.0;
        }
        self.handlers_succeeded as f64 / self.handlers_executed as f64
    }
    
    /// Returns the average handler execution time.
    pub fn avg_handler_execution_time(&self) -> Duration {
        if self.handler_results.is_empty() {
            return Duration::ZERO;
        }
        
        let total_nanos: u128 = self.handler_results
            .iter()
            .map(|r| r.execution_time.as_nanos())
            .sum();
        
        Duration::from_nanos((total_nanos / self.handler_results.len() as u128) as u64)
    }
}

impl HandlerResult {
    /// Creates a new successful HandlerResult.
    pub fn success(
        handler_id: String,
        execution_time: Duration,
        priority: String,
    ) -> Self {
        Self {
            handler_id,
            success: true,
            execution_time,
            priority,
            error: None,
            skipped_by_filter: false,
        }
    }
    
    /// Creates a new failed HandlerResult.
    pub fn failure(
        handler_id: String,
        execution_time: Duration,
        priority: String,
        error: String,
    ) -> Self {
        Self {
            handler_id,
            success: false,
            execution_time,
            priority,
            error: Some(error),
            skipped_by_filter: false,
        }
    }
    
    /// Creates a new HandlerResult for a handler skipped by filtering.
    pub fn skipped(
        handler_id: String,
        priority: String,
    ) -> Self {
        Self {
            handler_id,
            success: true,
            execution_time: Duration::ZERO,
            priority,
            error: None,
            skipped_by_filter: true,
        }
    }
}