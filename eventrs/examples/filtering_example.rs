//! Filtering system example demonstrating event filtering capabilities.

use eventrs::prelude::*;
use eventrs::{AllowAllFilter, RejectAllFilter};
use std::time::SystemTime;

// Define events for the example
#[derive(Event, Clone, Debug)]
struct UserActionEvent {
    user_id: u64,
    action: String,
    is_admin: bool,
    severity: u8,
    timestamp: SystemTime,
}

#[derive(Event, Clone, Debug)]
struct SystemEvent {
    component: String,
    level: String,
    message: String,
    error_code: Option<u32>,
}

#[derive(Event, Clone, Debug)]
struct SecurityEvent {
    threat_level: u8,
    source_ip: String,
    blocked: bool,
    details: String,
}

// Custom filter implementations
struct AdminFilter;

impl Filter<UserActionEvent> for AdminFilter {
    fn evaluate(&self, event: &UserActionEvent) -> bool {
        event.is_admin
    }
    
    fn filter_name(&self) -> &'static str {
        "AdminFilter"
    }
    
    fn description(&self) -> String {
        "Filter: Admin users only".to_string()
    }
}

struct SeverityFilter(u8);

impl Filter<UserActionEvent> for SeverityFilter {
    fn evaluate(&self, event: &UserActionEvent) -> bool {
        event.severity >= self.0
    }
    
    fn filter_name(&self) -> &'static str {
        "SeverityFilter"
    }
    
    fn description(&self) -> String {
        format!("Filter: Severity >= {}", self.0)
    }
}

struct ThreatLevelFilter {
    min_level: u8,
    only_blocked: bool,
}

impl Filter<SecurityEvent> for ThreatLevelFilter {
    fn evaluate(&self, event: &SecurityEvent) -> bool {
        event.threat_level >= self.min_level && (!self.only_blocked || event.blocked)
    }
    
