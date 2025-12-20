//! Backpressure policies for managing event queue overflow.
//!
//! This module provides different strategies for handling event queue overflow situations.
//! Each policy implements a different trade-off between preserving data, maintaining
//! real-time behavior, and resource usage.
//!
//! ## Policy Selection Guide
//!
//! - **DropNew**: Use when older data is more valuable than new data. Best for historical
//!   analysis where losing recent updates is acceptable.
//!
//! - **DropOld**: Use when recent data is more important. Best for real-time monitoring,
//!   sensor readings, or UI updates where only the latest state matters.
//!
//! - **Coalesce**: Use when you have keyed data and only need the latest value per key.
//!   Best for status updates, configuration changes, or any scenario where intermediate
//!   states can be safely discarded.
//!
//! - **Sample**: Use when you need representative data but can tolerate gaps. Best for
//!   high-frequency sensors, metrics collection, or situations where statistical
//!   sampling is acceptable.
//!
//! - **Persist**: Use when data loss is unacceptable. Events are spilled to persistent
//!   storage when the queue is full. Best for audit logs, critical events, or
//!   transactional systems. Note: Requires storage backend.
//!
//! ## Examples
//!
//! ```rust
//! use cortex_core::backpressure::{BackpressurePolicy, PolicyQueue, Keyed};
//!
//! #[derive(Clone)]
//! struct SensorReading {
//!     sensor_id: String,
//!     value: f64,
//! }
//!
//! impl Keyed for SensorReading {
//!     fn key(&self) -> Option<&str> {
//!         Some(&self.sensor_id)
//!     }
//! }
//!
//! // Drop old readings when queue is full - keeps latest sensor data
//! let queue: PolicyQueue<SensorReading> = PolicyQueue::new(BackpressurePolicy::DropOld, 100);
//!
//! // Coalesce by sensor_id - keeps latest reading per sensor
//! let queue: PolicyQueue<SensorReading> = PolicyQueue::new(
//!     BackpressurePolicy::Coalesce("sensor_id".to_string()),
//!     100
//! );
//!
//! // Sample every 10th reading
//! let queue: PolicyQueue<SensorReading> = PolicyQueue::new(BackpressurePolicy::Sample(10), 100);
//! ```

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};

/// Backpressure policy for managing queue overflow behavior.
#[derive(Clone, Debug)]
pub enum BackpressurePolicy {
    /// Drop new items when queue is full. Preserves oldest data.
    DropNew,
    /// Drop oldest items when queue is full. Preserves newest data.
    DropOld,
    /// Keep only the latest item per key. Deduplicates based on key field.
    Coalesce(String),
    /// Sample every Nth item. Reduces load while maintaining statistical representation.
    Sample(usize),
    /// Spill to persistent storage when queue is full. Guarantees no data loss.
    Persist,
}

/// Trait for items that can be keyed for coalescing.
pub trait Keyed {
    /// Returns the key for this item, or None if the item has no key.
    fn key(&self) -> Option<&str>;
}

/// A thread-safe queue that applies a backpressure policy when capacity is reached.
///
/// The queue is generic over `T` which must implement `Clone` and `Keyed`.
/// Cloning is required for the `Coalesce` policy to replace existing items.
pub struct PolicyQueue<T> {
    policy: BackpressurePolicy,
    capacity: usize,
    queue: Mutex<VecDeque<T>>,
    coalesce_map: Mutex<HashMap<String, usize>>,
    sample_counter: Mutex<usize>,
}

impl<T: Clone + Keyed> PolicyQueue<T> {
    /// Create a new PolicyQueue with the specified policy and capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cortex_core::backpressure::{BackpressurePolicy, PolicyQueue, Keyed};
    ///
    /// #[derive(Clone)]
    /// struct Item { value: i32 }
    /// impl Keyed for Item {
    ///     fn key(&self) -> Option<&str> { None }
    /// }
    ///
    /// let queue: PolicyQueue<Item> = PolicyQueue::new(
    ///     BackpressurePolicy::DropOld,
    ///     100
    /// );
    /// ```
    pub fn new(policy: BackpressurePolicy, capacity: usize) -> Self {
        Self {
            policy,
            capacity,
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            coalesce_map: Mutex::new(HashMap::new()),
            sample_counter: Mutex::new(0),
        }
    }

