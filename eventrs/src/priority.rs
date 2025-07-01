//! Priority system for events and handlers.
//!
//! This module provides a flexible priority system that allows controlling
//! the execution order of event handlers. Higher priority handlers are
//! executed before lower priority ones.

use std::cmp::Ordering;

/// Priority levels for events and handlers.
/// 
/// Priorities determine the order in which handlers are executed when
/// multiple handlers are registered for the same event type. Higher
/// priority handlers run before lower priority ones.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{Priority, PriorityValue};
/// 
/// let high = Priority::High;
/// let normal = Priority::Normal;
/// let custom = Priority::Custom(PriorityValue::new(600));
/// 
/// assert!(high > normal);
/// assert!(custom > normal);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// Critical priority - highest precedence (value: 1000).
    /// 
    /// Use for system-critical events that must be processed first,
    /// such as shutdown signals or error conditions.
    Critical,
    
    /// High priority (value: 750).
    /// 
    /// Use for important events that should be processed quickly,
    /// such as user actions or time-sensitive operations.
    High,
    
    /// Normal priority (value: 500) - default.
    /// 
    /// Use for regular application events that don't require
    /// special ordering considerations.
    Normal,
    
    /// Low priority (value: 250).
    /// 
    /// Use for background operations, cleanup tasks, or
    /// non-critical notifications.
    Low,
    
    /// Custom priority with explicit numeric value.
    /// 
    /// Allows fine-grained control over execution order.
    /// Higher values indicate higher priority.
    Custom(PriorityValue),
}

/// Custom priority value with a numeric representation.
/// 
/// Priority values are unsigned 16-bit integers where higher values
/// indicate higher priority. This allows for 65,536 distinct priority
/// levels, providing fine-grained control over execution order.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::PriorityValue;
/// 
/// let low = PriorityValue::new(100);
/// let medium = PriorityValue::new(500);
/// let high = PriorityValue::new(900);
/// 
/// assert!(high > medium);
/// assert!(medium > low);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriorityValue(u16);

impl PriorityValue {
    /// Creates a new priority value.
    /// 
    /// Higher values indicate higher priority. The value 0 is valid
    /// and represents the lowest possible priority.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::PriorityValue;
    /// 
    /// let lowest = PriorityValue::new(0);
    /// let highest = PriorityValue::new(u16::MAX);
    /// ```
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    
    /// Returns the numeric value of this priority.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::PriorityValue;
    /// 
    /// let priority = PriorityValue::new(42);
    /// assert_eq!(priority.value(), 42);
    /// ```
    pub const fn value(self) -> u16 {
        self.0
    }
    
    /// Creates a priority value representing the lowest priority.
    pub const fn min() -> Self {
        Self(u16::MIN)
    }
    
    /// Creates a priority value representing the highest priority.
    pub const fn max() -> Self {
        Self(u16::MAX)
    }
}

impl Priority {
    /// Returns the numeric value of this priority for comparison.
    /// 
    /// Higher values indicate higher priority.
    pub const fn value(self) -> u16 {
        match self {
            Priority::Critical => 1000,
            Priority::High => 750,
            Priority::Normal => 500,
            Priority::Low => 250,
            Priority::Custom(PriorityValue(value)) => value,
        }
    }
    
    /// Creates a custom priority from a numeric value.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::Priority;
    /// 
    /// let custom = Priority::from_value(600);
    /// assert!(custom > Priority::Normal);
    /// assert!(custom < Priority::High);
    /// ```
    pub const fn from_value(value: u16) -> Self {
        Priority::Custom(PriorityValue::new(value))
    }
    
    /// Returns whether this priority is higher than another priority.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::Priority;
    /// 
    /// assert!(Priority::High.is_higher_than(Priority::Normal));
    /// assert!(!Priority::Low.is_higher_than(Priority::High));
    /// ```
    pub const fn is_higher_than(self, other: Priority) -> bool {
        self.value() > other.value()
    }
    
    /// Returns whether this priority is lower than another priority.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::Priority;
    /// 
    /// assert!(Priority::Low.is_lower_than(Priority::Normal));
    /// assert!(!Priority::High.is_lower_than(Priority::Low));
    /// ```
    pub const fn is_lower_than(self, other: Priority) -> bool {
        self.value() < other.value()
    }
    
