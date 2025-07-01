//! Example demonstrating Phase 2.3 functionality: HandlerGroup and PriorityChain
//!
//! This example shows how to organize handlers into groups with different priorities
//! and manage them using priority chains for complex event processing workflows.

use eventrs::prelude::*;

#[derive(Clone, Debug)]
struct OrderEvent {
    order_id: u64,
    amount: f64,
    customer_id: u64,
}

impl Event for OrderEvent {
    fn event_type_name() -> &'static str {
        "OrderEvent"
    }
}

fn main() {
    println!("=== EventRS Phase 2.3: HandlerGroup and PriorityChain Example ===\n");

    // Demonstrate HandlerGroup functionality
    demonstrate_handler_groups();
    
    // Demonstrate PriorityChain functionality  
    demonstrate_priority_chain();

    // Demonstrate practical usage patterns
    demonstrate_practical_usage();
}

fn demonstrate_handler_groups() {
    println!("1. HandlerGroup Demonstration");
    println!("------------------------------");

    // Create different priority groups
    let mut auth_group = HandlerGroup::with_name(Priority::Critical, "authentication");
    auth_group.add_handler("auth_validator".to_string());
    auth_group.add_handler("permission_checker".to_string());

    let mut business_group = HandlerGroup::with_name(Priority::Normal, "business_logic");
    business_group.add_handler("inventory_checker".to_string());
    business_group.add_handler("pricing_calculator".to_string());
    business_group.add_handler("tax_calculator".to_string());

    let mut logging_group = HandlerGroup::with_name(Priority::Low, "logging");
    logging_group.add_handler("audit_logger".to_string());
    logging_group.add_handler("metrics_collector".to_string());

    println!("Created {} groups:", 3);
    println!("  • {} ({}): {} handlers", 
             auth_group.name().unwrap(), 
             auth_group.priority(),
             auth_group.handler_count());
    
    println!("  • {} ({}): {} handlers", 
             business_group.name().unwrap(), 
             business_group.priority(),
             business_group.handler_count());
    
    println!("  • {} ({}): {} handlers", 
             logging_group.name().unwrap(), 
             logging_group.priority(),
             logging_group.handler_count());

    // Demonstrate group management
    println!("\nGroup management operations:");
    println!("  • Auth group contains 'auth_validator': {}", 
             auth_group.contains_handler("auth_validator"));
    
    // Temporarily disable logging for performance
    logging_group.disable();
    println!("  • Logging group enabled: {}", logging_group.is_enabled());
    
    // Re-enable logging
    logging_group.enable();
    println!("  • Logging group re-enabled: {}", logging_group.is_enabled());

    println!();
}

fn demonstrate_priority_chain() {
    println!("2. PriorityChain Demonstration");
    println!("-------------------------------");

    let mut chain = PriorityChain::with_name("order_processing_chain");

    // Create handler groups (same as above)
    let mut auth_group = HandlerGroup::with_name(Priority::Critical, "authentication");
    auth_group.add_handler("auth_validator".to_string());
    auth_group.add_handler("permission_checker".to_string());

    let mut validation_group = HandlerGroup::with_name(Priority::High, "validation");
    validation_group.add_handler("schema_validator".to_string());
    validation_group.add_handler("business_rules_validator".to_string());

    let mut business_group = HandlerGroup::with_name(Priority::Normal, "business_logic");
    business_group.add_handler("inventory_checker".to_string());
    business_group.add_handler("pricing_calculator".to_string());

    let mut notification_group = HandlerGroup::with_name(Priority::Low, "notifications");
    notification_group.add_handler("email_notifier".to_string());
    notification_group.add_handler("sms_notifier".to_string());

    // Add groups to chain (order doesn't matter - they'll be sorted by priority)
    chain.add_group(business_group);      // Normal priority
    chain.add_group(notification_group); // Low priority  
    chain.add_group(auth_group);         // Critical priority
    chain.add_group(validation_group);   // High priority

    println!("Created chain '{}' with {} groups", 
             chain.name().unwrap(), 
             chain.group_count());
    
    println!("Total handlers across all groups: {}", chain.total_handler_count());

    // Show automatic priority ordering
    println!("\nExecution order (highest to lowest priority):");
    for (i, group) in chain.groups().iter().enumerate() {
        println!("  {}. {} ({}) - {} handlers", 
                 i + 1,
                 group.name().unwrap_or("unnamed"),
                 group.priority(),
                 group.handler_count());
    }

    println!("  Highest priority: {:?}", chain.highest_priority());
    println!("  Lowest priority: {:?}", chain.lowest_priority());

    // Demonstrate finding operations
    println!("\nFinding operations:");
    if let Some(group) = chain.find_group_by_name("authentication") {
        println!("  • Found authentication group with priority: {}", group.priority());
    }

    if let Some(group) = chain.find_group_by_priority(Priority::Normal) {
        println!("  • Found Normal priority group: {}", 
                 group.name().unwrap_or("unnamed"));
    }

    println!();
}