    fn filter_name(&self) -> &'static str {
        "ThreatLevelFilter"
    }
    
    fn is_expensive(&self) -> bool {
        true // Simulating expensive security analysis
    }
    
    fn description(&self) -> String {
        format!("Filter: Threat level >= {}, blocked: {}", self.min_level, self.only_blocked)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 EventRS Filtering System Example");
    println!();

    let mut bus = EventBus::new();

    // Example 1: Simple predicate filters
    println!("📋 Example 1: Simple Predicate Filters");
    
    // Filter for high-priority user actions
    let high_priority_filter = PredicateFilter::new("high_priority", |event: &UserActionEvent| {
        event.severity >= 8
    });
    
    bus.on(|event: UserActionEvent| {
        println!("🚨 High priority action: {} by user {}", event.action, event.user_id);
    });

    // Emit test events
    bus.emit(UserActionEvent {
        user_id: 1,
        action: "delete_database".to_string(),
        is_admin: true,
        severity: 9,
        timestamp: SystemTime::now(),
    })?;

    bus.emit(UserActionEvent {
        user_id: 2,
        action: "view_page".to_string(),
        is_admin: false,
        severity: 2,
        timestamp: SystemTime::now(),
    })?;

    println!();

    // Example 2: Complex filter combinations
    println!("📋 Example 2: Complex Filter Combinations");
    
    // Filter for admin actions OR high-severity actions
    let admin_or_critical = AdminFilter.or(SeverityFilter(8));
    
    println!("Filter description: {}", admin_or_critical.description());
    
    let event1 = UserActionEvent {
        user_id: 3,
        action: "configure_system".to_string(),
        is_admin: true,
        severity: 5,
        timestamp: SystemTime::now(),
    };
    
    let event2 = UserActionEvent {
        user_id: 4,
        action: "emergency_shutdown".to_string(),
        is_admin: false,
        severity: 10,
        timestamp: SystemTime::now(),
    };
    
    let event3 = UserActionEvent {
        user_id: 5,
        action: "read_file".to_string(),
        is_admin: false,
        severity: 3,
        timestamp: SystemTime::now(),
    };

    println!("Event 1 (admin, low severity): {}", admin_or_critical.evaluate(&event1));
    println!("Event 2 (non-admin, high severity): {}", admin_or_critical.evaluate(&event2));
    println!("Event 3 (non-admin, low severity): {}", admin_or_critical.evaluate(&event3));
    println!();

    // Example 3: Filter chains
    println!("📋 Example 3: Filter Chains");
    
    // Create a filter chain for security events
    let security_chain = FilterChain::all()
        .add_filter(ThreatLevelFilter { min_level: 5, only_blocked: false })
        .add_filter(PredicateFilter::new("internal_source", |event: &SecurityEvent| {
            event.source_ip.starts_with("192.168.")
        }))
        .optimize(); // Optimize by putting inexpensive filters first

    println!("Security filter chain: {}", security_chain.description());
    println!("Chain length: {}", security_chain.len());
    println!("Is expensive: {}", security_chain.is_expensive());
    
    let security_event1 = SecurityEvent {
        threat_level: 7,
        source_ip: "192.168.1.100".to_string(),
        blocked: true,
        details: "Suspicious file access".to_string(),
    };
    
    let security_event2 = SecurityEvent {
        threat_level: 3,
        source_ip: "192.168.1.50".to_string(),
        blocked: false,
        details: "Normal activity".to_string(),
    };
    
    let security_event3 = SecurityEvent {
        threat_level: 8,
        source_ip: "10.0.0.5".to_string(),
        blocked: true,
        details: "External threat detected".to_string(),
    };

    println!("Security Event 1 (high threat, internal): {}", security_chain.evaluate(&security_event1));
    println!("Security Event 2 (low threat, internal): {}", security_chain.evaluate(&security_event2));
    println!("Security Event 3 (high threat, external): {}", security_chain.evaluate(&security_event3));
    println!();

    // Example 4: NOT filter
    println!("📋 Example 4: NOT Filter");
    
    let non_admin_filter = AdminFilter.not();
    println!("Non-admin filter description: {}", non_admin_filter.description());
    
    println!("Admin event passes non-admin filter: {}", non_admin_filter.evaluate(&event1));
    println!("Non-admin event passes non-admin filter: {}", non_admin_filter.evaluate(&event2));
    println!();

    // Example 5: Different filter chain modes
    println!("📋 Example 5: Filter Chain Modes");
    
    // ANY mode - at least one filter must pass
    let any_chain = FilterChain::any()
        .add_filter(AdminFilter)
        .add_filter(SeverityFilter(9));
    
    println!("ANY chain description: {}", any_chain.description());
    
    let test_event = UserActionEvent {
        user_id: 6,
        action: "moderate_action".to_string(),
        is_admin: true,
        severity: 5,
        timestamp: SystemTime::now(),
    };
    
    println!("Test event passes ANY chain: {}", any_chain.evaluate(&test_event));
    
    // ALL mode - all filters must pass
    let all_chain = FilterChain::all()
        .add_filter(AdminFilter)
        .add_filter(SeverityFilter(9));
    
    println!("ALL chain description: {}", all_chain.description());
    println!("Test event passes ALL chain: {}", all_chain.evaluate(&test_event));
    println!();

    // Example 6: Expensive filters and optimization
    println!("📋 Example 6: Expensive Filters");
    
    let expensive_filter = PredicateFilter::new_expensive("complex_analysis", |event: &UserActionEvent| {
        // Simulate expensive computation
        std::thread::sleep(std::time::Duration::from_millis(1));
        event.action.len() > 10
    });
    
    println!("Expensive filter: {}", expensive_filter.filter_name());
    println!("Is expensive: {}", expensive_filter.is_expensive());
    
    // Create an optimized chain that puts inexpensive filters first
    let optimized_chain = FilterChain::all()
        .add_filter(expensive_filter)
        .add_filter(AdminFilter) // This is inexpensive
        .optimize();
    
    println!("Optimized chain will evaluate AdminFilter first for short-circuiting");
    
    let long_action_event = UserActionEvent {
        user_id: 7,
        action: "perform_complex_database_migration".to_string(),
        is_admin: false, // This will cause short-circuit
        severity: 5,
        timestamp: SystemTime::now(),
    };
    
    println!("Long action by non-admin passes optimized chain: {}", 
             optimized_chain.evaluate(&long_action_event));
    println!();

    // Example 7: Built-in utility filters
    println!("📋 Example 7: Built-in Utility Filters");
    
    let allow_all = AllowAllFilter::<UserActionEvent>::new();
    let reject_all = RejectAllFilter::<UserActionEvent>::new();
    
    println!("Allow-all filter: {}", allow_all.description());
    println!("Reject-all filter: {}", reject_all.description());
    
    println!("Any event passes allow-all: {}", allow_all.evaluate(&test_event));
    println!("Any event passes reject-all: {}", reject_all.evaluate(&test_event));
    println!();

    // Example 8: Dynamic filter composition using FilterChain
    println!("📋 Example 8: Dynamic Filter Composition");
    
    // Simulate runtime filter configuration
    let user_is_admin = true;
    let min_severity = 6;
    
    let dynamic_filter = if user_is_admin {
        // Admins see all events
        FilterChain::all()
            .add_filter(AllowAllFilter::<UserActionEvent>::new())
    } else {
        // Regular users only see events below certain severity
        FilterChain::all()
            .add_filter(SeverityFilter(min_severity).not())
    };
    
    println!("Dynamic filter created based on user role");
    println!("Filter description: {}", dynamic_filter.description());
    
    println!();
    println!("✅ Filtering example completed successfully!");
    println!("🔍 EventRS provides powerful, composable filtering capabilities");
    println!("📊 Filters support optimization, chaining, and complex logical operations");

    Ok(())
}