    /// Returns the name of this priority level as a string.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use eventrs::{Priority, PriorityValue};
    /// 
    /// assert_eq!(Priority::High.name(), "High");
    /// assert_eq!(Priority::Custom(PriorityValue::new(600)).name(), "Custom(600)");
    /// ```
    pub fn name(self) -> String {
        match self {
            Priority::Critical => "Critical".to_string(),
            Priority::High => "High".to_string(),
            Priority::Normal => "Normal".to_string(),
            Priority::Low => "Low".to_string(),
            Priority::Custom(value) => format!("Custom({})", value.value()),
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value().cmp(&other.value())
    }
}

impl PartialOrd for PriorityValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<u16> for Priority {
    fn from(value: u16) -> Self {
        Priority::Custom(PriorityValue::new(value))
    }
}

impl From<PriorityValue> for Priority {
    fn from(value: PriorityValue) -> Self {
        Priority::Custom(value)
    }
}

impl From<Priority> for u16 {
    fn from(priority: Priority) -> Self {
        priority.value()
    }
}

impl From<PriorityValue> for u16 {
    fn from(value: PriorityValue) -> Self {
        value.value()
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::fmt::Display for PriorityValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A priority-based ordering wrapper for use in collections.
/// 
/// This wrapper can be used to store items in priority-ordered collections
/// such as BinaryHeap, where higher priority items should be processed first.
/// 
/// # Examples
/// 
/// ```rust
/// use std::collections::BinaryHeap;
/// use eventrs::{Priority, PriorityOrdered};
/// 
/// let mut heap = BinaryHeap::new();
/// 
/// heap.push(PriorityOrdered::new("low task", Priority::Low));
/// heap.push(PriorityOrdered::new("high task", Priority::High));
/// heap.push(PriorityOrdered::new("normal task", Priority::Normal));
/// 
/// // Higher priority items come out first
/// assert_eq!(heap.pop().unwrap().item(), &"high task");
/// assert_eq!(heap.pop().unwrap().item(), &"normal task");
/// assert_eq!(heap.pop().unwrap().item(), &"low task");
/// ```
#[derive(Debug, Clone)]
pub struct PriorityOrdered<T> {
    item: T,
    priority: Priority,
}

impl<T> PriorityOrdered<T> {
    /// Creates a new priority-ordered item.
    pub fn new(item: T, priority: Priority) -> Self {
        Self { item, priority }
    }
    
    /// Returns a reference to the wrapped item.
    pub fn item(&self) -> &T {
        &self.item
    }
    
    /// Returns the priority of this item.
    pub fn priority(&self) -> Priority {
        self.priority
    }
    
    /// Consumes the wrapper and returns the item and priority.
    pub fn into_parts(self) -> (T, Priority) {
        (self.item, self.priority)
    }
    
    /// Consumes the wrapper and returns just the item.
    pub fn into_item(self) -> T {
        self.item
    }
}

impl<T> PartialEq for PriorityOrdered<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<T> Eq for PriorityOrdered<T> {}

impl<T> PartialOrd for PriorityOrdered<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PriorityOrdered<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Note: BinaryHeap is a max-heap, so we want higher priority items
        // to compare as "greater" so they come out first
        self.priority.cmp(&other.priority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BinaryHeap;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
        
        let custom_high = Priority::Custom(PriorityValue::new(800));
        let custom_low = Priority::Custom(PriorityValue::new(300));
        
        assert!(custom_high > Priority::Normal);
        assert!(custom_low < Priority::Normal);
        assert!(custom_high > custom_low);
    }

    #[test]
    fn test_priority_values() {
        assert_eq!(Priority::Critical.value(), 1000);
        assert_eq!(Priority::High.value(), 750);
        assert_eq!(Priority::Normal.value(), 500);
        assert_eq!(Priority::Low.value(), 250);
        
        let custom = Priority::Custom(PriorityValue::new(600));
        assert_eq!(custom.value(), 600);
    }

    #[test]
    fn test_priority_comparison_methods() {
        assert!(Priority::High.is_higher_than(Priority::Normal));
        assert!(Priority::Low.is_lower_than(Priority::Normal));
        assert!(!Priority::Normal.is_higher_than(Priority::High));
        assert!(!Priority::Normal.is_lower_than(Priority::Low));
    }

    #[test]
    fn test_priority_value() {
        let value = PriorityValue::new(42);
        assert_eq!(value.value(), 42);
        
        assert_eq!(PriorityValue::min().value(), 0);
        assert_eq!(PriorityValue::max().value(), u16::MAX);
    }

    #[test]
    fn test_priority_conversions() {
        let priority: Priority = 600u16.into();
        match priority {
            Priority::Custom(value) => assert_eq!(value.value(), 600),
            _ => panic!("Expected custom priority"),
        }
        
        let value: u16 = Priority::High.into();
        assert_eq!(value, 750);
    }

    #[test]
    fn test_priority_names() {
        assert_eq!(Priority::Critical.name(), "Critical");
        assert_eq!(Priority::High.name(), "High");
        assert_eq!(Priority::Normal.name(), "Normal");
        assert_eq!(Priority::Low.name(), "Low");
        assert_eq!(Priority::Custom(PriorityValue::new(600)).name(), "Custom(600)");
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(format!("{}", Priority::High), "High");
        assert_eq!(format!("{}", PriorityValue::new(123)), "123");
    }

    #[test]
    fn test_priority_ordered() {
        let mut heap = BinaryHeap::new();
        
        heap.push(PriorityOrdered::new("low", Priority::Low));
        heap.push(PriorityOrdered::new("critical", Priority::Critical));
        heap.push(PriorityOrdered::new("normal", Priority::Normal));
        heap.push(PriorityOrdered::new("high", Priority::High));
        
        assert_eq!(heap.pop().unwrap().item(), &"critical");
        assert_eq!(heap.pop().unwrap().item(), &"high");
        assert_eq!(heap.pop().unwrap().item(), &"normal");
        assert_eq!(heap.pop().unwrap().item(), &"low");
    }

    #[test]
    fn test_priority_ordered_methods() {
        let ordered = PriorityOrdered::new("test", Priority::High);
        
        assert_eq!(ordered.item(), &"test");
        assert_eq!(ordered.priority(), Priority::High);
        
        let (item, priority) = ordered.into_parts();
        assert_eq!(item, "test");
        assert_eq!(priority, Priority::High);
    }

    #[test]
    fn test_default_priority() {
        assert_eq!(Priority::default(), Priority::Normal);
    }

    #[test]
    fn test_priority_from_value() {
        let priority = Priority::from_value(600);
        assert_eq!(priority.value(), 600);
        assert!(priority > Priority::Normal);
        assert!(priority < Priority::High);
    }
}