fn demonstrate_practical_usage() {
    println!("3. Practical Usage Pattern");
    println!("---------------------------");

    // Create a comprehensive order processing chain
    let mut order_chain = PriorityChain::with_name("comprehensive_order_processing");

    // Security and authentication (Critical - must run first)
    let mut security_group = HandlerGroup::with_name(Priority::Critical, "security");
    security_group.add_handler("authentication".to_string());
    security_group.add_handler("authorization".to_string());
    security_group.add_handler("rate_limiting".to_string());

    // Input validation (High - must run before business logic)
    let mut validation_group = HandlerGroup::with_name(Priority::High, "validation");
    validation_group.add_handler("request_validator".to_string());
    validation_group.add_handler("data_sanitizer".to_string());

    // Core business logic (Normal - main processing)
    let mut core_group = HandlerGroup::with_name(Priority::Normal, "core_business");
    core_group.add_handler("inventory_manager".to_string());
    core_group.add_handler("payment_processor".to_string());
    core_group.add_handler("order_fulfillment".to_string());

    // Notifications and logging (Low - can run last)
    let mut notification_group = HandlerGroup::with_name(Priority::Low, "notifications");
    notification_group.add_handler("customer_notification".to_string());
    notification_group.add_handler("internal_alerts".to_string());
    notification_group.add_handler("audit_logging".to_string());

    // Custom priority for special processing
    let mut special_group = HandlerGroup::with_name(Priority::from_value(600), "special_processing");
    special_group.add_handler("fraud_detection".to_string());
    special_group.add_handler("compliance_check".to_string());

    // Add all groups to the chain
    order_chain.add_group(security_group);
    order_chain.add_group(validation_group);
    order_chain.add_group(core_group);
    order_chain.add_group(notification_group);
    order_chain.add_group(special_group);

    println!("Created comprehensive order processing chain:");
    println!("  Total groups: {}", order_chain.group_count());
    println!("  Total handlers: {}", order_chain.total_handler_count());

    // Show the complete execution order
    println!("\nComplete execution order:");
    for (i, group) in order_chain.groups().iter().enumerate() {
        let handlers: Vec<&String> = group.handler_ids().iter().collect();
        println!("  {}. {} ({}):", 
                 i + 1,
                 group.name().unwrap_or("unnamed"),
                 group.priority());
        for handler in handlers {
            println!("     → {}", handler);
        }
    }

    // Demonstrate enabled groups filtering
    println!("\nEnabled groups only:");
    let enabled_count = order_chain.enabled_groups().count();
    println!("  Enabled groups: {}/{}", enabled_count, order_chain.group_count());

    // Show all handler IDs in execution order
    println!("\nAll handler IDs in priority order:");
    for (i, handler_id) in order_chain.all_handler_ids().enumerate() {
        println!("  {}. {}", i + 1, handler_id);
    }

    println!("\n=== Phase 2.3 demonstration complete! ===");
}