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

/// A group of handlers with the same priority that can be executed together.
/// 
/// HandlerGroup allows organizing multiple handlers under a single priority level,
/// providing better organization and batch execution capabilities.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{Priority, HandlerGroup};
/// 
/// let mut group = HandlerGroup::new(Priority::High);
/// group.add_handler("auth_handler".to_string());
/// group.add_handler("validation_handler".to_string());
/// 
/// assert_eq!(group.handler_count(), 2);
/// assert_eq!(group.priority(), Priority::High);
/// ```
#[derive(Debug, Clone)]
pub struct HandlerGroup {
    priority: Priority,
    handler_ids: Vec<String>,
    name: Option<String>,
    enabled: bool,
}

impl HandlerGroup {
    /// Creates a new handler group with the specified priority.
    pub fn new(priority: Priority) -> Self {
        Self {
            priority,
            handler_ids: Vec::new(),
            name: None,
            enabled: true,
        }
    }
    
    /// Creates a new named handler group.
    pub fn with_name<S: Into<String>>(priority: Priority, name: S) -> Self {
        Self {
            priority,
            handler_ids: Vec::new(),
            name: Some(name.into()),
            enabled: true,
        }
    }
    
    /// Returns the priority of this handler group.
    pub fn priority(&self) -> Priority {
        self.priority
    }
    
    /// Sets the priority of this handler group.
    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
    }
    
    /// Returns the name of this handler group, if any.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    
    /// Sets the name of this handler group.
    pub fn set_name<S: Into<String>>(&mut self, name: S) {
        self.name = Some(name.into());
    }
    
    /// Returns whether this handler group is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Enables this handler group.
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disables this handler group.
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Adds a handler to this group.
    pub fn add_handler<S: Into<String>>(&mut self, handler_id: S) {
        self.handler_ids.push(handler_id.into());
    }
    
    /// Removes a handler from this group.
    pub fn remove_handler(&mut self, handler_id: &str) -> bool {
        if let Some(pos) = self.handler_ids.iter().position(|id| id == handler_id) {
            self.handler_ids.remove(pos);
            true
        } else {
            false
        }
    }
    
    /// Returns whether this group contains the specified handler.
    pub fn contains_handler(&self, handler_id: &str) -> bool {
        self.handler_ids.iter().any(|id| id == handler_id)
    }
    
    /// Returns the number of handlers in this group.
    pub fn handler_count(&self) -> usize {
        self.handler_ids.len()
    }
    
    /// Returns a slice of all handler IDs in this group.
    pub fn handler_ids(&self) -> &[String] {
        &self.handler_ids
    }
    
    /// Returns whether this group is empty.
    pub fn is_empty(&self) -> bool {
        self.handler_ids.is_empty()
    }
    
    /// Clears all handlers from this group.
    pub fn clear(&mut self) {
        self.handler_ids.clear();
    }
}

impl PartialEq for HandlerGroup {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for HandlerGroup {}

impl PartialOrd for HandlerGroup {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HandlerGroup {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

/// A chain of handler groups ordered by priority.
/// 
/// PriorityChain provides a way to organize multiple HandlerGroups in priority order,
/// allowing for complex execution patterns and group-based handler management.
/// 
/// # Examples
/// 
/// ```rust
/// use eventrs::{Priority, HandlerGroup, PriorityChain};
/// 
/// let mut chain = PriorityChain::new();
/// 
/// let mut auth_group = HandlerGroup::with_name(Priority::Critical, "authentication");
/// auth_group.add_handler("auth_handler".to_string());
/// 
/// let mut business_group = HandlerGroup::with_name(Priority::Normal, "business_logic");
/// business_group.add_handler("validation_handler".to_string());
/// business_group.add_handler("processing_handler".to_string());
/// 
/// chain.add_group(auth_group);
/// chain.add_group(business_group);
/// 
/// assert_eq!(chain.group_count(), 2);
/// // Authentication group will be first due to Critical priority
/// assert_eq!(chain.groups()[0].name(), Some("authentication"));
/// ```
#[derive(Debug, Clone)]
pub struct PriorityChain {
    groups: Vec<HandlerGroup>,
    name: Option<String>,
}

impl PriorityChain {
    /// Creates a new empty priority chain.
    pub fn new() -> Self {
        Self {
            groups: Vec::new(),
            name: None,
        }
    }
    
    /// Creates a new named priority chain.
    pub fn with_name<S: Into<String>>(name: S) -> Self {
        Self {
            groups: Vec::new(),
            name: Some(name.into()),
        }
    }
    
    /// Returns the name of this priority chain, if any.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    
    /// Sets the name of this priority chain.
    pub fn set_name<S: Into<String>>(&mut self, name: S) {
        self.name = Some(name.into());
    }
    
