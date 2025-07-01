//! Event filtering system for EventRS.
//!
//! This module provides the filtering infrastructure for events, allowing
//! handlers to be executed conditionally based on event properties.
//! Filters can be composed using logical operations and applied to both
//! synchronous and asynchronous event buses.

use crate::event::Event;
use std::fmt::Debug;
use std::sync::Arc;

/// Trait for event filters.
/// 
/// Filters determine whether an event should be processed by handlers.
/// They can be combined using logical operations (AND, OR, NOT) and
/// provide a powerful mechanism for selective event processing.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{Filter, Event};
/// 
/// #[derive(Clone, Debug)]
/// struct UserEvent { user_id: u64, is_admin: bool }
/// impl Event for UserEvent {}
/// 
/// struct AdminFilter;
/// 
/// impl Filter<UserEvent> for AdminFilter {
///     fn evaluate(&self, event: &UserEvent) -> bool {
///         event.is_admin
///     }
/// }
/// 
/// let filter = AdminFilter;
/// let admin_event = UserEvent { user_id: 1, is_admin: true };
/// let user_event = UserEvent { user_id: 2, is_admin: false };
/// 
/// assert!(filter.evaluate(&admin_event));
/// assert!(!filter.evaluate(&user_event));
/// ```
pub trait Filter<E: Event>: Send + Sync + 'static {
    /// Evaluates whether the given event should pass through this filter.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The event to evaluate
    /// 
    /// # Returns
    /// 
    /// Returns `true` if the event should be processed, `false` otherwise.
    fn evaluate(&self, event: &E) -> bool;
    
    /// Returns the name of this filter for debugging and logging.
    /// 
    /// The default implementation uses the type name.
    fn filter_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    
    /// Returns whether this filter is expensive to evaluate.
    /// 
    /// Expensive filters might involve I/O operations, complex computations,
    /// or external service calls. The event bus can use this information
    /// to optimize filter evaluation order.
    fn is_expensive(&self) -> bool {
        false
    }
    
    /// Returns a description of what this filter does.
    /// 
    /// This is useful for debugging and monitoring filter chains.
    fn description(&self) -> String {
        format!("Filter: {}", self.filter_name())
    }
    
    /// Combines this filter with another using logical AND.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{Filter, Event};
    /// 
    /// #[derive(Clone)]
    /// struct TestEvent { value: i32, flag: bool }
    /// impl Event for TestEvent {}
    /// 
    /// struct ValueFilter(i32);
    /// impl Filter<TestEvent> for ValueFilter {
    ///     fn evaluate(&self, event: &TestEvent) -> bool {
    ///         event.value > self.0
    ///     }
    /// }
    /// 
    /// struct FlagFilter;
    /// impl Filter<TestEvent> for FlagFilter {
    ///     fn evaluate(&self, event: &TestEvent) -> bool {
    ///         event.flag
    ///     }
    /// }
    /// 
    /// let combined = ValueFilter(10).and(FlagFilter);
    /// let event = TestEvent { value: 15, flag: true };
    /// assert!(combined.evaluate(&event));
    /// ```
    fn and<F: Filter<E>>(self, other: F) -> AndFilter<E, Self, F>
    where
        Self: Sized,
    {
        AndFilter::new(self, other)
    }
    
    /// Combines this filter with another using logical OR.
    fn or<F: Filter<E>>(self, other: F) -> OrFilter<E, Self, F>
    where
        Self: Sized,
    {
        OrFilter::new(self, other)
    }
    
    /// Negates this filter using logical NOT.
    fn not(self) -> NotFilter<E, Self>
    where
        Self: Sized,
    {
        NotFilter::new(self)
    }
}

