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
}