    /// Adds a handler group to this chain.
    /// 
    /// Groups are automatically sorted by priority after insertion.
    pub fn add_group(&mut self, group: HandlerGroup) {
        self.groups.push(group);
        // Sort groups by priority (highest first)
        self.groups.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }
    
    /// Removes a handler group by its priority.
    /// 
    /// If multiple groups have the same priority, only the first one is removed.
    pub fn remove_group_by_priority(&mut self, priority: Priority) -> Option<HandlerGroup> {
        if let Some(pos) = self.groups.iter().position(|g| g.priority() == priority) {
            Some(self.groups.remove(pos))
        } else {
            None
        }
    }
    
    /// Removes a handler group by its name.
    pub fn remove_group_by_name(&mut self, name: &str) -> Option<HandlerGroup> {
        if let Some(pos) = self.groups.iter().position(|g| {
            g.name().map(|n| n == name).unwrap_or(false)
        }) {
            Some(self.groups.remove(pos))
        } else {
            None
        }
    }
    
    /// Returns a reference to all groups in priority order.
    pub fn groups(&self) -> &[HandlerGroup] {
        &self.groups
    }
    
    /// Returns a mutable reference to all groups.
    /// 
    /// Note: After modifying group priorities, call `resort()` to maintain order.
    pub fn groups_mut(&mut self) -> &mut [HandlerGroup] {
        &mut self.groups
    }
    
    /// Resorts groups by priority after manual modifications.
    pub fn resort(&mut self) {
        self.groups.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }
    
    /// Returns the number of groups in this chain.
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
    
    /// Returns the total number of handlers across all groups.
    pub fn total_handler_count(&self) -> usize {
        self.groups.iter().map(|g| g.handler_count()).sum()
    }
    
    /// Returns whether this chain is empty.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
    
    /// Clears all groups from this chain.
    pub fn clear(&mut self) {
        self.groups.clear();
    }
    
    /// Finds a group by its priority.
    pub fn find_group_by_priority(&self, priority: Priority) -> Option<&HandlerGroup> {
        self.groups.iter().find(|g| g.priority() == priority)
    }
    
    /// Finds a group by its name.
    pub fn find_group_by_name(&self, name: &str) -> Option<&HandlerGroup> {
        self.groups.iter().find(|g| {
            g.name().map(|n| n == name).unwrap_or(false)
        })
    }
    
    /// Finds a mutable reference to a group by its priority.
    pub fn find_group_by_priority_mut(&mut self, priority: Priority) -> Option<&mut HandlerGroup> {
        self.groups.iter_mut().find(|g| g.priority() == priority)
    }
    
    /// Finds a mutable reference to a group by its name.
    pub fn find_group_by_name_mut(&mut self, name: &str) -> Option<&mut HandlerGroup> {
        self.groups.iter_mut().find(|g| {
            g.name().map(|n| n == name).unwrap_or(false)
        })
    }
    
    /// Returns an iterator over enabled groups in priority order.
    pub fn enabled_groups(&self) -> impl Iterator<Item = &HandlerGroup> {
        self.groups.iter().filter(|g| g.is_enabled())
    }
    
    /// Returns the highest priority among all groups.
    pub fn highest_priority(&self) -> Option<Priority> {
        self.groups.first().map(|g| g.priority())
    }
    
    /// Returns the lowest priority among all groups.
    pub fn lowest_priority(&self) -> Option<Priority> {
        self.groups.last().map(|g| g.priority())
    }
    
    /// Creates a flattened iterator over all handler IDs in priority order.
    pub fn all_handler_ids(&self) -> impl Iterator<Item = &String> {
        self.groups.iter()
            .filter(|g| g.is_enabled())
            .flat_map(|g| g.handler_ids().iter())
    }
}

impl Default for PriorityChain {
    fn default() -> Self {
        Self::new()
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
    
    #[test]
    fn test_handler_group_creation() {
        let group = HandlerGroup::new(Priority::High);
        assert_eq!(group.priority(), Priority::High);
        assert!(group.is_enabled());
        assert!(group.is_empty());
        assert_eq!(group.handler_count(), 0);
        assert!(group.name().is_none());
        
        let named_group = HandlerGroup::with_name(Priority::Low, "test_group");
        assert_eq!(named_group.name(), Some("test_group"));
        assert_eq!(named_group.priority(), Priority::Low);
    }
    
    #[test]
    fn test_handler_group_management() {
        let mut group = HandlerGroup::new(Priority::Normal);
        
        // Test adding handlers
        group.add_handler("handler1".to_string());
        group.add_handler("handler2".to_string());
        assert_eq!(group.handler_count(), 2);
        assert!(!group.is_empty());
        
        // Test contains handler
        assert!(group.contains_handler("handler1"));
        assert!(group.contains_handler("handler2"));
        assert!(!group.contains_handler("handler3"));
        
        // Test handler IDs
        let ids = group.handler_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"handler1".to_string()));
        assert!(ids.contains(&"handler2".to_string()));
        