/// A filter that accepts events based on a predicate function.
/// 
/// This is the most flexible filter type, allowing arbitrary logic
/// to be expressed as a closure or function.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{PredicateFilter, Filter, Event};
/// 
/// #[derive(Clone)]
/// struct NumberEvent { value: i32 }
/// impl Event for NumberEvent {}
/// 
/// let even_filter = PredicateFilter::new("even_numbers", |event: &NumberEvent| {
///     event.value % 2 == 0
/// });
/// 
/// let event1 = NumberEvent { value: 4 };
/// let event2 = NumberEvent { value: 5 };
/// 
/// assert!(even_filter.evaluate(&event1));
/// assert!(!even_filter.evaluate(&event2));
/// ```
pub struct PredicateFilter<E: Event, F> {
    name: &'static str,
    predicate: F,
    expensive: bool,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event, F> PredicateFilter<E, F>
where
    F: Fn(&E) -> bool + Send + Sync + 'static,
{
    /// Creates a new predicate filter.
    /// 
    /// # Arguments
    /// 
    /// * `name` - A descriptive name for this filter
    /// * `predicate` - The function that evaluates events
    pub fn new(name: &'static str, predicate: F) -> Self {
        Self {
            name,
            predicate,
            expensive: false,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Creates a new expensive predicate filter.
    /// 
    /// Use this for filters that perform I/O or complex computations.
    pub fn new_expensive(name: &'static str, predicate: F) -> Self {
        Self {
            name,
            predicate,
            expensive: true,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E: Event, F> Filter<E> for PredicateFilter<E, F>
where
    F: Fn(&E) -> bool + Send + Sync + 'static,
{
    fn evaluate(&self, event: &E) -> bool {
        (self.predicate)(event)
    }
    
    fn filter_name(&self) -> &'static str {
        self.name
    }
    
    fn is_expensive(&self) -> bool {
        self.expensive
    }
    
    fn description(&self) -> String {
        format!("PredicateFilter: {}", self.name)
    }
}

/// A filter that always accepts all events.
/// 
/// This is useful as a default filter or for testing.
pub struct AllowAllFilter<E: Event> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event> AllowAllFilter<E> {
    /// Creates a new allow-all filter.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E: Event> Default for AllowAllFilter<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Event> Filter<E> for AllowAllFilter<E> {
    fn evaluate(&self, _event: &E) -> bool {
        true
    }
    
    fn filter_name(&self) -> &'static str {
        "AllowAllFilter"
    }
    
    fn description(&self) -> String {
        "Filter: Allow all events".to_string()
    }
}

/// A filter that rejects all events.
/// 
/// This is useful for temporarily disabling event processing.
pub struct RejectAllFilter<E: Event> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event> RejectAllFilter<E> {
    /// Creates a new reject-all filter.
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E: Event> Default for RejectAllFilter<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Event> Filter<E> for RejectAllFilter<E> {
    fn evaluate(&self, _event: &E) -> bool {
        false
    }
    
    fn filter_name(&self) -> &'static str {
        "RejectAllFilter"
    }
    
    fn description(&self) -> String {
        "Filter: Reject all events".to_string()
    }
}

/// A filter that combines two filters with logical AND.
/// 
/// Both filters must evaluate to `true` for the event to pass.
pub struct AndFilter<E: Event, F1: Filter<E>, F2: Filter<E>> {
    filter1: F1,
    filter2: F2,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event, F1: Filter<E>, F2: Filter<E>> AndFilter<E, F1, F2> {
    /// Creates a new AND filter.
    pub fn new(filter1: F1, filter2: F2) -> Self {
        Self {
            filter1,
            filter2,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E: Event, F1: Filter<E>, F2: Filter<E>> Filter<E> for AndFilter<E, F1, F2> {
    fn evaluate(&self, event: &E) -> bool {
        self.filter1.evaluate(event) && self.filter2.evaluate(event)
    }
    
    fn filter_name(&self) -> &'static str {
        "AndFilter"
    }
    
    fn is_expensive(&self) -> bool {
        self.filter1.is_expensive() || self.filter2.is_expensive()
    }
    
    fn description(&self) -> String {
        format!("({}) AND ({})", 
                self.filter1.description(), 
                self.filter2.description())
    }
}

/// A filter that combines two filters with logical OR.
/// 
/// Either filter must evaluate to `true` for the event to pass.
pub struct OrFilter<E: Event, F1: Filter<E>, F2: Filter<E>> {
    filter1: F1,
    filter2: F2,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event, F1: Filter<E>, F2: Filter<E>> OrFilter<E, F1, F2> {
    /// Creates a new OR filter.
    pub fn new(filter1: F1, filter2: F2) -> Self {
        Self {
            filter1,
            filter2,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E: Event, F1: Filter<E>, F2: Filter<E>> Filter<E> for OrFilter<E, F1, F2> {
    fn evaluate(&self, event: &E) -> bool {
        self.filter1.evaluate(event) || self.filter2.evaluate(event)
    }
    
    fn filter_name(&self) -> &'static str {
        "OrFilter"
    }
    
    fn is_expensive(&self) -> bool {
        self.filter1.is_expensive() || self.filter2.is_expensive()
    }
    
    fn description(&self) -> String {
        format!("({}) OR ({})", 
                self.filter1.description(), 
                self.filter2.description())
    }
}

/// A filter that negates another filter with logical NOT.
/// 
/// The wrapped filter must evaluate to `false` for the event to pass.
pub struct NotFilter<E: Event, F: Filter<E>> {
    filter: F,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: Event, F: Filter<E>> NotFilter<E, F> {
    /// Creates a new NOT filter.
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E: Event, F: Filter<E>> Filter<E> for NotFilter<E, F> {
    fn evaluate(&self, event: &E) -> bool {
        !self.filter.evaluate(event)
    }
    
    fn filter_name(&self) -> &'static str {
        "NotFilter"
    }
    
    fn is_expensive(&self) -> bool {
        self.filter.is_expensive()
    }
    
    fn description(&self) -> String {
        format!("NOT ({})", self.filter.description())
    }
}

/// A collection of filters that can be evaluated as a group.
/// 
/// This allows for complex filter chains and provides optimization
/// opportunities such as short-circuiting and reordering.
pub struct FilterChain<E: Event> {
    filters: Vec<Box<dyn Filter<E>>>,
    mode: ChainMode,
}

/// The evaluation mode for a filter chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainMode {
    /// All filters must pass (AND logic)
    All,
    /// At least one filter must pass (OR logic)
    Any,
    /// Custom logic (requires manual evaluation)
    Custom,
}

impl<E: Event> FilterChain<E> {
    /// Creates a new filter chain with the specified mode.
    pub fn new(mode: ChainMode) -> Self {
        Self {
            filters: Vec::new(),
            mode,
        }
    }
    
    /// Creates a new filter chain that requires all filters to pass.
    pub fn all() -> Self {
        Self::new(ChainMode::All)
    }
    
    /// Creates a new filter chain that requires any filter to pass.
    pub fn any() -> Self {
        Self::new(ChainMode::Any)
    }
    
    /// Adds a filter to the chain.
    pub fn add_filter<F: Filter<E>>(mut self, filter: F) -> Self {
        self.filters.push(Box::new(filter));
        self
    }
    
    /// Returns the number of filters in the chain.
    pub fn len(&self) -> usize {
        self.filters.len()
    }
    
    /// Returns true if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
    
    /// Returns the evaluation mode of this chain.
    pub fn mode(&self) -> ChainMode {
        self.mode
    }
    
    /// Evaluates all filters in the chain according to the chain mode.
    pub fn evaluate_chain(&self, event: &E) -> bool {
        match self.mode {
            ChainMode::All => self.filters.iter().all(|f| f.evaluate(event)),
            ChainMode::Any => self.filters.iter().any(|f| f.evaluate(event)),
            ChainMode::Custom => {
                // For custom mode, users should implement their own logic
                // Default to All behavior
                self.filters.iter().all(|f| f.evaluate(event))
            }
        }
    }
    
    /// Optimizes the filter chain by reordering filters.
    /// 
    /// This puts inexpensive filters first to enable short-circuiting.
    pub fn optimize(mut self) -> Self {
        // Sort filters to put inexpensive ones first
        self.filters.sort_by_key(|f| f.is_expensive());
        self
    }
}

impl<E: Event> Filter<E> for FilterChain<E> {
    fn evaluate(&self, event: &E) -> bool {
        self.evaluate_chain(event)
    }
    
    fn filter_name(&self) -> &'static str {
        "FilterChain"
    }
    
    fn is_expensive(&self) -> bool {
        self.filters.iter().any(|f| f.is_expensive())
    }
    
    fn description(&self) -> String {
        let mode_str = match self.mode {
            ChainMode::All => "ALL",
            ChainMode::Any => "ANY",
            ChainMode::Custom => "CUSTOM",
        };
        
        if self.filters.is_empty() {
            format!("FilterChain({}): empty", mode_str)
        } else {
            let filter_descriptions: Vec<String> = self.filters
                .iter()
                .map(|f| f.description())
                .collect();
            format!("FilterChain({}): [{}]", mode_str, filter_descriptions.join(", "))
        }
    }
}

/// Type-erased filter for dynamic filter storage.
pub type BoxedFilter<E> = Box<dyn Filter<E>>;

/// A wrapper that allows filters to be shared across threads.
pub type SharedFilter<E> = Arc<dyn Filter<E>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;

    #[derive(Clone, Debug)]
    struct TestEvent {
        value: i32,
        flag: bool,
        name: String,
    }

    impl Event for TestEvent {
        fn event_type_name() -> &'static str {
            "TestEvent"
        }
    }

    struct ValueFilter(i32);
    
    impl Filter<TestEvent> for ValueFilter {
        fn evaluate(&self, event: &TestEvent) -> bool {
            event.value > self.0
        }
    }

    struct FlagFilter;
    
    impl Filter<TestEvent> for FlagFilter {
        fn evaluate(&self, event: &TestEvent) -> bool {
            event.flag
        }
    }

    #[test]
    fn test_predicate_filter() {
        let filter = PredicateFilter::new("even_values", |event: &TestEvent| {
            event.value % 2 == 0
        });
        
        let event1 = TestEvent { value: 4, flag: true, name: "test".to_string() };
        let event2 = TestEvent { value: 5, flag: true, name: "test".to_string() };
        
        assert!(filter.evaluate(&event1));
        assert!(!filter.evaluate(&event2));
        assert_eq!(filter.filter_name(), "even_values");
        assert!(!filter.is_expensive());
    }

    #[test]
    fn test_allow_all_filter() {
        let filter = AllowAllFilter::new();
        let event = TestEvent { value: 1, flag: false, name: "test".to_string() };
        
        assert!(filter.evaluate(&event));
        assert_eq!(filter.filter_name(), "AllowAllFilter");
    }

    #[test]
    fn test_reject_all_filter() {
        let filter = RejectAllFilter::new();
        let event = TestEvent { value: 1, flag: true, name: "test".to_string() };
        
        assert!(!filter.evaluate(&event));
        assert_eq!(filter.filter_name(), "RejectAllFilter");
    }

    #[test]
    fn test_and_filter() {
        let filter = ValueFilter(5).and(FlagFilter);
        
        let event1 = TestEvent { value: 10, flag: true, name: "test".to_string() };
        let event2 = TestEvent { value: 10, flag: false, name: "test".to_string() };
        let event3 = TestEvent { value: 3, flag: true, name: "test".to_string() };
        
        assert!(filter.evaluate(&event1)); // value > 5 AND flag = true
        assert!(!filter.evaluate(&event2)); // value > 5 AND flag = false
        assert!(!filter.evaluate(&event3)); // value <= 5 AND flag = true
    }

    #[test]
    fn test_or_filter() {
        let filter = ValueFilter(5).or(FlagFilter);
        
        let event1 = TestEvent { value: 10, flag: false, name: "test".to_string() };
        let event2 = TestEvent { value: 3, flag: true, name: "test".to_string() };
        let event3 = TestEvent { value: 3, flag: false, name: "test".to_string() };
        
        assert!(filter.evaluate(&event1)); // value > 5 OR flag = true
        assert!(filter.evaluate(&event2)); // value <= 5 OR flag = true
        assert!(!filter.evaluate(&event3)); // value <= 5 OR flag = false
    }

    #[test]
    fn test_not_filter() {
        let filter = FlagFilter.not();
        
        let event1 = TestEvent { value: 1, flag: true, name: "test".to_string() };
        let event2 = TestEvent { value: 1, flag: false, name: "test".to_string() };
        
        assert!(!filter.evaluate(&event1)); // NOT true = false
        assert!(filter.evaluate(&event2)); // NOT false = true
    }

    #[test]
    fn test_complex_filter_combination() {
        // (value > 5 AND flag = true) OR (value > -1)
        let complex_filter = ValueFilter(5).and(FlagFilter)
                                         .or(ValueFilter(-1));
        
        let event1 = TestEvent { value: 10, flag: true, name: "test".to_string() };
        let event2 = TestEvent { value: 10, flag: false, name: "test".to_string() };
        let event3 = TestEvent { value: -5, flag: false, name: "test".to_string() };
        let event4 = TestEvent { value: 3, flag: false, name: "test".to_string() };
        
        assert!(complex_filter.evaluate(&event1)); // (10 > 5 AND true) OR (10 > -1) = true OR true = true
        assert!(complex_filter.evaluate(&event2)); // (10 > 5 AND false) OR (10 > -1) = false OR true = true
        assert!(!complex_filter.evaluate(&event3)); // (-5 > 5 AND false) OR (-5 > -1) = false OR false = false
        assert!(complex_filter.evaluate(&event4)); // (3 > 5 AND false) OR (3 > -1) = false OR true = true
    }

    #[test]
    fn test_filter_chain_all() {
        let chain = FilterChain::all()
            .add_filter(ValueFilter(5))
            .add_filter(FlagFilter);
        
        let event1 = TestEvent { value: 10, flag: true, name: "test".to_string() };
        let event2 = TestEvent { value: 10, flag: false, name: "test".to_string() };
        
        assert!(chain.evaluate(&event1));
        assert!(!chain.evaluate(&event2));
        assert_eq!(chain.mode(), ChainMode::All);
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_filter_chain_any() {
        let chain = FilterChain::any()
            .add_filter(ValueFilter(15))
            .add_filter(FlagFilter);
        
        let event1 = TestEvent { value: 10, flag: true, name: "test".to_string() };
        let event2 = TestEvent { value: 20, flag: false, name: "test".to_string() };
        let event3 = TestEvent { value: 5, flag: false, name: "test".to_string() };
        
        assert!(chain.evaluate(&event1)); // flag is true
        assert!(chain.evaluate(&event2)); // value > 15
        assert!(!chain.evaluate(&event3)); // neither condition met
    }

    #[test]
    fn test_expensive_filter() {
        let expensive_filter = PredicateFilter::new_expensive("complex_calc", |event: &TestEvent| {
            // Simulate expensive operation
            event.value % 7 == 0
        });
        
        assert!(expensive_filter.is_expensive());
        
        let event1 = TestEvent { value: 14, flag: true, name: "test".to_string() };
        let event2 = TestEvent { value: 15, flag: true, name: "test".to_string() };
        
        assert!(expensive_filter.evaluate(&event1));
        assert!(!expensive_filter.evaluate(&event2));
    }

    #[test]
    fn test_filter_descriptions() {
        let value_filter = ValueFilter(10);
        let flag_filter = FlagFilter;
        let and_filter = value_filter.and(flag_filter);
        
        assert!(and_filter.description().contains("AND"));
        
        let predicate_filter = PredicateFilter::new("custom_logic", |_: &TestEvent| true);
        assert_eq!(predicate_filter.description(), "PredicateFilter: custom_logic");
    }

    #[test]
    fn test_empty_filter_chain() {
        let empty_chain = FilterChain::<TestEvent>::all();
        let event = TestEvent { value: 1, flag: true, name: "test".to_string() };
        
        assert!(empty_chain.evaluate(&event)); // Empty chain in ALL mode passes everything
        assert!(empty_chain.is_empty());
        assert_eq!(empty_chain.len(), 0);
    }
}