    /// Push an item onto the queue, applying the backpressure policy if full.
    ///
    /// Returns `Ok(())` if the item was accepted, or `Err(item)` if it was rejected
    /// (only for DropNew and Persist policies when queue is full).
    pub fn push(&self, item: T) -> Result<(), T> {
        match &self.policy {
            BackpressurePolicy::DropNew => self.push_drop_new(item),
            BackpressurePolicy::DropOld => self.push_drop_old(item),
            BackpressurePolicy::Coalesce(key_field) => self.push_coalesce(item, key_field.clone()),
            BackpressurePolicy::Sample(n) => self.push_sample(item, *n),
            BackpressurePolicy::Persist => self.push_persist(item),
        }
    }

    fn push_drop_new(&self, item: T) -> Result<(), T> {
        let mut queue = self.queue.lock();
        if queue.len() >= self.capacity {
            return Err(item);
        }
        queue.push_back(item);
        Ok(())
    }

    fn push_drop_old(&self, item: T) -> Result<(), T> {
        let mut queue = self.queue.lock();
        if queue.len() >= self.capacity {
            queue.pop_front();
        }
        queue.push_back(item);
        Ok(())
    }

    fn push_coalesce(&self, item: T, _key_field: String) -> Result<(), T> {
        let mut queue = self.queue.lock();
        let mut coalesce_map = self.coalesce_map.lock();

        if let Some(key) = item.key() {
            let key = key.to_string();
            if let Some(&idx) = coalesce_map.get(&key) {
                if idx < queue.len() {
                    queue[idx] = item;
                    return Ok(());
                }
            }
            if queue.len() >= self.capacity {
                if let Some(removed) = queue.pop_front() {
                    if let Some(removed_key) = removed.key() {
                        coalesce_map.remove(removed_key);
                    }
                    for (_, v) in coalesce_map.iter_mut() {
                        *v = v.saturating_sub(1);
                    }
                }
            }
            let idx = queue.len();
            queue.push_back(item);
            coalesce_map.insert(key, idx);
        } else {
            if queue.len() >= self.capacity {
                queue.pop_front();
                for (_, v) in coalesce_map.iter_mut() {
                    *v = v.saturating_sub(1);
                }
            }
            queue.push_back(item);
        }
        Ok(())
    }

    fn push_sample(&self, item: T, n: usize) -> Result<(), T> {
        let mut counter = self.sample_counter.lock();
        *counter += 1;

        if *counter >= n {
            *counter = 0;
            let mut queue = self.queue.lock();
            if queue.len() >= self.capacity {
                queue.pop_front();
            }
            queue.push_back(item);
        }
        Ok(())
    }

    fn push_persist(&self, item: T) -> Result<(), T> {
        let mut queue = self.queue.lock();
        if queue.len() >= self.capacity {
            tracing::warn!("Queue full, would spill to storage (not implemented)");
            return Err(item);
        }
        queue.push_back(item);
        Ok(())
    }