        // Test removing handlers
        assert!(group.remove_handler("handler1"));
        assert!(!group.remove_handler("nonexistent"));
        assert_eq!(group.handler_count(), 1);
        assert!(!group.contains_handler("handler1"));
        assert!(group.contains_handler("handler2"));
        
        // Test clear
        group.clear();
        assert!(group.is_empty());
        assert_eq!(group.handler_count(), 0);
    }
    
    #[test]
    fn test_handler_group_properties() {
        let mut group = HandlerGroup::new(Priority::High);
        
        // Test priority modification
        group.set_priority(Priority::Critical);
        assert_eq!(group.priority(), Priority::Critical);
        
        // Test name modification
        group.set_name("new_name");
        assert_eq!(group.name(), Some("new_name"));
        
        // Test enable/disable
        assert!(group.is_enabled());
        group.disable();
        assert!(!group.is_enabled());
        group.enable();
        assert!(group.is_enabled());
    }
    
    #[test]
    fn test_handler_group_ordering() {
        let high_group = HandlerGroup::new(Priority::High);
        let low_group = HandlerGroup::new(Priority::Low);
        let critical_group = HandlerGroup::new(Priority::Critical);
        
        assert!(critical_group > high_group);
        assert!(high_group > low_group);
        assert!(critical_group > low_group);
        
        // Test equality based on priority
        let another_high = HandlerGroup::new(Priority::High);
        assert_eq!(high_group, another_high);
    }
    
    #[test]
    fn test_priority_chain_creation() {
        let chain = PriorityChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.group_count(), 0);
        assert_eq!(chain.total_handler_count(), 0);
        assert!(chain.name().is_none());
        
        let named_chain = PriorityChain::with_name("test_chain");
        assert_eq!(named_chain.name(), Some("test_chain"));
    }
    
    #[test]
    fn test_priority_chain_group_management() {
        let mut chain = PriorityChain::new();
        
        // Create test groups
        let mut high_group = HandlerGroup::with_name(Priority::High, "high_group");
        high_group.add_handler("handler1".to_string());
        high_group.add_handler("handler2".to_string());
        
        let mut low_group = HandlerGroup::with_name(Priority::Low, "low_group");
        low_group.add_handler("handler3".to_string());
        
        let mut critical_group = HandlerGroup::with_name(Priority::Critical, "critical_group");
        critical_group.add_handler("handler4".to_string());
        
        // Add groups in random order
        chain.add_group(low_group);
        chain.add_group(critical_group);
        chain.add_group(high_group);
        
        // Verify automatic sorting by priority
        assert_eq!(chain.group_count(), 3);
        assert_eq!(chain.total_handler_count(), 4);
        
        let groups = chain.groups();
        assert_eq!(groups[0].priority(), Priority::Critical); // Highest first
        assert_eq!(groups[1].priority(), Priority::High);
        assert_eq!(groups[2].priority(), Priority::Low);     // Lowest last
        
        // Test highest and lowest priority
        assert_eq!(chain.highest_priority(), Some(Priority::Critical));
        assert_eq!(chain.lowest_priority(), Some(Priority::Low));
    }
    
    #[test]
    fn test_priority_chain_find_operations() {
        let mut chain = PriorityChain::new();
        
        let mut group1 = HandlerGroup::with_name(Priority::High, "group1");
        group1.add_handler("handler1".to_string());
        
        let mut group2 = HandlerGroup::with_name(Priority::Low, "group2");
        group2.add_handler("handler2".to_string());
        
        chain.add_group(group1);
        chain.add_group(group2);
        
        // Test find by priority
        let found_high = chain.find_group_by_priority(Priority::High);
        assert!(found_high.is_some());
        assert_eq!(found_high.unwrap().name(), Some("group1"));
        
        let found_critical = chain.find_group_by_priority(Priority::Critical);
        assert!(found_critical.is_none());
        
        // Test find by name
        let found_by_name = chain.find_group_by_name("group2");
        assert!(found_by_name.is_some());
        assert_eq!(found_by_name.unwrap().priority(), Priority::Low);
        
        let not_found = chain.find_group_by_name("nonexistent");
        assert!(not_found.is_none());
    }
    
    #[test]
    fn test_priority_chain_removal() {
        let mut chain = PriorityChain::new();
        
        let group1 = HandlerGroup::with_name(Priority::High, "group1");
        let group2 = HandlerGroup::with_name(Priority::Low, "group2");
        
        chain.add_group(group1);
        chain.add_group(group2);
        assert_eq!(chain.group_count(), 2);
        
        // Test remove by priority
        let removed = chain.remove_group_by_priority(Priority::High);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name(), Some("group1"));
        assert_eq!(chain.group_count(), 1);
        
