//! Backpressure policies for managing queue overload.
//!
//! CortexOS handles high-throughput event streams by providing configurable
//! backpressure policies. Each policy defines how the queue behaves when it
//! reaches capacity, allowing you to tune behavior based on your use case.
//!
//! # Policies
//!
//! - **DropNew**: Drop incoming events when queue is full (real-time priority)
//! - **DropOld**: Drop oldest events when queue is full (recency priority)
//! - **Coalesce**: Keep only latest event per key (stateful deduplication)
//! - **Sample**: Keep 1 out of every N events (downsampling)
//! - **Persist**: Spill to storage when full (durability priority)
//!
//! # Examples
//!
//! ```
//! use cortex_core::backpressure::{BackpressurePolicy, PolicyQueue, Keyed};
//!
//! #[derive(Clone)]
//! struct SensorReading {
//!     sensor_id: String,
//!     value: f32,
//! }
//!
//! impl Keyed for SensorReading {
//!     fn key(&self) -> Option<&str> {
//!         Some(&self.sensor_id)
//!     }
//! }
//!
//! // Drop new readings when queue is full
//! let queue: PolicyQueue<SensorReading> = PolicyQueue::new(
//!     BackpressurePolicy::DropNew,
//!     1000
//! );
//!
//! // Coalesce readings by sensor ID (keep latest per sensor)
//! let queue: PolicyQueue<SensorReading> = PolicyQueue::new(
//!     BackpressurePolicy::Coalesce("sensor_id".to_string()),
//!     1000
//! );
//! ```

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};

/// Backpressure policy for handling queue overload.
///
/// When a queue reaches capacity, the policy determines what happens:
/// - New items may be dropped
/// - Old items may be evicted
/// - Items may be coalesced by key
/// - Items may be sampled
/// - Items may be persisted to storage
#[derive(Clone, Debug)]
pub enum BackpressurePolicy {
    /// Drop new items when queue is full.
    ///
    /// Use for real-time systems where recent data in the queue is more
    /// important than new incoming data.
    DropNew,

    /// Drop oldest items when queue is full.
    ///
    /// Use for systems where recency matters more than completeness,
    /// such as real-time sensor feeds or live metrics.
    DropOld,

    /// Coalesce items by key, keeping only the latest value per key.
    ///
    /// Use for stateful updates where only the most recent value matters,
    /// such as sensor readings or configuration updates.
    ///
    /// The string parameter is the field name used for coalescing.
    Coalesce(String),

    /// Sample every Nth item, dropping the rest.
    ///
    /// Use for high-frequency event streams that can tolerate data loss,
    /// such as metrics or telemetry.
    ///
    /// The usize parameter is the sampling rate (e.g., 10 = keep 1 in 10).
    Sample(usize),

    /// Persist items to storage when queue is full.
    ///
    /// Use for critical events that must not be lost, such as financial
    /// transactions or audit logs.
    ///
    /// Note: Storage persistence is not yet fully implemented.
    Persist,
}

/// Trait for items that can be keyed for coalescing.
///
/// Implement this trait to enable `BackpressurePolicy::Coalesce` for your type.
pub trait Keyed {
    /// Returns the key for this item, used for coalescing.
    ///
    /// Return `None` if the item should not be coalesced.
    fn key(&self) -> Option<&str>;
}

/// A queue with configurable backpressure policy.
///
/// `PolicyQueue` manages a bounded queue with one of several backpressure
/// strategies. When the queue reaches capacity, the policy determines what
/// happens to new items.
pub struct PolicyQueue<T> {
    policy: BackpressurePolicy,
    capacity: usize,
    queue: Mutex<VecDeque<T>>,
    coalesce_map: Mutex<HashMap<String, usize>>,
    sample_counter: Mutex<usize>,
}

impl<T: Clone + Keyed> PolicyQueue<T> {
    /// Create a new queue with the specified policy and capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_core::backpressure::{BackpressurePolicy, PolicyQueue, Keyed};
    ///
    /// #[derive(Clone)]
    /// struct Item { value: i32 }
    /// impl Keyed for Item { fn key(&self) -> Option<&str> { None } }
    ///
    /// let queue: PolicyQueue<Item> = PolicyQueue::new(
    ///     BackpressurePolicy::DropNew,
    ///     1000
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