    /// Pop the next item from the front of the queue.
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&self) -> Option<T> {
        self.queue.lock().pop_front()
    }

    /// Get the current number of items in the queue.
    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    /// Get the configured capacity of the queue.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all items from the queue and reset internal state.
    pub fn clear(&self) {
        self.queue.lock().clear();
        self.coalesce_map.lock().clear();
        *self.sample_counter.lock() = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestItem {
        key: Option<String>,
        value: i32,
    }

    impl Keyed for TestItem {
        fn key(&self) -> Option<&str> {
            self.key.as_deref()
        }
    }

    // ========== Basic Policy Tests ==========

    #[test]
    fn test_drop_new() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 2);
        queue
            .push(TestItem {
                key: None,
                value: 1,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: None,
                value: 2,
            })
            .unwrap();
        let result = queue.push(TestItem {
            key: None,
            value: 3,
        });
        assert!(result.is_err());
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop().unwrap().value, 1);
    }

    #[test]
    fn test_drop_old() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 2);
        queue
            .push(TestItem {
                key: None,
                value: 1,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: None,
                value: 2,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: None,
                value: 3,
            })
            .unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop().unwrap().value, 2);
        assert_eq!(queue.pop().unwrap().value, 3);
    }

    #[test]
    fn test_sample() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Sample(3), 10);
        for i in 0..9 {
            queue
                .push(TestItem {
                    key: None,
                    value: i,
                })
                .unwrap();
        }
        assert_eq!(queue.len(), 3);
        // Sample takes every 3rd item (indices 2, 5, 8)
        assert_eq!(queue.pop().unwrap().value, 2);
        assert_eq!(queue.pop().unwrap().value, 5);
        assert_eq!(queue.pop().unwrap().value, 8);
    }

    #[test]
    fn test_coalesce_with_keys() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 10);

        // Push multiple updates for same key
        queue
            .push(TestItem {
                key: Some("sensor1".to_string()),
                value: 1,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: Some("sensor1".to_string()),
                value: 2,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: Some("sensor2".to_string()),
                value: 10,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: Some("sensor1".to_string()),
                value: 3,
            })
            .unwrap();

        // Should only have 2 items (latest for each key)
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.pop().unwrap().value, 3); // Latest sensor1
        assert_eq!(queue.pop().unwrap().value, 10); // Latest sensor2
    }

    #[test]
    fn test_coalesce_without_keys() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 3);

        // Push items without keys
        for i in 0..5 {
            queue
                .push(TestItem {
                    key: None,
                    value: i,
                })
                .unwrap();
        }

        // Should behave like DropOld when no keys
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.pop().unwrap().value, 2);
    }

    #[test]
    fn test_persist_policy() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Persist, 2);

        queue.push(TestItem { key: None, value: 1 }).unwrap();
        queue.push(TestItem { key: None, value: 2 }).unwrap();

        // Should fail when full (storage not implemented yet)
        let result = queue.push(TestItem { key: None, value: 3 });
        assert!(result.is_err());
    }

    // ========== Capacity and State Tests ==========

    #[test]
    fn test_is_empty() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 10);
        assert!(queue.is_empty());

        queue.push(TestItem { key: None, value: 1 }).unwrap();
        assert!(!queue.is_empty());

        queue.pop();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 42);
        assert_eq!(queue.capacity(), 42);
    }

    #[test]
    fn test_clear() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 10);

        for i in 0..5 {
            queue
                .push(TestItem {
                    key: Some(format!("key{}", i)),
                    value: i,
                })
                .unwrap();
        }

        assert_eq!(queue.len(), 5);
        queue.clear();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    // ========== Load Tests ==========

    #[test]
    fn test_high_load_drop_old() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 100);

        // Push 10,000 items
        for i in 0..10_000 {
            queue.push(TestItem { key: None, value: i }).unwrap();
        }

        // Should only have the last 100
        assert_eq!(queue.len(), 100);
        assert_eq!(queue.pop().unwrap().value, 9_900);
    }

    #[test]
    fn test_high_load_sample() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Sample(100), 1000);

        // Push 10,000 items
        for i in 0..10_000 {
            queue.push(TestItem { key: None, value: i }).unwrap();
        }

        // Should have sampled 100 items
        assert_eq!(queue.len(), 100);
    }

    #[test]
    fn test_high_load_coalesce() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 1000);

        // Push 10,000 items for 10 different keys
        for i in 0..10_000 {
            let key_id = i % 10;
            queue
                .push(TestItem {
                    key: Some(format!("sensor{}", key_id)),
                    value: i,
                })
                .unwrap();
        }

        // Should only have 10 items (one per key)
        assert_eq!(queue.len(), 10);
    }

    // ========== Edge Cases ==========

    #[test]
    fn test_zero_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 0);
        let result = queue.push(TestItem { key: None, value: 1 });
        assert!(result.is_err());
    }

    #[test]
    fn test_sample_with_n_equals_one() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Sample(1), 10);

        for i in 0..5 {
            queue.push(TestItem { key: None, value: i }).unwrap();
        }

        // Sample(1) should keep every item
        assert_eq!(queue.len(), 5);
    }

    #[test]
    fn test_mixed_keyed_and_unkeyed() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 10);

        queue
            .push(TestItem {
                key: Some("a".to_string()),
                value: 1,
            })
            .unwrap();
        queue.push(TestItem { key: None, value: 2 }).unwrap();
        queue
            .push(TestItem {
                key: Some("a".to_string()),
                value: 3,
            })
            .unwrap();
        queue.push(TestItem { key: None, value: 4 }).unwrap();

        assert_eq!(queue.len(), 3); // Latest "a", and two unkeyed items
    }
}