        // Test remove by name
        let removed = chain.remove_group_by_name("group2");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().priority(), Priority::Low);
        assert!(chain.is_empty());
        
        // Test remove non-existent
        let not_removed = chain.remove_group_by_priority(Priority::Critical);
        assert!(not_removed.is_none());
    }
    
    #[test]
    fn test_priority_chain_enabled_groups() {
        let mut chain = PriorityChain::new();
        
        let mut group1 = HandlerGroup::new(Priority::High);
        group1.add_handler("handler1".to_string());
        
        let mut group2 = HandlerGroup::new(Priority::Low);
        group2.add_handler("handler2".to_string());
        group2.disable(); // Disable this group
        
        let mut group3 = HandlerGroup::new(Priority::Normal);
        group3.add_handler("handler3".to_string());
        
        chain.add_group(group1);
        chain.add_group(group2);
        chain.add_group(group3);
        
        // Test enabled groups iterator
        let enabled: Vec<_> = chain.enabled_groups().collect();
        assert_eq!(enabled.len(), 2); // Only group1 and group3 should be enabled
        
        // Verify the disabled group is not included
        assert!(enabled.iter().all(|g| g.is_enabled()));
        assert!(enabled.iter().any(|g| g.priority() == Priority::High));
        assert!(enabled.iter().any(|g| g.priority() == Priority::Normal));
        assert!(!enabled.iter().any(|g| g.priority() == Priority::Low));
    }
    
    #[test]
    fn test_priority_chain_all_handler_ids() {
        let mut chain = PriorityChain::new();
        
        let mut group1 = HandlerGroup::new(Priority::High);
        group1.add_handler("handler1".to_string());
        group1.add_handler("handler2".to_string());
        
        let mut group2 = HandlerGroup::new(Priority::Low);
        group2.add_handler("handler3".to_string());
        group2.disable(); // This group's handlers should not be included
        
        let mut group3 = HandlerGroup::new(Priority::Normal);
        group3.add_handler("handler4".to_string());
        
        chain.add_group(group1);
        chain.add_group(group2);
        chain.add_group(group3);
        
        // Collect all handler IDs from enabled groups
        let all_ids: Vec<_> = chain.all_handler_ids().collect();
        
        // Should have 3 handlers (2 from high group, 1 from normal group)
        // The disabled low group should not contribute any handlers
        assert_eq!(all_ids.len(), 3);
        assert!(all_ids.contains(&&"handler1".to_string()));
        assert!(all_ids.contains(&&"handler2".to_string()));
        assert!(all_ids.contains(&&"handler4".to_string()));
        assert!(!all_ids.contains(&&"handler3".to_string())); // From disabled group
    }
    
    #[test]
    fn test_priority_chain_mutable_operations() {
        let mut chain = PriorityChain::new();
        
        let group1 = HandlerGroup::with_name(Priority::High, "group1");
        let group2 = HandlerGroup::with_name(Priority::Low, "group2");
        
        chain.add_group(group1);
        chain.add_group(group2);
        
        // Test mutable find operations
        {
            let group_mut = chain.find_group_by_priority_mut(Priority::High);
            assert!(group_mut.is_some());
            let group = group_mut.unwrap();
            group.add_handler("new_handler".to_string());
        }
        
        {
            let group_mut = chain.find_group_by_name_mut("group2");
            assert!(group_mut.is_some());
            let group = group_mut.unwrap();
            group.set_priority(Priority::Critical);
        }
        
        // Resort after modifying priorities
        chain.resort();
        
        // Verify the modified group is now first due to Critical priority
        let groups = chain.groups();
        assert_eq!(groups[0].name(), Some("group2"));
        assert_eq!(groups[0].priority(), Priority::Critical);
        
        // Verify the handler was added
        let high_group = chain.find_group_by_priority(Priority::High).unwrap();
        assert!(high_group.contains_handler("new_handler"));
    }
    
    #[test]
    fn test_priority_chain_clear() {
        let mut chain = PriorityChain::new();
        
        let group1 = HandlerGroup::new(Priority::High);
        let group2 = HandlerGroup::new(Priority::Low);
        
        chain.add_group(group1);
        chain.add_group(group2);
        assert_eq!(chain.group_count(), 2);
        
        chain.clear();
        assert!(chain.is_empty());
        assert_eq!(chain.group_count(), 0);
        assert_eq!(chain.total_handler_count(), 0);
        assert!(chain.highest_priority().is_none());
        assert!(chain.lowest_priority().is_none());
    }
}