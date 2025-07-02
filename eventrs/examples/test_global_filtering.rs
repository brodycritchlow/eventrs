//! Simple test for global filtering functionality

use eventrs::filter::PredicateAnyFilter;
use eventrs::prelude::*;

#[derive(Clone, Debug)]
struct TestEvent {
    pub value: i32,
}

impl Event for TestEvent {
    fn event_type_name() -> &'static str {
        "TestEvent"
    }
}

fn main() {
    println!("Testing global filtering...");

    let mut bus = EventBus::new();

    // Register a handler
    bus.on(|event: TestEvent| {
        println!("Handler received event with value: {}", event.value);
    });

    // Add a global filter that only allows positive values
    let positive_filter = PredicateAnyFilter::new("positive_values", |event| {
        println!("Filter evaluating event type: {}", event.event_type_name());
        if let Some(test_event) = event.as_any().downcast_ref::<TestEvent>() {
            println!("Filter found TestEvent with value: {}", test_event.value);
            let result = test_event.value > 0;
            println!("Filter result: {}", result);
            result
        } else {
            println!("Filter: Event is not TestEvent, allowing");
            true
        }
    });

    bus.add_global_filter("positive_values", Box::new(positive_filter));

    println!(
        "\\nGlobal filtering enabled: {}",
        bus.is_global_filtering_enabled()
    );
    println!("Global filter count: {}", bus.global_filter_count());

    // Emit events
    println!("\\nEmitting positive event (5):");
    let result1 = bus.emit(TestEvent { value: 5 });
    println!("Emit result: {:?}", result1);

    println!("\\nEmitting negative event (-3):");
    let result2 = bus.emit(TestEvent { value: -3 });
    println!("Emit result: {:?}", result2);

    println!("\\nEmitting positive event (10):");
    let result3 = bus.emit(TestEvent { value: 10 });
    println!("Emit result: {:?}", result3);

    println!("\\nDone!");
}