    /// Push an item onto the queue according to the backpressure policy.
    ///
    /// Returns `Ok(())` if the item was accepted, or `Err(item)` if it was
    /// rejected according to the policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_core::backpressure::{BackpressurePolicy, PolicyQueue, Keyed};
    ///
    /// #[derive(Clone)]
    /// struct Item { value: i32 }
    /// impl Keyed for Item {
    ///     fn key(&self) -> Option<&str> { None }
    /// }
    ///
    /// let queue = PolicyQueue::new(BackpressurePolicy::DropNew, 2);
    /// assert!(queue.push(Item { value: 1 }).is_ok());
    /// assert!(queue.push(Item { value: 2 }).is_ok());
    /// assert!(queue.push(Item { value: 3 }).is_err()); // Queue full
    /// ```
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

    /// Pop an item from the front of the queue.
    ///
    /// Returns `None` if the queue is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_core::backpressure::{BackpressurePolicy, PolicyQueue, Keyed};
    ///
    /// #[derive(Clone, Debug)]
    /// struct Item { value: i32 }
    /// impl Keyed for Item {
    ///     fn key(&self) -> Option<&str> { None }
    /// }
    ///
    /// let queue: PolicyQueue<Item> = PolicyQueue::new(BackpressurePolicy::DropOld, 10);
    /// queue.push(Item { value: 42 }).unwrap();
    /// let item = queue.pop().unwrap();
    /// assert_eq!(item.value, 42);
    /// ```
    pub fn pop(&self) -> Option<T> {
        self.queue.lock().pop_front()
    }

    /// Returns the current number of items in the queue.
    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    /// Returns `true` if the queue contains no items.
    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    /// Returns the maximum capacity of the queue.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestItem {
        key: Option<String>,
        value: i32,
    }

    impl Keyed for TestItem {
        fn key(&self) -> Option<&str> {
            self.key.as_deref()
        }
    }

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
    }

    #[test]
    fn test_coalesce_with_keys() {
        let queue: PolicyQueue<TestItem> = 
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 5);
        
        // Push items with the same key - should coalesce
        queue.push(TestItem {
            key: Some("k1".to_string()),
            value: 1,
        }).unwrap();
        
        queue.push(TestItem {
            key: Some("k1".to_string()),
            value: 2,
        }).unwrap();
        
        assert_eq!(queue.len(), 1);
        let item = queue.pop().unwrap();
        assert_eq!(item.value, 2); // Should have the latest value
    }

    #[test]
    fn test_coalesce_different_keys() {
        let queue: PolicyQueue<TestItem> = 
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 5);
        
        queue.push(TestItem {
            key: Some("k1".to_string()),
            value: 1,
        }).unwrap();
        
        queue.push(TestItem {
            key: Some("k2".to_string()),
            value: 2,
        }).unwrap();
        
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_persist_policy() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Persist, 2);
        
        queue.push(TestItem { key: None, value: 1 }).unwrap();
        queue.push(TestItem { key: None, value: 2 }).unwrap();
        
        // Third item should fail since persist is not fully implemented
        let result = queue.push(TestItem { key: None, value: 3 });
        assert!(result.is_err());
    }

    #[test]
    fn test_queue_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 10);
        assert_eq!(queue.capacity(), 10);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_drop_old_maintains_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 3);
        
        for i in 0..10 {
            queue.push(TestItem { key: None, value: i }).unwrap();
        }
        
        assert!(queue.len() <= 3);
        
        // Should have the most recent values
        let item = queue.pop().unwrap();
        assert!(item.value >= 7);
    }

    #[test]
    fn test_sample_exact_multiples() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Sample(2), 10);
        
        // Push exactly 6 items with sample rate 2
        for i in 0..6 {
            queue.push(TestItem { key: None, value: i }).unwrap();
        }
        
        // Should have 3 items (every 2nd item)
        assert_eq!(queue.len(), 3);
    }
}
