use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub enum BackpressurePolicy {
    DropNew,
    DropOld,
    Coalesce(String),
    Sample(usize),
    Persist,
}

pub trait Keyed {
    fn key(&self) -> Option<&str>;
}

pub struct PolicyQueue<T> {
    policy: BackpressurePolicy,
    capacity: usize,
    queue: Mutex<VecDeque<T>>,
    coalesce_map: Mutex<HashMap<String, usize>>,
    sample_counter: Mutex<usize>,
}

impl<T: Clone + Keyed> PolicyQueue<T> {
    pub fn new(policy: BackpressurePolicy, capacity: usize) -> Self {
        Self {
            policy,
            capacity,
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            coalesce_map: Mutex::new(HashMap::new()),
            sample_counter: Mutex::new(0),
        }
    }

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

    pub fn pop(&self) -> Option<T> {
        self.queue.lock().pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Edge cases documented for backpressure policies:
    ///
    /// DropNew:
    /// - When queue is full, new items are rejected and returned as Err
    /// - Empty queue returns None on pop
    /// - Works correctly with single-capacity queues
    ///
    /// DropOld:
    /// - When queue is full, oldest items are removed to make room
    /// - Maintains FIFO order for remaining items
    /// - Empty queue returns None on pop
    /// - Works correctly with single-capacity queues
    ///
    /// Coalesce(key):
    /// - Items with the same key overwrite previous items with that key
    /// - Items with different keys are kept separately
    /// - When capacity is reached, oldest item is dropped (oldest key)
    /// - Items without keys are treated as separate items (not coalesced)
    /// - Works with mixed keys and no-key items
    ///
    /// Sample(n):
    /// - Keeps 1 of every n items (counter-based sampling)
    /// - With n=1, all items are kept
    /// - With n > number of pushes, no items are kept
    /// - Counter wraps on every nth item
    /// - Respects capacity limits (drops oldest when full)
    ///
    /// Persist:
    /// - Stores items in memory until capacity is reached
    /// - Returns error when full (storage spillover not yet implemented)
    /// - Future: should spill to disk when memory is full

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

    // Comprehensive Coalesce tests
    #[test]
    fn test_coalesce_basic() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 5);
        queue
            .push(TestItem {
                key: Some("a".to_string()),
                value: 1,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: Some("b".to_string()),
                value: 2,
            })
            .unwrap();
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_coalesce_same_key_overwrites() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 5);
        queue
            .push(TestItem {
                key: Some("a".to_string()),
                value: 1,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: Some("a".to_string()),
                value: 10,
            })
            .unwrap();
        // Should still have 1 item since same key
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop().unwrap().value, 10);
    }

    #[test]
    fn test_coalesce_different_keys() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 5);
        queue
            .push(TestItem {
                key: Some("a".to_string()),
                value: 1,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: Some("b".to_string()),
                value: 2,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: Some("c".to_string()),
                value: 3,
            })
            .unwrap();
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn test_coalesce_at_capacity() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 2);
        queue
            .push(TestItem {
                key: Some("a".to_string()),
                value: 1,
            })
            .unwrap();
        queue
            .push(TestItem {
                key: Some("b".to_string()),
                value: 2,
            })
            .unwrap();
        // Queue is full, adding new key should drop oldest
        queue
            .push(TestItem {
                key: Some("c".to_string()),
                value: 3,
            })
            .unwrap();
        assert_eq!(queue.len(), 2);
        // First item (a=1) should be dropped
        let first = queue.pop().unwrap();
        assert_eq!(first.value, 2); // b=2
    }

    #[test]
    fn test_coalesce_without_keys() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 3);
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
        // Items without keys should still be added
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_coalesce_mixed_keys_and_no_keys() {
        let queue: PolicyQueue<TestItem> =
            PolicyQueue::new(BackpressurePolicy::Coalesce("key".to_string()), 5);
        queue
            .push(TestItem {
                key: Some("a".to_string()),
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
                key: Some("a".to_string()),
                value: 10,
            })
            .unwrap();
        // Should have 2 items: one with key "a" (updated to 10) and one without key
        assert_eq!(queue.len(), 2);
    }

    // Comprehensive Persist tests
    #[test]
    fn test_persist_basic() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Persist, 3);
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
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_persist_at_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Persist, 2);
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
        // Should fail when full (storage not implemented)
        let result = queue.push(TestItem {
            key: None,
            value: 3,
        });
        assert!(result.is_err());
        assert_eq!(queue.len(), 2);
    }

    // Edge case tests for DropNew
    #[test]
    fn test_drop_new_empty_queue() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 2);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_drop_new_single_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 1);
        queue
            .push(TestItem {
                key: None,
                value: 1,
            })
            .unwrap();
        let result = queue.push(TestItem {
            key: None,
            value: 2,
        });
        assert!(result.is_err());
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop().unwrap().value, 1);
    }

    #[test]
    fn test_drop_new_returns_rejected_item() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 1);
        queue
            .push(TestItem {
                key: None,
                value: 1,
            })
            .unwrap();
        let result = queue.push(TestItem {
            key: None,
            value: 2,
        });
        assert_eq!(result.unwrap_err().value, 2);
    }

    // Edge case tests for DropOld
    #[test]
    fn test_drop_old_empty_queue() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 2);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_drop_old_single_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 1);
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
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop().unwrap().value, 2);
    }

    #[test]
    fn test_drop_old_order_preservation() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 3);
        for i in 1..=5 {
            queue
                .push(TestItem {
                    key: None,
                    value: i,
                })
                .unwrap();
        }
        assert_eq!(queue.len(), 3);
        // Should have items 3, 4, 5
        assert_eq!(queue.pop().unwrap().value, 3);
        assert_eq!(queue.pop().unwrap().value, 4);
        assert_eq!(queue.pop().unwrap().value, 5);
    }

    // Edge case tests for Sample
    #[test]
    fn test_sample_n_equals_1() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Sample(1), 10);
        for i in 0..5 {
            queue
                .push(TestItem {
                    key: None,
                    value: i,
                })
                .unwrap();
        }
        // With n=1, every item should be kept
        assert_eq!(queue.len(), 5);
    }

    #[test]
    fn test_sample_n_greater_than_pushes() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Sample(10), 10);
        for i in 0..5 {
            queue
                .push(TestItem {
                    key: None,
                    value: i,
                })
                .unwrap();
        }
        // With n=10 and only 5 pushes, no items should be kept
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_sample_exact_ratio() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Sample(5), 10);
        for i in 0..20 {
            queue
                .push(TestItem {
                    key: None,
                    value: i,
                })
                .unwrap();
        }
        // With n=5 and 20 pushes, should have 4 items (every 5th: 4, 9, 14, 19)
        assert_eq!(queue.len(), 4);
    }

    #[test]
    fn test_sample_at_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::Sample(2), 2);
        for i in 0..10 {
            queue
                .push(TestItem {
                    key: None,
                    value: i,
                })
                .unwrap();
        }
        // Should sample every 2nd item and respect capacity of 2
        assert_eq!(queue.len(), 2);
    }

    // General PolicyQueue tests
    #[test]
    fn test_queue_capacity() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 5);
        assert_eq!(queue.capacity(), 5);
    }

    #[test]
    fn test_pop_from_empty() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 5);
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_is_empty_and_len() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropNew, 5);
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);

        queue
            .push(TestItem {
                key: None,
                value: 1,
            })
            .unwrap();
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);

        queue.pop();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_multiple_pops() {
        let queue: PolicyQueue<TestItem> = PolicyQueue::new(BackpressurePolicy::DropOld, 10);
        for i in 0..5 {
            queue
                .push(TestItem {
                    key: None,
                    value: i,
                })
                .unwrap();
        }
        for i in 0..5 {
            assert_eq!(queue.pop().unwrap().value, i);
        }
        assert!(queue.is_empty());
    }
}
